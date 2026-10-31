use std::collections::BTreeSet;
use std::path::Path;

use mockspace_lint_rules::verdict::VerdictCache;

use super::tests::{canon, configured, dir, registry, write};
use super::*;
use crate::test_support::Kept;

/// The findings of one run over `root`, flattened to what a reader sees, saved through `store` when
/// one is given.
fn stored_run(reg: &RegistryView, root: &Path, crates: &[&str], store: Option<&Kept>) -> Vec<String> {
    let crates: BTreeSet<String> = crates.iter().map(|c| (*c).to_string()).collect();
    let ctx = RepoContext {
        mock_dir: &root.join("mock"),
        repo_root: root,
        all_crates: &crates,
        src_dirs: &[],
        invocation: None,
        canon_paths: &[],
        open_panels: &[],
        registry: reg,
        verdicts: store.map(|s| s as &dyn VerdictCache),
    };
    let mut shown: Vec<String> = configured("superseded_by")
        .check_repo(&ctx)
        .into_iter()
        .map(|e| format!("{}:{}:{:?}:{}", e.crate_name, e.line, e.path, e.message))
        .collect();
    shown.sort();
    shown
}

/// Two prose files in one crate: `a.md` cites `old`, which `canon()` supersedes, and `b.md` cites
/// `left`, which it does not.
fn cited(root: &Path) {
    write(root, "mock/crates/alpha/a.md", "It cites `old` here.\n");
    write(root, "mock/crates/alpha/b.md", "It cites `left` here.\n");
}

/// `canon()` with `left` superseded by `right` too.
fn canon_with_left_replaced() -> RegistryView {
    registry(&[
        ("tenet::old", &[("superseded_by", "mid")]),
        ("tenet::mid", &[("superseded_by", "new")]),
        ("tenet::new", &[]),
        ("tenet::left", &[("superseded_by", "right")]),
        ("tenet::right", &[]),
    ])
}

/// What an unsaved run finds, and with it the check that a saved run finds the same.
fn agree(reg: &RegistryView, root: &Path, crates: &[&str], store: &Kept, when: &str) -> Vec<String> {
    let full = stored_run(reg, root, crates, None);
    assert_eq!(stored_run(reg, root, crates, Some(store)), full, "{when}");
    full
}

#[test]
fn a_warm_run_replays_every_file_and_judges_none() {
    let d = dir("warm");
    let store = Kept::default();
    cited(d.path());
    let reg = canon();
    assert_eq!(agree(&reg, d.path(), &["alpha"], &store, "cold").len(), 1);
    assert_eq!(store.hits(NAME), 0, "the control: a cold run replays nothing");
    assert_eq!(store.misses(NAME), 2, "and judges each of the two files");
    store.clear_counts();
    agree(&reg, d.path(), &["alpha"], &store, "warm");
    assert_eq!(store.hits(NAME), 2, "each of the two files is replayed");
    assert_eq!(store.misses(NAME), 0, "and none is judged again");
}

#[test]
fn a_row_superseded_in_the_registry_is_seen_in_a_file_that_did_not_move() {
    let d = dir("registry");
    let store = Kept::default();
    cited(d.path());
    assert_eq!(agree(&canon(), d.path(), &["alpha"], &store, "before").len(), 1);
    let after = agree(&canon_with_left_replaced(), d.path(), &["alpha"], &store, "after");
    assert_eq!(after.len(), 2, "b.md now cites a superseded row: {after:?}");
}

#[test]
fn a_successor_that_moved_is_seen_in_a_file_that_did_not_move() {
    let d = dir("successor");
    let store = Kept::default();
    cited(d.path());
    agree(&canon(), d.path(), &["alpha"], &store, "before");
    let moved = registry(&[("tenet::old", &[("superseded_by", "other")]), ("tenet::other", &[])]);
    let after = agree(&moved, d.path(), &["alpha"], &store, "after");
    assert!(after.iter().any(|m| m.contains("`other`")), "{after:?}");
}

#[test]
fn an_edited_renamed_or_deleted_file_is_seen_by_a_saved_run() {
    let d = dir("files");
    let store = Kept::default();
    cited(d.path());
    let reg = canon_with_left_replaced();
    assert_eq!(agree(&reg, d.path(), &["alpha"], &store, "cold").len(), 2);
    write(d.path(), "mock/crates/alpha/a.md", "Nothing here.\n");
    assert_eq!(agree(&reg, d.path(), &["alpha"], &store, "edited").len(), 1);
    fs::rename(d.path().join("mock/crates/alpha/b.md"), d.path().join("mock/crates/alpha/c.md")).unwrap();
    let renamed = agree(&reg, d.path(), &["alpha"], &store, "renamed");
    assert!(renamed[0].contains("c.md") && !renamed[0].contains("b.md"), "{renamed:?}");
    fs::remove_file(d.path().join("mock/crates/alpha/c.md")).unwrap();
    assert!(agree(&reg, d.path(), &["alpha"], &store, "deleted").is_empty());
}

#[test]
fn a_changed_crate_set_is_seen_by_a_saved_run() {
    let d = dir("crates");
    let store = Kept::default();
    cited(d.path());
    let reg = canon();
    let with = agree(&reg, d.path(), &["alpha"], &store, "alpha is a crate");
    assert!(with[0].starts_with("alpha:"), "{with:?}");
    let without = agree(&reg, d.path(), &[], &store, "alpha is no crate");
    assert!(without[0].starts_with("workspace:"), "{without:?}");
}
