//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The rules over the planning rows: chains and forks, stamps, the shape of a span, and what is read
//! of the rows as they stand now.

use mockspace_extra_planning::{Plan, rules};

const NS: [&str; 5] = ["plan_span", "member", "window", "milestone_merge", "milestone_scope"];

fn plan(text: &str) -> Plan {
    Plan::from_texts(&NS, [("plan.toml", text)]).unwrap()
}

const SPANS: &str = r#"
[[plan_span]]
id = "stretch"
planned = false

[[plan_span]]
id = "epoch_a"
planned = true
within = "stretch"
owner = "coord"
"#;

fn rows_of(found: Vec<ticket_model::Finding>) -> Vec<String> {
    found.into_iter().map(|f| f.row).collect()
}

#[test]
fn a_single_legacy_member_row_per_ticket_is_clean() {
    let p = plan(&format!(
        "{SPANS}\n[[member]]\nid = \"m1\"\nticket = \"t\"\nspan = \"epoch_a\"\n"
    ));
    assert!(
        rules::check_plan(&p).is_empty(),
        "{:?}",
        rules::check_plan(&p)
    );
}

#[test]
fn two_rows_over_one_predecessor_are_a_fork_and_a_chain_is_not() {
    let base = "[[member]]\nid = \"m1\"\nticket = \"t\"\nspan = \"epoch_a\"\n";
    let fork = format!(
        "{SPANS}\n{base}\n[[member]]\nid = \"m2\"\nticket = \"t\"\nover = \"m1\"\ncore = false\n\n[[member]]\nid = \"m3\"\nticket = \"t\"\nover = \"m1\"\ncore = true\n"
    );
    let found = rules::check_forks(&plan(&fork));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].message.contains("m2") && found[0].message.contains("m3"),
        "{found:?}"
    );
    let chain = format!(
        "{SPANS}\n{base}\n[[member]]\nid = \"m2\"\nticket = \"t\"\nover = \"m1\"\ncore = false\n\n[[member]]\nid = \"m3\"\nticket = \"t\"\nover = \"m2\"\ncore = true\n"
    );
    assert!(rules::check_forks(&plan(&chain)).is_empty());
}

#[test]
fn over_must_name_a_row_of_the_same_ticket_and_must_not_run_in_a_circle() {
    let other_ticket = "[[member]]\nid = \"a\"\nticket = \"t1\"\n\n[[member]]\nid = \"b\"\nticket = \"t2\"\nover = \"a\"\n";
    let found = rules::check_over(&plan(other_ticket));
    assert_eq!(rows_of(found), ["member::b"]);
    let dangling = "[[member]]\nid = \"a\"\nticket = \"t1\"\nover = \"nowhere\"\n";
    assert_eq!(rows_of(rules::check_over(&plan(dangling))), ["member::a"]);
    let circle = "[[member]]\nid = \"a\"\nticket = \"t\"\nover = \"b\"\n\n[[member]]\nid = \"b\"\nticket = \"t\"\nover = \"a\"\n";
    assert_eq!(rules::check_over(&plan(circle)).len(), 2);
    let fine = "[[member]]\nid = \"a\"\nticket = \"t\"\n\n[[member]]\nid = \"b\"\nticket = \"t\"\nover = \"a\"\n";
    assert!(rules::check_over(&plan(fine)).is_empty());
}

#[test]
fn at_and_by_come_together_or_not_at_all() {
    let half = "[[window]]\nid = \"w\"\nticket = \"t\"\nat = \"2026-11-02T10:00:00Z\"\n";
    assert_eq!(rows_of(rules::check_stamp(&plan(half))), ["window::w"]);
    let both =
        "[[window]]\nid = \"w\"\nticket = \"t\"\nat = \"2026-11-02T10:00:00Z\"\nby = \"coord\"\n";
    assert!(rules::check_stamp(&plan(both)).is_empty());
    let neither = "[[window]]\nid = \"w\"\nticket = \"t\"\n";
    assert!(rules::check_stamp(&plan(neither)).is_empty());
}

#[test]
fn an_epoch_names_its_stretch_and_owner_and_a_stretch_names_neither() {
    assert!(rules::check_spans(&plan(SPANS)).is_empty());
    let no_owner = "[[plan_span]]\nid = \"s\"\nplanned = false\n\n[[plan_span]]\nid = \"e\"\nplanned = true\nwithin = \"s\"\n";
    assert_eq!(rows_of(rules::check_spans(&plan(no_owner))), [
        "plan_span::e"
    ]);
    let no_stretch = "[[plan_span]]\nid = \"e\"\nplanned = true\nowner = \"coord\"\n";
    assert_eq!(rows_of(rules::check_spans(&plan(no_stretch))), [
        "plan_span::e"
    ]);
    let within_an_epoch = format!(
        "{SPANS}\n[[plan_span]]\nid = \"nested\"\nplanned = true\nwithin = \"epoch_a\"\nowner = \"coord\"\n"
    );
    let found = rules::check_spans(&plan(&within_an_epoch));
    assert_eq!(
        rows_of(found),
        ["plan_span::nested"],
        "an epoch does not run inside an epoch"
    );
    let stretch_with_owner = "[[plan_span]]\nid = \"s\"\nplanned = false\nowner = \"coord\"\n";
    assert_eq!(rows_of(rules::check_spans(&plan(stretch_with_owner))), [
        "plan_span::s"
    ]);
    let epoch_with_ends = format!(
        "{SPANS}\n[[plan_span]]\nid = \"bad\"\nplanned = true\nwithin = \"stretch\"\nowner = \"coord\"\nafter = \"m1\"\n"
    );
    assert_eq!(rows_of(rules::check_spans(&plan(&epoch_with_ends))), [
        "plan_span::bad"
    ]);
}

#[test]
fn members_now_reads_the_newest_head_and_an_absent_span_means_it_left() {
    let text = format!(
        "{SPANS}\n[[member]]\nid = \"m1\"\nticket = \"t\"\nspan = \"epoch_a\"\n\n[[member]]\nid = \"m2\"\nticket = \"t\"\nover = \"m1\"\ncore = false\nspan = \"epoch_a\"\nat = \"2026-11-02T10:00:00Z\"\nby = \"coord\"\n\n[[member]]\nid = \"m3\"\nticket = \"u\"\nspan = \"epoch_a\"\n\n[[member]]\nid = \"m4\"\nticket = \"u\"\nover = \"m3\"\nat = \"2026-11-02T11:00:00Z\"\nby = \"coord\"\n"
    );
    let now = plan(&text).members_now();
    assert_eq!(now["t"], ("epoch_a".to_string(), false));
    assert_eq!(
        now["u"].0, "",
        "a member row with no span is a ticket that left"
    );
}

#[test]
fn a_milestone_is_reached_only_where_a_merge_row_names_it() {
    let p = plan("[[milestone_merge]]\nid = \"mm\"\nmilestone = \"m1\"\n");
    assert!(p.reached("m1"));
    assert!(!p.reached("m2"));
}

#[test]
fn a_row_with_no_id_a_twice_held_id_and_a_text_that_is_not_toml_are_refused() {
    assert!(Plan::from_texts(&NS, [("f.toml", "[[member]]\nticket = \"t\"\n")]).is_err());
    assert!(
        Plan::from_texts(&NS, [(
            "f.toml",
            "[[member]]\nid = \"a\"\n[[member]]\nid = \"a\"\n"
        )])
        .is_err()
    );
    assert!(Plan::from_texts(&NS, [("f.toml", "[[member]\n")]).is_err());
    let p = Plan::from_texts(&NS, [("f.toml", "[[ticket]]\nid = \"x\"\n")]).unwrap();
    assert!(p.rows.is_empty(), "a namespace not asked for adds nothing");
}
