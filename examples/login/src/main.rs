//! Login form — exercises real form submission (`<form on_submit>`, which the
//! runtime `preventDefault`s so the page never navigates; pressing Enter in a
//! field submits too), per-field validation with inline errors, a transient
//! "busy" state, and a result-driven transition to a welcome screen.
//!
//! Auth is *simulated*: a [`Command::Timeout`] stands in for a network round
//! trip, so the example needs no backend. The shape — press send → busy →
//! success/error message — is exactly what an async `Command` gives you.

use wasm_bindgen::prelude::*;

use elmore::{App, Command, Html};

const MIN_USER: usize = 3;
const MIN_PASS: usize = 6;

enum Msg {
    User(String),
    Pass(String),
    Submit,
    /// The simulated auth round trip finished.
    Auth(Result<String, String>),
}

#[derive(Default)]
struct Model {
    username: String,
    password: String,
    phase: Phase,
    /// Errors keyed by field, shown inline under each input.
    errors: FieldErrors,
}

#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    Busy,
    Welcome { name: String },
}

/// Inline validation messages, one per field.
#[derive(Default)]
struct FieldErrors {
    username: Option<&'static str>,
    password: Option<&'static str>,
}

#[derive(Default)]
struct Login;

impl App for Login {
    type Message = Msg;
    type Model = Model;

    fn update(&mut self, msg: Msg, model: &mut Model) -> Option<Command<Msg>> {
        match msg {
            Msg::User(s) => {
                model.username = s;
                model.errors.username = None;
                Command::none()
            }
            Msg::Pass(s) => {
                model.password = s;
                model.errors.password = None;
                Command::none()
            }

            Msg::Submit => {
                // Validate locally before doing any work.
                model.errors.username = if model.username.len() < MIN_USER {
                    Some("Username must be at least 3 characters.")
                } else {
                    None
                };
                model.errors.password = if model.password.len() < MIN_PASS {
                    Some("Password must be at least 6 characters.")
                } else {
                    None
                };
                if model.errors.username.is_some() || model.errors.password.is_some() {
                    return Command::none();
                }

                model.phase = Phase::Busy;
                // Simulate the server. Capture the values we'll echo back.
                let (u, p) = (model.username.clone(), model.password.clone());
                Some(Command::Timeout {
                    millis: 700,
                    msg: Msg::Auth(fake_server(u, p)),
                })
            }

            Msg::Auth(Ok(name)) => {
                model.phase = Phase::Welcome { name };
                Command::none()
            }
            Msg::Auth(Err(_)) => {
                model.phase = Phase::Idle;
                model.errors.password = Some("Incorrect credentials. Try again.");
                Command::none()
            }
        }
    }

    fn view(&self, model: &Model) -> Html<Msg> {
        if let Phase::Welcome { name } = &model.phase {
            return Html::section()
                .class("welcome")
                .children([
                    Html::h1().text("Welcome"),
                    Html::p().text(format!("Glad to see you, {name}!")),
                ]);
        }

        let busy = matches!(model.phase, Phase::Busy);

        // A real `<form>`: the button submits it (as does Enter in a field),
        // and the runtime cancels the browser's default navigate behavior.
        Html::form()
            .class("login")
            .on_submit(|| Msg::Submit)
            .children([
                Html::h1().text("Sign in"),
                field(
                    "Username",
                    "login-username",
                    Html::input()
                        .input_type("text")
                        .id("login-username")
                        .value(model.username.clone())
                        .disabled(busy)
                        .on_input(Msg::User),
                    model.errors.username,
                ),
                field(
                    "Password",
                    "login-password",
                    Html::input()
                        .input_type("password")
                        .id("login-password")
                        .value(model.password.clone())
                        .disabled(busy)
                        .on_input(Msg::Pass),
                    model.errors.password,
                ),
                // Default `type` inside a form is `submit`.
                Html::button()
                    .text(if busy { "Signing in…" } else { "Sign in" })
                    .disabled(busy),
            ])
    }
}

/// Build a labeled input with an optional inline error underneath. The
/// `for`/`id` pair ties the label to its field, so clicking the label
/// focuses the input.
fn field(
    label: &'static str,
    id: &'static str,
    input: Html<Msg>,
    err: Option<&'static str>,
) -> Html<Msg> {
    let err_html = match err {
        Some(e) => Html::p().class("error").text(e),
        None => Html::p(),
    };
    Html::div().class("field").children([
        Html::label().attr("for", id).text(label),
        input,
        err_html,
    ])
}

/// The pretend backend: "succeeds" unless the password is the wrong one.
fn fake_server(username: String, password: String) -> Result<String, String> {
    if password == "hunter2" {
        Ok(username)
    } else {
        Err("invalid".into())
    }
}

#[wasm_bindgen(start)]
pub fn main() {
    elmore::run::<Login>();
}
