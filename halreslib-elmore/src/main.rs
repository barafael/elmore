//! halreslib-elmore — the HaL resource table, rebuilt with elmore.
//!
//! A port of the iced frontend (`../halreslib-iced`): a filterable, sortable,
//! paginated table over a static dataset. What the GUI port had to build by
//! hand — row hover states, the sliding column chooser, the dark theme — is
//! CSS here, so `Msg` shrinks to actual app concerns: no `HoverRow`, no
//! animation `Tick`.
//!
//! The dataset in `data.rs` is pasted verbatim from the halreslib project
//! (auto-generated, "do not edit"), which is why it is `include!`d rather
//! than `mod`ed: it references `Health` from this file's scope.

use wasm_bindgen::prelude::*;

use elmore::{App, Command, Html};

mod model;
mod table;

use model::{Column, Health, TablePreferences, Uri};
include!("data.rs");

#[derive(Clone)]
enum Msg {
    /// Sort by a column. `multi` keeps existing sort rules (the iced
    /// original's Shift-click; this UI always sends `false`).
    Sort(Column, bool),
    /// A per-column filter field changed (carries the current value).
    Filter(Column, String),
    /// The global search field changed (carries the current value).
    GlobalQuery(String),
    PrevPage,
    NextPage,
    FirstPage,
    LastPage,
    ToggleColumn(Column),
    ToggleColumnChooser,
}

struct Model {
    /// The dataset, built once when the runtime boots the default model.
    resources: Vec<Uri>,
    preferences: TablePreferences,
    /// One filter per column, indexed by `Column::index`.
    filters: Vec<String>,
    show_column_chooser: bool,
    global_query: String,
    page: usize,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            resources: all_uris(),
            preferences: TablePreferences::default(),
            filters: vec![String::new(); Column::count()],
            show_column_chooser: false,
            global_query: String::new(),
            page: 0,
        }
    }
}

#[derive(Default)]
struct Halreslib;

impl App for Halreslib {
    type Message = Msg;
    type Model = Model;

    fn update(&mut self, msg: Msg, model: &mut Model) -> Option<Command<Msg>> {
        match msg {
            Msg::Sort(column, multi) => {
                table::update_sort_rules(&mut model.preferences.sort_rules, column, multi);
                model.page = 0;
            }
            Msg::Filter(column, value) => {
                model.filters[column.index()] = value;
                model.page = 0;
            }
            Msg::GlobalQuery(value) => {
                model.global_query = value;
                model.page = 0;
            }
            Msg::PrevPage => model.page = model.page.saturating_sub(1),
            Msg::NextPage => model.page += 1,
            Msg::FirstPage => model.page = 0,
            Msg::LastPage => model.page = usize::MAX,
            Msg::ToggleColumn(column) => model.preferences.toggle_column(column),
            Msg::ToggleColumnChooser => model.show_column_chooser = !model.show_column_chooser,
        }
        Command::none()
    }

    fn view(&self, model: &Model) -> Html<Msg> {
        let rows = table::filter_and_sort(
            &model.resources,
            &model.preferences,
            &model.filters,
            &model.global_query,
        );
        let (current, page_count, page_rows) = table::page_rows(&rows, model.page);
        let columns = model.preferences.visible_in_order();

        // Header and rows share one grid template so the cells line up.
        let template = grid_template(&columns);

        let header_cells = columns.iter().copied().map(|column| {
            let indicator = table::sort_indicator(column, &model.preferences.sort_rules);
            let sort_label = if indicator.is_empty() { "-" } else { indicator };
            let class = if column.is_compact() { "th compact" } else { "th" };
            Html::div()
                .class(class)
                .children([
                    Html::div().class("th-top").children([
                        Html::span().class("th-title").text(column.title()),
                        Html::button()
                            .class("ind")
                            .text(sort_label)
                            .on_click(move || Msg::Sort(column, false)),
                    ]),
                    Html::input()
                        .input_type("text")
                        .class("filter")
                        .placeholder("Filter…")
                        .value(model.filters[column.index()].clone())
                        .on_input(move |value| Msg::Filter(column, value)),
                ])
        });

        let body_rows = page_rows.iter().map(|uri| {
            let cells = columns.iter().copied().map(|column| cell(column, uri));
            Html::div()
                .class("tr")
                .attr("style", template.clone())
                .children(cells)
        });

        let body: Html<Msg> = if page_rows.is_empty() {
            Html::div().class("nohits").text("No hits.")
        } else {
            Html::div().class("tbody").children(body_rows)
        };

        let toolbar = Html::div().class("toolbar").children([
            Html::div().class("spacer"),
            Html::input()
                .input_type("text")
                .class("search")
                .placeholder("Search all resources…")
                .value(model.global_query.clone())
                .on_input(Msg::GlobalQuery),
            Html::button()
                .class(if model.show_column_chooser { "cols on" } else { "cols" })
                .text("Columns")
                .on_click(|| Msg::ToggleColumnChooser),
            Html::div().class("spacer"),
        ]);

        // The chooser is always in the tree (element identity makes the CSS
        // slide run); `.open` just grows it from zero to content height.
        let chips = Column::ALL.iter().map(|column| {
            let visible = model.preferences.is_column_visible(*column);
            Html::label().class("chip").children([
                Html::input()
                    .input_type("checkbox")
                    .checked(visible)
                    .on_toggle(move |_| Msg::ToggleColumn(*column)),
                Html::span().text(column.title()),
            ])
        });
        let chooser = Html::div()
            .class(if model.show_column_chooser { "chooser open" } else { "chooser" })
            .child(
                Html::div().class("chooser-inner").child(
                    Html::div().class("card").children([
                        Html::span().class("chooser-title").text("Visible columns:"),
                        Html::div().class("chips").children(chips),
                    ]),
                ),
            );

        let table = Html::div().class("tablewrap").child(
            Html::div()
                .class("table")
                .children([
                    Html::div()
                        .class("thead")
                        .attr("style", template)
                        .children(header_cells),
                    body,
                ]),
        );

        let (first_row, last_row) = if rows.is_empty() {
            (0, 0)
        } else {
            (
                current * table::PAGE_SIZE + 1,
                current * table::PAGE_SIZE + page_rows.len(),
            )
        };
        let at_start = current == 0;
        let at_end = current + 1 >= page_count;
        let pagination = Html::div().class("pagination").children([
            Html::button().text("First").disabled(at_start).on_click(|| Msg::FirstPage),
            Html::button().text("Prev").disabled(at_start).on_click(|| Msg::PrevPage),
            Html::span().text(format!("Page {} of {}", current + 1, page_count)),
            Html::span().text(format!("{}–{} of {} rows", first_row, last_row, rows.len())),
            Html::button().text("Next").disabled(at_end).on_click(|| Msg::NextPage),
            Html::button().text("Last").disabled(at_end).on_click(|| Msg::LastPage),
            Html::span()
                .class("muted")
                .text(format!("Showing {} of {} rows", page_rows.len(), rows.len())),
        ]);

        Html::div()
            .class("rt")
            .children([toolbar, chooser, table, pagination])
    }
}

/// One body cell. The health column gets a colored status dot-text; the rest
/// are plain text.
fn cell(column: Column, uri: &Uri) -> Html<Msg> {
    let class = if column.is_compact() { "td compact" } else { "td" };
    let content = match column {
        Column::Health => {
            let status = uri.live_status;
            let hp_class = match status {
                Health::Available => "hp ok",
                Health::Unavailable => "hp bad",
                Health::Unknown => "hp unknown",
            };
            Html::span().class(hp_class).text(status.label())
        }
        _ => Html::span().text(column.value(uri)),
    };
    Html::div().class(class).child(content)
}

/// CSS `grid-template-columns` matching the iced layout: compact columns are
/// fixed-width, everything else shares the remaining space.
fn grid_template(columns: &[Column]) -> String {
    let parts: Vec<_> = columns
        .iter()
        .map(|column| {
            if column.is_compact() {
                "110px"
            } else {
                "minmax(0, 1fr)"
            }
        })
        .collect();
    format!("grid-template-columns: {}", parts.join(" "))
}

#[wasm_bindgen(start)]
pub fn main() {
    elmore::run::<Halreslib>();
}
