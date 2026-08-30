//! Pure table logic (filter, sort, paginate), ported verbatim from
//! `../halreslib-iced/src/table.rs`.

use std::cmp::Ordering;

use crate::model::{Column, SortDirection, SortRule, TablePreferences, Uri};

/// Rows rendered per page
pub const PAGE_SIZE: usize = 30;

/// Filter and sort URIs based on per-column filters and the global query.
pub fn filter_and_sort<'a>(
    uris: &'a [Uri],
    preferences: &TablePreferences,
    filters: &[String],
    global_query: &str,
) -> Vec<&'a Uri> {
    let mut result: Vec<_> = uris
        .iter()
        .filter(|uri| matches_all_filters(uri, filters) && matches_global_query(uri, global_query))
        .collect();

    apply_sort(&mut result, preferences);
    result
}

pub fn page_rows<'a>(
    rows: &'a [&'a Uri],
    page: usize,
) -> (usize, usize, &'a [&'a Uri]) {
    let page_count = rows.len().div_ceil(PAGE_SIZE).max(1);
    let current = page.min(page_count - 1);
    let start = current * PAGE_SIZE;
    let end = rows.len().min(start + PAGE_SIZE);
    (current, page_count, &rows[start..end])
}

fn matches_global_query(uri: &Uri, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }
    let searchable = Column::ALL
        .iter()
        .map(|column| column.value(uri))
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    query
        .split_whitespace()
        .all(|term| searchable.contains(term))
}

fn matches_all_filters(uri: &Uri, filters: &[String]) -> bool {
    if filters.len() != Column::count() {
        return true;
    }
    Column::ALL.iter().all(|column| {
        let value = column.value(uri).to_lowercase();
        let filter = &filters[column.index()].to_lowercase();
        value.contains(filter)
    })
}

fn apply_sort(uris: &mut [&Uri], preferences: &TablePreferences) {
    uris.sort_by(|left, right| {
        for rule in &preferences.sort_rules {
            let left_val = rule.column.value(left);
            let right_val = rule.column.value(right);
            let order = left_val.cmp(&right_val);

            if order != Ordering::Equal {
                return if rule.direction == SortDirection::Ascending {
                    order
                } else {
                    order.reverse()
                };
            }
        }
        Ordering::Equal
    });
}

/// Update sort rules on a column click. Passing `multi_sort` (e.g. Shift)
/// keeps existing rules and adds a secondary sort.
pub fn update_sort_rules(rules: &mut Vec<SortRule>, column: Column, multi_sort: bool) {
    let next = rules
        .iter()
        .find(|rule| rule.column == column)
        .map(|rule| rule.direction);

    if !multi_sort {
        rules.clear();
    } else {
        rules.retain(|rule| rule.column != column);
    }

    match next {
        None => rules.push(SortRule {
            column,
            direction: SortDirection::Ascending,
        }),
        Some(SortDirection::Ascending) => rules.push(SortRule {
            column,
            direction: SortDirection::Descending,
        }),
        Some(SortDirection::Descending) => {}
    }
}

/// Get sort indicator string for display
pub fn sort_indicator(column: Column, rules: &[SortRule]) -> &'static str {
    match rules
        .iter()
        .find(|rule| rule.column == column)
        .map(|rule| rule.direction)
    {
        Some(SortDirection::Ascending) => "↑",
        Some(SortDirection::Descending) => "↓",
        None => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Health;

    fn uri(title: &str, host: &str) -> Uri {
        Uri {
            uri_uuid: format!("uuid-{title}"),
            url: format!("https://{host}/"),
            scheme: "https".to_string(),
            host: Some(host.to_string()),
            path: Some("/".to_string()),
            live_status: Health::Available,
            title: Some(title.to_string()),
            auto_descr: None,
            man_descr: None,
            crea_user: None,
            crea_time: None,
            modi_user: None,
            modi_time: None,
            tags: vec![],
        }
    }

    #[test]
    fn sort_rules_cycle_asc_desc_off() {
        let mut rules = Vec::new();
        update_sort_rules(&mut rules, Column::Title, false);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].direction, SortDirection::Ascending);

        update_sort_rules(&mut rules, Column::Title, false);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].direction, SortDirection::Descending);

        update_sort_rules(&mut rules, Column::Title, false);
        assert!(rules.is_empty(), "third click clears the sort");
    }

    #[test]
    fn multi_sort_keeps_existing_rules() {
        let mut rules = Vec::new();
        update_sort_rules(&mut rules, Column::Title, false);
        update_sort_rules(&mut rules, Column::Host, true);
        assert_eq!(rules.len(), 2, "secondary sort is added");
        assert_eq!(rules[0].column, Column::Title);
        assert_eq!(rules[1].column, Column::Host);
    }

    #[test]
    fn filter_matches_all_columns_and_resets_to_nothing() {
        let preferences = TablePreferences::default();
        let uris = vec![uri("Rust Blog", "blog.rust-lang.org")];
        let no_filters = vec![String::new(); Column::count()];

        let rows = filter_and_sort(&uris, &preferences, &no_filters, "");
        assert_eq!(rows.len(), 1);

        let rows = filter_and_sort(&uris, &preferences, &no_filters, "rust blog");
        assert_eq!(rows.len(), 1, "global query hits across columns");

        let rows = filter_and_sort(&uris, &preferences, &no_filters, "no such term");
        assert!(rows.is_empty());

        let mut filters = no_filters.clone();
        filters[Column::Host.index()] = "unrelated".to_string();
        let rows = filter_and_sort(&uris, &preferences, &filters, "");
        assert!(rows.is_empty(), "per-column filter applies");
    }

    #[test]
    fn paging_clamps_past_the_end() {
        let uris: Vec<_> = (0..(PAGE_SIZE + 5)).map(|i| uri(&format!("t{i}"), "x.rs")).collect();
        let refs: Vec<&Uri> = uris.iter().collect();
        let (page, page_count, rows) = page_rows(&refs, usize::MAX);
        assert_eq!(page_count, 2);
        assert_eq!(page, 1, "page is clamped to the last one");
        assert_eq!(rows.len(), 5);
    }
}
