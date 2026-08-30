//! `elmore` — a deliberately minimal Iced-like framework for simple
//! interactive web pages, built around the Elm architecture.
//!
//! The whole model is: **State → View → Update**, with events and effects
//! streaming `Message`s back into a single `update` loop. On every message
//! `view` builds a complete fresh tree, which the renderer reconciles into
//! the live DOM in place — elements keep their identity, so focus, the
//! caret, and clicks in flight all survive. That simplicity is the point.
//!
//! ```no_run
//! use elmore::{App, Html, Command};
//!
//! #[derive(Default)]
//! struct Counter;
//!
//! impl App for Counter {
//!     type Message = i32;
//!     type Model = i32;
//!
//!     fn update(&mut self, msg: i32, model: &mut i32) -> Option<Command<i32>> {
//!         *model += msg;
//!         None
//!     }
//!
//!     fn view(&self, model: &i32) -> Html<i32> {
//!         Html::div().children([
//!             Html::button().text("-1").on_click(|| -1),
//!             Html::span().text(model.to_string()),
//!             Html::button().text("+1").on_click(|| 1),
//!         ])
//!     }
//! }
//!
//! fn main() { elmore::run::<Counter>(); }
//! ```

mod app;
mod command;
#[cfg(target_arch = "wasm32")]
mod dom;
mod html;
mod runtime;

pub use app::App;
pub use command::Command;
pub use html::{Attr, Bind, Event as HtmlEvent, Html, Node, Tag};
pub use runtime::run;
