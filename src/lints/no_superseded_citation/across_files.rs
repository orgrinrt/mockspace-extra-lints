use std::collections::BTreeSet;
use std::path::Path;

use super::tests::{canon, configured, dir, write};
use super::*;
use crate::test_support::run_repo_lint_in;

/// The findings of one run over `root`, flattened to what a reader sees and left in the order the
/// lint returned them.
fn run_in_order(reg: &RegistryView, root: &Path, crates: &[&str]) -> Vec<String> {
    let crates: BTreeSet<String> = crates.iter().map(|c| (*c).to_string()).collect();
    run_repo_lint_in(&configured("superseded_by"), root, &crates, reg, &[])
        .into_iter()
        .map(|e| format!("{}:{}:{:?}:{}", e.crate_name, e.line, e.path, e.message))
        .collect()
}

#[test]
fn a_changed_crate_set_moves_the_crate_a_finding_is_filed_under() {
    let d = dir("crates");
    write(d.path(), "mock/crates/alpha/a.md", "It cites `old` here.\n");
    let reg = canon();
    let with = run_in_order(&reg, d.path(), &["alpha"]);
    assert!(with[0].starts_with("alpha:"), "{with:?}");
    let without = run_in_order(&reg, d.path(), &[]);
    assert!(without[0].starts_with("workspace:"), "{without:?}");
}

#[test]
fn forty_files_find_fourteen_findings() {
    let d = dir("many");
    for n in 0 .. 40 {
        let text = if n % 3 == 0 { "It cites `old` here.\n" } else { "It cites `new` here.\n" };
        write(d.path(), &format!("mock/crates/alpha/f{n:02}.md"), text);
    }
    let found = run_in_order(&canon(), d.path(), &["alpha"]);
    assert_eq!(found.len(), 14, "{found:?}");
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
    assert_eq!(run_in_order(&canon(), d.path(), &["alpha"]), ordered_findings());
}
