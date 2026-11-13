//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The fields a kind requires, held to a committed ticket as `entry` holds a ticket being moved: a
//! ticket in a state that needs a holder has them filled, and in any other state it is held to none.

use mockspace_extra_ticket_model::{Store, rules};

const STATES: &str = r#"
[[ticket_state]]
id = "backlog"
column = 0

[[ticket_state]]
id = "waiting"
column = 1
needs_holder = true

[[ticket_state]]
id = "done"
column = 2
closed = true
satisfies = true
"#;

// the third kind is one a later row adds with a field of its own, and the rules know none of the three
const KINDS: &str = r#"
[[ticket_kind]]
id = "question"
states = ["backlog", "waiting", "done"]
initial = "backlog"
requires = ["related:subject"]

[[ticket_kind]]
id = "job"
states = ["backlog", "waiting", "done"]
initial = "backlog"
requires = ["brief", "branch"]

[[ticket_kind]]
id = "note"
states = ["backlog", "waiting", "done"]
initial = "backlog"
requires = ["where"]
"#;

/// A ticket of `kind` with `fields` as lines of its row, moved to `to` held by `holder`.
fn ticket(id: &str, kind: &str, fields: &str, to: &str, holder: &str) -> String {
    let holder = if holder.is_empty() {
        String::new()
    } else {
        format!("holder = \"{holder}\"\n")
    };
    format!(
        "[[ticket]]\nid = \"{id}\"\ntitle = \"t\"\nkind = \"{kind}\"\n{fields}\n\n[[ticket_move]]\nid = \"m_{id}\"\nticket = \"{id}\"\nto = \"{to}\"\nby = \"x\"\nat = \"2026-11-02T10:00:00Z\"\n{holder}"
    )
}

fn store(rows: &[String]) -> Store {
    let mut files: Vec<(String, &str)> =
        vec![("states.toml".to_string(), STATES), ("kinds.toml".to_string(), KINDS)];
    for (i, r) in rows.iter().enumerate() {
        files.push((format!("t{i}.toml"), r.as_str()));
    }
    Store::from_texts(files.iter().map(|(p, t)| (p.as_str(), *t))).unwrap()
}

fn findings(rows: &[String]) -> Vec<String> {
    rules::check_store(&store(rows))
        .into_iter()
        .map(|f| format!("{}: {}", f.row, f.message))
        .collect()
}

#[test]
fn a_question_held_by_op_with_no_subject_is_a_finding_and_with_one_it_is_clean() {
    let bare = ticket("bare", "question", "", "waiting", "op");
    let empty = ticket("empty", "question", "related = []", "waiting", "op");
    let named = ticket(
        "named",
        "question",
        "related = [\"subject::engine\"]",
        "waiting",
        "op",
    );
    // the control: a link to a row of another namespace is not a subject
    let other = ticket(
        "other",
        "question",
        "related = [\"tenet::engine\"]",
        "waiting",
        "op",
    );
    let found = findings(&[bare, empty, named, other]);
    assert_eq!(found.len(), 3, "{found:?}");
    assert!(found.iter().any(|f| {
        f.starts_with("ticket::bare: ") && f.contains("a `subject` link in `related`")
    }));
    assert!(found.iter().any(|f| {
        f.starts_with("ticket::empty: ") && f.contains("a `subject` link in `related`")
    }));
}

#[test]
fn a_ticket_in_a_state_that_needs_no_holder_is_held_to_no_required_field() {
    let rows = [
        ticket("pending", "question", "", "backlog", ""),
        ticket("answered", "question", "", "done", ""),
        ticket("brief_less", "job", "", "backlog", ""),
    ];
    assert_eq!(findings(&rows), Vec::<String>::new());
}

#[test]
fn every_field_a_kind_requires_is_named_when_it_is_missing() {
    let bare = ticket("j", "job", "branch = \"work/j\"", "waiting", "coord");
    let found = findings(&[bare]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("`brief`") && !found[0].contains("`branch`"),
        "{found:?}"
    );
    let filled = ticket(
        "j",
        "job",
        "brief = \"b\"\nbranch = \"work/j\"",
        "waiting",
        "coord",
    );
    assert_eq!(findings(&[filled]), Vec::<String>::new());
}

#[test]
fn a_kind_added_with_its_own_required_field_is_held_to_it_with_no_code_for_the_kind() {
    let bare = ticket("n", "note", "", "waiting", "op");
    let found = findings(&[bare]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("`note`") && found[0].contains("`where`"),
        "{found:?}"
    );
    let filled = ticket("n", "note", "where = \"somewhere\"", "waiting", "op");
    assert_eq!(findings(&[filled]), Vec::<String>::new());
}

#[test]
fn the_finding_and_the_refusal_are_one_sentence() {
    let row = ticket("bare", "question", "", "waiting", "op");
    let s = store(&[row]);
    let refused = rules::entry(
        &s,
        &s.kinds["question"],
        &s.tickets["bare"].fields,
        &[],
        "waiting",
        "op",
    )
    .unwrap_err();
    let found = rules::check_store(&s);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].message, refused);
}
