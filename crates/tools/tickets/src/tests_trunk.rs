//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The tickets read from the trunk, shown against real repositories: a bare origin, a tree that has
//! fallen behind it, and another clone that answers a question on `dev` while the tree sleeps.
//!
//! The control of every test is the tree read on its own, which holds the old answer, so a trunk read
//! that quietly fell back to the tree would fail them.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::trunk::{self, Opts};
use crate::write::{self, AddRequest, MoveRequest};

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.org",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A bare origin with `dev` holding one waiting question, a clone of it that sleeps (`tree`) and a
/// second clone (`other`) that answers the question and pushes.
struct World {
    base:  PathBuf,
    tree:  PathBuf,
    other: PathBuf,
}

impl World {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let base = std::env::temp_dir().join(format!(
            "trunk-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&base);
        let origin = base.join("origin.git");
        std::fs::create_dir_all(&origin).unwrap();
        git(&origin, &["init", "-q", "--bare", "-b", "dev"]);
        let other = base.join("other");
        git(&base, &["clone", "-q", origin.to_str().unwrap(), "other"]);
        git(&other, &["switch", "-q", "-c", "dev"]);
        let registry = other.join("mock/registry");
        std::fs::create_dir_all(registry.join("ticket")).unwrap();
        for f in ["states.toml", "kinds.toml"] {
            std::fs::copy(
                crate::testkit::committed_registry().join("ticket").join(f),
                registry.join("ticket").join(f),
            )
            .unwrap();
        }
        write::add(&registry, &AddRequest {
            title: "Which way".to_string(),
            kind: "question".to_string(),
            holder: "op".to_string(),
            state: Some("waiting".to_string()),
            by: "tester".to_string(),
            at: "2026-10-23T10:00:00Z".to_string(),
            fields: [
                ("what".to_string(), vec!["left or right".to_string()]),
                ("options".to_string(), vec![
                    "left".to_string(),
                    "right".to_string(),
                ]),
                ("related".to_string(), vec!["subject::engine".to_string()]),
            ]
            .into(),
            ..Default::default()
        })
        .unwrap();
        git(&other, &["add", "-A"]);
        git(&other, &["commit", "-q", "-m", "chore: a question"]);
        git(&other, &["push", "-q", "origin", "dev"]);
        git(&base, &[
            "clone",
            "-q",
            "-b",
            "dev",
            origin.to_str().unwrap(),
            "tree",
        ]);
        World {
            tree: base.join("tree"),
            other,
            base,
        }
    }

    /// Op answers the question, on `dev`, from the second clone.
    fn answer_on_dev(&self) {
        write::move_ticket(&self.other.join("mock/registry"), &MoveRequest {
            ticket: "which_way".to_string(),
            to: "done".to_string(),
            by: "op".to_string(),
            at: "2026-10-27T10:00:00Z".to_string(),
            answer: "left".to_string(),
            picks: vec!["left".to_string()],
            ..Default::default()
        })
        .unwrap();
        git(&self.other, &["add", "-A"]);
        git(&self.other, &["commit", "-q", "-m", "chore: an answer"]);
        git(&self.other, &["push", "-q", "origin", "dev"]);
    }

    fn read(&self, opts: Opts) -> trunk::Reading {
        trunk::read(&self.tree, &self.tree.join("mock"), opts).unwrap()
    }
}

impl Drop for World {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.base);
    }
}

fn closed(r: &trunk::Reading) -> bool {
    r.store.state_of("which_way").unwrap().closed
}

#[test]
fn a_tree_behind_the_trunk_answers_with_the_trunks_answer_and_says_nothing() {
    let w = World::new();
    w.answer_on_dev();
    // the control: the tree on its own still holds the question open
    let tree = w.read(Opts {
        tree:  true,
        fetch: false,
    });
    assert!(!closed(&tree), "the sleeping tree holds the question open");
    let r = w.read(Opts::default());
    assert!(closed(&r), "the trunk's answer was not read");
    assert_eq!(r.notice, "");
    assert!(!r.degraded);
}

#[test]
fn the_tree_asked_for_says_it_is_the_tree_and_how_far_behind_the_trunk_it_is() {
    let w = World::new();
    w.answer_on_dev();
    let r = w.read(Opts {
        tree:  true,
        fetch: true,
    });
    assert!(!closed(&r));
    assert!(r.notice.contains("working tree"), "{}", r.notice);
    assert!(r.notice.contains("--tree"), "{}", r.notice);
    assert!(
        r.notice.contains("1 commit behind origin/dev"),
        "{}",
        r.notice
    );
    assert!(
        !r.degraded,
        "asking for the tree is a choice and not a failure"
    );
}

#[test]
fn a_fetch_that_fails_reads_what_was_fetched_before_and_says_how_old_it_is() {
    let w = World::new();
    w.answer_on_dev();
    // fetched once, so the answer is in the repository's remote branch
    assert!(closed(&w.read(Opts::default())));
    git(&w.tree, &[
        "remote",
        "set-url",
        "origin",
        "/nowhere/at/all.git",
    ]);
    let r = w.read(Opts::default());
    assert!(closed(&r), "what was fetched before is still the trunk");
    assert!(r.degraded);
    assert!(r.notice.contains("was not fetched"), "{}", r.notice);
    assert!(
        r.notice.contains("op may have answered since"),
        "{}",
        r.notice
    );
}

#[test]
fn no_fetch_reads_the_trunk_as_it_was_and_says_so() {
    let w = World::new();
    let r = w.read(Opts {
        tree:  false,
        fetch: false,
    });
    assert!(!closed(&r));
    assert!(r.notice.contains("--no-fetch"), "{}", r.notice);
    assert!(r.degraded);
}

#[test]
fn a_repository_with_no_trunk_is_read_as_the_tree_with_a_notice_that_says_so() {
    let w = World::new();
    git(&w.tree, &["update-ref", "-d", "refs/remotes/origin/dev"]);
    git(&w.tree, &["remote", "remove", "origin"]);
    let r = w.read(Opts::default());
    assert!(!closed(&r));
    assert!(r.degraded);
    assert!(
        r.notice.contains("origin/dev is not in this repository"),
        "{}",
        r.notice
    );
}

#[test]
fn a_tree_current_with_the_trunk_gets_no_notice_from_the_tree_check() {
    let w = World::new();
    assert_eq!(trunk::tree_notice(&w.tree, true), "");
    w.answer_on_dev();
    let n = trunk::tree_notice(&w.tree, true);
    assert!(n.contains("1 commit behind origin/dev"), "{n}");
    assert!(n.contains("git merge origin/dev"), "{n}");
}

#[test]
fn the_scripted_verbs_refuse_rather_than_print_a_notice_among_their_answer() {
    use mockspace::tool::{Outcome, ToolContext};
    let w = World::new();
    git(&w.tree, &["remote", "remove", "origin"]);
    let crates = std::collections::BTreeSet::new();
    let registry = mockspace::RegistryView::default();
    let mock = w.tree.join("mock");
    let ask = |args: &[&str]| {
        crate::Tickets.answer(&ToolContext {
            mock_dir: &mock,
            repo_root: &w.tree,
            all_crates: &crates,
            src_dirs: &[],
            args,
            stdin: None,
            registry: &registry,
        })
    };
    // a refusal for the verb a script reads, naming why
    let (_, branch) = ask(&["branch", "work/x"]);
    match branch.outcome {
        Outcome::Inconclusive {
            reason,
        } => {
            assert!(
                reason.contains("origin/dev is not in this repository"),
                "{reason}"
            )
        },
        other => panic!("expected a refusal, got {other:?}"),
    }
    // the listing a person reads carries the notice and still lists
    let (notice, list) = ask(&["list"]);
    assert!(
        notice.contains("origin/dev is not in this repository"),
        "{notice}"
    );
    assert!(list.output.contains("which_way"), "{}", list.output);
    // and the tree asked for is not refused for the scripted verbs
    let (_, branch) = ask(&["branch", "work/x", "--tree"]);
    assert!(matches!(branch.outcome, Outcome::Clean { .. }));
}
