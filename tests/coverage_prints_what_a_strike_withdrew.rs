//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What the report prints about a struck row, and about a demand row struck
//! itself.
//!
//! The walk's side is in `coverage_reads_a_struck_row_as_struck`. These arms
//! read the text, because a walk that files a strike correctly and a report
//! that prints it as live pass every arm that only reads the walk.

mod coverage_harness;
mod struck_fixtures;

use coverage_harness::{DEMAND, NS, run, tier, unstamped, view};
use mockspace_extra_lints::tools::coverage::{Reach, struck_demand, tally};
use mockspace_lint_rules::RegistryView;
use struck_fixtures::{STUCK, WITHDRAWN, stamp, struck_proposal};

// ---------------------------------------------------------------------------
// A struck row
// ---------------------------------------------------------------------------

#[test]
fn the_report_names_a_struck_row_once_and_says_what_it_withdrew() {
    let (_, out) = run(&struck_proposal("a_strike"), &[NS]);
    let line = out
        .lines()
        .find(|l| l.contains("proposal::a_claim"))
        .unwrap_or_else(|| panic!("the struck row is not printed:\n{out}"));
    assert!(line.contains("struck by retirement::a_strike"), "{line}");
    assert!(line.contains("so it sets no tier"), "{line}");
    assert_eq!(line.matches("struck").count(), 1, "said twice: {line}");
    assert!(
        out.contains(WITHDRAWN),
        "the withdrawn summary is missing:\n{out}"
    );
}

#[test]
fn the_report_prints_a_withdrawn_stamp_and_a_live_stamper_on_a_struck_proposal() {
    let (_, out) = run(&stamp("a_strike", ""), &[NS]);
    assert!(
        out.contains(
            "ruling::he_said_so   (struck by retirement::a_strike, through `ratifies` on \
             proposal::a_claim, so it stamps nothing)"
        ),
        "{out}"
    );
    let (_, one) = run(&stamp("", "a_strike"), &[NS, "the_thing"]);
    assert!(
        one.contains("even though ruling::he_said_so still stamps it"),
        "{one}"
    );
    // The control: live, the stamper is printed as the stamp and nothing is
    // struck.
    let (_, live) = run(&stamp("", ""), &[NS]);
    assert!(live.contains("(stamped by ruling::he_said_so)"), "{live}");
    assert!(!live.contains("struck"), "{live}");
}

#[test]
fn the_one_row_report_does_not_say_nothing_names_a_row_it_then_lists_namers_of() {
    let (_, one) = run(&struck_proposal("a_strike"), &[NS, "the_thing"]);
    assert!(one.contains("No live row this can tier names it."), "{one}");
    assert!(!one.contains("Nothing names it"), "{one}");
    assert!(one.contains("Also named by rows since struck:"), "{one}");
    assert!(
        one.contains("proposal::a_claim   (struck by retirement::a_strike"),
        "{one}"
    );
}

#[test]
fn control_the_report_over_a_live_proposal_mentions_no_strike() {
    let (_, out) = run(&unstamped(), &[NS]);
    assert!(!out.contains("struck"), "{out}");
    let (_, one) = run(&unstamped(), &[NS, "the_thing"]);
    assert!(!one.contains("struck"), "{one}");
}

#[test]
fn a_row_named_by_a_struck_row_and_a_live_one_is_not_in_the_withdrawn_summary() {
    let v = view(&[
        DEMAND,
        ("proposal::live", &[("obligation", "the_thing")]),
        ("proposal::a_claim", &[
            ("obligation", "the_thing"),
            ("retired", "a_strike"),
        ]),
    ]);
    let (_, out) = run(&v, &[NS]);
    assert!(out.contains("so it sets no tier"), "{out}");
    assert!(!out.contains(WITHDRAWN), "{out}");
}

// ---------------------------------------------------------------------------
// A demand row struck itself
// ---------------------------------------------------------------------------

/// The demand row, carrying `retired`.
const STRUCK_DEMAND: (&str, &[(&str, &str)]) = ("obligation::the_thing", &[
    ("what", "a demand"),
    ("retired", "a_strike"),
]);

/// That row with whatever the arm plants beside it.
fn struck_demand_row(
    also: &[(&'static str, &'static [(&'static str, &'static str)])],
) -> RegistryView {
    let mut rows = vec![STRUCK_DEMAND];
    rows.extend_from_slice(also);
    view(&rows)
}

#[test]
fn a_struck_demand_row_is_marked_as_owed_nothing_in_both_reports() {
    let v = struck_demand_row(&[]);
    assert_eq!(
        struck_demand(&v, NS).get("the_thing").map(String::as_str),
        Some("a_strike")
    );
    let (_, out) = run(&v, &[NS]);
    assert!(
        out.contains("the_thing   (struck by retirement::a_strike, so it is owed nothing)"),
        "{out}"
    );
    assert!(out.contains("carry `retired` themselves"), "{out}");
    let (_, one) = run(&v, &[NS, "the_thing"]);
    assert!(
        one.contains("struck by retirement::a_strike, so it is owed nothing"),
        "{one}"
    );
}

#[test]
fn a_struck_demand_row_is_still_tallied_at_the_tier_its_edges_reach() {
    let v = struck_demand_row(&[("ruling::he_said_so", &[
        ("rung", "ratified"),
        ("obligation", "the_thing"),
    ])]);
    assert_eq!(tier(&v), Reach::Ratified);
    assert_eq!(tally(&v, NS)["ratified"], 1);
    let (_, out) = run(&v, &[NS]);
    assert!(
        out.contains("so it is owed nothing"),
        "the mark depends on the tier:\n{out}"
    );
}

#[test]
fn a_struck_demand_row_is_in_neither_outstanding_summary() {
    let neighbours: [(&str, &[(&str, &str)]); 2] = [
        ("proposal::a_claim", &[
            ("obligation", "the_thing"),
            ("retired", "b_strike"),
        ]),
        ("law::a_result", &[("precondition_for", "the_thing")]),
    ];
    let (_, out) = run(&struck_demand_row(&neighbours), &[NS]);
    assert!(!out.contains(WITHDRAWN), "{out}");
    assert!(!out.contains(STUCK), "{out}");
    // The control: the same neighbours around a live demand row put it in both.
    let mut live = vec![DEMAND];
    live.extend_from_slice(&neighbours);
    let (_, out) = run(&view(&live), &[NS]);
    assert!(out.contains(WITHDRAWN), "{out}");
    assert!(out.contains(STUCK), "{out}");
}

#[test]
fn control_a_live_demand_row_carries_no_mark() {
    let (_, out) = run(&unstamped(), &[NS]);
    assert!(!out.contains("owed nothing"), "{out}");
    assert!(struck_demand(&unstamped(), NS).is_empty());
    let blank = view(&[("obligation::the_thing", &[
        ("what", "a demand"),
        ("retired", " "),
    ])]);
    assert!(
        struck_demand(&blank, NS).is_empty(),
        "blank is absent here too"
    );
}
