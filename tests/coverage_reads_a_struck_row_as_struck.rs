//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! A struck row, kept whole as the record and bearing on nothing.
//!
//! A corpus that retires a row by marking it rather than deleting it keeps
//! every slug citing it resolvable, and coverage then has to read the mark or
//! it counts what the corpus withdrew. Every arm here plants the row struck and
//! live, because a strike that changes nothing and a strike the walk never
//! reads print the same report on a fixture that only plants one side.
//!
//! The stamp is here in both directions: a struck ruling whose stamp a live
//! proposal's tier rested on, and a struck proposal a live ruling still stamps.
//! Either one measured only by the tier passes on a walk that drops the stamp
//! on the floor, so each is also measured by what gets printed. What the
//! report prints of a struck row otherwise is in
//! `coverage_prints_what_a_strike_withdrew`.

mod coverage_harness;
mod struck_fixtures;

use coverage_harness::{DEMAND, NS, run, tier, unstamped, view};
use mockspace_extra_lints::tools::coverage::{
    Reach,
    StruckEdge,
    Through,
    also_named_by,
    preconditions,
    reach,
    stamps,
    struck,
    tally,
};
use mockspace_lint_rules::RegistryView;
use struck_fixtures::{STUCK, WITHDRAWN, stamp, struck_proposal};

/// Every struck edge onto `the_thing`.
fn on(v: &RegistryView) -> Vec<StruckEdge> {
    struck(v, NS)["the_thing"].clone()
}

fn edge(row: &str, by: &str, through: Through) -> StruckEdge {
    StruckEdge {
        row: row.to_string(),
        by: by.to_string(),
        through,
    }
}

fn answer() -> Through {
    Through::Answer {
        stamped_by: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// The walk
// ---------------------------------------------------------------------------

#[test]
fn a_struck_proposal_sets_no_tier() {
    assert_eq!(tier(&struck_proposal("a_strike")), Reach::Nothing);
    assert_eq!(tier(&struck_proposal("")), Reach::Proposed, "the control");
}

#[test]
fn a_struck_proposal_is_named_under_the_row_rather_than_dropped() {
    let v = struck_proposal("a_strike");
    assert_eq!(on(&v), [edge("proposal::a_claim", "a_strike", answer())]);
    assert!(reach(&v, NS)["the_thing"].1.is_empty());
}

#[test]
fn control_a_live_proposal_is_struck_by_nothing() {
    assert!(on(&unstamped()).is_empty());
}

#[test]
fn a_retired_field_present_and_blank_strikes_nothing() {
    for blank in ["", "   "] {
        let v = struck_proposal(blank);
        assert_eq!(tier(&v), Reach::Proposed, "`{blank}` read as a strike");
        assert!(on(&v).is_empty());
    }
}

#[test]
fn a_row_is_struck_whatever_namespace_it_sits_in() {
    // Each case: the row, the tier it sets live, and whether it is named from a
    // namespace this cannot tier. Struck, every one of them sets nothing and is
    // named in exactly one place.
    type Case = (
        &'static str,
        &'static [(&'static str, &'static str)],
        Reach,
        usize,
    );
    let cases: [Case; 3] = [
        (
            "ruling::he_said_so",
            &[("rung", "ratified"), ("obligation", "the_thing")],
            Reach::Ratified,
            0,
        ),
        (
            "retirement::a_dead_end",
            &[("obligation", "the_thing")],
            Reach::RouteClosed,
            0,
        ),
        (
            "law::a_result",
            &[("obligation", "the_thing")],
            Reach::Nothing,
            1,
        ),
    ];
    for (q, fields, live, others) in cases {
        let live_v = view(&[DEMAND, (q, fields)]);
        assert_eq!(tier(&live_v), live, "`{q}` live");
        assert_eq!(also_named_by(&live_v, NS)["the_thing"].len(), others);

        let mut with: Vec<(&str, &str)> = fields.to_vec();
        with.push(("retired", "a_strike"));
        let gone = view(&[DEMAND, (q, with.as_slice())]);
        assert_eq!(tier(&gone), Reach::Nothing, "`{q}` struck still tiers");
        assert!(also_named_by(&gone, NS)["the_thing"].is_empty(), "`{q}`");
        assert_eq!(on(&gone), [edge(q, "a_strike", answer())], "`{q}`");
    }
}

#[test]
fn a_struck_row_does_not_lower_what_a_live_one_reached() {
    let v = view(&[
        DEMAND,
        ("ruling::he_said_so", &[
            ("rung", "stated"),
            ("obligation", "the_thing"),
        ]),
        ("proposal::a_claim", &[
            ("obligation", "the_thing"),
            ("retired", "a_strike"),
        ]),
    ]);
    let r = reach(&v, NS);
    assert_eq!(r["the_thing"].0, Reach::Stated);
    assert_eq!(
        r["the_thing"].1.len(),
        1,
        "only the live ruling got it there"
    );
    assert_eq!(on(&v).len(), 1);
}

#[test]
fn a_struck_row_naming_through_both_fields_is_listed_once_per_field() {
    let v = view(&[
        DEMAND,
        ("proposal::a_claim", &[
            ("obligation", "the_thing"),
            ("precondition_for", "the_thing"),
            ("retired", "a_strike"),
        ]),
    ]);
    let e = on(&v);
    assert_eq!(e.len(), 2, "{e:?}");
    assert!(e.contains(&edge("proposal::a_claim", "a_strike", answer())));
    assert!(e.contains(&edge(
        "proposal::a_claim",
        "a_strike",
        Through::Precondition
    )));
}

// ---------------------------------------------------------------------------
// Preconditions
// ---------------------------------------------------------------------------

#[test]
fn a_struck_rows_precondition_establishes_none() {
    let v = |retired: &'static str| {
        view(&[
            DEMAND,
            ("proposal::a_result", &[
                ("precondition_for", "the_thing"),
                ("retired", retired),
            ]),
        ])
    };
    let gone = v("a_strike");
    assert!(preconditions(&gone, NS)["the_thing"].is_empty());
    assert_eq!(on(&gone), [edge(
        "proposal::a_result",
        "a_strike",
        Through::Precondition
    )]);
    assert_eq!(
        preconditions(&v(""), NS)["the_thing"].len(),
        1,
        "the control"
    );
}

#[test]
fn a_struck_precondition_does_not_put_a_row_on_the_stuck_list() {
    let v = view(&[
        DEMAND,
        ("proposal::a_result", &[
            ("precondition_for", "the_thing"),
            ("retired", "a_strike"),
        ]),
    ]);
    let (_, out) = run(&v, &[NS]);
    assert!(
        !out.contains(STUCK),
        "a struck precondition read as established:\n{out}"
    );
}

#[test]
fn a_struck_precondition_alone_does_not_put_a_row_in_the_withdrawn_summary() {
    // A withdrawn dependency is not a withdrawn answer, and the summary says it
    // was an answer that got withdrawn.
    let pre = view(&[
        DEMAND,
        ("proposal::a_result", &[
            ("precondition_for", "the_thing"),
            ("retired", "a_strike"),
        ]),
    ]);
    let (_, out) = run(&pre, &[NS]);
    assert!(!out.contains(WITHDRAWN), "{out}");
    // The control: the row struck as an answer is in it.
    let (_, out) = run(&struck_proposal("a_strike"), &[NS]);
    assert!(out.contains(WITHDRAWN), "{out}");
}

// ---------------------------------------------------------------------------
// The stamp, both ways round
// ---------------------------------------------------------------------------

#[test]
fn a_stamp_does_not_revive_a_struck_proposal_and_the_stamper_is_named() {
    let v = stamp("", "a_strike");
    assert_eq!(tier(&v), Reach::Nothing);
    assert_eq!(on(&v), [edge(
        "proposal::a_claim",
        "a_strike",
        Through::Answer {
            stamped_by: vec!["ruling::he_said_so".to_string()],
        }
    )]);
    // The control: live, the stamp governs and nothing is struck.
    let live = stamp("", "");
    assert_eq!(tier(&live), Reach::Ratified);
    assert!(on(&live).is_empty());
}

#[test]
fn a_struck_rulings_stamp_is_listed_where_the_tier_it_withdrew_would_have_shown() {
    let v = stamp("a_strike", "");
    assert!(stamps(&v).is_empty());
    assert_eq!(tier(&v), Reach::Proposed, "unstamped, not met");
    assert_eq!(on(&v), [edge(
        "ruling::he_said_so",
        "a_strike",
        Through::Stamp {
            proposal: "proposal::a_claim".to_string(),
        }
    )]);
}

#[test]
fn a_struck_stamp_on_a_struck_proposal_is_listed_and_is_not_a_live_stamper() {
    let e = on(&stamp("a_strike", "b_strike"));
    assert_eq!(e.len(), 2, "{e:?}");
    assert!(
        e.contains(&edge("proposal::a_claim", "b_strike", answer())),
        "a struck stamper named as a live one: {e:?}"
    );
    assert!(
        e.contains(&edge("ruling::he_said_so", "a_strike", Through::Stamp {
            proposal: "proposal::a_claim".to_string(),
        }))
    );
}

#[test]
fn control_a_struck_ruling_below_ratified_withdrew_no_stamp() {
    // It never stamped anything, since only a ratified ruling's stamp is one.
    for rung in ["stated", "in_force", "open"] {
        let v = view(&[
            DEMAND,
            ("ruling::he_said_so", &[
                ("rung", rung),
                ("ratifies", "a_claim"),
                ("retired", "a_strike"),
            ]),
            ("proposal::a_claim", &[("obligation", "the_thing")]),
        ]);
        assert!(on(&v).is_empty(), "`{rung}`: {:?}", on(&v));
        assert_eq!(tier(&v), Reach::Proposed, "`{rung}`");
    }
}

#[test]
fn striking_a_stamper_moves_its_stamp_from_the_live_line_to_the_struck_one() {
    // `reach` prints a stamp inside the stamped proposal's line rather than as
    // an edge of its own, so the edge count below cannot see it. Measured by the
    // stamper's name instead: live, it is printed as the stamper; struck, it is
    // gone from there and listed as the withdrawn stamp, once under every row
    // the proposal names.
    let stamper = "ruling::he_said_so";
    let v = |retired: &'static str| {
        view(&[
            ("obligation::first", &[("what", "one")]),
            ("obligation::second", &[("what", "two")]),
            ("ruling::he_said_so", &[
                ("rung", "ratified"),
                ("ratifies", "a_claim"),
                ("retired", retired),
            ]),
            ("proposal::a_claim", &[("obligation", "first, second")]),
        ])
    };
    let (live, gone) = (v(""), v("a_strike"));
    for row in ["first", "second"] {
        let printed = |r: &RegistryView| {
            reach(r, NS)[row]
                .1
                .iter()
                .filter(|l| l.contains(stamper))
                .count()
        };
        let withdrawn = struck(&gone, NS)[row]
            .iter()
            .filter(|e| e.row == stamper && matches!(e.through, Through::Stamp { .. }))
            .count();
        assert_eq!(printed(&live), 1, "`{row}` live");
        assert_eq!(printed(&gone), 0, "`{row}` struck still prints its stamper");
        assert_eq!(withdrawn, 1, "`{row}` lists no withdrawn stamp");
        assert!(struck(&live, NS)[row].is_empty(), "`{row}` control");
    }
}

// ---------------------------------------------------------------------------
// The accounting
// ---------------------------------------------------------------------------

/// Every edge naming `the_thing` the live side of the walk reads, in the demand
/// namespace given.
///
/// The three functions a struck row is kept out of, summed, which is what the
/// struck side has to account for edge for edge.
fn live_edges(v: &RegistryView, demand: &str) -> usize {
    reach(v, demand)["the_thing"].1.len()
        + also_named_by(v, demand)["the_thing"].len()
        + preconditions(v, demand)["the_thing"].len()
}

#[test]
fn striking_moves_every_answer_and_precondition_the_live_walk_reads_and_no_other() {
    // What `struck` claims about itself for these two fields: it collects what
    // the three live functions would have read. Planted from every source
    // namespace against every demand namespace, the demand's own included,
    // because which namespaces have the demand field read depends on both. A
    // namespace this tiers is read whatever the demand is, and one it does not
    // tier is read unless it is the demand's own.
    const WHAT: &[(&str, &str)] = &[("what", "a demand")];
    let demands = ["obligation", "ruling", "proposal", "retirement"];
    let sources = ["ruling", "proposal", "retirement", "law", "obligation", "probe"];
    for demand in demands {
        let target = format!("{demand}::the_thing");
        for source in sources {
            let row = format!("{source}::a_source");
            let fields: Vec<(&str, &str)> = match source {
                "probe" => vec![("what", "names nothing")],
                "ruling" => {
                    vec![
                        ("rung", "in_force"),
                        (demand, "the_thing"),
                        ("precondition_for", "the_thing"),
                    ]
                },
                _ => vec![(demand, "the_thing"), ("precondition_for", "the_thing")],
            };
            let mut with = fields.clone();
            with.push(("retired", "a_strike"));
            let live_v = view(&[(target.as_str(), WHAT), (row.as_str(), fields.as_slice())]);
            let gone = view(&[(target.as_str(), WHAT), (row.as_str(), with.as_slice())]);
            assert_eq!(
                struck(&gone, demand)["the_thing"].len(),
                live_edges(&live_v, demand),
                "`{row}` against `{demand}`: struck and live disagree on how many edges it carries"
            );
            assert_eq!(
                live_edges(&gone, demand),
                0,
                "`{row}` against `{demand}`: struck is still read as live"
            );
        }
    }
}

#[test]
fn the_tally_counts_a_row_named_only_by_a_struck_row_at_nothing() {
    let t = tally(&struck_proposal("a_strike"), NS);
    assert_eq!(t["nothing"], 1);
    assert_eq!(t["proposed"], 0);
}
