//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! A tenet's standing: a row states one from the set, and the verbs show it.

use mockspace::RegistryView;
use mockspace::tool::Outcome;
use registry_lints::standing::Standing;

use super::tests::{answered, fixture, run, scratch, view};
use crate::rows::tenets;

/// The tenets of `reg` whose word the pack does not know, and those it reads as held.
fn unstated_and_held(reg: &RegistryView) -> (Vec<String>, Vec<String>) {
    let all = tenets(reg);
    let ids = |keep: fn(Standing) -> bool| {
        all.iter()
            .filter(|t| keep(t.standing))
            .map(|t| t.id.to_string())
            .collect::<Vec<_>>()
    };
    (
        ids(|s| s == Standing::Unstated),
        ids(|s| s == Standing::Held),
    )
}

#[test]
fn a_missing_or_misspelt_standing_binds_and_a_held_one_binds_nothing() {
    // A word the pack does not know fails closed: the row stays in every check, so a typo never takes it
    // out of them.
    let reg = view(&[
        ("tenet::bare", &[("what", "No standing at all.")]),
        ("tenet::cap", &[
            ("standing", "Provisional"),
            ("what", "A misspelt word of the set."),
        ]),
        ("tenet::guessed", &[
            ("standing", "settled"),
            ("what", "A word outside the set."),
        ]),
        ("tenet::held", &[
            ("standing", "held"),
            ("what", "Brought to the maintainer."),
        ]),
        ("tenet::fine", &[
            ("standing", "provisional"),
            ("what", "In the set."),
        ]),
        ("tenet::done", &[
            ("standing", "ratified"),
            ("what", "In the set."),
        ]),
        ("tenet::shelved", &[
            ("standing", "demoted"),
            ("what", "In the set."),
        ]),
    ]);
    let (unstated, held) = unstated_and_held(&reg);
    assert_eq!(unstated, ["bare", "cap", "guessed"]);
    assert_eq!(held, ["held"]);
    let all = tenets(&reg);
    let row = |id: &str| all.iter().find(|t| t.id == id).expect(id);
    for id in ["bare", "cap", "guessed"] {
        assert!(row(id).binds(), "{id}: a row whose standing the pack cannot read stays in the checks");
    }
    // THE CONTROL: the words of the set read as themselves, held and demoted binding nothing.
    assert!(!row("held").binds());
    assert_eq!(row("held").standing.word(), "held");
    assert!(row("fine").binds() && row("done").binds() && !row("shelved").binds());
}

#[test]
fn check_blocks_on_a_held_row() {
    let reg = view(&[
        ("tenet::kept", &[("standing", "provisional"), ("what", "Matters now.")]),
        ("tenet::held", &[("standing", "held"), ("what", "Nobody backs it.")]),
    ]);
    let r = run(&reg, &scratch(), &["check", "kept", "held"]);
    match &r.outcome {
        Outcome::Findings(f) => {
            assert_eq!(f.len(), 1, "{f:?}");
            assert!(
                f[0].message.contains("`held` is held")
                    && f[0].message.contains("binds nothing"),
                "{}",
                f[0].message
            );
        },
        other => panic!("{other:?}"),
    }
    // THE CONTROL: the provisional row alone is clean.
    let r = run(&reg, &scratch(), &["check", "kept"]);
    assert!(matches!(r.outcome, Outcome::Clean { .. }), "{:?}", r.outcome);
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

/// A provisional row beside a demoted one and a row the demoted one never had: the demoted row binds
/// nothing and nothing replaces it.
fn shelf() -> RegistryView {
    view(&[
        ("tenet::kept", &[
            ("standing", "provisional"),
            ("what", "Matters now."),
        ]),
        ("tenet::shelved", &[
            ("standing", "demoted"),
            ("what", "Kept as reference."),
        ]),
    ])
}

#[test]
fn check_blocks_on_a_demoted_row_and_names_it_demoted() {
    let r = run(&shelf(), &scratch(), &["check", "kept", "shelved"]);
    match &r.outcome {
        Outcome::Findings(f) => {
            assert_eq!(f.len(), 1, "{f:?}");
            assert!(
                f[0].message.contains("`shelved` is demoted")
                    && f[0].message.contains("binds nothing"),
                "{}",
                f[0].message
            );
            // THE CONTROL: a demoted row is not read as a superseded one.
            assert!(!f[0].message.contains("superseded"), "{}", f[0].message);
        },
        other => panic!("{other:?}"),
    }
    assert!(r.outcome.blocks(mockspace::LintMode::Push));
    assert!(
        r.output.contains("`kept` is provisional, not ratified"),
        "{}",
        r.output
    );
    // THE CONTROL: the provisional row alone is clean.
    let r = run(&shelf(), &scratch(), &["check", "kept"]);
    assert!(
        matches!(r.outcome, Outcome::Clean { .. }),
        "{:?}",
        r.outcome
    );
}

#[test]
fn a_demoted_row_binds_nothing_and_has_no_heir() {
    let reg = shelf();
    let all = tenets(&reg);
    let row = |id: &str| all.iter().find(|t| t.id == id).expect(id);
    assert!(!row("shelved").binds());
    assert!(!row("shelved").replaced());
    // THE CONTROL: the provisional row binds.
    assert!(row("kept").binds());
    assert!(crate::rows::heirs(&all).is_empty());
}

/// A provisional row beside a stamped one of `standing`: a stamp is as good as the maintainer's word for
/// now and never a ratification, so the stamped row keeps its standing and binds as that standing does.
fn waiting_as(standing: &str) -> RegistryView {
    view(&[
        ("subject::time", &[("what", "The clock.")]),
        ("tenet::kept", &[
            ("standing", "provisional"),
            ("subject", "time"),
            ("what", "Matters now."),
        ]),
        ("tenet::asked", &[
            ("standing", standing),
            ("stamped_by", "experts"),
            ("stamp", "two experts agreed, recorded in ROWS.md, ticket `0123456789ab`"),
            ("subject", "time"),
            ("what", "Waits on the maintainer."),
        ]),
    ])
}

fn waiting() -> RegistryView {
    waiting_as("provisional")
}

#[test]
fn a_stamped_row_keeps_its_standing_and_binds_as_it_does() {
    let reg = waiting();
    let all = tenets(&reg);
    let row = |id: &str| all.iter().find(|t| t.id == id).expect(id);
    assert_eq!(row("asked").standing, Standing::Provisional);
    assert!(row("asked").stamped());
    assert!(row("asked").binds());
    assert!(!row("asked").replaced());
    assert!(crate::rows::heirs(&all).is_empty());
    // THE CONTROL: the same stamped row held binds nothing, so the standing decides and the stamp does not.
    let held = waiting_as("held");
    let all = tenets(&held);
    assert!(!all.iter().find(|t| t.id == "asked").expect("asked").binds());
}

#[test]
fn check_passes_a_stamped_row_and_notes_its_stamp() {
    let r = run(&waiting(), &scratch(), &["check", "kept", "asked"]);
    assert!(matches!(r.outcome, Outcome::Clean { .. }), "{:?}", r.outcome);
    assert!(!r.outcome.blocks(mockspace::LintMode::Push));
    assert!(
        r.output.contains("`asked` is provisional, stamped by experts, not ratified")
            && r.output.contains("never a ratification"),
        "{}",
        r.output
    );
    // THE CONTROL: the unstamped provisional row is noted with no stamp.
    assert!(r.output.contains("`kept` is provisional, not ratified"), "{}", r.output);
}

#[test]
fn a_listing_marks_a_stamped_row_and_the_subjects_count_it_binding() {
    let reg = waiting();
    let time = answered(&run(&reg, &scratch(), &["subject", "time"])).to_string();
    let asked = time
        .lines()
        .find(|l| l.trim_start().starts_with("asked"))
        .unwrap_or_default();
    assert!(asked.contains("(provisional, stamped)"), "{time}");
    let found = answered(&run(&reg, &scratch(), &["find", "maintainer"])).to_string();
    assert!(found.contains("(provisional, stamped)"), "{found}");
    let subs = answered(&run(&reg, &scratch(), &["subjects"])).to_string();
    assert!(subs.contains("2 tenets, 2 binding, 0 ratified"), "{subs}");
    // THE CONTROL: held in its place, the stamped row no longer counts.
    let subs = answered(&run(&waiting_as("held"), &scratch(), &["subjects"])).to_string();
    assert!(subs.contains("2 tenets, 1 binding, 0 ratified"), "{subs}");
}
