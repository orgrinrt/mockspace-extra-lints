//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The two edges a ticket names other tickets by, `needs` and `part_of`: each is held to tickets that
//! exist, not the ticket itself, and no cycle, by `check_links` on a ticket being edited and by
//! `check_store` over the committed rows.

use mockspace_extra_ticket_model::{Store, rules};

const VOCABULARY: &str = r#"
[[ticket_state]]
id = "backlog"
column = 1

[[ticket_kind]]
id = "task"
states = ["backlog"]
initial = "backlog"
"#;

/// A task with `links` as lines of its row, in the one state the vocabulary has.
fn task(id: &str, links: &str) -> String {
    format!(
        "[[ticket]]\nid = \"{id}\"\nkind = \"task\"\n{links}\n\n[[ticket_move]]\nid = \"m_{id}\"\nticket = \"{id}\"\nto = \"backlog\"\nby = \"coord\"\nat = \"2026-11-02T10:00:00Z\"\n"
    )
}

fn store(tasks: &[(&str, &str)]) -> Store {
    let texts: Vec<(String, String)> = tasks
        .iter()
        .map(|(id, links)| (format!("items/{id}.toml"), task(id, links)))
        .chain([("vocabulary.toml".to_string(), VOCABULARY.to_string())])
        .collect();
    Store::from_texts(texts.iter().map(|(p, s)| (p.as_str(), s.as_str()))).unwrap()
}

/// What the rule says of `ticket` in a store holding `tasks`, for each of the two edges.
fn refused(field: &str, tasks: &[(&str, &str)], ticket: &str) -> String {
    let s = store(tasks);
    let err = rules::check_links(&s, ticket).unwrap_err();
    assert!(err.contains(&format!("`{field}`")), "{field}: {err}");
    err
}

fn edge(field: &str, to: &str) -> String {
    if field == "needs" {
        format!("needs = [\"{to}\"]")
    } else {
        format!("part_of = \"{to}\"")
    }
}

#[test]
fn an_edge_naming_no_ticket_is_refused_for_needs_and_for_part_of() {
    for field in ["needs", "part_of"] {
        let err = refused(field, &[("a", &edge(field, "ghost"))], "a");
        assert!(
            err.contains("`ghost`") && err.contains("no ticket"),
            "{err}"
        );
    }
}

#[test]
fn an_edge_naming_the_ticket_itself_is_refused_for_needs_and_for_part_of() {
    for field in ["needs", "part_of"] {
        let err = refused(field, &[("a", &edge(field, "a"))], "a");
        assert!(err.contains("itself"), "{err}");
    }
}

#[test]
fn an_edge_that_goes_round_is_refused_for_needs_and_for_part_of() {
    for field in ["needs", "part_of"] {
        let err = refused(
            field,
            &[("a", &edge(field, "b")), ("b", &edge(field, "c")), ("c", &edge(field, "a"))],
            "a",
        );
        assert!(err.contains("round"), "{err}");
    }
}

#[test]
fn sound_edges_stand_and_a_cycle_in_one_field_is_not_read_into_the_other() {
    // a is part of b and needs b: two edges in the same direction between two tickets, and a ring in
    // neither field, so both stand
    let s = store(&[("a", "needs = [\"b\"]\npart_of = \"b\""), ("b", "")]);
    assert_eq!(rules::check_links(&s, "a"), Ok(()));
    assert_eq!(rules::check_links(&s, "b"), Ok(()));
    // a needs b and b is part of a: a ring only if the two fields were read as one
    let s = store(&[("a", "needs = [\"b\"]"), ("b", "part_of = \"a\"")]);
    assert_eq!(rules::check_links(&s, "a"), Ok(()));
    assert_eq!(rules::check_links(&s, "b"), Ok(()));
    assert_eq!(rules::check_store(&s), Vec::new());
}

#[test]
fn the_committed_rows_are_held_to_the_same_two_edges() {
    let s = store(&[("a", "part_of = \"ghost\""), ("b", "needs = [\"b\"]")]);
    let found: Vec<String> = rules::check_store(&s)
        .into_iter()
        .map(|f| format!("{}: {}", f.row, f.message))
        .collect();
    assert!(
        found.iter().any(|f| {
            f.starts_with("ticket::a: ") && f.contains("`part_of`") && f.contains("ghost")
        }),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|f| f.starts_with("ticket::b: ") && f.contains("`needs`") && f.contains("itself")),
        "{found:?}"
    );
}
