//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The tickets a tool may close for what the tree lost, and the state it closes them to. Each case that
//! must be refused stands beside the one that is not, so a rule that closed everything would fail.

use mockspace_extra_ticket_model::{Store, Tree, stale};

const VOCABULARY: &str = r#"
[[ticket_state]]
id = "backlog"
column = 1

[[ticket_state]]
id = "doing"
column = 2
needs_holder = true

[[ticket_state]]
id = "done"
column = 3
closed = true
satisfies = true

[[ticket_state]]
id = "dropped"
column = 4
closed = true

[[ticket_state]]
id = "set_aside"
column = 5
closed = true

[[ticket_kind]]
id = "task"
states = ["backlog", "doing", "done", "dropped"]
initial = "backlog"

[[ticket_kind]]
id = "question"
states = ["backlog", "done", "dropped"]
initial = "backlog"
satisfied_by = ["answer"]

[[ticket_kind]]
id = "twice"
states = ["backlog", "dropped", "set_aside"]
initial = "backlog"

[[ticket_kind]]
id = "never"
states = ["backlog", "done"]
initial = "backlog"
"#;

/// A tree that once held the names it is given and holds nothing else of them.
struct Gone(&'static [(&'static str, &'static str)]);

impl Tree for Gone {
    fn crate_prefix(&self) -> &str {
        "kaski"
    }

    fn gone(&self, name: &str) -> Option<String> {
        self.0
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, by)| (*by).to_string())
    }
}

const RENDER: Gone = Gone(&[("kaski-render", "ef19a8e0c")]);

/// A ticket: its id, its kind, the rest of its row, the state its one move enters and the holder it names.
struct T(
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

fn task(id: &'static str, lines: &'static str) -> T {
    T(id, "task", lines, "backlog", "")
}

fn store(rows: &[T]) -> Store {
    let mut texts = vec![("vocabulary.toml".to_string(), VOCABULARY.to_string())];
    for T(id, kind, lines, to, holder) in rows {
        texts.push((
            format!("items/{id}.toml"),
            format!(
                "[[ticket]]\nid = \"{id}\"\nkind = \"{kind}\"\n{lines}\n\n[[ticket_move]]\nid = \"m_{id}\"\nticket = \"{id}\"\nto = \"{to}\"\nholder = \"{holder}\"\nby = \"coord\"\nat = \"2026-11-02T10:00:00Z\"\n"
            ),
        ));
    }
    Store::from_texts(texts.iter().map(|(p, s)| (p.as_str(), s.as_str()))).unwrap()
}

fn ids(s: &Store, tree: &Gone) -> Vec<String> {
    stale::closable(s, tree)
        .into_iter()
        .map(|c| c.ticket)
        .collect()
}

#[test]
fn an_open_ticket_whose_only_name_is_gone_is_closable_with_the_commit_that_deleted_it() {
    let s = store(&[task("a", "title = \"port the pass of kaski-render\"")]);
    let out = stale::closable(&s, &RENDER);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].ticket, "a");
    assert_eq!(out[0].gone, [(
        "kaski-render".to_string(),
        "ef19a8e0c".to_string()
    )]);
}

#[test]
fn a_ticket_naming_a_live_name_beside_a_gone_one_is_not_closable() {
    let s = store(&[task("a", "title = \"move kaski-render into kaski-frame\"")]);
    let tree = Gone(&[("kaski-render", "ef19a8e0c")]);
    // `kaski-frame` is not gone: the tree says nothing of it, so it is a subject the ticket still has
    assert!(ids(&s, &tree).is_empty());
}

#[test]
fn a_ticket_naming_nothing_is_not_closable() {
    let s = store(&[task("a", "title = \"no crate here\"")]);
    assert!(ids(&s, &RENDER).is_empty());
}

#[test]
fn a_closed_ticket_is_not_closable() {
    let s = store(&[T("a", "task", "title = \"kaski-render\"", "done", "")]);
    assert!(ids(&s, &RENDER).is_empty());
}

#[test]
fn a_ticket_with_an_open_child_is_not_closable_and_one_with_a_closed_child_is() {
    let open = store(&[task("a", "title = \"kaski-render\""), task("c", "part_of = \"a\"")]);
    assert!(ids(&open, &RENDER).is_empty());
    let shut = store(&[
        task("a", "title = \"kaski-render\""),
        T("c", "task", "part_of = \"a\"", "done", ""),
    ]);
    assert_eq!(ids(&shut, &RENDER), ["a"]);
}

#[test]
fn a_ticket_an_open_ticket_needs_is_not_closable_and_one_a_closed_ticket_needed_is() {
    let open = store(&[task("a", "title = \"kaski-render\""), task("b", "needs = [\"a\"]")]);
    assert!(ids(&open, &RENDER).is_empty());
    let shut = store(&[
        task("a", "title = \"kaski-render\""),
        T("b", "task", "needs = [\"a\"]", "done", ""),
    ]);
    assert_eq!(ids(&shut, &RENDER), ["a"]);
}

#[test]
fn a_question_is_never_closable_since_his_answer_closes_it() {
    let s = store(&[T("a", "question", "title = \"kaski-render\"", "backlog", "")]);
    assert!(ids(&s, &RENDER).is_empty());
}

#[test]
fn a_ticket_somebody_holds_is_not_closable() {
    let s = store(&[T("a", "task", "title = \"kaski-render\"", "doing", "helper")]);
    assert!(ids(&s, &RENDER).is_empty());
}

/// The real registry files a backlog ticket under `coord` (`holder = "coord"` on its move), and that
/// ticket is nobody's work in hand: it is closable, where a ticket in `doing` is not.
#[test]
fn a_backlog_ticket_filed_under_a_holder_is_closable_and_one_in_hand_is_not() {
    let s = store(&[
        T(
            "filed",
            "task",
            "title = \"kaski-render\"",
            "backlog",
            "coord",
        ),
        T(
            "doing",
            "task",
            "title = \"kaski-render\"",
            "doing",
            "coord",
        ),
    ]);
    assert_eq!(ids(&s, &RENDER), ["filed"]);
}

#[test]
fn the_closing_state_is_the_kinds_one_closed_state_that_does_not_satisfy() {
    let s = store(&[]);
    assert_eq!(stale::closing_state(&s, "task").as_deref(), Ok("dropped"));
    assert_eq!(
        stale::closing_state(&s, "question").as_deref(),
        Ok("dropped")
    );
}

#[test]
fn a_kind_with_two_closing_states_or_none_or_no_kind_is_refused_by_name() {
    let s = store(&[]);
    for (kind, said) in [("twice", "several"), ("never", "no closed state"), ("nothing", "no kind")]
    {
        let e = stale::closing_state(&s, kind).unwrap_err();
        assert!(e.contains(kind) && e.contains(said), "{e}");
    }
}

#[test]
fn the_why_names_each_gone_name_with_its_commit() {
    let s = store(&[task("a", "title = \"kaski-render\"")]);
    let c = &stale::closable(&s, &RENDER)[0];
    let why = stale::why(c);
    assert!(
        why.contains("`kaski-render`") && why.contains("ef19a8e0c"),
        "{why}"
    );
}
