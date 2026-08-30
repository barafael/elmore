# Tutorial: building a chat app, step by step

This tutorial builds `examples/chat` from nothing, one small step at a time.
Along the way it introduces the central idea of `elmore`: the **Elm
architecture**, a loop of three functions that is so small it fits in your head.

If you haven't yet, skim the other examples first (especially `counter` — it's
the "hello world"). They share the same shape; this tutorial is about *why* it
looks that way, and how to grow a tiny app into something real.

---

## 0. The big idea: one loop, three functions

Interactive web apps are mostly the same problem wearing different clothes:

> State changes → show it on screen → the user does something → state changes.

`elmore` names the parts of that loop and makes you put every app in the same
mold:

```
        Message
           │
           ▼
   ┌───────────────┐   view()    ┌────────────┐
   │    update     │────────────▶│   HTML     │
   │ (state, msg)  │             │   tree     │
   └───────────────┘             └────────────┘
        │    ▲                        │
   Command    │                  events push
   (effect)   │                  Messages back
        │    └────────────────────────┘
        ▼
    browser
```

Three pieces, and you only ever write two of them:

- **`Model`** — your state. A plain Rust type.
- **`update(msg, model)`** — *this* changes the model.
- **`view(model)`** — *this* reads the model and builds HTML.
- **`Message`** — every change you can request ("user clicked", "timer rang").

The runtime owns the model and keeps the loop turning; you never touch the DOM
directly. That's the whole framework.

`elmore` makes one deliberate trade you should keep in mind: **`view` builds
the whole tree from scratch on every message** — no memoization, no
component state to think about. The renderer then reconciles that fresh tree
against the live DOM in place, updating only what actually changed. In
exchange, `view` is embarrassingly simple — it just builds a fresh tree from
the model, every time — while focus, the caret, scroll positions, and clicks
in flight all survive, because DOM elements keep their identity across
renders.

---

## 1. The skeleton

Start with an empty crate and a struct that will become your app.

```rust
use elmore::{App, Command, Html};

enum Msg {
    // what messages will a chat room need?
}

#[derive(Default)]
struct Model {
    // no state yet
}

#[derive(Default)]
struct Chat;

impl App for Chat {
    type Message = Msg;
    type Model = Model;

    fn update(&mut self, msg: Msg, model: &mut Model) -> Option<Command<Msg>> {
        match msg {
            // handle each message
        }
    }

    fn view(&self, model: &Model) -> Html<Msg> {
        Html::div()
    }
}

fn main() {
    elmore::run::<Chat>();
}
```

The `App` trait is the contract. Three things to notice:

- `type Message` and `type Model` tell the runtime what your app speaks and what
  it remembers.
- `update` gets the model `&mut` — the runtime owns it and hands it to you.
- `main` just calls `elmore::run::<Chat>()` and walks away. From here on, the
  runtime calls our `update`/`view`.

The `#[derive(Default)]`s aren't decoration: the runtime constructs both the
app and the model with `Default` at boot.

It compiles, but shows nothing yet. Let's give it something to say.

---

## 2. State: decide what a chat remembers

Before writing any HTML, decide **what the app needs to remember**. A chat
needs:

1. the list of messages,
2. whatever the user is currently typing,
3. whether a bot reply is on the way.

That maps directly to fields. Add a helper type for a single message:

```rust
#[derive(Default)]
struct Model {
    messages: Vec<Line>,
    draft: String,
    bot_typing: bool,
    turn: usize, // whose turn the bot is on
}

struct Line {
    who: &'static str,
    text: String,
}
```

Note we just stated the model and the runtime does the rest. No `RefCell`, no
`Rc`, no DOM queries. Everything the UI could ever show comes from `Model`.

---

## 3. Messages: list the things that can happen

A message is a **request to change the model**, nothing more, nothing less.
Chats need:

```rust
enum Msg {
    /// The entry box changed (carries the new text).
    Typed(String),
    /// The user pressed send.
    Send,
    /// A bot reply arrived.
    BotReply(String),
}
```

The `String` carried in `Typed` and `BotReply` is the standard elmore idiom for
"this event brings data along with it" — inputs and async results.

---

## 4. update: the only place state changes

Now the other half of the contract. For each message, mutate the model, and
optionally return a [`Command`] (an effect like a timer or a fetch). Start with
the plain cases:

```rust
fn update(&mut self, msg: Msg, model: &mut Model) -> Option<Command<Msg>> {
    match msg {
        Msg::Typed(text) => {
            model.draft = text;
            Command::none()
        }
        // Send, BotReply to come
    }
}
```

`Command::none()` says "no side effect this time." Every arm must produce
something — either `None`/`Command::none()`, or a real effect.

### Effects: `Send` asks the bot to "think"

`Send` does three things: record the user's line, clear the draft, and kick off
a bot reply. The last one is an effect. We fake the bot's network round-trip
with a timer — `Command::Timeout` re-injects `Msg::BotReply` into the loop after
a delay:

```rust
Msg::Send => {
    let text = model.draft.trim().to_string();
    if text.is_empty() {
        return Command::none();
    }
    model.messages.push(Line { who: "you", text });
    model.draft.clear();
    model.bot_typing = true;

    Some(Command::Timeout {
        millis: 600,
        msg: Msg::BotReply(scripted_reply(model)),
    })
}
```

This is the whole point of `Command`: `update` stays a **pure function of the
model** — it doesn't touch the network or a clock itself; it *describes* an
effect and lets the runtime run it and hand the result back as another message.

And the reply:

```rust
Msg::BotReply(text) => {
    let who = AVATARS[model.turn % AVATARS.len()];
    model.turn += 1;
    model.messages.push(Line { who, text });
    model.bot_typing = false;
    Command::none()
}
```

Notice: `update` never returns HTML and `view` never changes state. That split is
what makes the whole thing predictable — every rendering is a pure function of
`Model`, and `Model` only changes in `update`.

---

## 5. view: build the tree from the model

The runtime calls `view` after every message, with the fresh model, and
reconciles the result into the page. So `view` is: **read the model, build
HTML, done**.

Start with the shell:

```rust
fn view(&self, model: &Model) -> Html<Msg> {
    Html::div()
        .class("chat")
        .children([
            Html::h1().text("Chat"),
            Html::div().class("feed"),      // to come
            Html::div().class("composer"),  // to come
        ])
}
```

`Html::div()` is a **builder**. Unlike with macros, every step is just a method
that returns the element, so you can read it top-to-bottom and compose it in
plain Rust: `.class(..)` sets an attribute, `.children([..])` nests elements,
`.text(..)` sets content, and later `.on_click(..)` attaches a handler.

### Rendering a list: map into `children`

The feed is a `Vec<Line>`. `children` accepts *any* iterator of nodes —
arrays, `Vec`s, or a `map` straight off your model — so rendering a
collection is one expression:

```rust
let bubbles = model
    .messages
    .iter()
    .map(|line| {
        let mine = line.who == "you";
        let cls = if mine { "bubble mine" } else { "bubble" };
        Html::div().class(cls).children([
            Html::span().class("who").text(line.who),
            Html::span().text(line.text.clone()),
        ])
    });

let feed = Html::div().class("feed").children(bubbles);
```

Each item becomes one node; the parent just collects them. This is the
Elm-style "map over a list" in plain Rust clothes.

If the user can reorder, insert into, or delete from the middle of a list,
give each item a **key** — a stable identity among its siblings. Our `Line`
has no stable id (chat messages are append-only), but if it carried one —
say, `id: u64` — you'd write:

```rust
let bubbles = model.messages.iter().map(|line| {
    Html::div().key(line.id.to_string()).class(/* … */)/* … */
});
```

The renderer then matches children by key across renders: a reorder *moves*
the existing DOM nodes instead of rewriting their contents, so focus, scroll
positions, and CSS transitions ride along. The rule of thumb: keys for lists
the user edits or reorders, plain positional children for everything static.
(See `examples/playlist` for the full treatment.)

### Conditional presence: pick one of two nodes

`bot_typing` is a flag, not a list, so we branch in Rust and render a status
line only when it's true:

```rust
let typing = if model.bot_typing {
    Html::div().class("typing").text("…")
} else {
    Html::div() // empty
};
```

This is idiomatic: no vdom, no framework "if" — just a plain `if` building a
node.

### Inputs carry their value straight into a message

Finally, the composer. The key line is `.on_input(Msg::Typed)` — the whole
method name *is* the message constructor:

```rust
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
    ])
```

`on_input` fires whenever the field changes and forwards the current value as a
`String` into `Msg::Typed`. `on_click` fires with no value, so its closure just
returns `Msg::Send`. That's all the wiring there is — no `document.getElementById`,
no `add_event_listener` in your code.

---

## 6. Put it together

Thread the three pieces from steps 4 and 5 into the skeleton, plus the
`AVATARS` constant and the tiny `scripted_reply` helper, and you have a
working chat:

```rust
const AVATARS: [&str; 3] = ["alice", "bob", "eve"];

fn scripted_reply(model: &Model) -> String {
    let last = model.messages.last().map(|l| l.text.as_str()).unwrap_or("hello");
    format!("You said: “{last}”. Interesting!")
}
```

Run it, and the loop does the rest: you type → `Typed` updates the draft →
`Send` appends your line and schedules a `BotReply` → the timer fires →
`BotReply` appends the bot's line → `view` renders it all. The full source is in
`examples/chat/src/main.rs`. (The shipped example goes one step further:
`Command::batch` returns *two* effects from `Send` — the reply and a quick
follow-up — and `bot_typing` is a counter of in-flight replies, so the
"typing" line stays up until the last one lands. The single-effect, boolean
version built here works exactly as written.)

---

## 7. What we skipped (on purpose)

The point of `elmore` is giving things up to stay simple. In this chat we
deliberately did *not*:

- **Scroll to the newest message.** The feed doesn't auto-scroll. Real chat
  needs it; a toy doesn't, and the framework has no hook for it — another
  feature you'd have to add. (Element identity does survive reconciliation,
  so the feed's scroll position is at least *kept* rather than reset.)
- **Real networking.** The bot's reply is a fake `Timeout`, not a
  `Command::FetchText`. Swapping one in is a one-line change (see `weather`),
  but the "thinking" delay gives a nicer demo with zero server.
- **History / persistence.** Gone. Model lives only in the page's memory.

That's the trade, stated plainly: to keep `update`/`view` this clean, some
capabilities are simply off the table. Everything you *do* get composes the
same way, every time.

---

## Checklist for writing your own app

1. Write the `Model` type — the whole UI, as data.
2. List the `Message` variants — every change that can be requested.
3. Implement `update` — mutate the model; return a `Command` for effects.
4. Implement `view` — build a fresh `Html` tree from the model; use
   `.children(items.iter().map(…))` for lists (with `.key(id)` on items the
   user can reorder), `if` for optional nodes, and `on_input`/`on_click` for
   events.
5. Call `elmore::run::<YourApp>()` in `main`.

Every app in `examples/` follows this exact shape.
