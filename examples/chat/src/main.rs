//! Chat — a scrollable message feed with a scripted bot reply.
//!
//! Combines several ideas at once:
//! - a keyed collection rendered with `.children(…iter().map(…))`,
//! - an effect (`Command::Timeout`) used as the "bot thinking" delay,
//! - two effects out of one update via `Command::batch` (the bot follows up),
//! - a `bot_typing` counter of in-flight replies driving a status line,
//! - a "who said it" discrimination (left vs right alignment).

use wasm_bindgen::prelude::*;

use elmore::{App, Command, Html};

const AVATARS: [&str; 3] = ["alice", "bob", "eve"];

enum Msg {
    /// The entry box changed.
    Typed(String),
    /// The user pressed send.
    Send,
    /// The bot finished "thinking" and produced this line.
    BotReply(String),
}

#[derive(Default)]
struct Model {
    messages: Vec<Line>,
    draft: String,
    /// Bot replies in flight; the typing line shows while any are.
    bot_typing: usize,
    /// Which avatar talks next (rotates).
    turn: usize,
    /// Source of stable keys for message bubbles.
    next_id: usize,
}

/// One rendered message with its author.
struct Line {
    id: usize,
    who: &'static str,
    text: String,
}

#[derive(Default)]
struct Chat;

impl App for Chat {
    type Message = Msg;
    type Model = Model;

    fn update(&mut self, msg: Msg, model: &mut Model) -> Option<Command<Msg>> {
        match msg {
            Msg::Typed(text) => {
                model.draft = text;
                Command::none()
            }

            Msg::Send => {
                let text = model.draft.trim().to_string();
                if text.is_empty() {
                    return Command::none();
                }
                model.messages.push(Line { id: model.next_id, who: "you", text });
                model.next_id += 1;
                model.draft.clear();
                model.bot_typing += 1;

                // Two independent effects out of one update: the bot's reply
                // and a quick follow-up, each on its own timer.
                Command::batch([
                    Command::Timeout { millis: 600, msg: Msg::BotReply(scripted_reply(model)) },
                    Command::Timeout {
                        millis: 750,
                        msg: Msg::BotReply("Tell me more…".to_string()),
                    },
                ])
            }

            Msg::BotReply(text) => {
                let who = AVATARS[model.turn % AVATARS.len()];
                model.turn += 1;
                model.messages.push(Line { id: model.next_id, who, text });
                model.next_id += 1;
                model.bot_typing = model.bot_typing.saturating_sub(1);
                Command::none()
            }
        }
    }

    fn view(&self, model: &Model) -> Html<Msg> {
        // Render the feed by mapping each line to a bubble — keyed by id, so
        // the feed only ever *appends* real nodes — tagged with whether *we*
        // wrote it (right-aligned) or someone else did.
        let bubbles = model.messages.iter().map(|line| {
            let mine = line.who == "you";
            let cls = if mine { "bubble mine" } else { "bubble" };
            Html::div()
                .key(line.id.to_string())
                .class(cls)
                .children([
                    Html::span().class("who").text(line.who),
                    Html::span().text(line.text.clone()),
                ])
        });

        let typing = if model.bot_typing > 0 {
            Html::div().class("typing").text("…")
        } else {
            Html::div()
        };

        Html::div()
            .class("chat")
            .children([
                Html::h1().text("Chat"),
                Html::div().class("feed").children(bubbles),
                typing,
                Html::div()
                    .class("composer")
                    .children([
                        Html::input()
                            .input_type("text")
                            .placeholder("Say something…")
                            .value(model.draft.clone())
                            .on_input(Msg::Typed),
                        Html::button()
                            .text("Send")
                            .on_click(|| Msg::Send),
                    ]),
            ])
    }
}

/// Pick a canned reply for the bot. In a real app this would be a
/// `Command::FetchText`; here the illusion keeps the example dependency-free.
fn scripted_reply(model: &Model) -> String {
    let last = model
        .messages
        .last()
        .map(|l| l.text.as_str())
        .unwrap_or("hello");
    format!("You said: “{last}”. Interesting!")
}

#[wasm_bindgen(start)]
pub fn main() {
    elmore::run::<Chat>();
}
