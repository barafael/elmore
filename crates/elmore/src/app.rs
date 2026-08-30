//! The Elm-architecture `App` trait and its pieces.

use crate::{Command, Html};

/// Core of the Elm architecture.
///
/// Implement this for your application. The runtime owns the [`Model`] and
/// threads it through every [`App::update`] and [`App::view`]:
///
/// - [`App::Message`] — every possible event/effect your UI can produce.
/// - [`App::Model`] — your application state.
/// - [`App::update`] — given a message, mutate the model and optionally return
///   a [`Command`] effect to run (e.g. a timer or a fetch).
/// - [`App::view`] — render the model into an owned [`Html`] tree.
///
/// Note that the *first* `view` call happens on `Model::default()`, before
/// any `update` — so the default model must already be a renderable state
/// (seed collections, phase enums, everything `view` will index).
pub trait App: Sized + Default + 'static {
    /// The type of messages that drive [`App::update`].
    type Message: 'static;

    /// The application state, owned by the runtime.
    type Model: Default + 'static;

    /// Handle one message: mutate the model, and optionally emit a [`Command`].
    fn update(&mut self, msg: Self::Message, model: &mut Self::Model) -> Option<Command<Self::Message>>;

    /// Render the current model into an owned HTML tree.
    fn view(&self, model: &Self::Model) -> Html<Self::Message>;
}
