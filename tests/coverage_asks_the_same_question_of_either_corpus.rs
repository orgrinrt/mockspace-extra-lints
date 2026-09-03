//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Coverage of a demand namespace, over both corpus shapes.
//!
//! Every arm the tool came with is here, driven against a built registry rather
//! than against the declaration, including the ones that must not fire. A tool
//! whose only test is "it printed something" reports its own iteration count.
//!
//! Every planted ruling carries a rung, and that is load-bearing. A suite that
//! plants rows carrying only the demand field gives every arm a rungless ruling,
//! so the arm named for the top tier asserts it for a row that has no authority
//! at all. Setup that helps, exactly: every input is one the implementation
//! handles, so the path that breaks is never entered.
//!
//! The second half of the file is what generalising across two corpora had to
//! get right, and each of those arms fails on the version of this tool that
//! reads one corpus's vocabulary as the vocabulary.
//!
//! Both corpora are planted here rather than read off disk, so the arms say what
//! the shapes are and do not move when either repository does.

use std::collections::BTreeMap;

use mockspace_extra_lints::tools::coverage::{
    also_named_by, preconditions, reach, stamps, tally, Coverage, Reach, TIERS,
};
use mockspace_lint_rules::tool::{NotALint, Outcome, Tool, ToolContext};
use mockspace_lint_rules::RegistryView;

// ---------------------------------------------------------------------------
// The harness
// ---------------------------------------------------------------------------

/// A registry with the rows a test names.
///
/// The reverse edges are passed empty throughout, and deliberately: nothing here
/// reads `referrers`. Every edge this tool walks is a forward one it reads off
/// the row itself, which is what lets it tell an edge from a ruling apart from
/// an edge from a retirement. The engine's reverse index knows a row is
/// referenced and does not know through which field, and the field is the whole
/// of what decides a tier.
fn view(rows: &[(&str, &[(&str, &str)])]) -> RegistryView {
    let mut r: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for (q, fields) in rows {
        r.insert(
            (*q).to_string(),
            fields
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        );
    }
    RegistryView::new(r, BTreeMap::new())
}

fn run(v: &RegistryView, args: &[&str]) -> (Outcome, String) {
    let crates = Default::default();
    let dirs: Vec<std::path::PathBuf> = Vec::new();
    let ctx = ToolContext {
        mock_dir: std::path::Path::new("."),
        repo_root: std::path::Path::new("."),
        all_crates: &crates,
        src_dirs: &dirs,
        args,
        stdin: None,
        registry: v,
    };
    let rep = Coverage.run(&ctx);
    // An inconclusive verdict carries its reason on the outcome and leaves
    // `output` empty, so a test reading `output` alone cannot tell a refusal
    // from a silent pass.
    let text = match &rep.outcome {
        Outcome::Inconclusive { reason } => reason.clone(),
        _ => rep.output.clone(),
    };
    (rep.outcome, text)
}

/// The demand namespace the fixtures below use.
///
/// One of the two real spellings rather than an invented one, so the arms read
/// against a shape that exists. The parallel arms further down plant the other.
const NS: &str = "obligation";

/// The one demand row every fixture below is about.
const DEMAND: (&str, &[(&str, &str)]) = ("obligation::the_thing", &[("what", "a demand")]);

/// That row and nothing reaching it.
fn alone() -> RegistryView {
    view(&[DEMAND])
}

/// A ruling at a named rung naming the demand row directly.
fn ruling_at(rung: &str) -> RegistryView {
    view(&[
        DEMAND,
        (
            "ruling::he_said_so",
            &[("rung", rung), ("obligation", "the_thing")],
        ),
    ])
}

/// A ruling at a named rung stamping a proposal that names the demand row.
///
/// The two-hop shape: the ruling carries no demand edge of its own, so anything
/// this reaches is reached through `ratifies` and through nothing else.
fn stamped_by(rung: &str) -> RegistryView {
    view(&[
        DEMAND,
        (
            "ruling::he_said_so",
            &[("rung", rung), ("ratifies", "a_claim")],
        ),
        ("proposal::a_claim", &[("obligation", "the_thing")]),
    ])
}

/// A proposal naming the demand row with nothing stamping it.
fn unstamped() -> RegistryView {
    view(&[
        DEMAND,
        ("proposal::a_claim", &[("obligation", "the_thing")]),
    ])
}

/// A retirement naming the demand row and nothing else doing so.
fn retired() -> RegistryView {
    view(&[
        DEMAND,
        ("retirement::a_dead_end", &[("obligation", "the_thing")]),
    ])
}

/// The tier the fixtures above put `the_thing` at.
fn tier(v: &RegistryView) -> Reach {
    reach(v, NS)["the_thing"].0
}

// ---------------------------------------------------------------------------
// The contract it declares about itself
// ---------------------------------------------------------------------------

#[test]
fn it_declares_itself_as_the_shape_it_is_and_no_run_returns_a_blocking_finding() {
    // The contract's own enforcement on a `no-failing-case` tool: a run may not
    // return a finding that blocks a gate. Driven over the registries below
    // rather than asserted about the declaration alone, because the declaration
    // is what the tool says and the outcome is what it does.
    assert!(matches!(Coverage.not_a_lint(), NotALint::NoFailingCase));
    for v in [
        view(&[]),
        alone(),
        ruling_at("ratified"),
        ruling_at("stated"),
        ruling_at("open"),
        stamped_by("ratified"),
        stamped_by("stated"),
        unstamped(),
        retired(),
    ] {
        let (outcome, text) = run(&v, &[NS]);
        assert!(
            !matches!(outcome, Outcome::Findings(_)),
            "a no-failing-case tool returned findings: {text}"
        );
    }
}

#[test]
fn it_answers_to_the_name_the_subcommand_uses_and_demands_the_namespace() {
    // The name is the string that collides with a repository's local copy and
    // the reason one has to be deleted, so it is pinned rather than left to the
    // directory. The required argument is pinned beside it because the whole
    // generalisation rests on it: give it `required: false` and the tool starts
    // answering about whatever namespace happens to sort first.
    assert_eq!(Coverage.name(), "coverage");
    let args = Coverage.args();
    assert_eq!(args.len(), 2, "a namespace and an optional slug");
    assert_eq!(args[0].name, "namespace");
    assert!(args[0].required, "the demand namespace is not optional");
    assert_eq!(args[1].name, "slug");
    assert!(!args[1].required);
}

// ---------------------------------------------------------------------------
// The ladder and the tier words
// ---------------------------------------------------------------------------

#[test]
fn every_tier_has_a_word_of_its_own() {
    // `tally` keys a map by `word()`, so two variants sharing a word merge their
    // counts silently and the report loses a tier without printing anything
    // different. Nothing else would catch it.
    let mut seen: Vec<&str> = TIERS.iter().map(|t| t.word()).collect();
    let before = seen.len();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), before, "two tiers share a word: {seen:?}");
}

#[test]
fn the_ladder_is_listed_strongest_first() {
    // `TIERS` drives the tally's order and the report's column order, and `Ord`
    // drives which edge wins. The two are written separately and a disagreement
    // would order the report against the ranking it claims to show.
    for pair in TIERS.windows(2) {
        assert!(
            pair[0] < pair[1],
            "`TIERS` is out of order at {:?} then {:?}",
            pair[0].word(),
            pair[1].word()
        );
    }
    assert_eq!(TIERS.first().copied(), Some(Reach::Ratified));
    assert_eq!(TIERS.last().copied(), Some(Reach::Nothing));
}

#[test]
fn answered_holds_exactly_where_something_constructive_reaches_it() {
    // Driven over every tier rather than over the two a not-equals form would
    // name. That form puts a newly added tier on the unanswered side in
    // silence.
    for t in [
        Reach::Ratified,
        Reach::InForce,
        Reach::Stated,
        Reach::Proposed,
    ] {
        assert!(t.answered(), "{}", t.word());
    }
    for t in [Reach::Unsettled, Reach::RouteClosed, Reach::Nothing] {
        assert!(!t.answered(), "{}", t.word());
    }
}

// ---------------------------------------------------------------------------
// The rung decides a ruling's tier
// ---------------------------------------------------------------------------

#[test]
fn each_rung_lands_at_its_own_tier() {
    // Over all four rungs one corpus declares, in one arm rather than one rung
    // sampled, so a mapping right for `ratified` and wrong for `in_force`
    // cannot pass.
    for (rung, want) in [
        ("ratified", Reach::Ratified),
        ("in_force", Reach::InForce),
        ("stated", Reach::Stated),
        ("open", Reach::Unsettled),
    ] {
        assert_eq!(
            tier(&ruling_at(rung)),
            want,
            "a ruling at `rung = {rung}` should tier at `{}`",
            want.word()
        );
    }
}

#[test]
fn a_stated_ruling_does_not_reach_the_top_tier() {
    // The arm that catches the defect this shape exists to prevent: a walk that
    // reads the namespace meets its demand row whatever rung it sits at, and
    // the row that produces a corpus's only `met` is then a `stated` ruling
    // whose own note records the call being declined.
    //
    // Fails on any implementation that tiers from the namespace.
    assert_ne!(tier(&ruling_at("stated")), Reach::Ratified);
    assert_eq!(tier(&ruling_at("stated")), Reach::Stated);
}

#[test]
fn a_rung_the_walk_cannot_read_does_not_reach_the_top_tier() {
    // The pessimistic direction, on purpose: a rung the tool does not know must
    // never read stronger than it is, because the flattering direction is the
    // one a clean run cannot be told apart from a real one.
    //
    // An absent rung is the case both schemas say cannot happen, `rung` being
    // required, and is planted anyway because a loader defect would otherwise
    // land the row at the top tier in silence.
    for v in [
        ruling_at("whatever_comes_next"),
        ruling_at(""),
        view(&[
            DEMAND,
            ("ruling::he_said_so", &[("obligation", "the_thing")]),
        ]),
    ] {
        assert_eq!(tier(&v), Reach::Unsettled);
    }
}

#[test]
fn the_report_prints_a_rulings_rung_beside_it() {
    // `unsettled` holds a ruling at a rung that settles nothing and one whose
    // rung could not be read, so the tier alone cannot tell them apart and the
    // line has to.
    let (_, open) = run(&ruling_at("open"), &[NS]);
    assert!(open.contains("rung = open"), "{open}");
    let (_, absent) = run(
        &view(&[
            DEMAND,
            ("ruling::he_said_so", &[("obligation", "the_thing")]),
        ]),
        &[NS],
    );
    assert!(absent.contains("rung = (absent)"), "{absent}");
}

// ---------------------------------------------------------------------------
// The stamp is followed, and only from a ratification
// ---------------------------------------------------------------------------

#[test]
fn a_proposal_a_ratified_ruling_stamps_reaches_the_top_tier() {
    // A proposal's demand edge is what it would meet if it were stamped, and
    // `ratifies` is the stamp. Reading the naming namespace alone files a
    // stamped proposal as proposed forever.
    let v = stamped_by("ratified");
    assert_eq!(tier(&v), Reach::Ratified);
    assert_eq!(
        stamps(&v).get("a_claim").map(Vec::len),
        Some(1),
        "the stamp is collected keyed by the proposal's slug"
    );
}

#[test]
fn control_a_proposal_stamped_by_an_unratified_ruling_stays_proposed() {
    // The control on the arm above, and the case the walk must fail on. A stamp
    // from below `ratified` is a defect a gate catches, and a measurement that
    // assumed the gate had run would report the proposal as canon on exactly
    // the row that gate exists to catch.
    //
    // Fails if `stamps()` collects the edge without reading the stamper's rung.
    for rung in ["in_force", "stated", "open", "whatever_comes_next"] {
        let v = stamped_by(rung);
        assert_eq!(
            tier(&v),
            Reach::Proposed,
            "a stamp from a ruling at `rung = {rung}` is not a stamp"
        );
        assert!(stamps(&v).is_empty(), "{rung}");
    }
}

#[test]
fn the_report_names_the_ruling_that_stamped_a_proposal() {
    // `ratified` on a proposal-sourced line claims a ruling's authority, so the
    // line names which ruling and the two-hop path can be checked rather than
    // taken.
    let (_, text) = run(&stamped_by("ratified"), &[NS]);
    assert!(text.contains("proposal::a_claim"), "{text}");
    assert!(text.contains("stamped by ruling::he_said_so"), "{text}");
}

#[test]
fn several_stamps_on_one_ruling_are_all_followed() {
    // `ratifies` is a list and arrives joined, so a reader taking the whole
    // value as one slug follows neither. Every arm above plants one entry and
    // would pass through that defect.
    let v = view(&[
        ("obligation::first", &[("what", "one")]),
        ("obligation::second", &[("what", "two")]),
        (
            "ruling::he_said_so",
            &[("rung", "ratified"), ("ratifies", "a_claim, another_claim")],
        ),
        ("proposal::a_claim", &[("obligation", "first")]),
        ("proposal::another_claim", &[("obligation", "second")]),
    ]);
    let r = reach(&v, NS);
    assert_eq!(r["first"].0, Reach::Ratified);
    assert_eq!(r["second"].0, Reach::Ratified);
}

#[test]
fn a_stamp_naming_no_proposal_changes_nothing_and_does_not_panic() {
    let v = view(&[
        DEMAND,
        (
            "ruling::he_said_so",
            &[("rung", "ratified"), ("ratifies", "a_ghost")],
        ),
        ("proposal::a_claim", &[("obligation", "the_thing")]),
    ]);
    assert_eq!(tier(&v), Reach::Proposed);
}

#[test]
fn an_unstamped_proposal_only_proposes_it() {
    // A proposal is proposed rather than met, and reporting it otherwise closes
    // a coverage gap nobody has seen.
    assert_eq!(tier(&unstamped()), Reach::Proposed);
}

// ---------------------------------------------------------------------------
// The tally
// ---------------------------------------------------------------------------

#[test]
fn the_tally_counts_every_row_once_and_at_one_tier() {
    let v = view(&[
        ("obligation::ratified", &[("what", "a")]),
        ("obligation::in_force", &[("what", "b")]),
        ("obligation::stated", &[("what", "c")]),
        ("obligation::proposed", &[("what", "d")]),
        ("obligation::unsettled", &[("what", "e")]),
        ("obligation::closed", &[("what", "f")]),
        ("obligation::nothing", &[("what", "g")]),
        (
            "ruling::r",
            &[("rung", "ratified"), ("obligation", "ratified")],
        ),
        (
            "ruling::f",
            &[("rung", "in_force"), ("obligation", "in_force")],
        ),
        ("ruling::s", &[("rung", "stated"), ("obligation", "stated")]),
        (
            "ruling::o",
            &[("rung", "open"), ("obligation", "unsettled")],
        ),
        ("proposal::p", &[("obligation", "proposed")]),
        ("retirement::x", &[("obligation", "closed")]),
    ]);
    let t = tally(&v, NS);
    for word in [
        "ratified",
        "in_force",
        "stated",
        "proposed",
        "unsettled",
        "route-closed",
    ] {
        assert_eq!(t[word], 1, "{word} in {t:?}");
    }
    assert_eq!(t["nothing"], 1);
    assert_eq!(t.values().sum::<usize>(), 7, "one row, one tier");
}

#[test]
fn the_tally_names_every_tier_even_where_none_sits_there() {
    // A tier missing from the report reads as a tier nobody has reached, and a
    // reader cannot tell that from a tier the tool forgot to print.
    let t = tally(&alone(), NS);
    assert_eq!(t.len(), TIERS.len(), "{t:?}");
    assert_eq!(t["ratified"], 0);
    assert_eq!(t["nothing"], 1);
}

// ---------------------------------------------------------------------------
// Preconditions, which are never a tier
// ---------------------------------------------------------------------------

#[test]
fn a_precondition_is_never_a_tier_and_never_counted_as_coverage() {
    // The arithmetic temptation, refused. A row with a precondition and nothing
    // else is further from met than one with nothing at all.
    let v = view(&[
        DEMAND,
        ("law::a_result", &[("precondition_for", "the_thing")]),
    ]);
    assert_eq!(reach(&v, NS)["the_thing"].0, Reach::Nothing);
    assert_eq!(preconditions(&v, NS)["the_thing"].len(), 1);
}

#[test]
fn a_precondition_is_read_from_every_namespace_that_carries_the_field() {
    // The origin read two named namespaces, because a lint of its own refuses
    // the field anywhere else, so the two lists were kept in step by hand. A
    // pack has no such lint to lean on, and a named list is a fact about one
    // corpus: a corpus without those namespaces would report every row as
    // unencumbered, which is the flattering direction.
    //
    // So the field is its own evidence. A row carrying it is establishing a
    // precondition whatever namespace it sits in, and the arm below is the
    // origin's own control with its assertion turned over for that reason.
    for q in [
        "law::a_result",
        "proposal::a_result",
        "probe::an_instrument",
        "mechanism::a_part",
    ] {
        let v = view(&[DEMAND, (q, &[("precondition_for", "the_thing")])]);
        assert_eq!(
            preconditions(&v, NS)["the_thing"].len(),
            1,
            "`{q}` carries the field and is not read"
        );
    }
}

#[test]
fn control_a_row_carrying_no_precondition_field_establishes_none() {
    // The control on the arm above. Without it a `preconditions` that counted
    // every row in every namespace would satisfy all four cases and the
    // widening would read as working.
    let v = view(&[
        DEMAND,
        ("law::a_result", &[("what", "an unrelated result")]),
        (
            "ruling::he_said_so",
            &[("rung", "ratified"), ("obligation", "the_thing")],
        ),
    ]);
    assert!(preconditions(&v, NS)["the_thing"].is_empty());
}

#[test]
fn a_stamp_does_not_turn_a_proposals_precondition_into_coverage() {
    // A stamped proposal reaches the top tier through the demand field. Its
    // `precondition_for` edges are a different field meaning a different thing,
    // and the stamp does not move them.
    let v = view(&[
        DEMAND,
        ("obligation::other", &[("what", "another")]),
        (
            "ruling::he_said_so",
            &[("rung", "ratified"), ("ratifies", "a_claim")],
        ),
        (
            "proposal::a_claim",
            &[("obligation", "the_thing"), ("precondition_for", "other")],
        ),
    ]);
    let r = reach(&v, NS);
    assert_eq!(r["the_thing"].0, Reach::Ratified);
    assert_eq!(r["other"].0, Reach::Nothing, "a precondition is not a tier");
    assert_eq!(preconditions(&v, NS)["other"].len(), 1);
}

// ---------------------------------------------------------------------------
// The other edges, and the walk itself
// ---------------------------------------------------------------------------

#[test]
fn a_retirement_is_a_closed_route_and_not_an_answer() {
    assert_eq!(tier(&retired()), Reach::RouteClosed);
}

#[test]
fn a_row_nothing_names_reaches_nothing() {
    let v = alone();
    assert_eq!(tier(&v), Reach::Nothing);
    assert!(reach(&v, NS)["the_thing"].1.is_empty());
}

/// Every tier one edge can produce, strongest first, with the row that produces
/// it: the namespace it sits in and the fields it carries beside its slug.
///
/// A table rather than a pair, because the arm below plants each of these
/// against each of the others and the interesting property is over all of them
/// at once. `Reach::Nothing` is absent on purpose: it is what a row no edge
/// names reaches, so there is no row that produces it and nothing to plant.
type Rung = (Reach, &'static str, &'static [(&'static str, &'static str)]);

const LADDER: [Rung; 6] = [
    (
        Reach::Ratified,
        "ruling",
        &[("rung", "ratified"), ("obligation", "the_thing")],
    ),
    (
        Reach::InForce,
        "ruling",
        &[("rung", "in_force"), ("obligation", "the_thing")],
    ),
    (
        Reach::Stated,
        "ruling",
        &[("rung", "stated"), ("obligation", "the_thing")],
    ),
    (Reach::Proposed, "proposal", &[("obligation", "the_thing")]),
    (
        Reach::Unsettled,
        "ruling",
        &[("rung", "open"), ("obligation", "the_thing")],
    ),
    (
        Reach::RouteClosed,
        "retirement",
        &[("obligation", "the_thing")],
    ),
];

#[test]
fn the_strongest_edge_decides_the_tier_whichever_order_the_walk_takes() {
    // The arm this replaces planted one pair in one arrangement and could not
    // fail: rows are yielded in slug order, so naming the strong row later made
    // it the last edge walked, and an implementation taking the last edge
    // produced the same answer. Mutating `entry.0 = entry.0.min(tier)` to
    // `entry.0 = tier` passed every arm.
    //
    // So every pair, both ways round. `a_first` sorts before `b_second`, so
    // giving the stronger row `a_first` walks it first and swapping walks it
    // last.
    //
    // Only the pairs drawn from one namespace vary. Across namespaces the order
    // is the edge table, fixed at ruling, proposal, retirement, so a ruling
    // against a retirement is walked rulings-first however the rows are named
    // and the swap changes nothing.
    for (i, (strong, strong_ns, strong_fields)) in LADDER.iter().enumerate() {
        for (weak, weak_ns, weak_fields) in LADDER.iter().skip(i + 1) {
            for (strong_slug, weak_slug) in [("a_first", "b_second"), ("b_second", "a_first")] {
                let s = format!("{strong_ns}::{strong_slug}");
                let w = format!("{weak_ns}::{weak_slug}");
                let v = view(&[DEMAND, (&s, strong_fields), (&w, weak_fields)]);
                let entry = &reach(&v, NS)["the_thing"];
                assert_eq!(
                    entry.0, *strong,
                    "{strong:?} planted as {s} against {weak:?} as {w} must tier {strong:?}"
                );
                assert_eq!(
                    entry.1.len(),
                    2,
                    "both rows are named as having got it there, whichever decided the tier"
                );
            }
        }
    }
}

#[test]
fn control_the_pairs_that_walk_the_stronger_row_first_are_the_ones_that_bite() {
    // The arm above asserts over thirty views and only some of them can catch a
    // last-edge implementation. This names which, so nobody reads the table as
    // thirty load-bearing rows: a pair bites where the walk reaches the stronger
    // edge first, and then a walk that overwrote would end on the weaker one.
    //
    // Fifteen pairs, each planted twice, and twenty-two of the thirty bite. Six
    // pairs are ruling against ruling and bite in the one arrangement that sorts
    // the stronger slug first. Nine reach across namespaces with the stronger
    // one earlier in the edge table, so those bite in both arrangements and the
    // swap buys nothing. The last three are a proposal against a ruling at a
    // rung that settles nothing, where the weaker row walks first whatever it is
    // called, so the pair asserts the right answer and catches nothing.
    let mut bites = 0;
    for (i, (_, strong_ns, strong_fields)) in LADDER.iter().enumerate() {
        for (_, weak_ns, weak_fields) in LADDER.iter().skip(i + 1) {
            for (strong_slug, weak_slug) in [("a_first", "b_second"), ("b_second", "a_first")] {
                let s = format!("{strong_ns}::{strong_slug}");
                let w = format!("{weak_ns}::{weak_slug}");
                let v = view(&[DEMAND, (&s, strong_fields), (&w, weak_fields)]);
                // What a walk that took the last edge would report, derived the
                // same way the walk derives it: namespace order first, slug
                // order inside one namespace.
                let ns_rank = |ns: &str| match ns {
                    "ruling" => 0,
                    "proposal" => 1,
                    _ => 2,
                };
                let strong_last = (ns_rank(strong_ns), strong_slug) > (ns_rank(weak_ns), weak_slug);
                if !strong_last {
                    bites += 1;
                }
                assert!(
                    reach(&v, NS)["the_thing"].1.len() == 2,
                    "the derivation above is about order and both rows are named either way"
                );
            }
        }
    }
    assert!(
        bites > 0,
        "a table where no arrangement reaches the stronger edge first would assert nothing"
    );
    assert_eq!(
        bites, 22,
        "six ruling pairs bite once each, nine cross-namespace pairs bite twice, \
         and the three where a ruling at a settling-nothing rung walks before a \
         proposal never do"
    );
}

#[test]
fn a_slug_naming_no_demand_row_contributes_nothing_rather_than_panicking() {
    let v = view(&[
        DEMAND,
        (
            "ruling::he_said_so",
            &[("rung", "ratified"), ("obligation", "a_ghost")],
        ),
    ]);
    assert_eq!(reach(&v, NS)["the_thing"].0, Reach::Nothing);
}

#[test]
fn several_demand_slugs_on_one_row_are_all_reached() {
    // The field is a list and arrives joined, so a reader taking the whole value
    // as one slug reaches neither.
    let v = view(&[
        ("obligation::first", &[("what", "one")]),
        ("obligation::second", &[("what", "two")]),
        (
            "ruling::he_said_so",
            &[("rung", "ratified"), ("obligation", "first, second")],
        ),
    ]);
    let r = reach(&v, NS);
    assert_eq!(r["first"].0, Reach::Ratified);
    assert_eq!(r["second"].0, Reach::Ratified);
}

// ---------------------------------------------------------------------------
// The report
// ---------------------------------------------------------------------------

#[test]
fn a_registry_with_no_demand_rows_is_inconclusive_rather_than_clean() {
    // The verdict that exists so a broken run cannot claim a pass it never
    // established. An empty demand side measures nothing, and reporting `Clean`
    // over it would say the canon is exhaustive because nobody wrote down what
    // it owes.
    let (outcome, text) = run(&view(&[]), &[NS]);
    assert!(matches!(outcome, Outcome::Inconclusive { .. }), "{text}");
    assert!(text.contains("no `obligation` rows"), "{text}");
}

#[test]
fn the_report_names_every_tier_and_the_rows_that_got_each_there() {
    let v = view(&[
        ("obligation::reached", &[("what", "a")]),
        ("obligation::nothing", &[("what", "d")]),
        (
            "ruling::he_said_so",
            &[("rung", "ratified"), ("obligation", "reached")],
        ),
    ]);
    let (outcome, text) = run(&v, &[NS]);
    assert!(matches!(outcome, Outcome::Clean { examined: 2 }), "{text}");
    assert!(text.contains("ruling::he_said_so"), "{text}");
    for t in TIERS {
        assert!(text.contains(t.word()), "{} missing: {text}", t.word());
    }
}

#[test]
fn the_report_orders_the_tally_strongest_first_and_the_rows_weakest_first() {
    // Nothing pinned either order, so a change from tier-major to slug-major, or
    // a flip of the body, would move every line of the output and no arm would
    // fire. Both orders carry meaning and they are deliberately opposite: the
    // tally reads as a ladder from met downward, and the body puts what is least
    // reached at the top, where a reader looking for work finds it first.
    let v = view(&[
        ("obligation::zeta", &[("what", "a")]),
        ("obligation::alpha", &[("what", "b")]),
        ("obligation::mid", &[("what", "c")]),
        (
            "ruling::ratifies_zeta",
            &[("rung", "ratified"), ("obligation", "zeta")],
        ),
        (
            "ruling::ratifies_alpha",
            &[("rung", "ratified"), ("obligation", "alpha")],
        ),
        ("proposal::proposes_mid", &[("obligation", "mid")]),
    ]);
    let (_, text) = run(&v, &[NS]);
    let at = |needle: &str| {
        text.find(needle)
            .unwrap_or_else(|| panic!("{needle} missing from the report: {text}"))
    };

    // The tally, strongest first, in the order the ladder itself declares.
    let mut previous = 0;
    for t in TIERS {
        let here = at(t.word());
        assert!(
            here > previous,
            "the tally follows the ladder and `{}` is out of place: {text}",
            t.word()
        );
        previous = here;
    }

    // The body, weakest first, with two rows at one tier so the ordering inside
    // a tier is exercised by the same report.
    assert!(
        at("mid") < at("alpha"),
        "a weaker tier's rows come before a stronger tier's, whatever the slugs \
         sort like: {text}"
    );
    assert!(
        at("alpha") < at("zeta"),
        "inside one tier the rows are in the registry's own slug order: {text}"
    );
    assert!(
        text.contains("weakest first"),
        "the body says which way round it is, because the tally above it is the \
         other way and a reader cannot infer either from two rows: {text}"
    );
}

#[test]
fn a_ruling_whose_kind_is_a_refusal_still_tiers_by_its_rung_today() {
    // This records what the walk does, not what it should do. The walk reads
    // `rung` and never `kind`, so a ruling at `ratified` whose kind is a refusal
    // or a deferral tiers as met, and no fixture anywhere planted one until this
    // arm.
    //
    // Whether a refusal that named a demand row has met it is a canon reading,
    // which belongs to whoever owns that canon rather than to this tool. So the
    // assertion is the current answer, and when somebody settles the question
    // this is the arm that fails and says where the decision lives.
    for kind in ["refusal", "deferral"] {
        let v = view(&[
            DEMAND,
            (
                "ruling::he_said_so",
                &[
                    ("rung", "ratified"),
                    ("kind", kind),
                    ("obligation", "the_thing"),
                ],
            ),
        ]);
        assert_eq!(
            tier(&v),
            Reach::Ratified,
            "a `{kind}` at `ratified` currently reads as met, which is the \
             behaviour recorded rather than the behaviour argued for"
        );
    }
}

#[test]
fn the_report_says_which_tier_is_met() {
    // The word `met` is not a tier, so the sentence that says which tier is one
    // has to be in the report rather than only in the source.
    let (_, text) = run(&ruling_at("ratified"), &[NS]);
    assert!(text.contains("only tier that is met"), "{text}");
}

#[test]
fn the_report_marks_a_route_closed_row_rather_than_letting_it_read_as_untouched() {
    let (_, text) = run(&retired(), &[NS]);
    assert!(
        text.contains("named only by a retirement"),
        "a retired route reads identically to nobody having looked on a flat \
         list, which is the distinction the field was added for: {text}"
    );
}

#[test]
fn the_report_names_an_unanswered_row_carrying_a_precondition() {
    for v in [
        view(&[
            DEMAND,
            ("law::a_result", &[("precondition_for", "the_thing")]),
        ]),
        view(&[
            DEMAND,
            ("retirement::a_dead_end", &[("obligation", "the_thing")]),
            ("law::a_result", &[("precondition_for", "the_thing")]),
        ]),
        view(&[
            DEMAND,
            (
                "ruling::he_said_so",
                &[("rung", "open"), ("obligation", "the_thing")],
            ),
            ("law::a_result", &[("precondition_for", "the_thing")]),
        ]),
    ] {
        let (_, text) = run(&v, &[NS]);
        assert!(
            text.contains("answered by nothing and carry an established precondition"),
            "{text}"
        );
        assert!(text.contains("the_thing"), "{text}");
    }
}

#[test]
fn control_an_answered_row_carrying_a_precondition_is_not_in_that_list() {
    // The pair is about being stuck, so a row something constructive reaches is
    // not one however many preconditions were established along the way. Over
    // all four answering tiers, because the predicate deciding this is where a
    // not-equals form would sit and where a new tier would land on the wrong
    // side.
    for reaching in [
        vec![(
            "ruling::he_said_so",
            &[("rung", "ratified"), ("obligation", "the_thing")][..],
        )],
        vec![(
            "ruling::he_said_so",
            &[("rung", "in_force"), ("obligation", "the_thing")][..],
        )],
        vec![(
            "ruling::he_said_so",
            &[("rung", "stated"), ("obligation", "the_thing")][..],
        )],
        vec![("proposal::a_claim", &[("obligation", "the_thing")][..])],
    ] {
        let mut rows: Vec<(&str, &[(&str, &str)])> = vec![
            DEMAND,
            ("law::a_result", &[("precondition_for", "the_thing")]),
        ];
        rows.extend(reaching.iter().copied());
        let (_, text) = run(&view(&rows), &[NS]);
        assert!(
            !text.contains("answered by nothing and carry an established precondition"),
            "{text}"
        );
    }
}

#[test]
fn one_row_can_be_read_in_full_by_its_slug() {
    let v = view(&[
        (
            "obligation::the_thing",
            &[("what", "a demand"), ("note", "a note")],
        ),
        ("obligation::other", &[("what", "another")]),
        (
            "ruling::he_said_so",
            &[("rung", "ratified"), ("obligation", "the_thing")],
        ),
    ]);
    let (outcome, text) = run(&v, &[NS, "the_thing"]);
    assert!(matches!(outcome, Outcome::Clean { examined: 1 }), "{text}");
    assert!(text.contains("a demand"), "{text}");
    assert!(text.contains("a note"), "{text}");
    assert!(text.contains("tier: ratified"), "{text}");
    assert!(text.contains("rung = ratified"), "{text}");
    assert!(
        !text.contains("another"),
        "the other row is not reported: {text}"
    );
}

#[test]
fn a_slug_that_names_nothing_is_inconclusive_rather_than_an_empty_report() {
    let (outcome, text) = run(&alone(), &[NS, "nosuch"]);
    assert!(matches!(outcome, Outcome::Inconclusive { .. }), "{text}");
    assert!(
        text.contains("spelling"),
        "an empty report here would read as a row nothing reaches: {text}"
    );
}

// ---------------------------------------------------------------------------
// What generalising it across two corpora had to get right
// ---------------------------------------------------------------------------

/// The other corpus's shape: a different demand namespace, a rung vocabulary of
/// two rather than four, and no retirement namespace at all.
fn other_corpus() -> RegistryView {
    view(&[
        ("slot::a_person_needs_this", &[("what", "a demand")]),
        ("slot::nothing_answers_this", &[("what", "another")]),
        (
            "ruling::he_said_so",
            &[("rung", "ratified"), ("slot", "a_person_needs_this")],
        ),
    ])
}

#[test]
fn the_demand_namespace_is_an_argument_and_either_corpus_answers() {
    // The whole of the generalisation in one arm. Give the tool the other
    // corpus's namespace and it measures that corpus, with no arm anywhere
    // knowing which one it is running against.
    let v = other_corpus();
    let r = reach(&v, "slot");
    assert_eq!(r["a_person_needs_this"].0, Reach::Ratified);
    assert_eq!(r["nothing_answers_this"].0, Reach::Nothing);
    let (outcome, text) = run(&v, &["slot"]);
    assert!(matches!(outcome, Outcome::Clean { examined: 2 }), "{text}");
    assert!(text.contains("2 `slot` rows"), "{text}");
    assert!(text.contains("By slot, weakest first"), "{text}");
}

#[test]
fn naming_the_wrong_namespace_is_inconclusive_rather_than_a_clean_nothing() {
    // The report a corpus gets when the caller names a namespace it does not
    // declare. Reporting `Clean` over it would say that corpus owes nothing.
    let (outcome, text) = run(&other_corpus(), &["obligation"]);
    assert!(matches!(outcome, Outcome::Inconclusive { .. }), "{text}");
    assert!(text.contains("no `obligation` rows"), "{text}");
}

#[test]
fn a_corpus_spelling_fewer_rungs_gets_empty_tiers_rather_than_a_miscount() {
    // A rung vocabulary is the corpus's own. The two tiers this corpus has no
    // word for hold nobody, and the two it does hold exactly their rows, so the
    // ladder degrades by having empty rungs rather than by misfiling anything.
    let v = view(&[
        ("slot::met", &[("what", "a")]),
        ("slot::acked", &[("what", "b")]),
        ("slot::untouched", &[("what", "c")]),
        ("ruling::r", &[("rung", "ratified"), ("slot", "met")]),
        ("ruling::s", &[("rung", "stated"), ("slot", "acked")]),
    ]);
    let t = tally(&v, "slot");
    assert_eq!(t["ratified"], 1);
    assert_eq!(t["stated"], 1);
    assert_eq!(t["nothing"], 1);
    assert_eq!(t["in_force"], 0, "no row spells it, so nobody sits there");
    assert_eq!(t["unsettled"], 0, "and nothing was misfiled into it");
    assert_eq!(t["route-closed"], 0);
    assert_eq!(t.values().sum::<usize>(), 3);
}

#[test]
fn a_namespace_the_corpus_does_not_declare_contributes_nothing_without_panicking() {
    // A lookup rather than an assumption. This corpus declares no retirement,
    // so that tier is empty and no arm has to know which corpus it is on.
    let v = other_corpus();
    assert!(v.rows_in("retirement").is_empty(), "the fixture's premise");
    let t = tally(&v, "slot");
    assert_eq!(t["route-closed"], 0);
    let (_, text) = run(&v, &["slot"]);
    assert!(
        !text.contains("named only by a retirement"),
        "a corpus with no retirements is not told about retirements: {text}"
    );
}

#[test]
fn a_corpus_where_no_edge_was_read_at_all_says_so_and_still_reports() {
    // The line that exists because two different corpora produce the identical
    // report: one recording the relation under another name, and one where
    // nothing has answered anything yet. Only the schema separates them and a
    // tool is handed no parsed copy of it, so the report names both readings
    // rather than picking one.
    //
    // The first version of this refused the run outright, as inconclusive. That
    // was wrong in the expensive direction: it refuses the ordinary early state
    // of a canon being written, which is the case this tool exists to serve.
    // Two arms carried over from the origin caught it, both planting a corpus
    // with a precondition and no coverage.
    let v = view(&[
        ("slot::a_person_needs_this", &[("what", "a demand")]),
        (
            "ruling::he_said_so",
            &[("rung", "ratified"), ("answers", "a_person_needs_this")],
        ),
    ]);
    let (outcome, text) = run(&v, &["slot"]);
    assert!(
        matches!(outcome, Outcome::Clean { .. }),
        "a corpus nothing has answered yet is reported, not refused: {text}"
    );
    assert!(text.contains("carries a `slot` field"), "{text}");
    assert!(text.contains("records the relation under another name"), "{text}");
}

#[test]
fn control_a_corpus_that_does_spell_the_field_is_not_given_that_line() {
    // The positive control. Without it a report that always printed the caveat
    // would satisfy the arm above, and every corpus would be told its edges may
    // not have been read.
    let (_, text) = run(&other_corpus(), &["slot"]);
    assert!(
        !text.contains("no edge was read at all"),
        "one row carrying the field is enough, and the caveat is not printed: {text}"
    );
    // An edge naming no row still counts as the field being spelled: the
    // question is whether the corpus records the relation, not whether the
    // relation resolves, which is a different defect with a different report.
    let dangling = view(&[
        ("slot::a_person_needs_this", &[("what", "a demand")]),
        (
            "ruling::he_said_so",
            &[("rung", "ratified"), ("slot", "a_ghost")],
        ),
    ]);
    let (_, text) = run(&dangling, &["slot"]);
    assert!(!text.contains("no edge was read at all"), "{text}");
}

#[test]
fn a_field_holding_only_whitespace_does_not_count_as_the_corpus_spelling_it() {
    // A field present and holding nothing tells a reader exactly what a missing
    // one does, so the check trims before asking.
    let v = view(&[
        ("slot::a_person_needs_this", &[("what", "a demand")]),
        ("ruling::he_said_so", &[("rung", "ratified"), ("slot", "  \n ")]),
    ]);
    let (_, text) = run(&v, &["slot"]);
    assert!(text.contains("no edge was read at all"), "{text}");
}

#[test]
fn a_namespace_whose_authority_cannot_be_read_is_named_and_sets_no_tier() {
    // The origin dropped these silently, which is wrong in the direction a
    // reader cannot see: the edge exists, it is somebody's work, and the report
    // said nothing named the row. Tiering it would be worse, because it would
    // invent an authority nobody declared. So it is printed and counts for
    // nothing.
    let v = view(&[
        DEMAND,
        ("sketch::somebody_tried", &[("obligation", "the_thing")]),
    ]);
    assert_eq!(
        reach(&v, NS)["the_thing"].0,
        Reach::Nothing,
        "an unreadable authority is not an authority"
    );
    assert_eq!(
        also_named_by(&v, NS)["the_thing"],
        vec!["sketch::somebody_tried".to_string()],
        "and it is not dropped either"
    );
    let (_, text) = run(&v, &[NS]);
    assert!(text.contains("sketch::somebody_tried"), "{text}");
    assert!(text.contains("cannot tier"), "{text}");
}

#[test]
fn control_the_three_namespaces_that_can_be_tiered_are_not_reported_as_untierable() {
    // The control on the arm above. Without it an `also_named_by` that returned
    // every referrer would satisfy it, and every tiered edge would be printed
    // twice: once as its tier and once as unreadable.
    for v in [ruling_at("ratified"), unstamped(), retired()] {
        assert!(
            also_named_by(&v, NS)["the_thing"].is_empty(),
            "a namespace the walk tiers is not also reported as untierable"
        );
    }
    // And the demand namespace itself is not reported against its own rows.
    let self_naming = view(&[
        DEMAND,
        ("obligation::other", &[("obligation", "the_thing")]),
    ]);
    assert!(also_named_by(&self_naming, NS)["the_thing"].is_empty());
}

#[test]
fn the_arms_above_run_against_both_spellings_of_the_demand_namespace() {
    // The whole suite above plants one namespace. This drives the same shapes
    // through the other one, so a constant that crept back in and happened to
    // match the first spelling cannot pass.
    for demand in ["obligation", "slot", "requirement"] {
        let row = format!("{demand}::the_thing");
        let v = view(&[
            (&row, &[("what", "a demand")]),
            (
                "ruling::he_said_so",
                &[("rung", "stated"), (demand, "the_thing")],
            ),
        ]);
        assert_eq!(
            reach(&v, demand)["the_thing"].0,
            Reach::Stated,
            "the demand namespace `{demand}` is read from the argument"
        );
        let (outcome, text) = run(&v, &[demand]);
        assert!(matches!(outcome, Outcome::Clean { examined: 1 }), "{text}");
        assert!(text.contains(&format!("1 `{demand}` rows")), "{text}");
    }
}

#[test]
fn no_argument_is_refused_rather_than_indexed_into() {
    // Unreachable through the engine, which refuses a missing required argument
    // before `run`. A direct caller is still a caller, and the alternative to
    // this arm is a panic on an empty slice.
    let (outcome, text) = run(&alone(), &[]);
    assert!(matches!(outcome, Outcome::Inconclusive { .. }), "{text}");
    assert!(text.contains("no namespace was named"), "{text}");
}
