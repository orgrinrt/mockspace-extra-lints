//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

use std::collections::BTreeSet;
use std::path::PathBuf;

use mockspace::RegistryView;
use mockspace::tool::{Outcome, Tool, ToolContext, ToolReport};

use super::{KnownRed, KnownRedSet, store};

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("known-red-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("registry")).unwrap();
    d
}

fn run(tool: &dyn Tool, mock: &PathBuf, args: &[&str]) -> ToolReport {
    let crates = BTreeSet::new();
    let registry = RegistryView::default();
    tool.run(&ToolContext {
        mock_dir: mock,
        repo_root: mock,
        all_crates: &crates,
        src_dirs: &[],
        args,
        stdin: None,
        registry: &registry,
    })
}

const ADD: &[&str] = &[
    "--crate",
    "kaski-sim",
    "--test",
    "tests::hurry::tires",
    "--sha",
    "abc123",
    "--by",
    "me",
    "--why",
    "ticket x",
    "--at",
    "2026-11-02T10:00:00Z",
];

fn refused(r: &ToolReport) -> bool {
    matches!(r.outcome, Outcome::Inconclusive { .. })
}

#[test]
fn an_added_test_is_known_with_its_sha_and_why() {
    let m = scratch("add");
    assert!(!refused(&run(&KnownRedSet, &m, ADD)));
    let r = run(&KnownRed, &m, &["kaski-sim", "tests::hurry::tires"]);
    assert_eq!(r.output, "known abc123 ticket x\n");
}

#[test]
fn a_test_not_in_the_catalogue_is_unknown_and_so_is_the_same_filter_in_another_crate() {
    let m = scratch("unknown");
    run(&KnownRedSet, &m, ADD);
    assert_eq!(
        run(&KnownRed, &m, &["kaski-sim", "other"]).output,
        "unknown\n"
    );
    assert_eq!(
        run(&KnownRed, &m, &["kaski-world", "tests::hurry::tires"]).output,
        "unknown\n"
    );
}

#[test]
fn an_empty_catalogue_answers_unknown() {
    let m = scratch("empty");
    assert_eq!(run(&KnownRed, &m, &["kaski-sim", "t"]).output, "unknown\n");
}

#[test]
fn adding_the_same_test_again_replaces_its_row() {
    let m = scratch("replace");
    run(&KnownRedSet, &m, ADD);
    let mut again = ADD.to_vec();
    again[5] = "def456";
    run(&KnownRedSet, &m, &again);
    assert_eq!(store::load(&m).unwrap().len(), 1);
    assert_eq!(
        run(&KnownRed, &m, &["kaski-sim", "tests::hurry::tires"]).output,
        "known def456 ticket x\n"
    );
}

#[test]
fn a_cleared_test_is_unknown_again_and_clearing_what_is_not_there_is_refused() {
    let m = scratch("clear");
    run(&KnownRedSet, &m, ADD);
    let c = ["--clear", "--crate", "kaski-sim", "--test", "tests::hurry::tires"];
    assert!(!refused(&run(&KnownRedSet, &m, &c)));
    assert_eq!(
        run(&KnownRed, &m, &["kaski-sim", "tests::hurry::tires"]).output,
        "unknown\n"
    );
    assert!(refused(&run(&KnownRedSet, &m, &c)));
}

#[test]
fn an_add_missing_a_field_or_naming_a_flag_it_does_not_take_is_refused_and_writes_nothing() {
    let m = scratch("refuse");
    assert!(refused(&run(&KnownRedSet, &m, &[
        "--crate",
        "kaski-sim",
        "--test",
        "t"
    ])));
    let mut bad = ADD.to_vec();
    bad.push("--nope");
    assert!(refused(&run(&KnownRedSet, &m, &bad)));
    assert!(!store::path(&m).exists());
}

#[test]
fn a_wrong_arity_question_is_refused() {
    let m = scratch("arity");
    run(&KnownRedSet, &m, ADD);
    assert!(refused(&run(&KnownRed, &m, &[])));
    assert!(refused(&run(&KnownRed, &m, &["kaski-sim"])));
}

#[test]
fn two_long_filters_that_slug_alike_get_two_rows() {
    let m = scratch("long");
    let long =
        "tests::a_very_long_module_name::a_very_long_test_name_that_goes_past_the_slug_limit";
    for tail in ["_one", "_two"] {
        let t = format!("{long}{tail}");
        run(&KnownRedSet, &m, &[
            "--crate", "c", "--test", &t, "--sha", "s", "--by", "b", "--why", "w",
        ]);
    }
    assert_eq!(store::load(&m).unwrap().len(), 2);
}

#[test]
fn a_why_with_quotes_and_a_newline_survives_the_file() {
    let m = scratch("quote");
    run(&KnownRedSet, &m, &[
        "--crate",
        "c",
        "--test",
        "t",
        "--sha",
        "s",
        "--by",
        "b",
        "--why",
        "say \"hi\"\nthere",
    ]);
    assert_eq!(store::load(&m).unwrap()[0].why, "say \"hi\"\nthere");
}
