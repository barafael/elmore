//! Notes — sticky notes in a keyed list, each row holding a bound
//! `<textarea>`.
//!
//! Exercises the `textarea` element (`on_input` works exactly like an
//! input's), and the reason keys exist: new notes are *prepended*, yet the
//! renderer matches existing rows by key — so every open textarea keeps its
//! element, its scroll position, and its caret while the list grows above
//! it. Positional matching would have shifted every row's content down.

use wasm_bindgen::prelude::*;

use elmore::{App, Command, Html};

#[derive(Clone)]
enum Msg {
    /// New note goes to the *top* of the list.
    Add,
    /// A note's text changed (carries the note id and the new text).
    Text(u64, String),
    Remove(u64),
}

#[derive(Default)]
struct Model {
    notes: Vec<Note>,
    next_id: u64,
}

struct Note {
    id: u64,
    text: String,
}

#[derive(Default)]
struct Notes;

impl App for Notes {
    type Message = Msg;
    type Model = Model;

    fn update(&mut self, msg: Msg, model: &mut Model) -> Option<Command<Msg>> {
        match msg {
            Msg::Add => {
                model.notes.insert(0, Note { id: model.next_id, text: String::new() });
                model.next_id += 1;
            }
            Msg::Text(id, text) => {
                if let Some(note) = model.notes.iter_mut().find(|n| n.id == id) {
                    note.text = text;
                }
            }
            Msg::Remove(id) => model.notes.retain(|n| n.id != id),
        }
        Command::none()
    }

    fn view(&self, model: &Model) -> Html<Msg> {
        let rows = model.notes.iter().map(|note| {
            let id = note.id;
            Html::li()
                // Keyed by id: prepending shifts positions, not identities.
                .key(id.to_string())
                .class("note")
                .children([
                    Html::textarea()
                        .placeholder("Write something…")
                        .value(note.text.clone())
                        .on_input(move |text| Msg::Text(id, text)),
                    Html::button()
                        .text("✕")
                        .on_click(move || Msg::Remove(id)),
                ])
        });

        let total: usize = model.notes.iter().map(|n| n.text.chars().count()).sum();

        Html::div()
            .class("app")
            .children([
                Html::h1().text("Notes"),
                Html::button().text("+ New note").on_click(|| Msg::Add),
                Html::ul().class("notes").children(rows),
                Html::p()
                    .class("total")
                    .text(format!("{} notes, {} characters", model.notes.len(), total)),
            ])
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    elmore::run::<Notes>();
}
