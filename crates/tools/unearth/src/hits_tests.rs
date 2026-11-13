//! What a hit line says: when, how long ago, by whom, whether the canon has spoken since, and how
//! many hits there are. The ranking behind them is the index's and is tested where it lives.
//!
//! Every hit here is a real file in a planted directory read by the real index, dated by the stamp
//! in its name and classed by the directory it sits in; the canon's last change is a real commit.

use std::path::Path;
use std::process::Command;

use corpus::day::ymd;
use mockspace::RegistryView;

use super::*;
use crate::corpus_tests::{DOC, Planted, registry, run};

fn put(dir: &Planted, rel: &str, text: &str) {
    let path = dir.0.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// The stamp a path carries for a day, which is how the index dates a file with no commit.
fn stamp(day: i64) -> String {
    format!("{}0000", ymd(day).replace('-', ""))
}

fn git(dir: &Path, day: i64, args: &[&str]) {
    let when = format!("{} +0000", day * 86_400 + 43_200);
    let status = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .env("GIT_COMMITTER_DATE", &when)
        .env("GIT_AUTHOR_DATE", &when)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

/// Commit everything planted so far as of `day`.
fn commit_all(dir: &Planted, day: i64) {
    git(&dir.0, day, &["init", "-q"]);
    git(&dir.0, day, &["add", "-A"]);
    git(&dir.0, day, &["commit", "-q", "-m", "plant"]);
}

const SOURCE: &str = "registry/tenet/secrets.toml";

fn canon_file(dir: &Planted, id: &str) {
    put(dir, SOURCE, &format!("[[tenet]]\nid = \"{id}\"\n"));
}

#[test]
fn a_hit_is_one_line_of_date_age_class_and_place_with_the_excerpt_under_it() {
    let dir = Planted::new("hitline");
    let name = format!("{}_a.md", stamp(today() - 12));
    put(&dir, &format!("design_rounds/{name}"), DOC);
    let out = run(&dir, &registry(&[]), &["passphrase"]).output;

    let line = out
        .lines()
        .find(|l| l.contains(&format!("design_rounds/{name}:1")))
        .unwrap_or_else(|| panic!("the hit is named with its line: {out}"));
    assert!(line.contains(&ymd(today() - 12)), "its date: {line}");
    assert!(line.contains("12d"), "its age: {line}");
    assert!(line.contains("design"), "its class: {line}");
    assert!(line.contains("[UNCITED]"), "its marker: {line}");

    // THE CONTROL: a document with no date says so and invents no age.
    let bare = Planted::new("hitline_undated");
    put(&bare, "design_rounds/b.md", DOC);
    let out = run(&bare, &registry(&[]), &["passphrase"]).output;
    let line = out
        .lines()
        .find(|l| l.contains("design_rounds/b.md"))
        .unwrap();
    assert!(line.contains("undated") && !line.contains("0d"), "{line}");
}

/// A planted directory whose one research note is stamped `note` and whose canon file was
/// committed on `canon`, or never committed at all where that is `None`.
fn stale(name: &str, note: i64, canon: Option<i64>, row_id: &str) -> String {
    let dir = Planted::new(name);
    put(&dir, &format!("research/{}_a.md", stamp(note)), DOC);
    canon_file(&dir, row_id);
    if let Some(day) = canon {
        commit_all(&dir, day);
    }
    let row = [("says", "the passphrase is theirs")];
    let reg: RegistryView = registry(&[("tenet::r", &row)]);
    run(&dir, &reg, &["passphrase"]).output
}

#[test]
fn a_hit_older_than_the_canon_last_changed_is_marked_and_a_newer_one_is_not() {
    let out = stale("stale", 20_000, Some(20_500), "r");
    assert!(
        out.contains(&format!("OLDER THAN THE CANON ({})", ymd(20_500))),
        "{out}"
    );

    // THE CONTROLS: a canon that changed before the document, a canon file git has no day for,
    // and a row that was never told where it lives each mark nothing.
    assert!(!stale("stale_before", 20_000, Some(19_000), "r").contains("OLDER THAN"));
    assert!(!stale("stale_uncommitted", 20_000, None, "r").contains("OLDER THAN"));
    assert!(!stale("stale_elsewhere", 20_000, Some(20_500), "another").contains("OLDER THAN"));
}

#[test]
fn five_hits_are_shown_and_the_rest_are_counted_not_listed() {
    let dir = Planted::new("limit");
    for i in 0 .. 8 {
        put(&dir, &format!("research/{}_n{i}.md", stamp(20_000)), DOC);
    }
    let out = run(&dir, &registry(&[]), &["passphrase"]).output;
    // The verdict lists uncited paths a second time, so only the hit lines carry the marker.
    let hit_lines = out.lines().filter(|l| l.contains("[UNCITED")).count();
    assert_eq!(hit_lines, 5, "{out}");
    assert!(out.contains("3 more documents matched"), "{out}");

    // THE CONTROL: fewer than the limit says nothing about more.
    let few = Planted::new("limit_few");
    put(&few, "research/only.md", DOC);
    assert!(
        !run(&few, &registry(&[]), &["passphrase"])
            .output
            .contains("more documents")
    );
}
