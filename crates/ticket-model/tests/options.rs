//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! An option that holds a further lettered option is refused by the check, and every shape that only
//! looks like one is not.

use kaski_tickets::{Store, rules};

const VOCABULARY: &str = r#"
[[ticket_state]]
id = "backlog"
column = 0

[[ticket_kind]]
id = "task"
states = ["backlog"]
initial = "backlog"

[[ticket_kind]]
id = "question"
states = ["backlog"]
initial = "backlog"
"#;

/// A ticket of `kind` whose `options` are the given TOML array text.
fn ticket(id: &str, kind: &str, options: &str) -> String {
    format!(
        "[[ticket]]\nid = \"{id}\"\ntitle = \"t\"\nkind = \"{kind}\"\noptions = {options}\n\n[[ticket_move]]\nid = \"m_{id}\"\nticket = \"{id}\"\nto = \"backlog\"\nby = \"x\"\nat = \"2026-11-02T10:00:00Z\"\n"
    )
}

fn findings(rows: &[String]) -> Vec<String> {
    let mut files: Vec<(String, &str)> = vec![("vocabulary.toml".to_string(), VOCABULARY)];
    for (i, r) in rows.iter().enumerate() {
        files.push((format!("items/{i}.toml"), r.as_str()));
    }
    let store = Store::from_texts(files.iter().map(|(p, t)| (p.as_str(), *t))).unwrap();
    rules::check_store(&store)
        .into_iter()
        .map(|f| format!("{}: {}", f.row, f.message))
        .collect()
}

#[test]
fn the_old_shape_of_one_string_holding_several_options_is_refused_naming_it() {
    let found = findings(&[ticket(
        "old",
        "question",
        r#"["a: keep it,b: drop it,c: ask again"]"#,
    )]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].starts_with("ticket::old: ")
            && found[0].contains("`a: keep it,b: drop it,c: ask again`")
            && found[0].contains("vertical bar"),
        "{found:?}"
    );
}

#[test]
fn a_space_after_the_comma_and_any_ticket_kind_are_refused_alike() {
    let found = findings(&[ticket("spaced", "task", r#"["a: x, b: y"]"#)]);
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn the_second_option_of_a_list_is_found_as_well_as_the_first() {
    let found = findings(&[ticket("second", "question", r#"["a: one", "b: two,c: three"]"#)]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("`b: two,c: three`"), "{found:?}");
}

#[test]
fn options_each_in_a_string_of_their_own_are_clean() {
    let found = findings(&[ticket(
        "clean",
        "question",
        r#"["a: keep it", "b: drop it", "c: ask again"]"#,
    )]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn commas_that_do_not_start_a_lettered_option_are_clean() {
    // the control for the refusals above: each of these holds a comma and the words around it
    for options in [
        r#"["a: use x, or b: nothing"]"#,
        r#"["a: yes, keep: all of it"]"#,
        r#"["a: one, two and three"]"#,
        r#"["a: pick b, c and d"]"#,
        r#"[]"#,
    ] {
        let found = findings(&[ticket("t", "question", options)]);
        assert!(found.is_empty(), "{options}: {found:?}");
    }
}
