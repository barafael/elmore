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
mkdir -p _site
for d in counter todo stopwatch weather survey chat login playlist notes tictactoe mixer timer gallery; do
  (cd "$d" && trunk build --release --public-url "/$d/")
  cp -r "$d/dist" "_site/$d"
done
cp index.html _site/
python3 -m http.server 8000 --directory _site
# open http://localhost:8000/  (redirects to the gallery)
```

The same site deploys to GitHub Pages on every push to `main`
(see `.github/workflows/deploy.yml`) — **https://barafael.github.io/elmore/**.

To hack on a single example, `trunk serve` rebuilds on every save:

```sh
cd examples/counter
trunk serve
# open the printed address (usually http://localhost:8080)
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

## A real app: halreslib-elmore

[`halreslib-elmore/`](halreslib-elmore/) is a full port of the HaL resource
table originally built with iced (`../halreslib-iced`): a filterable, sortable,
paginated table over a static 2,000-row dataset. Everything the GUI port built
by hand — row hover states, the sliding column chooser, the dark theme — is
CSS here, and the dataset is pasted in verbatim as generated Rust
(`include!("data.rs")`). Clicking a row's title opens that resource's
details page — title, link, health, tags, and provenance fields — and
`< back to resources` restores the exact table state. Pure table logic
lives in `src/table.rs` with unit tests that run natively.

```sh
cd halreslib-elmore
trunk serve
# open the printed address
```

It deploys alongside the gallery as its own sub page:
**https://barafael.github.io/elmore/halreslib-elmore/**.

New to the Elm architecture? Read [`TUTORIAL.md`](TUTORIAL.md) — it builds the
chat example from nothing, one step at a time.

## Requirements

- `rustup target add wasm32-unknown-unknown`
- [trunk](https://trunkrs.dev) (`cargo install trunk --locked`)
- the `wasm-bindgen-cli` matching the crate's version (`cargo install wasm-bindgen-cli --version 0.2.127 --locked`)
