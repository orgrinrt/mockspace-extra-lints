//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The rows a planning edit appends: each continues the head of its chain, a fork is refused, and a
//! row that would break a rule of the plan is refused with the rule's sentence.

use mockspace_extra_planning::write::{
    Stamp,
    Value,
    epoch_fact,
    member_fact,
    milestone_fact,
    scope_fact,
    window_fact,
};
use mockspace_extra_planning::{Home, NAMESPACES, Plan};

fn plan(text: &str) -> Plan {
    Plan::from_texts(&NAMESPACES, [("p.toml", text)]).unwrap()
}

const BASE: &str = r#"
[[plan_span]]
id = "stretch"
title = "S"
what = "w"
planned = false

[[plan_span]]
id = "epoch"
title = "E"
what = "w"
planned = true
within = "stretch"
owner = "coord"

[[milestone]]
id = "m1"
title = "M1"

[[member]]
id = "pm1"
ticket = "t"
span = "epoch"
"#;

const STAMP: Stamp = Stamp {
    at:  "2026-11-02T10:15:30Z",
    by:  "console",
    tag: "ab12",
};

fn text(row: &mockspace_extra_planning::NewRow, field: &str) -> Option<String> {
    row.fields
        .iter()
        .find(|(k, _)| k == field)
        .and_then(|(_, v)| {
            match v {
                Value::Text(t) => Some(t.clone()),
                Value::Flag(f) => Some(f.to_string()),
            }
        })
}

#[test]
fn a_member_fact_continues_the_head_and_names_it_in_over() {
    let p = plan(BASE);
    let row = member_fact(&p, "t", Some("stretch"), false, STAMP).unwrap();
    assert_eq!(row.ns, "member");
    assert_eq!(row.id, "pm20261102101530_console_ab12");
    assert_eq!(row.home, Home::Ticket("t".into()));
    assert_eq!(text(&row, "over").as_deref(), Some("pm1"));
    assert_eq!(text(&row, "core").as_deref(), Some("false"));
    assert_eq!(
        (text(&row, "at").as_deref(), text(&row, "by").as_deref()),
        (Some("2026-11-02T10:15:30Z"), Some("console"))
    );
    // a ticket with no fact yet starts a chain: no `over`
    let first = member_fact(&p, "new", Some("epoch"), true, STAMP).unwrap();
    assert_eq!(text(&first, "over"), None);
    // leaving is a row with no span
    assert_eq!(
        text(&member_fact(&p, "t", None, true, STAMP).unwrap(), "span"),
        None
    );
}

#[test]
fn a_chain_that_is_a_fork_already_is_refused_and_so_is_a_span_that_does_not_exist() {
    let forked = format!(
        "{BASE}\n[[member]]\nid = \"pm2\"\nticket = \"t\"\nspan = \"stretch\"\nover = \"pm1\"\n\n[[member]]\nid = \"pm3\"\nticket = \"t\"\nspan = \"epoch\"\nover = \"pm1\"\n"
    );
    let err = member_fact(&plan(&forked), "t", Some("epoch"), true, STAMP).unwrap_err();
    assert!(err.contains("fork"), "{err}");
    // control: the unforked plan takes it
    assert!(member_fact(&plan(BASE), "t", Some("epoch"), true, STAMP).is_ok());
    let err = member_fact(&plan(BASE), "t", Some("ghost"), true, STAMP).unwrap_err();
    assert!(err.contains("ghost"), "{err}");
}

#[test]
fn a_window_takes_days_in_order_and_continues_its_chain() {
    let p = plan(&format!(
        "{BASE}\n[[window]]\nid = \"w1\"\nticket = \"t\"\nplanned_end = \"2026-11-09\"\n"
    ));
    let row = window_fact(&p, "t", "2026-11-03", "2026-11-10", STAMP).unwrap();
    assert_eq!(text(&row, "over").as_deref(), Some("w1"));
    assert_eq!(text(&row, "planned_start").as_deref(), Some("2026-11-03"));
    assert!(
        window_fact(&p, "t", "2026-11-10", "2026-11-03", STAMP)
            .unwrap_err()
            .contains("before")
    );
    assert!(
        window_fact(&p, "t", "next week", "", STAMP)
            .unwrap_err()
            .contains("not a date")
    );
    // an empty end is an unplanned end, not a field
    assert_eq!(
        text(
            &window_fact(&p, "t", "2026-11-03", "", STAMP).unwrap(),
            "planned_end"
        ),
        None
    );
}

#[test]
fn a_scope_fact_targets_a_milestone_that_exists_or_takes_the_ticket_out() {
    let p = plan(BASE);
    assert_eq!(
        text(
            &scope_fact(&p, "t", Some("m1"), STAMP).unwrap(),
            "milestone"
        )
        .as_deref(),
        Some("m1")
    );
    assert_eq!(
        text(&scope_fact(&p, "t", None, STAMP).unwrap(), "milestone"),
        None
    );
    assert!(scope_fact(&p, "t", Some("m9"), STAMP).is_err());
}

#[test]
fn an_epoch_names_a_stretch_and_an_owner_and_a_second_epoch_cannot_reuse_an_id() {
    let p = plan(BASE);
    let row = epoch_fact(
        &p,
        "fresh",
        "Fresh",
        "what",
        "stretch",
        "render-coord",
        "2026-11-03",
        "2026-11-20",
    )
    .unwrap();
    assert_eq!(row.home, Home::Plan("spans"));
    assert_eq!(text(&row, "planned").as_deref(), Some("true"));
    assert_eq!(text(&row, "owner").as_deref(), Some("render-coord"));
    assert!(
        epoch_fact(&p, "epoch", "Dup", "what", "stretch", "coord", "", "")
            .unwrap_err()
            .contains("held already")
    );
    assert!(epoch_fact(&p, "x", "X", "w", "ghost", "coord", "", "").is_err());
    assert!(epoch_fact(&p, "x", "X", "w", "stretch", "", "", "").is_err());
    assert!(
        epoch_fact(&p, "Bad Id", "X", "w", "stretch", "coord", "", "")
            .unwrap_err()
            .contains("slug")
    );
    // an epoch inside an epoch breaks the rule of the plan, and the rule's sentence comes back
    let err = epoch_fact(&p, "nested", "N", "w", "epoch", "coord", "", "").unwrap_err();
    assert!(err.contains("plan_span::nested"), "{err}");
}

#[test]
fn a_milestone_is_a_goalpost_with_an_optional_goal_and_day() {
    let p = plan(BASE);
    let row = milestone_fact(
        &p,
        "m2",
        "The second",
        "A place to live for an hour.",
        "2026-12-15",
    )
    .unwrap();
    assert_eq!(row.home, Home::Plan("milestones"));
    assert_eq!(
        text(&row, "goal").as_deref(),
        Some("A place to live for an hour.")
    );
    let bare = milestone_fact(&p, "m3", "The third", "", "").unwrap();
    assert_eq!(
        (text(&bare, "goal"), text(&bare, "planned_on")),
        (None, None)
    );
    assert!(milestone_fact(&p, "m1", "Dup", "", "").is_err());
    assert!(milestone_fact(&p, "m4", "", "", "").is_err());
    assert!(milestone_fact(&p, "m4", "T", "", "2026-02-30").is_err());
}
