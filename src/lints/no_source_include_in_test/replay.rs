use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use mockspace_lint_rules::RegistryView;

use super::tests::{allowing, check, paths, write};
use super::*;
use super::PARSED;
use crate::test_support::Kept;



fn check_kept_with(lint: &NoSourceIncludeInTest, root: &Path, store: &Kept) -> Vec<LintError> {
    check_kept_in(lint, root, store, &["alpha"])
}

/// As `check_kept_with`, with `crates` the names the engine says are crates.
fn check_kept_in(lint: &NoSourceIncludeInTest, root: &Path, store: &Kept, crates: &[&str]) -> Vec<LintError> {
    let crates: BTreeSet<String> = crates.iter().map(|c| c.to_string()).collect();
    let reg = RegistryView::new(BTreeMap::new(), BTreeMap::new());
    let ctx = RepoContext {
        mock_dir: &root.join("mock"),
        repo_root: root,
        all_crates: &crates,
        src_dirs: &[],
        invocation: None,
        canon_paths: &[],
        open_panels: &[],
        registry: &reg,
        verdicts: Some(store),
    };
    lint.check_repo(&ctx)
}

fn check_kept(root: &Path, store: &Kept) -> Vec<LintError> {
    check_kept_with(&NoSourceIncludeInTest::default(), root, store)
}

/// How many per-file facts were judged again since the counts were cleared.
fn facts_judged(store: &Kept) -> usize {
    store.misses(FACTS) + store.misses(FACTS_IN_TEST)
}

/// How many per-file facts were replayed since the counts were cleared.
fn facts_replayed(store: &Kept) -> usize {
    store.hits(FACTS) + store.hits(FACTS_IN_TEST)
}

fn shown(found: &[LintError]) -> Vec<String> {
    found.iter().map(|e| format!("{}:{}:{}:{:?}", e.crate_name, e.line, e.message, e.path)).collect()
}

#[test]
fn a_saved_run_finds_what_an_unsaved_one_finds_across_an_edit_that_moves_a_test_module() {
    let d = tempfile::tempdir().unwrap();
    let store = Kept::default();
    write(d.path(), "mock/crates/alpha/src/lib.rs", "#[cfg(test)]\nmod probe;\n");
    write(d.path(), "mock/crates/alpha/src/probe.rs", "fn t() { let s = include_str!(\"lib.rs\"); }\n");
    let full = check(d.path());
    assert_eq!(full.len(), 1, "the control: the declared test module's include is reported");
    assert_eq!(shown(&check_kept(d.path(), &store)), shown(&full), "cold");
    assert_eq!(shown(&check_kept(d.path(), &store)), shown(&full), "warm");

    // The module stops being a test module: the include in the untouched file stops being a finding.
    write(d.path(), "mock/crates/alpha/src/lib.rs", "mod probe;\n");
    let full = check(d.path());
    assert!(full.is_empty(), "{full:?}");
    assert_eq!(shown(&check_kept(d.path(), &store)), shown(&full), "after the declaring file moved");
}

/// A repository of a crate whose `lib.rs` declares the test module `probe`, which includes source,
/// beside `other.rs`, which none declares.
fn declared_probe(root: &Path) {
    write(root, "mock/crates/alpha/src/lib.rs", "#[cfg(test)]\nmod probe;\n");
    write(root, "mock/crates/alpha/src/probe.rs", "fn t() { let s = include_str!(\"lib.rs\"); }\n");
    write(root, "mock/crates/alpha/src/other.rs", "fn u() {}\n");
}

#[test]
fn a_warm_run_replays_the_whole_verdict_and_judges_nothing() {
    let d = tempfile::tempdir().unwrap();
    let store = Kept::default();
    declared_probe(d.path());
    let cold = check_kept(d.path(), &store);
    assert_eq!(cold.len(), 1, "{cold:?}");
    assert_eq!(store.hits(NAME), 0, "the control: a cold run replays nothing");
    assert_eq!(store.misses(NAME), 1);
    store.clear_counts();
    let warm = check_kept(d.path(), &store);
    assert_eq!(shown(&warm), shown(&cold));
    assert_eq!(store.hits(NAME), 1, "the warm run is one replay");
    assert_eq!(store.misses(NAME), 0);
    assert_eq!(facts_judged(&store), 0, "and parses and walks no file");
}

#[test]
fn an_edit_to_one_file_judges_that_file_alone() {
    let d = tempfile::tempdir().unwrap();
    let store = Kept::default();
    declared_probe(d.path());
    check_kept(d.path(), &store);
    store.clear_counts();
    write(d.path(), "mock/crates/alpha/src/other.rs", "fn u() { let _ = 1; }\n");
    let kept = check_kept(d.path(), &store);
    assert_eq!(shown(&kept), shown(&check(d.path())));
    assert_eq!(store.misses(NAME), 1, "the whole verdict follows every file, so it is judged again");
    assert_eq!(facts_judged(&store), 1, "but only the file that moved is parsed");
    // `lib.rs` and `probe.rs` as plain files, then `probe.rs` again as the test file the first pass found.
    assert_eq!(facts_replayed(&store), 3, "and the others are replayed, each under the pair it is asked under");
}

#[test]
fn a_change_to_the_including_file_is_seen_by_a_saved_run() {
    let d = tempfile::tempdir().unwrap();
    let store = Kept::default();
    declared_probe(d.path());
    assert_eq!(check_kept(d.path(), &store).len(), 1);
    write(d.path(), "mock/crates/alpha/src/probe.rs", "fn t() { let s = include_str!(\"data.json\"); }\n");
    assert!(check_kept(d.path(), &store).is_empty());
    write(d.path(), "mock/crates/alpha/src/probe.rs", "fn t() { let s = include_bytes!(\"lib.rs\"); }\n");
    assert_eq!(shown(&check_kept(d.path(), &store)), shown(&check(d.path())));
    assert_eq!(check_kept(d.path(), &store).len(), 1);
}

#[test]
fn a_renamed_or_deleted_file_is_seen_by_a_saved_run() {
    let d = tempfile::tempdir().unwrap();
    let store = Kept::default();
    declared_probe(d.path());
    assert_eq!(check_kept(d.path(), &store).len(), 1);
    fs::rename(
        d.path().join("mock/crates/alpha/src/probe.rs"),
        d.path().join("mock/crates/alpha/src/renamed.rs"),
    )
    .unwrap();
    let renamed = check_kept(d.path(), &store);
    assert!(renamed.is_empty(), "lib.rs declares probe, which is gone: {renamed:?}");
    assert_eq!(shown(&renamed), shown(&check(d.path())));
    write(d.path(), "mock/crates/alpha/src/lib.rs", "#[cfg(test)]\nmod renamed;\n");
    let found = check_kept(d.path(), &store);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(paths(&found), ["mock/crates/alpha/src/renamed.rs"]);
    fs::remove_file(d.path().join("mock/crates/alpha/src/renamed.rs")).unwrap();
    assert!(check_kept(d.path(), &store).is_empty());
}

#[test]
fn a_changed_allow_list_is_not_answered_from_a_verdict_saved_under_another() {
    let d = tempfile::tempdir().unwrap();
    let store = Kept::default();
    declared_probe(d.path());
    let none = NoSourceIncludeInTest::default();
    let lets = allowing("data.json");
    let anywhere = allowing("**/lib.rs");
    assert_eq!(check_kept_with(&none, d.path(), &store).len(), 1);
    assert_eq!(check_kept_with(&lets, d.path(), &store).len(), 1, "a list naming something else lets nothing through");
    store.clear_counts();
    assert!(check_kept_with(&anywhere, d.path(), &store).is_empty(), "the control: this list lets it through");
    assert_eq!(store.hits(NAME), 0, "a verdict saved under another list was not replayed");
    store.clear_counts();
    assert_eq!(check_kept_with(&none, d.path(), &store).len(), 1, "the first list again");
    assert_eq!(store.hits(NAME), 1, "is replayed, since its verdict is still saved");
}

#[test]
fn a_changed_crate_set_is_seen_by_a_saved_run() {
    let d = tempfile::tempdir().unwrap();
    let store = Kept::default();
    declared_probe(d.path());
    let lint = NoSourceIncludeInTest::default();
    let with = check_kept_in(&lint, d.path(), &store, &["alpha"]);
    assert_eq!(with.len(), 1, "the control: the include is reported");
    assert_eq!(with[0].crate_name, "alpha");
    store.clear_counts();
    let without = check_kept_in(&lint, d.path(), &store, &[]);
    assert_eq!(without[0].crate_name, "workspace", "alpha is no crate, so the finding is the workspace's");
    assert_eq!(store.hits(NAME), 0, "a verdict saved under another crate set was not replayed");
}

#[test]
fn a_run_with_no_store_parses_each_file_once_per_pair_it_is_asked_under() {
    let d = tempfile::tempdir().unwrap();
    declared_probe(d.path());
    PARSED.with(|p| p.set(0));
    let found = check(d.path());
    assert_eq!(found.len(), 1, "the control: the include is reported");
    // `lib.rs`, `probe.rs` and `other.rs` as plain files in the first round, then `probe.rs` again as
    // the test file the first round found. Every later round asks again for the same pairs.
    assert_eq!(PARSED.with(|p| p.get()), 4);
}
