//! Effects/commands produced by [`App::update`], dispatched by the runtime.

/// A side-effect that an update can request. Return one of these (or `None`)
/// from [`App::update`] to run work outside the pure update/render loop.
///
/// The set is deliberately tiny: everything must be expressible with the
/// primitives available in a browser tab. No filesystem, no network beyond
/// text fetch, no parallelism machinery.
pub enum Command<Msg> {
    /// After `millis`, enqueue `msg` back into the update loop.
    Timeout {
        millis: u32,
        msg: Msg,
    },

    /// Fire a message `every` `millis`, keyed by `id`, until a matching
    /// [`Command::Cancel`].
    ///
    /// The message is built fresh by a closure each tick (a named interval
    /// fires repeatedly, so it can't hand over a single owned `Msg`). The
    /// runtime owns the pulse: subscribing an `id` that is already active is
    /// a **no-op**, so re-issuing `Every` (say, on a repeated Start) never
    /// doubles the tick rate — even if a [`Command::Cancel`] raced a
    /// re-subscribe while an old tick was still in flight. Cancelling and
    /// re-subscribing starts a fresh interval (the tick phase resets), and
    /// cancelling is itself a no-op if no interval holds that `id`.
    Every {
        id: &'static str,
        millis: u32,
        msg: Box<dyn Fn() -> Msg>,
    },

    /// Stop the named interval started by [`Command::Every`]; no-op if none
    /// is active.
    Cancel {
        id: &'static str,
    },

    /// Fetch the text at `url`; when it resolves, produce a message via
    /// `on_result`. Errors are surfaced as `Err(String)`.
    FetchText {
        url: String,
        on_result: Box<dyn Fn(Result<String, String>) -> Msg>,
    },

    /// Several effects out of one update, run concurrently. The runtime
    /// flattens this; an empty batch (see [`Command::batch`]) does nothing.
    Batch(Vec<Command<Msg>>),
}

impl<Msg> Command<Msg> {
    /// The most common command of all: no effect at all.
    pub fn none() -> Option<Command<Msg>> {
        None
    }

    /// Bundle several effects into the single return value `update` allows.
    /// An empty batch collapses to [`Command::none`].
    pub fn batch(cmds: impl IntoIterator<Item = Command<Msg>>) -> Option<Command<Msg>> {
        let cmds: Vec<_> = cmds.into_iter().collect();
        (!cmds.is_empty()).then_some(Command::Batch(cmds))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_is_none() {
        assert!(Command::<()>::none().is_none());
    }

    #[test]
    fn timeout_carries_its_payload() {
        let cmd = Command::Timeout { millis: 5, msg: 7u32 };
        assert!(matches!(cmd, Command::Timeout { millis: 5, msg: 7 }));
    }

    #[test]
    fn every_carries_a_name_and_a_message_factory() {
        let cmd = Command::<u32>::Every { id: "tick", millis: 9, msg: Box::new(|| 1) };
        match cmd {
            Command::Every { id, millis, msg } => {
                assert_eq!(id, "tick");
                assert_eq!(millis, 9);
                assert_eq!(msg(), 1, "message factory produces a fresh message each call");
            }
            _ => panic!("expected Every"),
        }
    }

    #[test]
    fn cancel_carries_its_name() {
        let cmd = Command::<u32>::Cancel { id: "tick" };
        assert!(matches!(cmd, Command::Cancel { id: "tick" }));
    }

    #[test]
    fn batch_groups_its_commands() {
        let cmd = Command::batch([
            Command::Timeout { millis: 5, msg: 1u32 },
            Command::Cancel { id: "tick" },
        ]);
        let Some(Command::Batch(cmds)) = cmd else {
            panic!("expected a batch");
        };
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn empty_batch_is_none() {
        let empty: [Command<u32>; 0] = [];
        assert!(Command::batch(empty).is_none());
    }
}
