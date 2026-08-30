//! Playlist — exercises **keyed list reconciliation**: reordering (move up /
//! down, shuffle), inserting, and removing from the middle of a list.
//!
//! Every track row carries `.key(id)`, so the renderer matches rows by
//! identity instead of by position: a shuffle *moves* the existing DOM nodes
//! rather than rewriting their contents. Watch with devtools open — the
//! `<li>` elements keep their identity across every operation here.

use wasm_bindgen::prelude::*;

use elmore::{App, Command, Html};

/// Tracks handed out by "Add"; the pool cycles.
const POOL: [(&str, &str); 5] = [
    ("So What", "Miles Davis"),
    ("Blue in Green", "Miles Davis"),
    ("Giant Steps", "John Coltrane"),
    ("Naima", "John Coltrane"),
    ("Round Midnight", "Thelonious Monk"),
];

#[derive(Clone)]
enum Msg {
    /// Move a track up one slot (no-op at the top).
    Up(u64),
    /// Move a track down one slot (no-op at the bottom).
    Down(u64),
    /// Remove a track.
    Remove(u64),
    /// Append the next track from the pool.
    Add,
    /// Shuffle the whole list.
    Shuffle,
}

#[derive(Default)]
struct Model {
    tracks: Vec<Track>,
    next_id: u64,
}

struct Track {
    id: u64,
    title: &'static str,
    artist: &'static str,
}

#[derive(Default)]
struct Playlist;

impl App for Playlist {
    type Message = Msg;
    type Model = Model;

    fn update(&mut self, msg: Msg, model: &mut Model) -> Option<Command<Msg>> {
        let position = |model: &Model, id: u64| model.tracks.iter().position(|t| t.id == id);

        match msg {
            Msg::Up(id) => {
                if let Some(i) = position(model, id)
                    && i > 0
                {
                    model.tracks.swap(i, i - 1);
                }
            }
            Msg::Down(id) => {
                if let Some(i) = position(model, id)
                    && i + 1 < model.tracks.len()
                {
                    model.tracks.swap(i, i + 1);
                }
            }
            Msg::Remove(id) => model.tracks.retain(|t| t.id != id),
            Msg::Add => {
                let (title, artist) = POOL[(model.next_id % POOL.len() as u64) as usize];
                model.tracks.push(Track { id: model.next_id, title, artist });
                model.next_id += 1;
            }
            Msg::Shuffle => {
                // A tiny xorshift stands in for a rand crate: deterministic,
                // dependency-free, plenty for a demo.
                let mut seed = model
                    .next_id
                    .wrapping_mul(0x9E3779B97F4A7C15)
                    .wrapping_add(model.tracks.len() as u64)
                    | 1;
                for i in (1..model.tracks.len()).rev() {
                    seed ^= seed << 13;
                    seed ^= seed >> 7;
                    seed ^= seed << 17;
                    let j = (seed >> 32) as usize % (i + 1);
                    model.tracks.swap(i, j);
                }
            }
        }
        Command::none()
    }

    fn view(&self, model: &Model) -> Html<Msg> {
        // Each row is keyed by its stable id: the renderer matches rows by
        // key across renders, so reorders move nodes instead of morphing
        // them, and handlers always point at the right track.
        let rows = model.tracks.iter().map(|track| {
            let id = track.id;
            Html::li()
                .key(id.to_string())
                .class("track")
                .children([
                    Html::button()
                        .class("mover")
                        .text("▲")
                        .on_click(move || Msg::Up(id)),
                    Html::span()
                        .class("title")
                        .text(format!("{} — {}", track.title, track.artist)),
                    Html::button()
                        .class("mover")
                        .text("▼")
                        .on_click(move || Msg::Down(id)),
                    Html::button()
                        .class("mover")
                        .text("✕")
                        .on_click(move || Msg::Remove(id)),
                ])
        });

        Html::div()
            .class("playlist")
            .children([
                Html::h1().text("Playlist"),
                Html::div()
                    .class("controls")
                    .children([
                        Html::button().text("+ Add a track").on_click(|| Msg::Add),
                        Html::button().text("⇄ Shuffle").on_click(|| Msg::Shuffle),
                    ]),
                if model.tracks.is_empty() {
                    Html::p().text("Nothing queued. Add a track!")
                } else {
                    Html::ul().class("tracks").children(rows)
                },
                Html::p().text(format!("{} tracks", model.tracks.len())),
            ])
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    elmore::run::<Playlist>();
}
