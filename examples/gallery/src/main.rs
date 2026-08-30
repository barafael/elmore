//! Gallery — the examples index, written in elmore itself. Exercises the
//! framework's newest tag (`<iframe>`) and not much else: a sidebar of
//! examples, a stage that embeds the selected one.
//!
//! Each example stays a complete, isolated app (its own `#root`, runtime,
//! and listeners) inside an iframe; the gallery never touches their DOM. Run
//! from the `examples/` directory: build every example with wasm-pack, then
//! serve the directory and open `/gallery/`.

use wasm_bindgen::prelude::*;

use elmore::{App, Command, Html};

/// One gallery entry: a label, the path to its page (relative to the
/// gallery), and a one-line hint of what the example demonstrates.
struct Example {
    name: &'static str,
    path: &'static str,
    blurb: &'static str,
}

const EXAMPLES: [Example; 12] = [
    Example { name: "Counter", path: "../counter/", blurb: "update + view + clicks — hello world" },
    Example { name: "Todo", path: "../todo/", blurb: "on_input, Enter-to-add, lists from iterators" },
    Example { name: "Stopwatch", path: "../stopwatch/", blurb: "named interval via Command::Every / Cancel" },
    Example { name: "Timer", path: "../timer/", blurb: "phased countdown via chained timeouts" },
    Example { name: "Weather", path: "../weather/", blurb: "Command::FetchText against a real API" },
    Example { name: "Survey", path: "../survey/", blurb: "tabs, <select> on_change, summary" },
    Example { name: "Chat", path: "../chat/", blurb: "keyed feed, batched bot replies" },
    Example { name: "Login", path: "../login/", blurb: "real form submission, validation" },
    Example { name: "Playlist", path: "../playlist/", blurb: "keyed reordering, shuffle, removal" },
    Example { name: "Notes", path: "../notes/", blurb: "keyed textareas, prepend-without-losing-focus" },
    Example { name: "Tic-tac-toe", path: "../tictactoe/", blurb: "pure game logic, derived phase, zero effects" },
    Example { name: "Mixer", path: "../mixer/", blurb: "range sliders, derived view state, inline style" },
];

#[derive(Clone)]
enum Msg {
    /// Show this example in the stage.
    Select(usize),
}

#[derive(Default)]
struct Model {
    selected: usize,
}

#[derive(Default)]
struct Gallery;

impl App for Gallery {
    type Message = Msg;
    type Model = Model;

    fn update(&mut self, msg: Msg, model: &mut Model) -> Option<Command<Msg>> {
        match msg {
            Msg::Select(i) => model.selected = i % EXAMPLES.len(),
        }
        Command::none()
    }

    fn view(&self, model: &Model) -> Html<Msg> {
        let current = &EXAMPLES[model.selected];

        let nav = EXAMPLES.iter().enumerate().map(|(i, example)| {
            let active = i == model.selected;
            Html::button()
                .class(if active { "active" } else { "" })
                .text(example.name)
                .on_click(move || Msg::Select(i))
        });

        Html::div()
            .class("gallery")
            .children([
                Html::div().class("sidebar").children([
                    Html::h1().text("elmore"),
                    Html::p().class("blurb").text(current.blurb),
                    Html::nav().children(nav),
                ]),
                Html::div().class("stage").child(
                    // The selected example, embedded whole. `src` only
                    // changes when the selection does, so switching picks
                    // another example without reloading this page.
                    Html::iframe()
                        .class("frame")
                        .src(current.path)
                        .attr("title", current.name),
                ),
            ])
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    elmore::run::<Gallery>();
}
