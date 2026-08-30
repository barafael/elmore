//! Survey — the biggest example. Exercises tabs, a single-select rating row,
//! a `<select>` dropdown, free text, and a computed summary. Shows how a
//! larger `Model` plus small rendering helpers keep `view` readable.

use wasm_bindgen::prelude::*;

use elmore::{App, Command, Html};

const RATINGS: [&str; 5] = ["1", "2", "3", "4", "5"];
const COLORS: [&str; 4] = ["red", "green", "blue", "other"];
const QUESTIONS: [&str; 3] = ["Rating", "Color", "Comments"];

enum Msg {
    /// Switch to a tab.
    Tab(usize),
    /// Set the rating (1-based).
    Rate(usize),
    /// Set the selected color.
    Color(String),
    /// Update the free-text comments (carries the current value).
    Comments(String),
    /// Submit the survey.
    Submit,
}

struct Model {
    /// One answer slot per question (`None` = unanswered). Indexed by the tab.
    answers: Vec<Option<Answer>>,
    /// Currently visible tab (question).
    active: usize,
    /// True once submit has been pressed (enables the summary check).
    submitted: bool,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            // Seeded here so the *initial* `view` (rendered on the default model,
            // before any `update` runs) can index `answers[i]` safely.
            answers: vec![None; QUESTIONS.len()],
            active: 0,
            submitted: false,
        }
    }
}

#[derive(Default, Clone)]
enum Answer {
    #[default]
    Unanswered,
    Rating(usize),
    Color(String),
    Comments(String),
}

#[derive(Default)]
struct Survey;

impl App for Survey {
    type Message = Msg;
    type Model = Model;

    fn update(&mut self, msg: Msg, model: &mut Model) -> Option<Command<Msg>> {
        if model.answers.len() != QUESTIONS.len() {
            model.answers = vec![None; QUESTIONS.len()];
        }
        match msg {
            Msg::Tab(i) => model.active = i,
            Msg::Rate(r) => model.answers[0] = Some(Answer::Rating(r)),
            Msg::Color(c) => model.answers[1] = Some(Answer::Color(c)),
            Msg::Comments(s) => model.answers[2] = Some(Answer::Comments(s)),
            Msg::Submit => model.submitted = true,
        }
        Command::none()
    }

    fn view(&self, model: &Model) -> Html<Msg> {
        // Tab bar: one button per question; the active one is highlighted.
        let tab_buttons = QUESTIONS.iter().enumerate().map(|(i, title)| {
            let active = i == model.active;
            Html::button()
                .class(if active { "tab active" } else { "tab" })
                .text(*title)
                .on_click(move || Msg::Tab(i))
        });
        let tabs = Html::div().class("tabs").children(tab_buttons);

        let panel = match model.active {
            0 => rating_panel(&model.answers[0]),
            1 => color_panel(&model.answers[1]),
            2 => comments_panel(&model.answers[2]),
            _ => Html::div(),
        };

        let summary = if model.submitted {
            Html::section().class("summary").children([
                Html::h2().text("Your answers"),
                summary_rows(&model.answers),
            ])
        } else {
            Html::div()
        };

        Html::div()
            .class("survey")
            .children([
                Html::h1().text("Satisfaction survey"),
                tabs,
                panel,
                Html::button()
                    .text("Submit")
                    .on_click(|| Msg::Submit),
                summary,
            ])
    }
}

/// The rating question: a row of buttons, the selected one highlighted.
fn rating_panel(current: &Option<Answer>) -> Html<Msg> {
    let selected = match current {
        Some(Answer::Rating(r)) => Some(*r),
        _ => None,
    };
    let buttons = RATINGS.iter().enumerate().map(|(i, label)| {
        let rating = i + 1;
        let active = selected == Some(rating);
        Html::button()
            .class(if active { "rating on" } else { "rating" })
            .text(*label)
            .on_click(move || Msg::Rate(rating))
    });
    Html::section().class("question").children([
        Html::h3().text("How would you rate us?"),
        Html::div().class("ratings").children(buttons),
    ])
}

/// The color question: a dropdown whose change event carries the value.
fn color_panel(current: &Option<Answer>) -> Html<Msg> {
    let selected = match current {
        Some(Answer::Color(c)) => c.clone(),
        _ => String::new(),
    };
    let opts = COLORS
        .iter()
        .map(|c| Html::option().value(*c).text(*c));

    Html::section().class("question").children([
        Html::h3().text("Favorite color?"),
        Html::select().class("colors").children(opts).on_change(Msg::Color).value(selected),
    ])
}

/// The comments question: a free-text input.
fn comments_panel(current: &Option<Answer>) -> Html<Msg> {
    let text = match current {
        Some(Answer::Comments(s)) => s.clone(),
        _ => String::new(),
    };
    Html::section().class("question").children([
        Html::h3().text("Any comments?"),
        Html::input()
            .input_type("text")
            .placeholder("Optional…")
            .value(text)
            .on_input(Msg::Comments),
    ])
}

/// Flatten the answers into a summary list.
fn summary_rows(answers: &[Option<Answer>]) -> Html<Msg> {
    let unpack = |a: &Option<Answer>| match a {
        Some(Answer::Rating(r)) => format!("{r}/5"),
        Some(Answer::Color(c)) => c.clone(),
        Some(Answer::Comments(c)) if !c.is_empty() => c.clone(),
        _ => "— unanswered —".to_string(),
    };

    let rows = answers.iter().enumerate().map(|(i, a)| {
        // Mixed content: the question as a bare text node, the answer in a
        // span — the one case `.text(..)` can't express, since it replaces
        // all children.
        Html::li().children([
            Html::text_node(format!("{}: ", QUESTIONS[i])),
            Html::span().text(unpack(a)),
        ])
    });

    Html::ul().class("summary-list").children(rows)
}

#[wasm_bindgen(start)]
pub fn main() {
    elmore::run::<Survey>();
}
