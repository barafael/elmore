//! Color mixer — three range sliders, one derived color.
//!
//! Exercises `input[type=range]` bound with `on_input`, the *derived view
//! state* pattern (the color is never stored: it is computed from the three
//! channels during `view`), and inline `style` via a plain attribute.

use wasm_bindgen::prelude::*;

use elmore::{App, Command, Html};

#[derive(Clone)]
enum Msg {
    Red(String),
    Green(String),
    Blue(String),
}

#[derive(Default)]
struct Model {
    red: u8,
    green: u8,
    blue: u8,
}

/// The `on_input` payload is the raw text of the slider; parse defensively.
fn channel(raw: &str, fallback: u8) -> u8 {
    raw.parse().unwrap_or(fallback)
}

#[derive(Default)]
struct Mixer;

impl App for Mixer {
    type Message = Msg;
    type Model = Model;

    fn update(&mut self, msg: Msg, model: &mut Model) -> Option<Command<Msg>> {
        match msg {
            Msg::Red(s) => model.red = channel(&s, model.red),
            Msg::Green(s) => model.green = channel(&s, model.green),
            Msg::Blue(s) => model.blue = channel(&s, model.blue),
        }
        Command::none()
    }

    fn view(&self, model: &Model) -> Html<Msg> {
        let (r, g, b) = (model.red, model.green, model.blue);
        let slider = |label: &'static str, value: u8, msg: fn(String) -> Msg| {
            Html::div()
                .class("row")
                .children([
                    Html::label().text(label),
                    Html::input()
                        .input_type("range")
                        .attr("min", "0")
                        .attr("max", "255")
                        .value(value.to_string())
                        .on_input(msg),
                    Html::output().text(value.to_string()),
                ])
        };

        Html::div()
            .class("mixer")
            .children([
                Html::h1().text("Color mixer"),
                slider("R", r, Msg::Red),
                slider("G", g, Msg::Green),
                slider("B", b, Msg::Blue),
                // The swatch and the hex code are pure functions of the
                // model — nothing about them is stored anywhere.
                Html::div()
                    .class("swatch")
                    .attr("style", format!("background: rgb({r}, {g}, {b})")),
                Html::p().child(Html::span().class("hex").text(format!("#{:02X}{:02X}{:02X}", r, g, b))),
            ])
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    elmore::run::<Mixer>();
}
