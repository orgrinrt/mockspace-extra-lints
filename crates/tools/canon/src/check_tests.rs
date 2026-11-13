//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The `check` verb: silent and clean where every named row exists and binds, and naming each that does not.

use mockspace::tool::{Outcome, ToolReport};

use super::tests::{fixture, run, scratch};

/// What `check` found, as the messages of its findings; `None` where it found nothing.
fn problems(r: &ToolReport) -> Option<Vec<String>> {
    match &r.outcome {
        Outcome::Clean {
            ..
        } => None,
        Outcome::Findings(f) => Some(f.iter().map(|e| e.message.clone()).collect()),
        other => panic!("{other:?}"),
    }
}

#[test]
fn check_is_silent_and_clean_when_every_row_exists_and_binds() {
    let reg = fixture();
    let r = run(&reg, &scratch(), &[
        "check",
        "the_sun_rises",
        "days_follow_the_sun",
    ]);
    assert!(problems(&r).is_none(), "{:?}", r.outcome);
    assert!(r.output.is_empty(), "{}", r.output);
    assert!(!r.outcome.blocks(mockspace::LintMode::Push));
}

#[test]
fn check_blocks_on_a_row_that_does_not_exist_and_names_it() {
    let reg = fixture();
    let r = run(&reg, &scratch(), &["check", "the_moon_sets"]);
    let found = problems(&r).expect("a finding");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("the_moon_sets"), "{}", found[0]);
    assert!(r.outcome.blocks(mockspace::LintMode::Push));
}

#[test]
fn check_blocks_on_a_superseded_row_and_names_what_replaced_it() {
    let reg = fixture();
    let r = run(&reg, &scratch(), &["check", "the_sun_rose"]);
    let found = problems(&r).expect("a finding");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("the_sun_rose") && found[0].contains("superseded by the_sun_rises"),
        "{}",
        found[0]
    );
    assert!(r.outcome.blocks(mockspace::LintMode::Push));
}

#[test]
fn check_reports_every_bad_row_and_none_of_the_good_ones() {
    let reg = fixture();
    let r = run(&reg, &scratch(), &[
        "check",
        "the_sun_rises",
        "the_sun_rose",
        "no_such_row",
        "nobody_names_this",
    ]);
    let found = problems(&r).expect("findings");
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found.iter().any(|m| m.contains("the_sun_rose")));
    assert!(found.iter().any(|m| m.contains("no_such_row")));
    assert!(!found.iter().any(|m| m.contains("nobody_names_this")));
}

#[test]
fn check_reads_the_namespace_prefix_a_citation_carries_and_skips_flags() {
    let reg = fixture();
    let ok = run(&reg, &scratch(), &["check", "tenet::the_sun_rises", "-q"]);
    assert!(problems(&ok).is_none(), "{:?}", ok.outcome);
    let bad = run(&reg, &scratch(), &["check", "tenet::the_sun_rose"]);
    assert!(problems(&bad).is_some());
}

#[test]
fn check_with_no_row_is_inconclusive_rather_than_a_pass() {
    let reg = fixture();
    for args in [&["check"][..], &["check", "-q"][..]] {
        match run(&reg, &scratch(), args).outcome {
            Outcome::Inconclusive {
                reason,
            } => assert!(reason.contains("tenet ids"), "{reason}"),
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn check_is_one_of_the_verbs_the_help_lists() {
    assert!(crate::VERBS.contains(&"check"));
    let reg = fixture();
    match run(&reg, &scratch(), &["nosuch"]).outcome {
        Outcome::Inconclusive {
            reason,
        } => assert!(reason.contains("check"), "{reason}"),
        other => panic!("{other:?}"),
    }
}
