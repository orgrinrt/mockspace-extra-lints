//! The walk of the repository, over directories a test builds on disk.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use mockspace_lint_rules::Reading;

use super::*;

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn crates(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|n| (*n).to_string()).collect()
}

fn scan(root: &Path, all: &BTreeSet<String>, skip: &[String]) -> Vec<RustFile> {
    let reading = Reading::new(root);
    rust_files(&reading.view(), root, &root.join("mock"), all, skip)
}

#[test]
fn the_walk_reads_source_and_integration_tests_and_names_their_crate() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "mock/crates/alpha/src/lib.rs", "");
    write(d.path(), "mock/crates/alpha/tests/it.rs", "");
    write(d.path(), "mock/tools/x.rs", "");
    write(d.path(), "mock/crates/alpha/README.md", "");
    let found = scan(d.path(), &crates(&["alpha"]), &[]);
    let got: Vec<_> = found.iter().map(|f| (f.rel.as_str(), f.crate_name.as_str())).collect();
    assert_eq!(
        got,
        [
            ("mock/crates/alpha/src/lib.rs", "alpha"),
            ("mock/crates/alpha/tests/it.rs", "alpha"),
            ("mock/tools/x.rs", WORKSPACE),
        ]
    );
}

#[test]
fn build_output_vendored_sources_and_hidden_directories_are_not_entered() {
    let d = tempfile::tempdir().unwrap();
    for dir in ["target", "mock/vendor", "node_modules", ".git", "mock/.cache"] {
        write(d.path(), &format!("{dir}/x.rs"), "");
    }
    assert!(scan(d.path(), &crates(&[]), &[]).is_empty());
}

#[test]
fn the_record_directories_of_the_mock_directory_are_not_entered_but_a_namesake_elsewhere_is() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "mock/research/sketches/x/probe.rs", "");
    write(d.path(), "mock/design_rounds/1/probe.rs", "");
    write(d.path(), "mock/crates/alpha/research/real.rs", "");
    write(d.path(), "other/research/real.rs", "");
    let found = scan(d.path(), &crates(&["alpha"]), &[]);
    let got: Vec<_> = found.iter().map(|f| f.rel.as_str()).collect();
    assert_eq!(got, ["mock/crates/alpha/research/real.rs", "other/research/real.rs"]);
}

#[test]
fn a_path_matching_a_skip_glob_is_not_read() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "mock/crates/alpha/tests/fixtures/a.rs", "");
    write(d.path(), "mock/crates/alpha/tests/it.rs", "");
    let skip = ["**/fixtures/*.rs".to_string()];
    let found = scan(d.path(), &crates(&["alpha"]), &skip);
    let got: Vec<_> = found.iter().map(|f| f.rel.as_str()).collect();
    assert_eq!(got, ["mock/crates/alpha/tests/it.rs"]);
}

#[test]
fn matching_still_reads_the_record_directories_since_ledgers_live_there() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "mock/research/a/LEDGER.md", "");
    let found = matching_in(d.path(), &["mock/research/*/LEDGER.md".to_string()]);
    assert_eq!(found, ["mock/research/a/LEDGER.md"]);
}

#[test]
fn matching_takes_the_globs_and_nothing_without_any() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "mock/research/a/LEDGER.md", "");
    write(d.path(), "mock/research/a/NOTES.md", "");
    write(d.path(), "LEDGER.md", "");
    assert!(matching_in(d.path(), &[]).is_empty());
    let one = matching_in(d.path(), &["mock/research/*/LEDGER.md".to_string()]);
    assert_eq!(one, ["mock/research/a/LEDGER.md"]);
    let deep = matching_in(d.path(), &["**/LEDGER.md".to_string()]);
    assert_eq!(deep, ["LEDGER.md", "mock/research/a/LEDGER.md"]);
    let either = matching_in(d.path(), &["LEDGER.md".to_string(), "mock/**/NOTES.md".to_string()]);
    assert_eq!(either, ["LEDGER.md", "mock/research/a/NOTES.md"]);
}

/// The files among `files` that `pattern` reaches through [`matching`], as the lints see it.
fn reached(pattern: &str, files: &[&str]) -> Vec<String> {
    let d = tempfile::tempdir().unwrap();
    for f in files {
        write(d.path(), f, "");
    }
    matching_in(d.path(), &[pattern.to_string()])
}

#[test]
fn a_star_stops_at_a_slash_and_a_double_star_segment_does_not() {
    let files = ["a/b.md", "a/b/c.md", "a/b/d/c.md", "a/bc"];
    assert_eq!(reached("a/*.md", &files), ["a/b.md"]);
    assert_eq!(reached("a/**/c.md", &files), ["a/b/c.md", "a/b/d/c.md"]);
    assert_eq!(reached("a/b", &files), Vec::<String>::new());
}

#[test]
fn a_double_star_slash_matches_zero_directories_one_and_several() {
    let files = ["a/b/c/x.toml", "a/b/x.toml", "a/x.toml"];
    assert_eq!(reached("a/**/*.toml", &files), files);
    let tenet = ["mock/registry/tenet/k/x.toml", "mock/registry/tenet/x.toml"];
    assert_eq!(reached("mock/registry/tenet/**/*.toml", &tenet), tenet);
}

#[test]
fn a_double_star_slash_still_refuses_what_its_suffix_refuses() {
    let files = ["b/x.toml", "a/x.rs", "ax.toml", "a/x.toml/y"];
    assert_eq!(reached("a/**/*.toml", &files), Vec::<String>::new());
}

#[test]
fn a_question_mark_is_one_character_and_never_a_slash() {
    let files = ["mock/canon/a.md", "mock/canon/ab.md", "mock/canon/.md", "mock/cano/n.md"];
    assert_eq!(reached("mock/canon/?.md", &files), ["mock/canon/a.md"]);
    assert_eq!(reached("mock/cano?n.md", &files), Vec::<String>::new());
}

#[test]
fn a_double_star_inside_a_segment_is_a_star_and_does_not_cross_a_slash() {
    let files = ["mock/canon/x.md", "mock/canon/sub/x.md", "mock/c/x.md"];
    assert_eq!(reached("mock/c**/x.md", &files), ["mock/c/x.md", "mock/canon/x.md"]);
}

#[test]
fn a_trailing_double_star_takes_everything_below_and_stays_anchored() {
    let files = ["tenet/a.toml", "tenet/k/b.toml", "mock/tenet/c.toml"];
    assert_eq!(reached("tenet/**", &files), ["tenet/a.toml", "tenet/k/b.toml"]);
}

#[test]
fn a_bare_name_means_the_file_at_the_root_and_a_leading_dot_slash_is_dropped() {
    let files = ["LEDGER.md", "mock/LEDGER.md"];
    assert_eq!(reached("LEDGER.md", &files), ["LEDGER.md"]);
    assert_eq!(reached("./LEDGER.md", &files), ["LEDGER.md"]);
    assert_eq!(reached("**/LEDGER.md", &files), files);
}

fn matching_in(root: &Path, globs: &[String]) -> Vec<String> {
    let reading = Reading::new(root);
    matching(&reading.view(), root, globs)
}
