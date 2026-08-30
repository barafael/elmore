//! Counter — the smallest app. Proves `update` + `view` + a click event.

use wasm_bindgen::prelude::*;

use elmore::{App, Command, Html};

enum Msg {
    Increment,
    Decrement,
    Reset,
}

// NOTE: The `Model` is a simple alias — the runtime owns it. `#[derive(Default)]`
// is all `App::Model` requires.
#[derive(Default)]
struct Model {
    value: i64,
}

#[derive(Default)]
struct Counter;

impl App for Counter {
    type Message = Msg;
    type Model = Model;

    fn update(&mut self, msg: Msg, model: &mut Model) -> Option<Command<Msg>> {
        match msg {
            Msg::Increment => model.value += 1,
            Msg::Decrement => model.value -= 1,
            Msg::Reset => model.value = 0,
        }
        // No effects: a pure, synchronous update.
        Command::none()
    }

    fn view(&self, model: &Model) -> Html<Msg> {
        Html::div()
            .class("counter")
            .children([
                Html::h1().text("Counter"),
                Html::button().text("−1").on_click(|| Msg::Decrement),
                Html::span().class("value").text(model.value.to_string()),
                Html::button().text("+1").on_click(|| Msg::Increment),
                Html::button().text("reset").on_click(|| Msg::Reset),
            ])
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    elmore::run::<Counter>();
}
