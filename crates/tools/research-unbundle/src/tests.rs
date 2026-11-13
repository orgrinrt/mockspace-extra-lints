//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The maker writes a bundle as the files of a note, cuts what is long where a heading begins, and
//! refuses what it cannot place whole: a name that leaves the directory, a file already there, a bundle
//! that holds no file.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use mockspace::RegistryView;
use mockspace::tool::{Outcome, Tool, ToolContext, ToolReport, contract_faults, missing_required};

use crate::split::parts;
use crate::{LIMIT, ResearchUnbundle, bundle, unbundle};

const DIR: &str = "mock/research/202610230400_a-note";

fn scratch() -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let d = std::env::temp_dir().join(format!(
        "research-unbundle-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(d.join("mock/research")).expect("a scratch repository");
    d
}

fn put(repo: &Path, text: &str) -> String {
    std::fs::write(repo.join("bundle.txt"), text).expect("the bundle");
    "bundle.txt".to_string()
}

fn read(repo: &Path, name: &str) -> String {
    std::fs::read_to_string(repo.join(DIR).join(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn lines(n: usize, tag: &str) -> String {
    (0 .. n).map(|i| format!("{tag} line {i}\n")).collect()
}

/// A section: its heading and `n - 1` lines under it.
fn section(title: &str, n: usize) -> String {
    format!("## {title}\n{}", lines(n - 1, title))
}

fn run_tool(repo: &Path, args: &[&str]) -> ToolReport {
    let crates = BTreeSet::new();
    let reg = RegistryView::new(Default::default(), Default::default());
    let ctx = ToolContext {
        mock_dir: &repo.join("mock"),
        repo_root: repo,
        all_crates: &crates,
        src_dirs: &[],
        args,
        stdin: None,
        registry: &reg,
    };
    ResearchUnbundle.run(&ctx)
}

fn err(repo: &Path, text: &str) -> String {
    let b = put(repo, text);
    match unbundle(repo, &b, DIR) {
        Err(e) => e,
        Ok(_) => panic!("the bundle was accepted: {text:?}"),
    }
}

#[test]
fn the_declaration_is_a_maker_the_engine_accepts() {
    assert!(
        contract_faults(&ResearchUnbundle, None).is_empty(),
        "{:?}",
        contract_faults(&ResearchUnbundle, None)
    );
    let missing: Vec<&str> = missing_required(&ResearchUnbundle, &[])
        .iter()
        .map(|a| a.name)
        .collect();
    assert_eq!(missing, ["bundle", "dir"]);
}

#[test]
fn every_file_is_written_under_the_directory_with_its_text_and_one_final_newline() {
    let repo = scratch();
    let b = put(
        &repo,
        "=== FILE: README.md ===\n# note\n\nbody\n\n\n=== FILE: gaps/sky.md ===\nthe sky\n=== FILE: data.toml ===\nk = 1\n",
    );
    let w = unbundle(&repo, &b, DIR).expect("written");
    assert_eq!(w.files, 3);
    assert_eq!(w.report, "");
    assert_eq!(read(&repo, "README.md"), "# note\n\nbody\n");
    assert_eq!(read(&repo, "gaps/sky.md"), "the sky\n");
    assert_eq!(read(&repo, "data.toml"), "k = 1\n");
}

#[test]
fn a_file_over_the_limit_is_cut_before_headings_and_loses_nothing() {
    let repo = scratch();
    let whole = format!(
        "# top\n{}{}{}",
        section("a", 300),
        section("b", 300),
        section("c", 300)
    );
    let b = put(&repo, &format!("=== FILE: long.md ===\n{whole}"));
    let w = unbundle(&repo, &b, DIR).expect("written");
    assert_eq!(w.files, 3);
    assert!(
        w.report
            .contains("long.md into long.md, long-2.md, long-3.md"),
        "{}",
        w.report
    );
    let got = [read(&repo, "long.md"), read(&repo, "long-2.md"), read(&repo, "long-3.md")];
    assert_eq!(got.concat(), whole, "the parts, in order, are the file");
    for p in &got {
        assert!(p.lines().count() <= LIMIT);
    }
    assert!(got[1].starts_with("## b\n") && got[2].starts_with("## c\n"));
}

#[test]
fn whole_sections_are_packed_into_a_part_while_they_fit() {
    let t = format!(
        "{}{}{}",
        section("a", 200),
        section("b", 200),
        section("c", 200)
    );
    let p = parts("x.md", &t, LIMIT);
    assert_eq!(
        p.len(),
        2,
        "{:?}",
        p.iter()
            .map(|(n, b)| (n, b.lines().count()))
            .collect::<Vec<_>>()
    );
    assert_eq!(p[0].1.lines().count(), 400);
    assert_eq!(p[1].1.lines().count(), 200);
}

#[test]
fn a_part_may_be_exactly_the_limit() {
    let t = format!(
        "{}{}{}",
        section("a", 300),
        section("b", 200),
        section("c", 10)
    );
    let p = parts("x.md", &t, LIMIT);
    assert_eq!(p.len(), 2);
    assert_eq!(p[0].1.lines().count(), LIMIT);
}

#[test]
fn a_file_at_the_limit_and_a_file_that_is_not_markdown_are_left_whole() {
    let at = format!("{}{}", section("a", 250), section("b", 250));
    assert_eq!(parts("x.md", &at, LIMIT).len(), 1);
    let toml = lines(900, "k");
    let p = parts("x.toml", &toml, LIMIT);
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].1, toml);
}

#[test]
fn a_section_longer_than_the_limit_is_cut_at_a_blank_line_and_no_part_is_over() {
    let mut body = String::from("## big\n");
    for block in 0 .. 8 {
        body.push_str(&lines(149, &format!("blk{block}")));
        body.push('\n');
    }
    let p = parts("x.md", &body, LIMIT);
    assert!(p.len() >= 3);
    for (n, b) in &p {
        assert!(b.lines().count() <= LIMIT, "{n} has {}", b.lines().count());
    }
    assert_eq!(p.iter().map(|(_, b)| b.as_str()).collect::<String>(), body);
    // every cut but the last fell after a blank line, so no paragraph was cut through
    for (n, b) in &p[.. p.len() - 1] {
        assert!(b.ends_with("\n\n"), "{n} was cut inside a paragraph");
    }
}

#[test]
fn a_heading_inside_a_code_fence_is_not_a_cut_point() {
    // Cut at the fenced heading the file would be two parts of 401 and 152 lines, the second opening on it.
    let t = format!(
        "## a\n{}```\n## not a heading\n```\n{}",
        lines(399, "a"),
        lines(150, "tail")
    );
    let p = parts("x.md", &t, LIMIT);
    assert!(p.len() >= 2);
    assert!(
        p.iter().all(|(_, b)| !b.starts_with("## not a heading")),
        "a fenced line was taken for a heading"
    );
    assert_eq!(p.iter().map(|(_, b)| b.as_str()).collect::<String>(), t);
}

#[test]
fn a_bundle_that_holds_no_file_or_text_outside_one_is_refused() {
    let repo = scratch();
    assert!(err(&repo, "").contains("names no file"));
    assert!(err(&repo, "some words\n=== FILE: a.md ===\nx\n").contains("belongs to no file"));
    assert!(err(&repo, "=== FILE: a.md ===\n\n\n").contains("`a.md` is empty"));
}

#[test]
fn a_name_that_leaves_the_directory_or_repeats_is_refused() {
    let repo = scratch();
    for name in ["../escape.md", "/abs.md", "a//b.md", "./a.md", "a b.md", "a/../b.md"] {
        let e = err(&repo, &format!("=== FILE: {name} ===\nx\n"));
        assert!(e.contains("is not a name"), "{name}: {e}");
    }
    assert!(
        err(&repo, "=== FILE: a.md ===\nx\n=== FILE: a.md ===\ny\n").contains("is given twice")
    );
    assert!(!repo.join(DIR).exists(), "a refused bundle writes nothing");
}

#[test]
fn nothing_is_overwritten_and_a_refusal_leaves_no_file_of_the_others() {
    let repo = scratch();
    let b = put(
        &repo,
        "=== FILE: a.md ===\nnew a\n=== FILE: b.md ===\nnew b\n",
    );
    std::fs::create_dir_all(repo.join(DIR)).expect("the note");
    std::fs::write(repo.join(DIR).join("b.md"), "old b\n").expect("old");
    let e = unbundle(&repo, &b, DIR).err().expect("refused");
    assert!(e.contains("b.md is already there"), "{e}");
    assert_eq!(read(&repo, "b.md"), "old b\n");
    assert!(
        !repo.join(DIR).join("a.md").exists(),
        "a.md is not written beside a refusal"
    );
}

#[test]
fn only_a_note_directory_in_the_research_directory_is_written() {
    let repo = scratch();
    let b = put(&repo, "=== FILE: a.md ===\nx\n");
    for dir in [
        "mock/research/a-note",
        "mock/research/2026102304_a-note",
        "mock/research/202610230400_A-Note",
        "mock/research/202610230400_",
        "mock/research/sketches/202610230400_a-note",
        "mock/other/202610230400_a-note",
        "202610230400_a-note",
    ] {
        let e = unbundle(&repo, &b, dir)
            .err()
            .unwrap_or_else(|| panic!("{dir} was accepted"));
        assert!(e.contains("is not a note's directory"), "{dir}: {e}");
    }
    assert!(
        unbundle(&repo, &b, &format!("{DIR}/")).is_ok(),
        "a trailing slash is the same directory"
    );
}

#[test]
fn the_run_reports_a_missing_argument_and_an_unreadable_bundle_as_inconclusive() {
    let repo = scratch();
    for args in [&[][..], &["bundle.txt"][..], &["nosuch.txt", DIR][..]] {
        match run_tool(&repo, args).outcome {
            Outcome::Inconclusive {
                reason,
            } => assert!(!reason.is_empty()),
            other => panic!("{args:?}: {other:?}"),
        }
    }
    put(&repo, "=== FILE: a.md ===\nx\n");
    let r = run_tool(&repo, &["bundle.txt", DIR]);
    assert!(
        matches!(r.outcome, Outcome::Clean {
            examined: 1,
        }),
        "{:?}",
        r.outcome
    );
}

#[test]
fn the_header_is_the_exact_line_and_nothing_else_opens_a_file() {
    let files = bundle::parse("=== FILE: a.md ===\n=== FILE: not a header\n==== FILE: b.md ===\n=== FILE: c.md === \ntext\n")
        .expect("one file");
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].0, "a.md");
    assert!(files[0].1.contains("=== FILE: c.md === "), "{}", files[0].1);
}
