# elmore

A deliberately minimal framework for simple interactive web pages, in Rust,
compiled to WebAssembly.

`elmore` follows the **Elm architecture**: your whole app is three pieces —

- a **`Model`** (your state, a plain Rust type),
- an **`update`** function (the only place state changes), and
- a **`view`** function (reads the model, builds an HTML tree).

The runtime owns the model, mounts your view into `#root`, and keeps the loop
turning: events become `Message`s, `update` applies them, `view` re-renders.
Effects (timers, intervals, fetches) are `Command`s that deliver their
results back as `Message`s.

There is no virtual DOM: on every message `view` builds a *complete fresh
tree* from the model, and the renderer reconciles it against the live DOM in
place, updating only what actually changed. Elements keep their identity
across renders, so focus, the text caret, scroll positions, and clicks in
flight all Just Work — while `view` stays embarrassingly simple.

## A complete app

```rust
use elmore::{App, Command, Html};

enum Msg {
    Increment,
}

#[derive(Default)]
struct Model {
    count: i32,
}

#[derive(Default)]
struct Counter;

impl App for Counter {
    type Message = Msg;
    type Model = Model;

    fn update(&mut self, msg: Msg, model: &mut Model) -> Option<Command<Msg>> {
        match msg {
            Msg::Increment => model.count += 1,
        }
        Command::none()
    }

    fn view(&self, model: &Model) -> Html<Msg> {
        Html::div()
            .class("counter")
            .child(Html::button().text("+1").on_click(|| Msg::Increment))
            .child(Html::span().text(model.count.to_string()))
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    elmore::run::<Counter>();
}
```

## Run the examples

The **gallery** is an elmore app that embeds every example in an iframe —
build them all, serve once, browse:

```sh
cd examples
for d in */; do (cd "$d" && wasm-pack build --target web --out-dir pkg); done
python3 -m http.server 8000
# open http://localhost:8000/  (redirects to the gallery)
```

To run a single example, the same steps from its own directory:

```sh
cd examples/counter
wasm-pack build --target web --out-dir pkg
python3 -m http.server 8000
# open http://localhost:8000/
```

## Examples

| Example     | Exercises                                                     |
| ----------- | ------------------------------------------------------------- |
| `counter`   | `update` + `view` + click events — the "hello world"          |
| `todo`      | `on_input`, Enter-to-add (`on_key_up`), lists via `.children(iter.map(…))` |
| `stopwatch` | named interval via `Command::Every` / `Command::Cancel`  |
| `weather`   | `Command::FetchText` against the real Open-Meteo API; loading / error / loaded states |
| `survey`    | tabs, a `<select>` dropdown (`on_change`), computed summary   |
| `chat`      | keyed feed, bot replies via `Command::Timeout` + `Command::batch` |
| `login`     | real form submission (`on_submit` + `preventDefault`), validation, busy states |
| `playlist`  | keyed lists: reorder (move up/down, shuffle), insert, remove from the middle |
| `tictactoe` | pure game logic: win detection, derived phase, locked cells, zero effects |
| `mixer`     | range sliders (`on_input`), derived view state, inline `style` attribute |
| `timer`     | phased countdown: chained `Command::Timeout`s across phase transitions |
| `notes`     | keyed rows of bound `<textarea>`s — caret survival in a prepend-heavy list |

Rendering a list? Map it into `.children(...)`, and give each item `.key(id)`
so reorders move DOM nodes instead of rewriting them — focus and identity ride
along.

New to the Elm architecture? Read [`TUTORIAL.md`](TUTORIAL.md) — it builds the
chat example from nothing, one step at a time.

## Requirements

- `rustup target add wasm32-unknown-unknown`
- [wasm-pack](https://rustwasm.github.io/wasm-pack/)
