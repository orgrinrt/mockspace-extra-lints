//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! A branch names one job: no two tickets hold the same `branch`, found by the store, refused by
//! `check_branch` and named by `check_store` where a merge left two.

use mockspace_extra_ticket_model::{Store, rules};

const VOCABULARY: &str = r#"
[[ticket_state]]
id = "backlog"
column = 1

[[ticket_kind]]
id = "job"
states = ["backlog"]
initial = "backlog"
"#;

fn job(id: &str, branch: &str) -> String {
    format!(
        "[[ticket]]\nid = \"{id}\"\nkind = \"job\"\nbranch = \"{branch}\"\n\n[[ticket_move]]\nid = \"m_{id}\"\nticket = \"{id}\"\nto = \"backlog\"\nby = \"coord\"\nat = \"2026-10-28T10:00:00Z\"\n"
    )
}

fn store(jobs: &[(&str, &str)]) -> Store {
    let texts: Vec<(String, String)> = jobs
        .iter()
        .map(|(id, branch)| (format!("items/{id}.toml"), job(id, branch)))
        .chain([("vocabulary.toml".to_string(), VOCABULARY.to_string())])
        .collect();
    Store::from_texts(texts.iter().map(|(p, s)| (p.as_str(), s.as_str()))).unwrap()
}

#[test]
fn the_store_gives_the_tickets_holding_a_branch() {
    let s = store(&[("one", "work/one"), ("two", "work/two")]);
    let held: Vec<&str> = s.with_branch("work/two").map(|t| t.id.as_str()).collect();
    assert_eq!(held, ["two"]);
    assert_eq!(s.with_branch("work/none").count(), 0);
    assert_eq!(s.with_branch("").count(), 0, "no branch is held by nobody");
}

#[test]
fn a_ticket_whose_branch_another_holds_is_refused_and_one_with_its_own_is_not() {
    let s = store(&[("one", "work/one"), ("two", "work/one")]);
    let err = rules::check_branch(&s, "two").unwrap_err();
    assert!(err.contains("work/one") && err.contains("`one`"), "{err}");
    let alone = store(&[("one", "work/one"), ("two", "work/two")]);
    assert_eq!(rules::check_branch(&alone, "two"), Ok(()));
}

#[test]
fn a_ticket_with_no_branch_is_never_refused_for_it() {
    let s = store(&[("one", ""), ("two", "")]);
    assert_eq!(rules::check_branch(&s, "one"), Ok(()));
    assert!(rules::check_store(&s).is_empty());
}

#[test]
fn the_check_names_every_branch_two_tickets_hold_once_on_the_later_ticket() {
    let s = store(&[("a", "work/x"), ("b", "work/x"), ("c", "work/y")]);
    let found = rules::check_store(&s);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].row, "ticket::b");
    assert!(
        found[0].message.contains("work/x") && found[0].message.contains("`a`"),
        "{found:?}"
    );
}

#[test]
fn a_branch_is_held_once_the_next_holder_is_named_by_the_one_before() {
    let s = store(&[("a", "work/x"), ("b", "work/x"), ("c", "work/x")]);
    let rows: Vec<String> = rules::check_store(&s).into_iter().map(|f| f.row).collect();
    assert_eq!(rows, ["ticket::b", "ticket::c"]);
}

fn set_job(id: &str, branch: &str, set: &str) -> String {
    let extra = if set.is_empty() { String::new() } else { format!("set = \"{set}\"\n") };
    job(id, branch).replacen("kind = \"job\"\n", &format!("kind = \"job\"\n{extra}"), 1)
}

fn set_store(jobs: &[(&str, &str, &str)]) -> Store {
    let texts: Vec<(String, String)> = jobs
        .iter()
        .map(|(id, branch, set)| (format!("items/{id}.toml"), set_job(id, branch, set)))
        .chain([("vocabulary.toml".to_string(), VOCABULARY.to_string())])
        .collect();
    Store::from_texts(texts.iter().map(|(p, s)| (p.as_str(), s.as_str()))).unwrap()
}

#[test]
fn tickets_of_one_set_may_hold_one_branch_and_the_check_is_silent() {
    let s = set_store(&[("a", "work/x", "s"), ("b", "work/x", "s"), ("c", "work/x", "s")]);
    assert_eq!(rules::check_branch(&s, "b"), Ok(()));
    assert!(
        rules::check_store(&s).is_empty(),
        "{:?}",
        rules::check_store(&s)
    );
}

#[test]
fn a_ticket_of_another_set_or_of_none_is_refused_a_branch_a_set_holds() {
    let other = set_store(&[("a", "work/x", "s"), ("b", "work/x", "t")]);
    assert!(rules::check_branch(&other, "b").is_err());
    assert_eq!(rules::check_store(&other).len(), 1);
    let none = set_store(&[("a", "work/x", "s"), ("b", "work/x", "")]);
    assert!(rules::check_branch(&none, "b").is_err());
    let bare = set_store(&[("a", "work/x", ""), ("b", "work/x", "")]);
    assert!(
        rules::check_branch(&bare, "b").is_err(),
        "no set is not a shared set"
    );
}

#[test]
fn a_third_ticket_outside_the_set_is_named_though_two_of_the_set_share() {
    let s = set_store(&[("a", "work/x", "s"), ("b", "work/x", "s"), ("c", "work/x", "")]);
    let rows: Vec<String> = rules::check_store(&s).into_iter().map(|f| f.row).collect();
    assert_eq!(rows, ["ticket::c"]);
}
