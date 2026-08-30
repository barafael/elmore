//! Todo — exercises value-carrying events (`on_input`), a key event
//! (`on_key_up`, pressing Enter adds the task), and a keyed list rendered
//! from a `Vec` via `.children(…iter…map…)`: items carry `.key(id)`, so
//! removing one doesn't shift anything, and `Message`s address items by id,
//! not by position.

use wasm_bindgen::prelude::*;

use elmore::{App, Command, Html};

enum Msg {
    /// The text field changed (carries the current value).
    Typed(String),
    /// A key was released in the text field (carries the key name).
    KeyUp(String),
    /// The add button was pressed.
    Add,
    /// Toggle an item's done state (by id).
    Toggle(u64),
    /// Remove an item (by id).
    Remove(u64),
}

#[derive(Default)]
struct Model {
    draft: String,
    items: Vec<Item>,
    /// Source of stable keys; never reused within a session.
    next_id: u64,
}

struct Item {
    id: u64,
    text: String,
    done: bool,
}

#[derive(Default)]
struct Todo;

impl App for Todo {
    type Message = Msg;
    type Model = Model;

    fn update(&mut self, msg: Msg, model: &mut Model) -> Option<Command<Msg>> {
        match msg {
            Msg::Typed(text) => model.draft = text,
            // Enter in the field does the same as the Add button.
            Msg::KeyUp(key) if key == "Enter" => add_item(model),
            Msg::KeyUp(_) => {}
            Msg::Add => add_item(model),
            Msg::Toggle(id) => {
                if let Some(item) = model.items.iter_mut().find(|i| i.id == id) {
                    item.done = !item.done;
                }
            }
            Msg::Remove(id) => model.items.retain(|i| i.id != id),
        }
        Command::none()
    }

    fn view(&self, model: &Model) -> Html<Msg> {
        // Rendering a `Vec` is just `.children` over a mapped iterator — no
        // manual folding needed. Each row is keyed by its stable id.
        let rows = model
            .items
            .iter()
            .map(|item| {
                let cls = if item.done { "item done" } else { "item" };
                // Hoist the id: `move || Msg::Toggle(item.id)` would capture
                // the reference, not the copy (capture truncates at derefs).
                let id = item.id;
                Html::li()
                    .key(id.to_string())
                    .class(cls)
                    .children([
                        Html::button()
                            .text(if item.done { "☑" } else { "☐" })
                            .on_click(move || Msg::Toggle(id)),
                        Html::span().text(item.text.clone()),
                        Html::button()
                            .text("✕")
                            .on_click(move || Msg::Remove(id)),
                    ])
            });

        let list = Html::ul().class("items").children(rows);

        let remaining = model.items.iter().filter(|i| !i.done).count();

        Html::div()
            .class("todo")
            .children([
                Html::h1().text("Things to do"),
                Html::div()
                    .class("new")
                    .children([
                        Html::input()
                            .input_type("text")
                            .placeholder("Add a task…")
                            .value(model.draft.clone())
                            .on_input(Msg::Typed)
                            .on_key_up(Msg::KeyUp),
                        Html::button().text("Add").on_click(|| Msg::Add),
                    ]),
                list,
                Html::p().text(format!("{remaining} remaining")),
            ])
    }
}

/// Push the draft onto the list (if it isn't blank) and clear the field.
fn add_item(model: &mut Model) {
    let text = model.draft.trim().to_string();
    if !text.is_empty() {
        model.items.push(Item { id: model.next_id, text, done: false });
        model.next_id += 1;
        model.draft.clear();
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    elmore::run::<Todo>();
}
