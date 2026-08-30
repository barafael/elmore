//! The runtime: mounts the app, owns its state, and drives the
//! update/render cycle.
//!
//! Design notes:
//!
//! - One [`Runtime`] per app, `Box::leak`ed for the page lifetime. Every
//!   callback (frame, event routing, timers, fetches) captures a plain
//!   `&'static` copy of it, so nothing needs `Rc` to outlive its scope.
//! - wasm is single-threaded, so interior mutability is plain
//!   `RefCell`/`Cell` fields — no atomics, no locks.
//! - Frames run on demand: pushing a message schedules one animation frame
//!   (guarded by a flag). An idle app costs nothing — no spinning loop.
//! - The only leaked allocations are six closures created once at boot
//!   (five event-delegation listeners and one frame callback). Everything
//!   else — handler rows, one-shot timers, fetches — is dropped when its
//!   render or task ends; named intervals run until cancelled.

#[cfg(not(target_arch = "wasm32"))]
use crate::App;

#[cfg(not(target_arch = "wasm32"))]
pub fn run<A: App>() {
    let _ = core::marker::PhantomData::<A>;
}

#[cfg(target_arch = "wasm32")]
pub use imp::run;

#[cfg(target_arch = "wasm32")]
pub(crate) use imp::Runtime;

#[cfg(target_arch = "wasm32")]
mod imp {
    use std::cell::{Cell, RefCell};
    use std::collections::{HashMap, VecDeque};

    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;
    use web_sys::{Element, Response, Window, window};

    use crate::App;
    use crate::command::Command;
    use crate::dom;
    use crate::html::Html;

    /// Everything the runtime owns for one app.
    pub(crate) struct Runtime<A: App> {
        app: RefCell<A>,
        model: RefCell<A::Model>,
        /// Messages waiting for the next frame.
        pub(crate) sink: RefCell<VecDeque<A::Message>>,
        /// The tree rendered last render; the renderer diffs removed
        /// attributes against it.
        pub(crate) prev: RefCell<Option<Html<A::Message>>>,
        /// Handlers armed by the last render, indexed by the expando id each
        /// live element carries (see `dom::HANDLER_PROP`).
        pub(crate) handlers: RefCell<Vec<dom::HandlerRow>>,
        /// Named intervals currently active (see
        /// `Command::Every`/`Command::Cancel`), mapped to the generation each
        /// interval task was spawned under. Membership keeps a task pulsing
        /// and makes re-subscribing an active id a no-op; a generation
        /// mismatch tells a stale task its subscription was cancelled —
        /// possibly and immediately replaced — so it stops.
        subscriptions: RefCell<HashMap<&'static str, u64>>,
        /// The single, permanent frame callback.
        frame: RefCell<Option<Closure<dyn FnMut()>>>,
        /// True while a frame has been requested but has not run yet.
        scheduled: Cell<bool>,
        pub(crate) mount: Element,
        pub(crate) window: Window,
    }

    impl<A: App> Runtime<A> {
        pub(crate) fn new(mount: Element, window: Window) -> Self {
            Runtime {
                app: RefCell::new(A::default()),
                model: RefCell::new(A::Model::default()),
                sink: RefCell::new(VecDeque::new()),
                prev: RefCell::new(None),
                handlers: RefCell::new(Vec::new()),
                subscriptions: RefCell::new(HashMap::new()),
                frame: RefCell::new(None),
                scheduled: Cell::new(false),
                mount,
                window,
            }
        }

        /// Enqueue a message and make sure a frame runs soon.
        pub(crate) fn push(&'static self, msg: A::Message) {
            self.sink.borrow_mut().push_back(msg);
            self.schedule();
        }

        /// Create and store the permanent frame callback. `run` calls this
        /// at boot; tests call it so their bare runtimes can schedule too.
        pub(crate) fn install_frame_callback(&'static self) {
            let cb = Closure::wrap(Box::new(move || self.frame()) as Box<dyn FnMut()>);
            *self.frame.borrow_mut() = Some(cb);
        }

        /// Request one animation frame, unless one is already pending.
        fn schedule(&'static self) {
            if self.scheduled.replace(true) {
                return;
            }
            let frame = self.frame.borrow();
            let cb = frame.as_ref().expect("frame callback installed at boot");
            let _ = self.window.request_animation_frame(cb.as_ref().unchecked_ref());
        }

        /// One frame of work: drain the sink, run `update` per message,
        /// dispatch returned commands, re-render if anything happened.
        pub(crate) fn frame(&'static self) {
            self.scheduled.set(false);
            let pending: Vec<_> = self.sink.borrow_mut().drain(..).collect();
            if pending.is_empty() {
                return;
            }
            let mut commands = Vec::new();
            for msg in pending {
                if let Some(cmd) = self.app.borrow_mut().update(msg, &mut self.model.borrow_mut()) {
                    commands.push(cmd);
                }
            }
            for cmd in commands {
                self.dispatch(cmd);
            }
            let view = self.app.borrow().view(&self.model.borrow());
            dom::render_into(self, view);
        }

        /// Run a command's side effect; its result comes back via `push`.
        fn dispatch(&'static self, command: Command<A::Message>) {
            match command {
                Command::Timeout { millis, msg } => {
                    wasm_bindgen_futures::spawn_local(async move {
                        gloo_timers::future::TimeoutFuture::new(millis).await;
                        self.push(msg);
                    });
                }
                Command::Every { id, millis, msg } => {
                    // Subscribing an already-active id is a no-op: the first
                    // subscriber owns the pulse, so re-issuing `Every` (e.g. a
                    // repeated Start) can't double the tick rate.
                    let mut subs = self.subscriptions.borrow_mut();
                    if subs.contains_key(&id) {
                        return;
                    }
                    // Tag this task with a fresh generation. If the same id
                    // is cancelled and subscribed again while an old task is
                    // still mid-sleep, the stale task wakes to a *different*
                    // generation and stops — instead of double-ticking.
                    let generation = subs.values().copied().max().unwrap_or(0) + 1;
                    subs.insert(id, generation);
                    drop(subs);
                    wasm_bindgen_futures::spawn_local(async move {
                        loop {
                            gloo_timers::future::TimeoutFuture::new(millis).await;
                            // Stop once cancelled or replaced (our generation
                            // is gone), no matter where that lands in the
                            // sleep/tick cycle.
                            if self.subscriptions.borrow().get(&id) != Some(&generation) {
                                break;
                            }
                            self.push(msg());
                        }
                    });
                }
                Command::Cancel { id } => {
                    // Removing the id (and its generation) tells the interval
                    // task to stop; no-op when nothing holds it.
                    self.subscriptions.borrow_mut().remove(&id);
                }
                Command::Batch(cmds) => {
                    // A batch is just several effects at once; dispatch each.
                    for cmd in cmds {
                        self.dispatch(cmd);
                    }
                }
                Command::FetchText { url, on_result } => {
                    wasm_bindgen_futures::spawn_local(async move {
                        let result = fetch_text(&url).await;
                        self.push(on_result(result));
                    });
                }
            }
        }
    }

    /// Boot the app: mount its first `view` into `#root`, then run on demand.
    ///
    /// The cycle, once per frame of work:
    ///
    /// 1. A message arrives (from an event or an effect) and schedules a frame.
    /// 2. `App::update` mutates the model and may return a [`Command`].
    /// 3. The command is dispatched (a timeout or a fetch).
    /// 4. `App::view` produces a fresh tree, which the renderer reconciles
    ///    into the live DOM in place — elements keep their identity.
    ///
    /// On native targets there is no web host, so this is a no-op.
    pub fn run<A: App>() {
        std::panic::set_hook(Box::new(console_error_panic_hook::hook));

        let window = window().expect("no window; are we in a web context?");
        let mount = dom::root_element("root");

        let state: &'static Runtime<A> = Box::leak(Box::new(Runtime::new(mount, window)));

        // Five delegation listeners, permanent for the page lifetime.
        dom::install_event_routing(state);

        // The one permanent frame callback. It captures only `state` (a
        // `&'static` copy), so it never needs replacing or re-arming.
        state.install_frame_callback();

        // First render.
        let view = state.app.borrow().view(&state.model.borrow());
        dom::render_into(state, view);
    }

    /// Fetch `url` and return its response body as text.
    async fn fetch_text(url: &str) -> Result<String, String> {
        let window = window().ok_or_else(|| "no window".to_string())?;
        let promise = window.fetch_with_str(url);
        let resp = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|e| format!("response error: {e:?}"))?;
        let resp: Response = JsCast::dyn_into(resp).map_err(|_| "not a response".to_string())?;
        if !resp.ok() {
            return Err(format!("HTTP {}", resp.status()));
        }
        let text_promise = resp.text().map_err(|e| format!("text error: {e:?}"))?;
        let text = wasm_bindgen_futures::JsFuture::from(text_promise)
            .await
            .map_err(|e| format!("text await error: {e:?}"))?;
        text.as_string().ok_or_else(|| "non-text response".to_string())
    }
}
