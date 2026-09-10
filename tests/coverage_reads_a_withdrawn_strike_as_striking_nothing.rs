//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! A retirement of kind `withdrawn` is a strike that was itself taken back.
//!
//! Its claim stands, so it closes no route and a row naming it in `retired` is
//! struck by nothing. Every arm plants the same registry under `withdrawn` and
//! under a kind that does strike, because a walk that never reads the kind
//! prints the second report for both.

mod coverage_harness;

use coverage_harness::{DEMAND, NS, run, tier, unstamped, view};
use mockspace_extra_lints::tools::coverage::{Reach, struck, struck_demand, tally};
use mockspace_lint_rules::RegistryView;

/// The kinds a retirement strikes with.
const KINDS_THAT_STRIKE: [&str; 5] =
    ["wrong", "superseded", "unpayable", "misattributed", "unpredicated"];

/// A demand row named only by a retirement of the given kind.
fn closed_by(kind: &str) -> RegistryView {
    view(&[
        DEMAND,
        ("retirement::a_dead_end", &[
            ("obligation", "the_thing"),
            ("kind", kind),
        ]),
    ])
}

/// A demand row named by a live proposal that names a retirement of the given
/// kind in `retired`.
fn named_by_a_proposal_retired_by(kind: &str) -> RegistryView {
    view(&[
        DEMAND,
        ("proposal::a_claim", &[
            ("obligation", "the_thing"),
            ("retired", "the_strike"),
        ]),
        ("retirement::the_strike", &[
            ("claim", "what it struck"),
            ("kind", kind),
        ]),
    ])
}

/// How many struck edges land on `the_thing`, where the map carries it at all.
fn struck_edges(v: &RegistryView) -> usize {
    struck(v, NS).get("the_thing").map_or(0, Vec::len)
}

// ---------------------------------------------------------------------------
// The route a retirement closes
// ---------------------------------------------------------------------------

#[test]
fn a_withdrawn_retirement_closes_no_route() {
    assert_eq!(tier(&closed_by("withdrawn")), Reach::Nothing);
    assert_eq!(tier(&closed_by("  withdrawn ")), Reach::Nothing, "padded");
}

#[test]
fn control_every_other_kind_still_closes_the_route() {
    // `Withdrawn` is not the value, and an empty kind is no kind.
    for kind in KINDS_THAT_STRIKE.into_iter().chain(["Withdrawn", ""]) {
        assert_eq!(tier(&closed_by(kind)), Reach::RouteClosed, "{kind:?}");
    }
}

#[test]
fn control_a_retirement_with_no_kind_closes_the_route() {
    let v = view(&[DEMAND, ("retirement::a_dead_end", &[("obligation", "the_thing")])]);
    assert_eq!(tier(&v), Reach::RouteClosed);
}

#[test]
fn the_tally_counts_a_row_named_only_by_a_withdrawn_strike_at_nothing() {
    let t = tally(&closed_by("withdrawn"), NS);
    assert_eq!(t[&Reach::Nothing.word()], 1, "{t:?}");
    assert_eq!(t[&Reach::RouteClosed.word()], 0, "{t:?}");
    let t = tally(&closed_by("superseded"), NS);
    assert_eq!(t[&Reach::RouteClosed.word()], 1, "the control: {t:?}");
}

#[test]
fn the_report_over_a_withdrawn_strike_is_the_report_over_one_naming_another_row() {
    // The field is still carried, so the baseline carries it too: over a bare
    // registry the report adds that no row carries the demand field at all.
    let (_, bare) = run(
        &view(&[DEMAND, ("retirement::a_dead_end", &[("obligation", "another_thing")])]),
        &[NS],
    );
    let (_, out) = run(&closed_by("withdrawn"), &[NS]);
    assert_eq!(out, bare);
    let (_, out) = run(&closed_by("superseded"), &[NS]);
    assert_ne!(out, bare, "the control");
}

// ---------------------------------------------------------------------------
// A row naming the retirement in `retired`
// ---------------------------------------------------------------------------

#[test]
fn a_row_naming_a_withdrawn_strike_in_retired_is_struck_by_nothing() {
    // Read exactly as the same proposal carrying no `retired` at all.
    let v = named_by_a_proposal_retired_by("withdrawn");
    assert_eq!(tier(&v), tier(&unstamped()));
    assert_eq!(tier(&v), Reach::Proposed);
    assert_eq!(struck_edges(&v), 0);
    let (_, out) = run(&v, &[NS]);
    let (_, bare) = run(&unstamped(), &[NS]);
    assert_eq!(out, bare);
}

#[test]
fn control_a_row_naming_any_other_strike_in_retired_is_struck() {
    for kind in KINDS_THAT_STRIKE {
        let v = named_by_a_proposal_retired_by(kind);
        assert_eq!(tier(&v), Reach::Nothing, "{kind}");
        assert_eq!(struck_edges(&v), 1, "{kind}");
    }
}

#[test]
fn a_retired_field_naming_no_retirement_still_strikes() {
    // The reading this does not change: a name resolving to no row carries no
    // kind, so it is not a withdrawn strike and the row stays struck. Whether
    // the name resolves is the schema's report.
    let v = view(&[
        DEMAND,
        ("proposal::a_claim", &[
            ("obligation", "the_thing"),
            ("retired", "nowhere"),
        ]),
    ]);
    assert_eq!(tier(&v), Reach::Nothing);
    assert_eq!(struck_edges(&v), 1);
}

#[test]
fn a_demand_row_naming_a_withdrawn_strike_is_not_marked_struck() {
    let demand_retired_by = |kind: &str| {
        view(&[
            ("obligation::the_thing", &[
                ("what", "a demand"),
                ("retired", "the_strike"),
            ]),
            ("retirement::the_strike", &[
                ("claim", "what it struck"),
                ("kind", kind),
            ]),
        ])
    };
    let v = demand_retired_by("withdrawn");
    assert!(
        struck_demand(&v, NS).is_empty(),
        "{:?}",
        struck_demand(&v, NS)
    );
    let v = demand_retired_by("superseded");
    assert_eq!(struck_demand(&v, NS).len(), 1, "the control");
}
