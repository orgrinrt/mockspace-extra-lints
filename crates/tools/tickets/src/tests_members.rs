//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The planning layer through the tool: a member row appended by `--span` and `--core`, naming the
//! row it replaces; the listing's epoch tag read from the newest member row; the priority and the
//! set a ticket holds; and the findings of the planning rules in the check.

use kaski_planning::rules;

use crate::args;
use crate::testkit::Scratch;
use crate::view::{self, Filter};

fn file(s: &Scratch, id: &str) -> String {
    s.read(&format!("ticket/items/{id}.toml"))
}

fn plan(s: &Scratch) -> kaski_planning::Plan {
    crate::load_plan(&s.root).unwrap()
}

fn listed(s: &Scratch, filter: &Filter) -> Vec<String> {
    let store = crate::load(&s.root).unwrap();
    view::list(&store, &plan(s), filter)
        .iter()
        .map(|r| view::line(r, false))
        .collect()
}

fn flags(words: &[&str]) -> args::ListRequest {
    let mut all = vec!["list"];
    all.extend_from_slice(words);
    args::tickets(&all).unwrap()
}

#[test]
fn a_span_appends_a_member_row_and_the_next_row_names_the_one_it_replaces() {
    let s = Scratch::new().with_spans();
    let id = s.add("Work", &[]).unwrap();
    s.member(&id, Some("plan"), None, "2026-11-02T10:00:00Z")
        .unwrap();
    let first = plan(&s);
    assert_eq!(first.rows_of("member").count(), 1);
    let one = first.rows_of("member").next().unwrap().clone();
    assert_eq!(one.one("span"), "plan");
    assert_eq!(one.one("over"), "", "the first row replaces nothing");
    assert_eq!(one.one("by"), "tester");
    s.member(&id, Some("fixes"), Some(false), "2026-11-02T11:00:00Z")
        .unwrap();
    let both = plan(&s);
    let now = both.members_now();
    assert_eq!(now[&id], ("fixes".to_string(), false));
    let heads = both.heads("member", "ticket", &id);
    assert_eq!(heads.len(), 1);
    assert_eq!(heads[0].one("over"), one.id);
    assert!(
        rules::check_plan(&both).is_empty(),
        "{:?}",
        rules::check_plan(&both)
    );
}

#[test]
fn a_core_alone_keeps_the_span_and_an_empty_span_lets_the_ticket_leave() {
    let s = Scratch::new().with_spans();
    let id = s.add("Work", &[]).unwrap();
    s.member(&id, Some("plan"), None, "2026-11-02T10:00:00Z")
        .unwrap();
    s.member(&id, None, Some(false), "2026-11-02T11:00:00Z")
        .unwrap();
    assert_eq!(plan(&s).members_now()[&id], ("plan".to_string(), false));
    s.member(&id, Some(""), None, "2026-11-02T12:00:00Z")
        .unwrap();
    assert_eq!(plan(&s).members_now()[&id].0, "");
}

#[test]
fn a_span_that_is_no_row_is_refused_and_the_file_is_as_it_was() {
    let s = Scratch::new().with_spans();
    let id = s.add("Work", &[]).unwrap();
    let before = file(&s, &id);
    let err = s
        .member(&id, Some("ghost"), None, "2026-11-02T10:00:00Z")
        .unwrap_err();
    assert!(err.contains("`ghost`") && err.contains("no span"), "{err}");
    assert_eq!(file(&s, &id), before);
}

#[test]
fn a_ticket_whose_member_rows_fork_is_not_written_over_until_they_are_settled() {
    let s = Scratch::new().with_spans();
    let id = s.add("Work", &[]).unwrap();
    s.member(&id, Some("plan"), None, "2026-11-02T10:00:00Z")
        .unwrap();
    let head = plan(&s).rows_of("member").next().unwrap().id.clone();
    for (n, core) in [("a", "true"), ("b", "false")] {
        s.append(
            &format!("ticket/items/{id}.toml"),
            &format!("\n[[member]]\nid = \"fork_{n}\"\nticket = \"{id}\"\nover = \"{head}\"\ncore = {core}\n"),
        );
    }
    let found = rules::check_forks(&plan(&s));
    assert_eq!(found.len(), 1, "{found:?}");
    let err = s
        .member(&id, Some("fixes"), None, "2026-11-02T12:00:00Z")
        .unwrap_err();
    assert!(err.contains("two writers"), "{err}");
}

#[test]
fn the_listing_tags_an_epoch_and_not_a_stretch_and_filters_by_it_and_by_none() {
    let s = Scratch::new().with_spans();
    let a = s.add("Alpha", &[]).unwrap();
    let b = s.add("Beta", &[]).unwrap();
    s.add("Gamma", &[]).unwrap();
    s.member(&a, Some("plan"), None, "2026-11-02T10:00:00Z")
        .unwrap();
    s.member(&b, Some("stretch"), None, "2026-11-02T10:00:00Z")
        .unwrap();
    let all = listed(&s, &Filter::default());
    assert!(
        all.iter().any(|l| l.ends_with("Alpha [epoch plan]")),
        "{all:?}"
    );
    assert!(
        all.iter().any(|l| l.ends_with("Beta")),
        "a stretch is no epoch tag: {all:?}"
    );
    let plan_only = listed(&s, &flags(&["--epoch", "plan"]).filter);
    assert_eq!(plan_only.len(), 1);
    assert!(plan_only[0].contains("Alpha"));
    let none = listed(&s, &flags(&["--epoch", "none"]).filter);
    assert_eq!(
        none.len(),
        2,
        "the stretch member and the unplaced one: {none:?}"
    );
}

#[test]
fn the_flags_carry_the_span_the_core_the_set_and_the_priority_and_refuse_what_is_retired() {
    let m = args::move_(&["t", "--span", "plan", "--core", "false"]).unwrap();
    let w = m.member.unwrap();
    assert_eq!((w.span.as_deref(), w.core), (Some("plan"), Some(false)));
    assert!(
        m.links.is_empty(),
        "membership is no link of the ticket's row"
    );
    assert!(args::move_(&["t", "--core", "maybe"]).is_err());
    let a = args::add(&["Title", "--set", "s", "--priority", "3"]).unwrap();
    assert_eq!(
        (a.fields["set"].clone(), a.fields["priority"].clone()),
        (vec!["s".to_string()], vec!["3".to_string()])
    );
    assert!(
        args::add(&["Title", "--epoch", "plan"]).is_err(),
        "the epoch left the ticket"
    );
    assert!(args::move_(&["t", "--pr", "x"]).is_err());
    assert!(
        args::move_(&["t"]).is_err(),
        "nothing to move, record or place"
    );
}

#[test]
fn a_priority_is_a_whole_number_written_plain_and_a_word_is_refused() {
    let s = Scratch::new();
    let id = s.add("Work", &[("priority", &["7"])]).unwrap();
    assert_eq!(
        crate::load(&s.root).unwrap().tickets[&id].one("priority"),
        "7"
    );
    assert!(s.add("Other", &[("priority", &["high"])]).is_err());
    let err = s
        .links(&id, "", None, &[("priority", &["soon"])])
        .unwrap_err();
    assert!(err.contains("whole number"), "{err}");
    s.links(&id, "", None, &[("priority", &["9"])]).unwrap();
    assert_eq!(
        crate::load(&s.root).unwrap().tickets[&id].one("priority"),
        "9"
    );
}

#[test]
fn two_tickets_of_one_set_add_with_one_branch_and_one_outside_it_is_refused() {
    let s = Scratch::new();
    s.add_as("One", "job", None, "", &[
        ("brief", &["b"]),
        ("branch", &["work/x"]),
        ("set", &["s"]),
    ])
    .unwrap();
    s.add_as("Two", "job", None, "", &[
        ("brief", &["b"]),
        ("branch", &["work/x"]),
        ("set", &["s"]),
    ])
    .unwrap();
    let err = s
        .add_as("Three", "job", None, "", &[
            ("brief", &["b"]),
            ("branch", &["work/x"]),
        ])
        .unwrap_err();
    assert!(err.contains("work/x"), "{err}");
}
