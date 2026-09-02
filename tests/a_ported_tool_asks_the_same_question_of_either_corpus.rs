//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What generalising a tool across two corpora had to get right.
//!
//! The tool came from a corpus recording `ratified_by`, and the exclusion built
//! on that field is correct there. A second corpus declares no such field, and
//! the failure this file pins is the one that reads as working: treating the
//! absent field as though it carried the excluding value, which silently
//! shrinks the population on every corpus that never had the concept and
//! reports a cleaner answer than the corpus earned.
//!
//! Both corpora are planted here rather than read off disk, so the arms say
//! what the shapes are and do not move when either repository does.

use std::collections::BTreeMap;

use mockspace_extra_lints::tools::rulings_with_no_verbatim::RulingsWithNoVerbatim;
use mockspace_lint_rules::RegistryView;
use mockspace_lint_rules::tool::{Outcome, Tool, ToolContext};

fn row(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

fn view(rows: &[(&str, BTreeMap<String, String>)]) -> RegistryView {
    let map: BTreeMap<String, BTreeMap<String, String>> = rows
        .iter()
        .map(|(q, f)| ((*q).to_string(), f.clone()))
        .collect();
    RegistryView::new(map, BTreeMap::new())
}

fn run(registry: &RegistryView, args: &[&str]) -> (Outcome, String) {
    let crates = std::collections::BTreeSet::new();
    let ctx = ToolContext {
        mock_dir: std::path::Path::new("/tmp"),
        repo_root: std::path::Path::new("/tmp"),
        all_crates: &crates,
        src_dirs: &[],
        args,
        stdin: None,
        registry,
    };
    let r = RulingsWithNoVerbatim.run(&ctx);
    (r.outcome, r.output)
}

fn examined(outcome: &Outcome) -> usize {
    match outcome {
        Outcome::Clean {
            examined,
        } => *examined,
        _ => panic!("expected a clean report, got {outcome:?}"),
    }
}

/// The control. Without it every arm below is satisfied by a tool that reports
/// nothing whatever it is handed.
#[test]
fn a_ruling_carrying_no_quote_is_reported() {
    let v = view(&[(
        "ruling::a_call_nobody_quoted",
        row(&[("says", "somebody's restatement")]),
    )]);
    let (outcome, out) = run(&v, &[]);
    assert_eq!(examined(&outcome), 1);
    assert!(
        out.contains("a_call_nobody_quoted"),
        "a ruling with no quote is the whole point of the tool: {out}"
    );
}

/// The other side of the control: a corpus with nothing to report says so.
#[test]
fn a_ruling_carrying_its_quote_is_not_reported() {
    let v = view(&[(
        "ruling::a_call_in_his_own_words",
        row(&[("says", "restated"), ("quote", "the words themselves")]),
    )]);
    let (_, out) = run(&v, &[]);
    assert!(
        out.contains("every one of the 1 rulings"),
        "a corpus whose rulings all carry their words must report clean: {out}"
    );
}

/// The exclusion, on the corpus that declares the field it rests on.
#[test]
fn a_ruling_ratified_by_experts_is_out_of_scope_where_the_corpus_records_that() {
    let v = view(&[(
        "ruling::the_experts_proposed_and_a_coordinator_gated",
        row(&[("says", "restated"), ("ratified_by", "experts")]),
    )]);
    let (_, out) = run(&v, &[]);
    assert!(
        out.contains("every one of the 1 rulings"),
        "such a row never passed through the person whose words are at issue, so there is no \
         verbatim to have lost: {out}"
    );
}

/// The generalisation, and the arm the port exists for.
///
/// A corpus declaring no `ratified_by` makes no such distinction, so every one
/// of its rulings is in scope. Reading the absent field as though it excluded
/// would report this corpus clean, which is the failure that looks like a pass.
#[test]
fn a_corpus_declaring_no_ratified_by_excludes_nothing() {
    let v = view(&[
        (
            "ruling::one",
            row(&[("says", "restated"), ("note", "the corpus holds no verbatim")]),
        ),
        ("ruling::two", row(&[("says", "restated")])),
    ]);
    let (outcome, out) = run(&v, &[]);
    assert_eq!(examined(&outcome), 2);
    assert!(
        out.contains("2 of 2 rulings"),
        "a corpus without the field makes no such distinction and every ruling is in scope: \
         {out}"
    );
}

/// A note is what tells a reader somebody has already looked at a hole, so the
/// listing has to carry it rather than only the slug.
#[test]
fn the_listing_distinguishes_a_hole_somebody_has_looked_at() {
    let v = view(&[
        (
            "ruling::looked_at",
            row(&[("says", "s"), ("note", "no verbatim exists, the call was made in a meeting")]),
        ),
        ("ruling::untouched", row(&[("says", "s")])),
    ]);
    let (_, out) = run(&v, &[]);
    assert!(out.contains("note: no verbatim exists"), "{out}");
    assert!(out.contains("no note either"), "{out}");
}

/// An empty namespace is not a clean bill of health, and the contract has a
/// third value for exactly this.
#[test]
fn a_corpus_with_no_rulings_is_inconclusive_rather_than_clean() {
    let v = view(&[("proposal::not_a_ruling", row(&[("says", "s")]))]);
    let (outcome, _) = run(&v, &[]);
    assert!(
        matches!(outcome, Outcome::Inconclusive { .. }),
        "examining nothing is not a pass, it is a run nobody should trust: {outcome:?}"
    );
}

/// A slug that names no row is a statement about the spelling, not the canon.
#[test]
fn an_unmatched_slug_is_inconclusive_rather_than_a_silent_empty_report() {
    let v = view(&[("ruling::real", row(&[("says", "s")]))]);
    let (outcome, _) = run(&v, &["no_such_row"]);
    assert!(
        matches!(outcome, Outcome::Inconclusive { .. }),
        "{outcome:?}"
    );
}

/// The single-row form resolves a bare slug, which is how anybody would type it.
#[test]
fn the_single_row_form_takes_a_bare_slug_and_prints_the_fields_that_carry_anything() {
    let v = view(&[(
        "ruling::real",
        row(&[("says", "what it says"), ("rung", "ratified"), ("note", "")]),
    )]);
    let (outcome, out) = run(&v, &["real"]);
    assert_eq!(examined(&outcome), 1);
    assert!(out.contains("what it says"), "{out}");
    assert!(out.contains("ratified"), "{out}");
    assert!(
        !out.contains("note:"),
        "an empty field carries nothing and must not be printed as though it did: {out}"
    );
}
