//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The words a tenet rules out: how a line is read, which files are, what lets a line through, and the
//! two verbs that report it. The fixtures rule out an invented word, `zorblax`, so that no file of the
//! repository these tests sit in names a word the real canon rules out.

use std::path::Path;

use crate::retired_words::{
    EXCUSE_REACH,
    Retiring,
    names,
    scanned,
    split_text,
    split_under,
    unit_of,
    words,
};
use crate::tests::{answered, layout, run, scratch, view, write};

/// The lines of a text that stand, which is what the lint holds a unit to.
fn scan_text(rel: &Path, body: &str, rules: &[Retiring]) -> Vec<crate::retired_words::Hit> {
    split_text(rel, body, rules, &layout()).0
}

fn w(text: &str) -> Vec<String> {
    words(text)
}

fn rule() -> Retiring {
    Retiring {
        id:    "there_is_no_zorblax".to_owned(),
        terms: vec!["zorblax".to_owned(), "zorblaxes".to_owned(), "first zorblax".to_owned()],
    }
}

#[test]
fn an_identifier_is_read_as_the_words_it_is_made_of() {
    assert_eq!(w("ZorblaxVolume"), ["zorblax", "volume"]);
    assert_eq!(w("zorblax_cells"), ["zorblax", "cells"]);
    assert_eq!(w("HTTPServer"), ["http", "server"]);
    assert_eq!(w("zorblax2"), ["zorblax", "2"]);
    assert_eq!(w("`Zorblax`::new(3)"), ["zorblax", "new", "3"]);
    assert_eq!(w("contract"), ["contract"]);
}

#[test]
fn a_word_that_contains_a_retired_one_is_not_it() {
    let term = w("zorblax");
    for line in ["pazorblax", "zorblaxer", "unzorblaxed", "zorblaxes are many"] {
        assert!(!names(&w(line), &term), "{line}");
    }
    for line in ["ZorblaxVolume", "a zorblax", "zorblax_cells", "(Zorblax)", "// zorblax."] {
        assert!(names(&w(line), &term), "{line}");
    }
}

#[test]
fn a_term_of_several_words_matches_only_those_words_in_a_row() {
    let term = w("first zorblax");
    assert!(names(&w("the first zorblax of a meadow"), &term));
    assert!(names(&w("FirstZorblax"), &term));
    assert!(!names(&w("the first and then a zorblax"), &term));
    assert!(!names(&w("zorblax first"), &term));
    assert!(!names(&w("anything"), &w("")));
}

#[test]
fn a_line_near_the_ruling_tenet_is_a_sentence_about_the_retirement() {
    let rules = [rule()];
    let near = format!("{}there_is_no_zorblax\n", "a zorblax stands here\n");
    assert!(scan_text(Path::new("mock/DESIGN.md.tmpl"), &near, &rules).is_empty());
    let gap = "\n".repeat(EXCUSE_REACH);
    let within = format!("a zorblax stands here{gap}after `there_is_no_zorblax`\n");
    assert!(scan_text(Path::new("mock/DESIGN.md.tmpl"), &within, &rules).is_empty());
    let beyond = format!("a zorblax stands here\n{gap}after `there_is_no_zorblax`\n");
    assert_eq!(
        scan_text(Path::new("mock/DESIGN.md.tmpl"), &beyond, &rules).len(),
        1
    );
    let other = "a zorblax stands here, after `there_is_no_zorblax_at_all`\n";
    assert_eq!(
        scan_text(Path::new("mock/DESIGN.md.tmpl"), other, &rules).len(),
        1
    );
}

#[test]
fn a_line_is_one_hit_however_many_terms_it_names() {
    let rules = [rule()];
    let hits = scan_text(
        Path::new("mock/a.md.tmpl"),
        "the first zorblax and a zorblax\n",
        &rules,
    );
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].line, 1);
}

#[test]
fn only_designs_code_content_shaders_and_the_root_readme_are_read() {
    for p in [
        "mock/DESIGN.md.tmpl",
        "mock/crates/kaski-clock/DESIGN.md.tmpl",
        "mock/crates/kaski-clock/src/lib.rs",
        "mock/lints/a.rs",
        "content/kaski/items.toml",
        "mock/crates/kaski-render/shaders/a.slang",
        "mock/crates/kaski-render/shaders/a.wgsl",
        "README.md",
    ] {
        assert!(scanned(Path::new(p), &layout()), "{p}");
    }
    for p in [
        "mock/research/202610241000_x/NOTES.md",
        "mock/design_rounds/202610240930_topic.x.md",
        "mock/registry/tenet/planet.toml",
        "mockspace.toml",
        "docs/PLANET.md",
        "crates/README.md",
    ] {
        assert!(!scanned(Path::new(p), &layout()), "{p}");
    }
}

#[test]
fn a_file_belongs_to_the_unit_the_debt_list_names() {
    for (p, unit) in [
        ("mock/crates/kaski-clock/src/lib.rs", "kaski-clock"),
        ("mock/crates/kaski-clock/DESIGN.md.tmpl", "kaski-clock"),
        ("mock/lints/a.rs", "lints"),
        ("mock/tools/canon/src/lib.rs", "tools"),
        ("mock/DESIGN.md.tmpl", "workspace"),
        ("content/kaski/items.toml", "content"),
    ] {
        assert_eq!(unit_of(Path::new(p), &layout()), unit, "{p}");
    }
}

fn tree() -> std::path::PathBuf {
    let d = scratch();
    write(
        &d,
        "mock/crates/kaski-a/DESIGN.md.tmpl",
        "a zorblax stands\nin the meadow\n",
    );
    write(
        &d,
        "mock/crates/kaski-a/src/lib.rs",
        "pub struct ZorblaxCell;\n",
    );
    write(
        &d,
        "mock/crates/kaski-b/DESIGN.md.tmpl",
        "the meadow stands on its ground\n",
    );
    write(&d, "mock/research/x/NOTES.md", "the zorblax was a window\n");
    d
}

fn registry() -> mockspace::RegistryView {
    view(&[
        ("subject::planet", &[("what", "The planet.")]),
        ("tenet::there_is_no_zorblax", &[
            ("subject", "planet"),
            ("what", "There is no zorblax."),
            ("rules_out", "zorblax, zorblaxes"),
        ]),
        ("tenet::the_zorblax_grows", &[
            ("subject", "planet"),
            ("what", "The zorblax grows."),
            ("rules_out", "grows"),
            ("superseded_by", "there_is_no_zorblax"),
        ]),
        ("tenet::the_ground_is_one", &[
            ("subject", "planet"),
            ("what", "One ground."),
        ]),
    ])
}

#[test]
fn contradicts_finds_the_design_and_the_code_naming_it_and_not_the_notes() {
    let d = tree();
    let reg = registry();
    let r = run(&reg, &d, &["contradicts"]);
    let text = answered(&r);
    assert!(text.contains("2 lines in the repository"), "{text}");
    assert!(
        text.contains("design  mock/crates/kaski-a/DESIGN.md.tmpl:1"),
        "{text}"
    );
    assert!(text.contains("by unit (1): kaski-a 2"), "{text}");
    assert!(!text.contains("research"), "{text}");
    assert!(
        !text.contains("the_zorblax_grows"),
        "a superseded tenet binds nothing: {text}"
    );
}

#[test]
fn the_control_a_design_that_conforms_is_not_flagged() {
    let d = tree();
    let reg = registry();
    let r = run(&reg, &d, &[
        "contradicts",
        "mock/crates/kaski-b/DESIGN.md.tmpl",
    ]);
    let text = answered(&r);
    assert!(
        text.contains("0 lines in mock/crates/kaski-b/DESIGN.md.tmpl"),
        "{text}"
    );
    assert!(text.contains("nothing names them"), "{text}");
    let flagged = run(&reg, &d, &["contradicts", "mock/crates/kaski-a"]);
    let text = answered(&flagged);
    assert!(text.contains("2 lines in mock/crates/kaski-a"), "{text}");
    assert!(
        text.contains("DESIGN.md.tmpl:1: a zorblax stands"),
        "{text}"
    );
}

#[test]
fn contradicts_with_a_path_also_says_which_tenets_the_path_names() {
    let d = tree();
    write(
        &d,
        "mock/crates/kaski-b/DESIGN.md.tmpl",
        "stands on `the_ground_is_one` and `there_is_no_zorblax`\n",
    );
    let reg = registry();
    let r = run(&reg, &d, &["contradicts", "mock/crates/kaski-b"]);
    let text = answered(&r);
    assert!(
        text.contains("the_ground_is_one (binds, 1 lines, rules out: nothing)"),
        "{text}"
    );
    assert!(
        text.contains("there_is_no_zorblax (binds, 1 lines, rules out: zorblax, zorblaxes)"),
        "{text}"
    );
}

#[test]
fn contradicts_names_a_path_that_is_not_there_and_a_registry_that_rules_nothing_out() {
    let d = tree();
    let reg = registry();
    assert!(answered(&run(&reg, &d, &["contradicts", "mock/crates/nope"])).contains("no path"));
    let bare = view(&[("tenet::the_ground_is_one", &[("what", "One ground.")])]);
    assert!(
        answered(&run(&bare, &d, &["contradicts"])).contains("no binding tenet rules out a word")
    );
}

#[test]
fn forbids_says_what_a_tenet_rules_out_and_what_it_replaces() {
    let d = tree();
    let reg = registry();
    let text = answered(&run(&reg, &d, &["forbids", "there_is_no_zorblax"])).to_owned();
    assert!(text.contains("binds:      yes"), "{text}");
    assert!(text.contains("rules out:  zorblax, zorblaxes"), "{text}");
    assert!(
        text.contains("supersedes: the_zorblax_grows (rules out: grows)"),
        "{text}"
    );
    let bare = answered(&run(&reg, &d, &["forbids", "the_ground_is_one"])).to_owned();
    assert!(bare.contains("names no word it rules out"), "{bare}");
    assert!(matches!(
        run(&reg, &d, &["forbids", "nope"]).outcome,
        mockspace::tool::Outcome::Inconclusive { .. }
    ));
}

#[test]
fn forbids_with_no_tenet_lists_the_replaced_tenets_that_name_no_word() {
    let d = tree();
    let reg = view(&[
        ("subject::planet", &[("what", "The planet.")]),
        ("tenet::there_is_no_zorblax", &[
            ("subject", "planet"),
            ("what", "There is no zorblax."),
            ("rules_out", "zorblax"),
        ]),
        ("tenet::the_zorblax_grows", &[
            ("subject", "planet"),
            ("what", "The zorblax grows."),
            ("superseded_by", "there_is_no_zorblax"),
        ]),
        ("tenet::the_ground_is_one", &[
            ("subject", "planet"),
            ("what", "One ground."),
        ]),
        ("tenet::the_ground_is_flat", &[
            ("subject", "planet"),
            ("what", "The ground is flat."),
            ("superseded_by", "the_ground_is_one"),
        ]),
    ]);
    let text = answered(&run(&reg, &d, &["forbids"])).to_owned();
    assert!(
        text.starts_with("1 of 2 replaced tenets name no word"),
        "{text}"
    );
    assert!(
        text.contains("the_ground_is_flat (replaced by the_ground_is_one)"),
        "{text}"
    );
    assert!(
        !text.contains("the_zorblax_grows"),
        "a tenet whose successor names the word is not owed one: {text}"
    );
}

#[test]
fn a_line_let_through_by_the_ruling_tenet_is_counted_and_listed_for_a_path() {
    let d = tree();
    write(
        &d,
        "mock/crates/kaski-b/DESIGN.md.tmpl",
        "a zorblax stands here\nafter `there_is_no_zorblax`\n",
    );
    let reg = registry();
    let whole = answered(&run(&reg, &d, &["contradicts"])).to_owned();
    assert!(
        whole.contains("let through, within 3 lines of the ruling tenet's id: 1"),
        "{whole}"
    );
    assert!(whole.contains("name a path to list them"), "{whole}");
    let one = answered(&run(&reg, &d, &["contradicts", "mock/crates/kaski-b"])).to_owned();
    assert!(
        one.contains("mock/crates/kaski-b/DESIGN.md.tmpl:1: a zorblax stands here"),
        "{one}"
    );
    assert!(one.contains("0 lines in mock/crates/kaski-b"), "{one}");
}

#[test]
fn a_line_that_only_cites_the_ruling_tenet_names_nothing_held_or_let_through() {
    let rules = [rule()];
    let path = Path::new("mock/DESIGN.md.tmpl");
    let (held, excused) = split_text(path, "it stands on `there_is_no_zorblax`\n", &rules, &layout());
    assert!(
        held.is_empty() && excused.is_empty(),
        "{held:?} {excused:?}"
    );
    // the control: the same words outside the id are a hit, and the id beside them lets it through
    let (held, excused) = split_text(path, "a zorblax stands on `there_is_no_zorblax`\n", &rules, &layout());
    assert!(held.is_empty());
    assert_eq!(excused.len(), 1);
    // a longer id is another tenet's, and neither blanked nor an excuse
    let (held, excused) = split_text(path, "stands on `there_is_no_zorblax_at_all`\n", &rules, &layout());
    assert_eq!((held.len(), excused.len()), (1, 0));
}

#[test]
fn contradicts_reads_several_paths_in_one_pass_and_lists_the_lines_of_those_only() {
    // Mutation: only the first path is read, or the paths after it are read as the whole repository.
    let d = tree();
    let reg = registry();
    let r = run(&reg, &d, &[
        "contradicts",
        "mock/crates/kaski-a/src/lib.rs",
        "mock/crates/kaski-b/DESIGN.md.tmpl",
    ]);
    let text = answered(&r);
    assert!(text.contains("1 lines in 2 paths"), "{text}");
    assert!(
        text.contains("mock/crates/kaski-a/src/lib.rs:1: pub struct ZorblaxCell;"),
        "{text}"
    );
    assert!(
        !text.contains("DESIGN.md.tmpl:1: a zorblax"),
        "a path that is not named is not read: {text}"
    );
}

#[test]
fn contradicts_lists_the_lines_of_every_path_it_is_given() {
    let d = tree();
    let reg = registry();
    let r = run(&reg, &d, &[
        "contradicts",
        "mock/crates/kaski-a/DESIGN.md.tmpl",
        "mock/crates/kaski-a/src/lib.rs",
        "mock/crates/kaski-b/DESIGN.md.tmpl",
    ]);
    let text = answered(&r);
    assert!(text.contains("2 lines in 3 paths"), "{text}");
    assert!(
        text.contains("mock/crates/kaski-a/DESIGN.md.tmpl:1: a zorblax stands"),
        "{text}"
    );
    assert!(
        text.contains("mock/crates/kaski-a/src/lib.rs:1: pub struct ZorblaxCell;"),
        "{text}"
    );
    assert!(
        text.contains("there_is_no_zorblax (rules out: zorblax, zorblaxes)"),
        "each line sits under the row that rules its word out: {text}"
    );
}

#[test]
fn contradicts_with_a_path_among_several_that_is_not_there_names_it() {
    let d = tree();
    let reg = registry();
    let r = run(&reg, &d, &[
        "contradicts",
        "mock/crates/kaski-a/src/lib.rs",
        "mock/crates/gone/src/lib.rs",
    ]);
    let text = answered(&r);
    assert!(
        text.contains("there is no path `mock/crates/gone/src/lib.rs`"),
        "{text}"
    );
}

#[test]
fn split_under_with_no_path_reads_the_whole_repository_and_with_paths_only_those() {
    let d = tree();
    let rules = [rule()];
    let whole = split_under(&mockspace::reading::Reading::new(&d).view(), &d, &rules, &[], &layout()).0;
    assert_eq!(whole.len(), 2, "{whole:?}");
    let one = split_under(&mockspace::reading::Reading::new(&d).view(), &d, &rules, &[Path::new("mock/crates/kaski-a/src")], &layout()).0;
    assert_eq!(one.len(), 1, "{one:?}");
    let two = split_under(&mockspace::reading::Reading::new(&d).view(), &d, &rules, &[
        Path::new("mock/crates/kaski-a/src"),
        Path::new("mock/crates/kaski-a/DESIGN.md.tmpl"),
    ], &layout())
    .0;
    assert_eq!(two.len(), 2, "{two:?}");
}

#[test]
fn contradicts_reads_no_word_a_pending_or_demoted_tenet_rules_out() {
    let d = tree();
    let held = |standing: &str| {
        view(&[("tenet::there_is_no_zorblax", &[
            ("subject", "planet"),
            ("what", "There is no zorblax."),
            ("rules_out", "zorblax"),
            ("standing", standing),
        ])])
    };
    for unbound in ["pending", "demoted"] {
        let reg = held(unbound);
        let r = run(&reg, &d, &["contradicts"]);
        let text = answered(&r);
        assert!(!text.contains("kaski-a/DESIGN.md.tmpl"), "a {unbound} tenet binds nothing: {text}");
    }
    let reg = held("ratified");
    let r = run(&reg, &d, &["contradicts"]);
    let text = answered(&r);
    assert!(text.contains("design  mock/crates/kaski-a/DESIGN.md.tmpl:1"), "{text}");
}
