//! Kitchen timer — a phased countdown driven entirely by chained timeouts.
//!
//! Where the stopwatch example counts *up* on a named interval
//! (`Command::Every`), this one counts *down* through phases on a chain of
//! one-shot `Command::Timeout`s: when a phase hits zero the update flips to
//! the next phase and the tick chain keeps going — effects choreographing
//! state transitions, with no code outside `update` deciding what happens
//! next.

use wasm_bindgen::prelude::*;

use elmore::{App, Command, Html};

/// Short durations so the transitions are easy to watch.
const FOCUS_SECS: u32 = 10;
const BREAK_SECS: u32 = 5;
const TICK_MS: u32 = 1000;

#[derive(Clone)]
enum Msg {
    /// One second elapsed (fired by `Command::Timeout` while running).
    Tick,
    Start,
    Pause,
    Reset,
    /// Skip straight to the next phase.
    Skip,
}

#[derive(Default)]
struct Model {
    phase: Phase,
    /// Seconds remaining in the current phase.
    secs_left: u32,
    running: bool,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum Phase {
    #[default]
    Focus,
    Break,
}

impl Phase {
    fn duration(self) -> u32 {
        match self {
            Phase::Focus => FOCUS_SECS,
            Phase::Break => BREAK_SECS,
        }
    }

    fn next(self) -> Self {
        match self {
            Phase::Focus => Phase::Break,
            Phase::Break => Phase::Focus,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Phase::Focus => "Focus",
            Phase::Break => "Break",
        }
    }
}

#[derive(Default)]
struct Timer;

impl App for Timer {
    type Message = Msg;
    type Model = Model;

    fn update(&mut self, msg: Msg, model: &mut Model) -> Option<Command<Msg>> {
        match msg {
            Msg::Tick if model.running => {
                model.secs_left = model.secs_left.saturating_sub(1);
                if model.secs_left == 0 {
                    // Phase over: flip and roll straight into the next one.
                    model.phase = model.phase.next();
                    model.secs_left = model.phase.duration();
                }
                self.schedule(model)
            }
            // A trailing tick after Pause or Reset is discarded.
            Msg::Tick => Command::none(),

            Msg::Start if !model.running => {
                if model.secs_left == 0 {
                    model.secs_left = model.phase.duration();
                }
                model.running = true;
                self.schedule(model)
            }
            Msg::Start => Command::none(),
            Msg::Pause => {
                model.running = false;
                Command::none()
            }
            Msg::Reset => {
                *model = Model::default();
                Command::none()
            }
            Msg::Skip => {
                model.phase = model.phase.next();
                model.secs_left = model.phase.duration();
                if model.running {
                    self.schedule(model)
                } else {
                    Command::none()
                }
            }
        }
    }

    fn view(&self, model: &Model) -> Html<Msg> {
        let elapsed = model.phase.duration() - model.secs_left;
        let pct = 100 * elapsed / model.phase.duration();

        let start_stop = if model.running {
            Html::button().text("Pause").on_click(|| Msg::Pause)
        } else {
            Html::button().text("Start").on_click(|| Msg::Start)
        };

        Html::div()
            .class("timer")
            .children([
                Html::h1().text("Kitchen timer"),
                Html::p().class("phase").text(model.phase.label()),
                Html::div().class("time").text(clock(model.secs_left)),
                Html::div()
                    .class("bar")
                    .child(Html::div().attr("style", format!("width: {pct}%"))),
                Html::div()
                    .class("controls")
                    .children([
                        start_stop,
                        Html::button().text("Skip").on_click(|| Msg::Skip),
                        Html::button().text("Reset").on_click(|| Msg::Reset),
                    ]),
            ])
    }
}

impl Timer {
    /// Keep the tick chain alive while the timer runs.
    fn schedule(&self, model: &Model) -> Option<Command<Msg>> {
        if model.running {
            Some(Command::Timeout { millis: TICK_MS, msg: Msg::Tick })
        } else {
            Command::none()
        }
    }
}

/// `m:ss`, the only correct way to show a countdown.
fn clock(secs: u32) -> String {
    format!("{}:{:02}", secs / 60, secs % 60)
}

#[wasm_bindgen(start)]
pub fn main() {
    elmore::run::<Timer>();
}
