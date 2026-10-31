use std::collections::BTreeSet;
use std::path::Path;

use mockspace_lint_rules::verdict::VerdictCache;

use super::tests::{canon, configured, dir, registry, write};
use super::*;
use crate::test_support::Kept;

/// The findings of one run over `root`, flattened to what a reader sees and left in the order the
/// lint returned them, saved through `store` when one is given.
fn run_in_order(reg: &RegistryView, root: &Path, crates: &[&str], store: Option<&Kept>) -> Vec<String> {
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
    configured("superseded_by")
        .check_repo(&ctx)
        .into_iter()
        .map(|e| format!("{}:{}:{:?}:{}", e.crate_name, e.line, e.path, e.message))
        .collect()
}

/// `run_in_order` sorted, for the tests that compare runs and not the order of what they find.
fn stored_run(reg: &RegistryView, root: &Path, crates: &[&str], store: Option<&Kept>) -> Vec<String> {
    let mut shown = run_in_order(reg, root, crates, store);
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

/// What asking per file saved: each of `rels` in `alpha`, judged and kept under the key the lint
/// has always used, with no batch involved.
fn saved_one_by_one(reg: &RegistryView, root: &Path, rels: &[&str], store: &Kept) {
    let crates: BTreeSet<String> = BTreeSet::from(["alpha".to_string()]);
    let ctx = RepoContext {
        mock_dir: &root.join("mock"),
        repo_root: root,
        all_crates: &crates,
        src_dirs: &[],
        invocation: None,
        canon_paths: &[],
        open_panels: &[],
        registry: reg,
        verdicts: Some(store as &dyn VerdictCache),
    };
    let table = format!("{:?}", replaced(reg, "superseded_by", ""));
    let table_of = replaced(reg, "superseded_by", "");
    for rel in rels {
        let text = fs::read_to_string(root.join(rel)).unwrap();
        let parts: [&[u8]; 4] = [rel.as_bytes(), b"alpha", table.as_bytes(), text.as_bytes()];
        ctx.remembered(NAME, &parts, || file_findings(rel, "alpha", &text, &table_of));
    }
}

#[test]
fn a_verdict_saved_one_file_at_a_time_replays_in_the_batch() {
    let d = dir("sequential");
    let store = Kept::default();
    cited(d.path());
    let reg = canon();
    saved_one_by_one(&reg, d.path(), &["mock/crates/alpha/a.md", "mock/crates/alpha/b.md"], &store);
    assert_eq!(store.misses(NAME), 2, "the control: saving one at a time judged both");
    store.clear_counts();
    let full = stored_run(&reg, d.path(), &["alpha"], None);
    assert_eq!(stored_run(&reg, d.path(), &["alpha"], Some(&store)), full);
    assert_eq!(store.hits(NAME), 2, "both verdicts saved one at a time are replayed");
    assert_eq!(store.misses(NAME), 0, "and none is judged again");
}

#[test]
fn only_the_file_that_changed_is_judged_again() {
    let d = dir("one-changed");
    let store = Kept::default();
    cited(d.path());
    let reg = canon();
    agree(&reg, d.path(), &["alpha"], &store, "cold");
    write(d.path(), "mock/crates/alpha/a.md", "It cites nothing now.\n");
    store.clear_counts();
    let after = agree(&reg, d.path(), &["alpha"], &store, "after the edit");
    assert!(after.is_empty(), "{after:?}");
    assert_eq!(store.misses(NAME), 1, "the edited file is judged again");
    assert_eq!(store.hits(NAME), 1, "and the other is replayed");
}

#[test]
fn forty_files_find_fourteen_findings_and_miss_forty_times_cold() {
    let d = dir("many");
    let store = Kept::default();
    for n in 0 .. 40 {
        let text = if n % 3 == 0 { "It cites `old` here.\n" } else { "It cites `new` here.\n" };
        write(d.path(), &format!("mock/crates/alpha/f{n:02}.md"), text);
    }
    let reg = canon();
    let found = agree(&reg, d.path(), &["alpha"], &store, "cold");
    assert_eq!(found.len(), 14, "{found:?}");
    assert_eq!(store.misses(NAME), 40, "every file is judged once on a cold run");
}

/// Two files whose own order is the reverse of their paths' and of their slugs', so none of those
/// gives `ordered_findings`.
fn ordered(root: &Path) {
    write(root, "mock/crates/alpha/b.md", "It cites `split` here.\n\nIt cites `old` here.\n");
    write(root, "mock/crates/alpha/a.md", "It cites `old` here.\n\nIt cites `split` here.\n");
}

/// What `ordered` yields: path order across files, line order within one. Unsorted on purpose, as
/// the order is what is pinned.
fn ordered_findings() -> [String; 4] {
    let old = "which `new` supersedes";
    let split = "which `left`, `right` supersede";
    let tail = "write to the successor, or say in the sentence that the row is superseded";
    let said = |line: usize, rel: &str, cited: &str, which: &str| {
        format!("alpha:{line}:Some(\"mock/crates/alpha/{rel}\"):`mock/crates/alpha/{rel}` cites `{cited}`, {which}; {tail}")
    };
    [
        said(1, "a.md", "old", old),
        said(3, "a.md", "split", split),
        said(1, "b.md", "split", split),
        said(3, "b.md", "old", old),
    ]
}

#[test]
fn findings_come_in_path_order_across_files_and_line_order_within_one() {
    let d = dir("order");
    ordered(d.path());
    assert_eq!(run_in_order(&canon(), d.path(), &["alpha"], None), ordered_findings());
}

#[test]
fn a_saved_run_keeps_that_order_when_the_first_file_is_judged_again_and_the_second_replayed() {
    let d = dir("order-saved");
    let store = Kept::default();
    ordered(d.path());
    let reg = canon();
    assert_eq!(run_in_order(&reg, d.path(), &["alpha"], Some(&store)), ordered_findings(), "cold");
    // Edited so its citations keep their lines: the first file in path order is a miss and the
    // second a hit, the mix in which replayed verdicts could be put ahead of judged ones.
    write(d.path(), "mock/crates/alpha/a.md", "It cites `old` here.\n\nIt cites `split` here.\n\nA closing line.\n");
    store.clear_counts();
    assert_eq!(run_in_order(&reg, d.path(), &["alpha"], Some(&store)), ordered_findings(), "after the edit");
    assert_eq!(store.misses(NAME), 1, "the control: the edited file was judged again");
    assert_eq!(store.hits(NAME), 1, "and the other replayed");
}
