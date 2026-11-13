//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

use std::collections::BTreeSet;

use crate::items::{Open, answered, names_crate, row_items, ticket_items};
use crate::touched::{crate_of, touched};
use registry_lints::layout::{ContentDirs, Layout};

/// The layout the fixtures are written in, the mock tree under `mock`.
fn layout() -> Layout {
    Layout::new("mock", ContentDirs::default())
}

#[test]
fn a_crate_is_read_under_the_mock_directory_the_project_names() {
    let linux = Layout::new("linux", ContentDirs::default());
    assert_eq!(crate_of("linux/crates/a/src/x.rs", &linux), Some("a"));
    assert_eq!(crate_of("linux/tools/b/Cargo.toml", &linux), Some("b"));
    // THE CONTROL: a path under `mock` is outside this repository's mock directory.
    assert_eq!(crate_of("mock/crates/a/src/x.rs", &linux), None);
}

#[test]
fn a_crate_directory_names_its_crate() {
    assert_eq!(
        crate_of("mock/crates/kaski-clock/src/span.rs", &layout()),
        Some("kaski-clock")
    );
    assert_eq!(crate_of("mock/tools/tickets/Cargo.toml", &layout()), Some("tickets"));
}

#[test]
fn a_path_outside_a_crate_names_none() {
    assert_eq!(crate_of("mock/WORKFLOW.md.tmpl", &layout()), None);
    assert_eq!(crate_of("mock/tools/README.md", &layout()), None);
    assert_eq!(crate_of("mock/crates", &layout()), None);
    assert_eq!(crate_of("docs/crates/x/y.md", &layout()), None);
    assert_eq!(crate_of("mock/lints/fixme_markers.rs", &layout()), None);
}

#[test]
fn touched_collects_each_crate_once() {
    let paths: Vec<String> = [
        "mock/crates/a/src/x.rs",
        "mock/crates/a/src/y.rs",
        "mock/tools/b/src/lib.rs",
        "mock/WORKFLOW.md.tmpl",
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    let got: Vec<String> = touched(&paths, &layout()).into_iter().collect();
    assert_eq!(got, ["a", "b"]);
}

#[test]
fn a_crate_is_named_as_a_whole_word_only() {
    assert!(names_crate(
        "kaski-clock/DESIGN.md.tmpl says",
        "kaski-clock"
    ));
    assert!(names_crate("the `kaski-clock` crate", "kaski-clock"));
    assert!(names_crate("kaski-clock", "kaski-clock"));
    assert!(!names_crate("kaski-clock-extra owns it", "kaski-clock"));
    assert!(!names_crate("the pre-kaski-clock era", "kaski-clock"));
    assert!(!names_crate("kaski_clock", "kaski-clock"));
}

fn open(id: &str, title: &str, what: &str) -> Open {
    Open {
        id:    id.to_owned(),
        state: "ready".to_owned(),
        title: title.to_owned(),
        what:  what.to_owned(),
    }
}

#[test]
fn a_ticket_naming_a_touched_crate_is_listed_once_and_the_own_ticket_never() {
    let tickets = [
        open("one", "fix kaski-clock span", "and kaski-body too"),
        open("two", "unrelated", "nothing here"),
        open("own", "the job", "touches kaski-clock"),
    ];
    let touched: BTreeSet<String> = ["kaski-clock", "kaski-body"]
        .iter()
        .map(ToString::to_string)
        .collect();
    let items = ticket_items(&tickets, &touched, "own");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].key, "ticket:one");
    assert!(items[0].line.contains("kaski-clock"));
    assert!(items[0].line.contains("kaski-body"));
}

#[test]
fn a_row_joined_to_the_brief_is_listed_unless_cited_or_already_in_the_brief() {
    let joined = |id: &str| -> Vec<String> {
        match id {
            "a" => vec!["b".to_owned(), "c".to_owned(), "d".to_owned()],
            "b" => vec!["a".to_owned(), "d".to_owned()],
            _ => Vec::new(),
        }
    };
    let cited = |id: &str| id == "c";
    let brief = vec!["a".to_owned(), "b".to_owned()];
    let items = row_items(&brief, &joined, &cited);
    let keys: Vec<&str> = items.iter().map(|i| i.key.as_str()).collect();
    assert_eq!(
        keys,
        ["row:d"],
        "b is in the brief, c is cited, d is joined to two rows and listed once"
    );
}

#[test]
fn a_report_answers_a_key_by_one_of_three_lines() {
    let report = "\
## Finish
- ticket:one done 0123abc
- ticket:two parted some_new_ticket_id
- row:x left: the canon is silent on it
- row:y left:
- ticket:three done
- ticket:four done zzzz
- ticket:five maybe 0123abc
";
    let got: Vec<String> = answered(report).into_iter().collect();
    assert_eq!(got, ["row:x", "ticket:one", "ticket:two"]);
}
