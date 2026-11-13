//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Adding a ticket is judged here once, for the tickets tool that writes the file and for the console
//! that stages the rows: a request is refused by the rules, and a request they accept gives the ticket's
//! fields and its first move.

use mockspace_extra_ticket_model::rules::{self, NewRequest, slugify};
use mockspace_extra_ticket_model::{Fields, Store};

const TEXT: &str = r#"
[[ticket_state]]
id = "backlog"
column = 1

[[ticket_state]]
id = "doing"
column = 2
needs_holder = true

[[ticket_kind]]
id = "task"
states = ["backlog", "doing"]
initial = "backlog"

[[ticket]]
id = "taken"
title = "taken"
kind = "task"
branch = "work/taken"

[[ticket_move]]
id = "m1"
ticket = "taken"
to = "backlog"
by = "coord"
at = "2026-11-01T10:00:00Z"
"#;

fn store() -> Store {
    Store::from_texts([("registry.toml", TEXT)]).unwrap()
}

fn request(title: &str) -> NewRequest {
    NewRequest {
        title: title.into(),
        kind: "task".into(),
        by: "op".into(),
        at: "2026-11-08T10:00:00Z".into(),
        move_id: "m20261108100000_op_ab12".into(),
        ..NewRequest::default()
    }
}

fn with(mut r: NewRequest, key: &str, values: &[&str]) -> NewRequest {
    let mut f: Fields = r.fields.clone();
    f.insert(
        key.into(),
        values.iter().map(|v| (*v).to_string()).collect(),
    );
    r.fields = f;
    r
}

#[test]
fn a_title_is_a_slug_of_lowercase_words() {
    assert_eq!(slugify("Add a ticket, please!"), "add_a_ticket_please");
    assert_eq!(
        slugify("9 lives"),
        "t_9_lives",
        "an id starts with a letter"
    );
    let long = slugify(&"word ".repeat(30));
    assert!(long.len() <= 60 && !long.ends_with('_'), "{long}");
}

#[test]
fn a_request_the_rules_accept_gives_the_ticket_and_its_first_move() {
    let made = rules::new_ticket(&store(), &request("Add a ticket")).unwrap();
    assert_eq!(made.id, "add_a_ticket");
    assert_eq!(made.fields["title"], ["Add a ticket"]);
    assert_eq!(made.fields["kind"], ["task"]);
    assert_eq!(made.first.ticket, "add_a_ticket");
    assert_eq!(
        made.first.to, "backlog",
        "a ticket starts in its kind's initial state"
    );
    assert_eq!(
        (made.first.by.as_str(), made.first.at.as_str()),
        ("op", "2026-11-08T10:00:00Z")
    );
    assert_eq!(made.first.id, "m20261108100000_op_ab12");
}

#[test]
fn an_id_given_wins_over_the_title_and_must_be_free() {
    let mut r = request("Whatever");
    r.id = Some("mine".into());
    assert_eq!(rules::new_ticket(&store(), &r).unwrap().id, "mine");
    r.id = Some("taken".into());
    let err = rules::new_ticket(&store(), &r).unwrap_err();
    assert!(err.contains("`taken` is a ticket already"), "{err}");
    r.id = Some("Not A Slug".into());
    assert!(
        rules::new_ticket(&store(), &r)
            .unwrap_err()
            .contains("is not a slug")
    );
}

#[test]
fn a_ticket_with_no_title_is_refused() {
    for blank in ["", "   "] {
        let err = rules::new_ticket(&store(), &request(blank)).unwrap_err();
        assert!(err.contains("needs a title"), "{err}");
    }
}

#[test]
fn a_kind_the_registry_lacks_is_refused() {
    let mut r = request("x");
    r.kind = "epic".into();
    assert!(
        rules::new_ticket(&store(), &r)
            .unwrap_err()
            .contains("`epic` is no ticket kind")
    );
}

#[test]
fn a_state_that_needs_a_holder_is_refused_without_one_and_taken_with_one() {
    let mut r = request("Start at once");
    r.state = Some("doing".into());
    let err = rules::new_ticket(&store(), &r).unwrap_err();
    assert!(err.contains("`doing` needs a holder"), "{err}");
    r.holder = "op".into();
    let made = rules::new_ticket(&store(), &r).unwrap();
    assert_eq!(
        (made.first.to.as_str(), made.first.holder.as_str()),
        ("doing", "op")
    );
}

#[test]
fn a_rank_is_a_whole_number_and_written_plain() {
    let bad = with(request("x"), "rank", &["high"]);
    assert!(
        rules::new_ticket(&store(), &bad)
            .unwrap_err()
            .contains("is a whole number")
    );
    let good = with(request("y"), "rank", &["7"]);
    assert_eq!(
        rules::new_ticket(&store(), &good).unwrap().fields["rank"],
        ["7"]
    );
}

#[test]
fn needs_and_part_of_name_tickets_that_exist() {
    let r = with(request("x"), "needs", &["nowhere"]);
    assert!(
        rules::new_ticket(&store(), &r)
            .unwrap_err()
            .contains("`needs` names `nowhere`")
    );
    let r = with(request("y"), "part_of", &["taken"]);
    assert!(rules::new_ticket(&store(), &r).is_ok());
}

#[test]
fn a_branch_another_ticket_holds_is_refused() {
    let r = with(request("x"), "branch", &["work/taken"]);
    let err = rules::new_ticket(&store(), &r).unwrap_err();
    assert!(
        err.contains("work/taken") && err.contains("`taken`"),
        "{err}"
    );
}
