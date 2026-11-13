//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! A tenet's standing: a row states one from the set, and the verbs show it.

use mockspace::RegistryView;
use mockspace::tool::Outcome;

use super::tests::{answered, fixture, run, scratch, view};
use crate::rows::{Standing, tenets};

/// The tenets of `reg` whose `standing` is missing or outside the set `mockspace.toml` declares.
fn unstated(reg: &RegistryView) -> Vec<String> {
    tenets(reg)
        .into_iter()
        .filter(|t| t.standing == Standing::Unstated)
        .map(|t| t.id.to_string())
        .collect()
}

#[test]
fn a_tenet_with_no_standing_or_a_misspelt_one_is_caught() {
    // THE CONTROL: the law above, shown failing on rows that break it, and passing the two words.
    let reg = view(&[
        ("tenet::bare", &[("what", "No standing at all.")]),
        ("tenet::misspelt", &[
            ("standing", "Provisional"),
            ("what", "Capitalised."),
        ]),
        ("tenet::guessed", &[
            ("standing", "settled"),
            ("what", "A word outside the set."),
        ]),
        ("tenet::fine", &[
            ("standing", "provisional"),
            ("what", "In the set."),
        ]),
        ("tenet::done", &[
            ("standing", "ratified"),
            ("what", "In the set."),
        ]),
    ]);
    assert_eq!(unstated(&reg), ["bare", "guessed", "misspelt"]);
}

#[test]
fn every_word_of_the_set_reads_back_as_itself() {
    for w in Standing::WORDS {
        assert_eq!(Standing::of(w).word(), w);
    }
    assert_eq!(Standing::of("").word(), "unstated");
}

#[test]
fn check_names_a_provisional_row_and_still_exits_clean() {
    let reg = fixture();
    let r = run(&reg, &scratch(), &[
        "check",
        "nobody_names_this",
        "the_sun_rises",
    ]);
    assert!(
        matches!(r.outcome, Outcome::Clean { .. }),
        "{:?}",
        r.outcome
    );
    assert!(!r.outcome.blocks(mockspace::LintMode::Push));
    assert!(
        r.output
            .contains("`nobody_names_this` is provisional, not ratified"),
        "{}",
        r.output
    );
    // THE CONTROL: the ratified row named beside it is not mentioned.
    assert!(!r.output.contains("the_sun_rises"), "{}", r.output);
}

#[test]
fn check_still_blocks_on_a_superseded_row_beside_a_provisional_one() {
    let reg = fixture();
    let r = run(&reg, &scratch(), &[
        "check",
        "nobody_names_this",
        "the_sun_rose",
    ]);
    match &r.outcome {
        Outcome::Findings(f) => {
            assert_eq!(f.len(), 1, "{f:?}");
            assert!(f[0].message.contains("the_sun_rose"), "{}", f[0].message);
        },
        other => panic!("{other:?}"),
    }
    assert!(r.outcome.blocks(mockspace::LintMode::Push));
    assert!(r.output.contains("nobody_names_this"), "{}", r.output);
}

#[test]
fn a_listing_and_related_show_the_standing() {
    let reg = fixture();
    let time = answered(&run(&reg, &scratch(), &["subject", "time"])).to_string();
    let line = |id: &str| {
        time.lines()
            .find(|l| l.trim_start().starts_with(id))
            .unwrap_or_default()
            .to_string()
    };
    assert!(
        line("nobody_names_this").contains("(provisional)"),
        "{time}"
    );
    // THE CONTROL: a ratified row carries no mark.
    assert!(
        !line("days_follow_the_sun").contains("(provisional)"),
        "{time}"
    );
    let rel = answered(&run(&reg, &scratch(), &["related", "nobody_names_this"])).to_string();
    assert!(rel.contains("standing:      provisional"), "{rel}");
    let rel = answered(&run(&reg, &scratch(), &["related", "the_sun_rises"])).to_string();
    assert!(rel.contains("standing:      ratified"), "{rel}");
    let subs = answered(&run(&reg, &scratch(), &["subjects"])).to_string();
    assert!(subs.contains("4 tenets, 3 binding, 3 ratified"), "{subs}");
}
