//! The default mode and the narrowed one, which read git: a branch cut from a trunk, what it
//! committed, what is changed in the tree and what is written and not yet added.

use std::collections::BTreeSet;

use mockspace_lint_rules::tool::{Tool, ToolContext};
use mockspace_lint_rules::{Outcome, RegistryView};

use super::tests::{both, found, write};
use super::{NearCopies, render};

/// A repository of two crates on a trunk branch `base`, and a work branch cut from it.
struct Repo {
    dir:    tempfile::TempDir,
    crates: BTreeSet<String>,
}

impl Repo {
    fn git(&self, args: &[&str]) {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(self.dir.path())
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@example.com",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?}");
    }

    fn new() -> Self {
        let repo = Self {
            dir:    tempfile::tempdir().unwrap(),
            crates: ["alpha", "beta", "gamma"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        };
        write(
            repo.dir.path(),
            "mock/crates/alpha/src/lib.rs",
            "pub struct Frame { pub index: u64, pub at: u32 }",
        );
        write(
            repo.dir.path(),
            "mock/crates/beta/src/lib.rs",
            "pub struct Unrelated { pub index: u64, pub at: u32 }",
        );
        repo.git(&["init", "-q", "-b", "base"]);
        repo.git(&["add", "."]);
        repo.git(&["commit", "-q", "-m", "base"]);
        repo.git(&["switch", "-q", "-c", "work"]);
        repo
    }

    fn run(&self, args: &[&str]) -> mockspace_lint_rules::tool::ToolReport {
        let registry = RegistryView::default();
        let mock = self.dir.path().join("mock");
        let ctx = ToolContext {
            mock_dir: &mock,
            repo_root: self.dir.path(),
            all_crates: &self.crates,
            src_dirs: &[],
            args,
            stdin: None,
            registry: &registry,
        };
        NearCopies.run(&ctx)
    }
}

#[test]
fn a_branch_that_touched_nothing_examines_nothing_and_says_so() {
    let repo = Repo::new();
    let report = repo.run(&["--against", "base"]);
    assert!(
        matches!(report.outcome, Outcome::Clean {
            examined: 0,
        }),
        "{:?}",
        report.outcome
    );
    assert_eq!(report.output, "");
}

#[test]
fn a_crate_the_branch_committed_is_compared_with_every_other_crate() {
    let repo = Repo::new();
    write(
        repo.dir.path(),
        "mock/crates/beta/src/lib.rs",
        "pub struct Tick { pub index: u64, pub at: u32 }",
    );
    repo.git(&["add", "."]);
    repo.git(&["commit", "-q", "-m", "work"]);
    let report = repo.run(&["--against", "base"]);
    assert!(
        report.output.contains("struct Tick") && report.output.contains("struct Frame"),
        "{}",
        report.output
    );
    assert!(
        matches!(report.outcome, Outcome::Clean {
            examined: 1,
        }),
        "{:?}",
        report.outcome
    );
}

#[test]
fn a_crate_written_and_not_yet_added_is_touched() {
    let repo = Repo::new();
    write(
        repo.dir.path(),
        "mock/crates/gamma/src/lib.rs",
        "pub struct Tick { pub index: u64, pub at: u32 }",
    );
    let report = repo.run(&["--against", "base"]);
    assert!(
        report.output.contains("struct Tick (gamma"),
        "an untracked crate is the new one: {}",
        report.output
    );
}

#[test]
fn a_change_in_the_tree_not_yet_committed_is_touched() {
    let repo = Repo::new();
    write(
        repo.dir.path(),
        "mock/crates/beta/src/lib.rs",
        "pub struct Tick { pub index: u64, pub at: u32 }",
    );
    let report = repo.run(&["--against", "base"]);
    assert!(report.output.contains("struct Tick"), "{}", report.output);
}

#[test]
fn named_narrows_the_comparison_to_the_crates_a_touched_crate_names() {
    let repo = Repo::new();
    write(
        repo.dir.path(),
        "mock/crates/gamma/src/lib.rs",
        "pub struct Tick { pub index: u64, pub at: u32 }\nfn uses(_: Unrelated) {}",
    );
    let wide = repo.run(&["--against", "base"]);
    assert!(
        wide.output.contains("struct Frame"),
        "every other crate is read: {}",
        wide.output
    );
    let narrow = repo.run(&["--against", "base", "--named"]);
    assert!(
        !narrow.output.contains("struct Frame"),
        "alpha is not named by gamma: {}",
        narrow.output
    );
    assert!(
        narrow.output.contains("struct Unrelated"),
        "beta is named by gamma and is a near-copy of its Tick, so the narrow run reads it: {}",
        narrow.output
    );
}

#[test]
fn an_unknown_trunk_is_inconclusive_and_not_clean() {
    let repo = Repo::new();
    assert!(matches!(
        repo.run(&["--against", "no-such-ref"]).outcome,
        Outcome::Inconclusive { .. }
    ));
}

#[test]
fn every_same_named_pair_prints_at_least_one_difference() {
    const SHAPES: &[&str] = &[
        "pub struct X { pub a: u8 }",
        "pub struct X;",
        "pub struct X {}",
        "pub struct X(pub u8);",
        "pub enum X { A, B }",
        "#[repr(u8)] pub enum X { A, B }",
        "#[repr(u8)] pub enum X { B, A }",
        "pub trait X { fn put(&self); }",
        "pub trait X: Copy { fn put(&self); }",
        "pub struct X<T> { pub a: T }",
    ];
    for (i, a) in SHAPES.iter().enumerate() {
        for b in &SHAPES[i + 1 ..] {
            let types = both(a, b);
            let found = found(&types);
            if types[0].shape == types[1].shape {
                continue;
            }
            assert_eq!(found.same_named.len(), 1, "`{a}` against `{b}`");
            let text = render::report(&found, 30);
            let detail = text
                .lines()
                .skip(1)
                .filter(|l| !l.trim_start().starts_with(char::is_numeric))
                .count();
            assert!(
                text.contains("only in")
                    || text.contains("renamed")
                    || text.contains("keywords differ")
                    || text.contains("differs beside")
                    || text.contains("generics differ")
                    || text.contains("moved"),
                "`{a}` against `{b}` prints no difference ({detail} detail lines):\n{text}"
            );
        }
    }
}

#[test]
fn one_crate_argument_is_compared_with_every_other_crate_and_not_read_alone() {
    let repo = Repo::new();
    let report = repo.run(&["alpha"]);
    assert!(
        report.output.contains("struct Frame") && report.output.contains("struct Unrelated"),
        "{}",
        report.output
    );
}

#[test]
fn examined_counts_only_the_types_pub_only_leaves_in() {
    let repo = Repo::new();
    write(
        repo.dir.path(),
        "mock/crates/gamma/src/lib.rs",
        "struct Hidden { a: u8 }\npub struct Seen { pub a: u8 }",
    );
    let all = repo.run(&["--against", "base"]);
    assert!(
        matches!(all.outcome, Outcome::Clean {
            examined: 2,
        }),
        "{:?}",
        all.outcome
    );
    let only = repo.run(&["--against", "base", "--pub-only"]);
    assert!(
        matches!(only.outcome, Outcome::Clean {
            examined: 1,
        }),
        "{:?}",
        only.outcome
    );
}
