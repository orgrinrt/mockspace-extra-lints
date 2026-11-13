//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The words a replaced tenet used and its successors do not, and where lines still name them. The
//! fixtures rename an invented `snorf` to a `plimp`, so that no file of the repository these tests sit
//! in is counted.

use crate::dropped::{Counts, dropped, still_named, word_counts};
use crate::rows::tenets;
use crate::tests::{answered, run, scratch, view, write};

fn registry() -> mockspace::RegistryView {
    view(&[
        ("subject::planet", &[("what", "The planet.")]),
        ("tenet::a_snorf_holds_the_state", &[
            ("subject", "planet"),
            ("what", "A snorf holds the state of every building."),
            ("superseded_by", "a_plimp_holds_the_state"),
        ]),
        ("tenet::a_plimp_holds_the_state", &[
            ("subject", "planet"),
            ("what", "A plimp holds the state of every building."),
        ]),
        ("tenet::the_ground_is_one", &[
            ("subject", "planet"),
            ("what", "One ground."),
            ("superseded_by", "a_plimp_holds_the_state"),
        ]),
    ])
}

fn tree() -> std::path::PathBuf {
    let d = scratch();
    write(
        &d,
        "mock/crates/kaski-a/DESIGN.md.tmpl",
        "a snorf stands here\nthe snorf and the snorf again\n",
    );
    write(
        &d,
        "mock/crates/kaski-a/src/lib.rs",
        "pub struct SnorfCell;\n",
    );
    write(&d, "mock/research/x/NOTES.md", "the snorf was a window\n");
    d
}

#[test]
fn the_dropped_words_are_the_ones_only_the_replaced_tenet_carried() {
    let reg = registry();
    let all = tenets(&reg);
    let old = all
        .iter()
        .find(|t| t.id == "a_snorf_holds_the_state")
        .unwrap();
    assert_eq!(dropped(&all, old), ["snorf"]);
    // the connectives and numerals (`the`, `one`, `is`) are not read as concepts
    let ground = all.iter().find(|t| t.id == "the_ground_is_one").unwrap();
    assert_eq!(dropped(&all, ground), ["ground"]);
}

#[test]
fn a_line_is_counted_once_per_word_by_tier_and_notes_are_not_read() {
    let corpus = word_counts(&tree());
    assert_eq!(
        corpus.named("snorf"),
        Some(Counts {
            design:       2,
            code:         1,
            content:      0,
            design_files: 1,
        }),
        "two design lines (the second names it twice) and one code line; the research note is not read"
    );
    assert!(corpus.named("plimp").is_none());
}

#[test]
fn the_control_a_dropped_word_no_line_names_is_not_listed() {
    let reg = registry();
    let all = tenets(&reg);
    let ground = all.iter().find(|t| t.id == "the_ground_is_one").unwrap();
    let corpus = word_counts(&tree());
    // `ground` was dropped by the second replaced tenet and no line of this tree names it
    assert!(still_named(&all, ground, &corpus).is_empty());
    let old = all
        .iter()
        .find(|t| t.id == "a_snorf_holds_the_state")
        .unwrap();
    let named = still_named(&all, old, &corpus);
    assert_eq!(named.len(), 1);
    assert_eq!(named[0].0, "snorf");
}

#[test]
fn a_word_most_designs_use_is_prose_and_not_a_concept() {
    let d = tree();
    for n in 0 .. 8 {
        write(
            &d,
            &format!("mock/crates/kaski-{n}/DESIGN.md.tmpl"),
            "the snorf is everywhere in how these are written\n",
        );
    }
    let corpus = word_counts(&d);
    assert!(
        corpus.named("snorf").is_none(),
        "nine design files, nine naming it"
    );
    // the control: a word in one design of nine is a concept
    write(
        &d,
        "mock/crates/kaski-a/DESIGN.md.tmpl",
        "a snorf stands here\nthe wumble\n",
    );
    for n in 0 .. 8 {
        write(
            &d,
            &format!("mock/crates/kaski-{n}/DESIGN.md.tmpl"),
            "a plain design\n",
        );
    }
    let corpus = word_counts(&d);
    assert!(corpus.named("wumble").is_some());
}

#[test]
fn forbids_names_the_words_a_replaced_tenet_dropped_that_lines_still_name() {
    let d = tree();
    let reg = registry();
    let one = answered(&run(&reg, &d, &["forbids", "a_snorf_holds_the_state"])).to_owned();
    assert!(
        one.contains("binds:      no, superseded by a_plimp_holds_the_state"),
        "{one}"
    );
    assert!(one.contains("snorf 2/1/0"), "{one}");
    let none = answered(&run(&reg, &d, &["forbids", "the_ground_is_one"])).to_owned();
    assert!(
        none.contains("no word it used and its successors do not is named"),
        "{none}"
    );
    let all = answered(&run(&reg, &d, &["forbids"])).to_owned();
    assert!(
        all.starts_with("2 of 2 replaced tenets name no word"),
        "{all}"
    );
    assert!(
        all.contains("still named, design/code/content: snorf 2/1/0"),
        "{all}"
    );
}
