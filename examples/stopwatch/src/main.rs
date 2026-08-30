//! Stopwatch — exercises `Command::Every` and `Command::Cancel`.
//!
//! `Start` subscribes a named interval that delivers `Tick` every `TICK_MS`;
//! `Stop` cancels it. Nothing is ever re-armed: the runtime owns the pulse,
//! and subscribing the same `id` twice is a no-op, so mashing Start can't
//! double the tick rate.

use wasm_bindgen::prelude::*;

use elmore::{App, Command, Html};

const TICK_MS: u32 = 100;
/// Identity of the tick interval, for `Command::Every`/`Command::Cancel`.
const TICKER: &str = "stopwatch-tick";

enum Msg {
    /// Fired on a timer every `TICK_MS` while running (carries elapsed ms).
    Tick(u32),
    Start,
    Stop,
    Reset,
}

#[derive(Default)]
struct Model {
    /// Elapsed milliseconds.
    ms: u32,
    /// True while the stopwatch is running.
    running: bool,
}

#[derive(Default)]
struct Stopwatch;

impl App for Stopwatch {
    type Message = Msg;
    type Model = Model;

    fn update(&mut self, msg: Msg, model: &mut Model) -> Option<Command<Msg>> {
        match msg {
            // The interval delivers a `Tick` every `TICK_MS` while subscribed.
            Msg::Tick(delta) if model.running => {
                model.ms += delta;
                Command::none()
            }
            // A trailing tick after a stop is discarded.
            Msg::Tick(_) => Command::none(),

            Msg::Start => {
                model.running = true;
                Some(Command::Every {
                    id: TICKER,
                    millis: TICK_MS,
                    // A fresh `Tick` each pulse — `Every` builds its message
                    // per tick rather than reusing one owned value.
                    msg: Box::new(|| Msg::Tick(TICK_MS)),
                })
            }
            Msg::Stop => {
                model.running = false;
                Some(Command::Cancel { id: TICKER })
            }
            Msg::Reset => {
                model.ms = 0;
                model.running = false;
                Some(Command::Cancel { id: TICKER })
            }
        }
    }

    fn view(&self, model: &Model) -> Html<Msg> {
        let (secs, hundredths) = (model.ms / 1000, (model.ms % 1000) / 10);
        let start_stop = if model.running {
            Html::button().text("⏸ Stop").on_click(|| Msg::Stop)
        } else {
            Html::button().text("▶ Start").on_click(|| Msg::Start)
        };

        Html::div()
            .class("stopwatch")
            .children([
                Html::h1().text("Stopwatch"),
                Html::div()
                    .class("time")
                    .text(format!("{secs}.{hundredths:02} s")),
                start_stop,
                Html::button()
                    .text("Reset")
                    .on_click(|| Msg::Reset),
            ])
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    elmore::run::<Stopwatch>();
}
