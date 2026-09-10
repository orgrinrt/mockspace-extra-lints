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
use mockspace_extra_lints::tools::coverage::{
    Reach,
    also_named_by,
    preconditions,
    struck,
    struck_demand,
    tally,
    withdrawn_namers,
};
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
fn withdrawn_namers_lists_only_a_withdrawn_retirement() {
    assert_eq!(
        withdrawn_namers(&closed_by("withdrawn"), NS)["the_thing"],
        vec!["retirement::a_dead_end".to_string()]
    );
    for kind in KINDS_THAT_STRIKE.into_iter().chain(["Withdrawn", ""]) {
        let v = closed_by(kind);
        assert!(withdrawn_namers(&v, NS)["the_thing"].is_empty(), "{kind:?}");
    }
}

#[test]
fn the_report_prints_a_withdrawn_retirement_under_its_row_without_a_tier() {
    // Silence would be wrong in the direction a reader cannot see: the edge
    // exists, and a report that drops it names the row as named by nothing.
    let (_, out) = run(&closed_by("withdrawn"), &[NS]);
    assert!(out.contains("  nothing       the_thing\n"), "{out}");
    assert!(
        out.contains("retirement::a_dead_end, withdrawn, so it closes no route"),
        "{out}"
    );
    assert!(!out.contains("named only by a retirement"), "{out}");
    let (_, out) = run(&closed_by("superseded"), &[NS]);
    assert!(
        out.contains("  route-closed  the_thing\n"),
        "the control: {out}"
    );
    assert!(!out.contains("closes no route"), "the control: {out}");
}

#[test]
fn the_one_row_report_names_a_withdrawn_retirement_apart_from_what_tiers_it() {
    let (_, out) = run(&closed_by("withdrawn"), &[NS, "the_thing"]);
    assert!(out.contains("tier: nothing"), "{out}");
    assert!(out.contains("No live row this can tier names it."), "{out}");
    assert!(
        out.contains(
            "Also named by withdrawn retirements, which close no route:\n    \
             retirement::a_dead_end\n"
        ),
        "{out}"
    );
    let (_, out) = run(&closed_by("superseded"), &[NS, "the_thing"]);
    assert!(out.contains("tier: route-closed"), "the control: {out}");
    assert!(!out.contains("withdrawn retirements"), "the control: {out}");
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

// ---------------------------------------------------------------------------
// The other readers of `retired`, each through the same function
// ---------------------------------------------------------------------------

#[test]
fn a_ruling_naming_a_withdrawn_strike_in_retired_still_stamps() {
    let stamped_by_a_ruling_retired_by = |kind: &str| {
        view(&[
            DEMAND,
            ("proposal::a_claim", &[("obligation", "the_thing")]),
            ("ruling::the_stamp", &[
                ("rung", "ratified"),
                ("ratifies", "a_claim"),
                ("retired", "the_strike"),
            ]),
            ("retirement::the_strike", &[
                ("claim", "what it struck"),
                ("kind", kind),
            ]),
        ])
    };
    assert_eq!(
        tier(&stamped_by_a_ruling_retired_by("withdrawn")),
        Reach::Ratified
    );
    assert_eq!(
        tier(&stamped_by_a_ruling_retired_by("superseded")),
        Reach::Proposed,
        "the control"
    );
}

#[test]
fn a_row_from_a_namespace_this_cannot_tier_naming_a_withdrawn_strike_is_listed() {
    let named_from_elsewhere_retired_by = |kind: &str| {
        view(&[
            DEMAND,
            ("law::a_result", &[
                ("obligation", "the_thing"),
                ("retired", "the_strike"),
            ]),
            ("retirement::the_strike", &[
                ("claim", "what it struck"),
                ("kind", kind),
            ]),
        ])
    };
    assert_eq!(
        also_named_by(&named_from_elsewhere_retired_by("withdrawn"), NS)["the_thing"],
        vec!["law::a_result".to_string()]
    );
    let v = named_from_elsewhere_retired_by("superseded");
    assert!(also_named_by(&v, NS)["the_thing"].is_empty(), "the control");
}

#[test]
fn a_precondition_naming_a_withdrawn_strike_in_retired_is_still_established() {
    let established_retired_by = |kind: &str| {
        view(&[
            DEMAND,
            ("law::a_result", &[
                ("precondition_for", "the_thing"),
                ("retired", "the_strike"),
            ]),
            ("retirement::the_strike", &[
                ("claim", "what it struck"),
                ("kind", kind),
            ]),
        ])
    };
    let v = established_retired_by("withdrawn");
    assert_eq!(preconditions(&v, NS)["the_thing"].len(), 1);
    let v = established_retired_by("superseded");
    assert!(preconditions(&v, NS)["the_thing"].is_empty(), "the control");
}
