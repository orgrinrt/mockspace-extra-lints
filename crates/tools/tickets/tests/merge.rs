//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Two branches that each change one ticket merge without either losing its change.
//!
//! Run against real git and the registry merge driver `mock merge-registry`, wired the way kaski's
//! `.gitattributes` and clone configuration wire it: a ticket is one file holding its row and its
//! moves, so two writers of one ticket append to the same end of one file, which git alone cannot
//! merge. The control removes the driver and shows the same two branches conflict, which is what
//! makes a clean merge with it mean something.

use std::path::Path;
use std::process::Command;

use kaski_tickets::rules;
use tickets::load;
use tickets::testkit::Scratch;

const T1: &str = "2026-10-23T11:00:00Z";
const T2: &str = "2026-10-23T12:00:00Z";

fn git(dir: &Path, args: &[&str]) -> (bool, String) {
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
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

fn ok(dir: &Path, args: &[&str]) -> String {
    let (good, text) = git(dir, args);
    assert!(good, "git {args:?} failed: {text}");
    text
}

/// The driver binary: `MOCK_BIN` where set, else `mock` on the path. A missing one is a failure, not a
/// skip, because a skipped merge test is a merge nobody tested.
fn driver() -> String {
    let bin = std::env::var("MOCK_BIN").unwrap_or_else(|_| "mock".to_string());
    let found = Command::new(&bin).arg("help").output();
    assert!(
        found.is_ok(),
        "the registry merge driver `{bin}` is not on the path; set MOCK_BIN"
    );
    bin
}

/// A repository whose root is the scratch registry, holding one ticket in `ready`, on `dev`.
fn repo(with_driver: bool) -> (Scratch, String) {
    let s = Scratch::new();
    let id = s
        .add_as("Shared ticket", "task", Some("ready"), "", &[])
        .unwrap();
    ok(&s.root, &["init", "-q", "-b", "dev"]);
    std::fs::write(
        s.root.join(".gitattributes"),
        "*.toml merge=mockspace-registry\n",
    )
    .unwrap();
    if with_driver {
        // the launcher reads the engine pin from the project it is told of, which a scratch repository is not
        let root = tickets::testkit::repo_root()
            .canonicalize()
            .expect("the repository root");
        let cfg = format!(
            "MOCK_ROOT={} {} merge-registry %O %A %B %P %L",
            root.display(),
            driver()
        );
        ok(&s.root, &[
            "config",
            "merge.mockspace-registry.driver",
            &cfg,
        ]);
    }
    ok(&s.root, &["add", "-A"]);
    ok(&s.root, &["commit", "-q", "-m", "chore: a ticket"]);
    (s, id)
}

fn branch_with(s: &Scratch, name: &str, change: impl FnOnce(&Scratch)) {
    ok(&s.root, &["switch", "-q", "-c", name, "dev"]);
    change(s);
    ok(&s.root, &["add", "-A"]);
    ok(&s.root, &[
        "commit",
        "-q",
        "-m",
        &format!("chore: change on {name}"),
    ]);
    ok(&s.root, &["switch", "-q", "dev"]);
}

#[test]
fn two_moves_of_one_ticket_from_two_branches_both_stay() {
    let (s, id) = repo(true);
    branch_with(&s, "a", |s| {
        s.mv(&id, "waiting", Some("alpha"), T1).unwrap();
    });
    branch_with(&s, "b", |s| {
        s.mv(&id, "doing", Some("beta"), T2).unwrap();
    });
    ok(&s.root, &["merge", "-q", "--no-edit", "a"]);
    ok(&s.root, &["merge", "-q", "--no-edit", "b"]);
    let store = load(&s.root).unwrap();
    assert_eq!(
        store.moves_of(&id).count(),
        3,
        "the first move and both branches' moves"
    );
    assert_eq!(
        store.now_of(&id).unwrap().to,
        "doing",
        "the newer move is the state"
    );
    assert!(rules::check_store(&store).is_empty());
}

#[test]
fn without_the_driver_the_same_two_branches_conflict() {
    let (s, id) = repo(false);
    branch_with(&s, "a", |s| {
        s.mv(&id, "waiting", Some("alpha"), T1).unwrap();
    });
    branch_with(&s, "b", |s| {
        s.mv(&id, "doing", Some("beta"), T2).unwrap();
    });
    ok(&s.root, &["merge", "-q", "--no-edit", "a"]);
    let (merged, text) = git(&s.root, &["merge", "-q", "--no-edit", "b"]);
    assert!(
        !merged,
        "the control must conflict, or the fixture proves nothing: {text}"
    );
}

#[test]
fn a_move_and_an_edit_of_the_title_from_two_branches_both_stay() {
    let (s, id) = repo(true);
    branch_with(&s, "mover", |s| {
        s.mv(&id, "doing", Some("alpha"), T1).unwrap();
    });
    branch_with(&s, "editor", |s| {
        let path = s.root.join(format!("ticket/items/{id}.toml"));
        let text = std::fs::read_to_string(&path)
            .unwrap()
            .replace("Shared ticket", "Shared ticket, renamed");
        std::fs::write(path, text).unwrap();
    });
    ok(&s.root, &["merge", "-q", "--no-edit", "mover"]);
    ok(&s.root, &["merge", "-q", "--no-edit", "editor"]);
    let store = load(&s.root).unwrap();
    assert_eq!(store.tickets[&id].one("title"), "Shared ticket, renamed");
    assert_eq!(store.now_of(&id).unwrap().to, "doing");
}

#[test]
fn two_tickets_added_on_two_branches_both_stay() {
    let (s, _) = repo(true);
    branch_with(&s, "a", |s| {
        s.add("Added on a", &[]).unwrap();
    });
    branch_with(&s, "b", |s| {
        s.add("Added on b", &[]).unwrap();
    });
    ok(&s.root, &["merge", "-q", "--no-edit", "a"]);
    ok(&s.root, &["merge", "-q", "--no-edit", "b"]);
    let store = load(&s.root).unwrap();
    assert!(store.tickets.contains_key("added_on_a") && store.tickets.contains_key("added_on_b"));
}

#[test]
fn one_row_changed_two_ways_is_a_conflict_and_loses_neither_side() {
    let (s, id) = repo(true);
    for (branch, title) in [("a", "Title from a"), ("b", "Title from b")] {
        branch_with(&s, branch, |s| {
            let path = s.root.join(format!("ticket/items/{id}.toml"));
            let text = std::fs::read_to_string(&path)
                .unwrap()
                .replace("Shared ticket", title);
            std::fs::write(path, text).unwrap();
        });
    }
    ok(&s.root, &["merge", "-q", "--no-edit", "a"]);
    let (merged, _) = git(&s.root, &["merge", "-q", "--no-edit", "b"]);
    assert!(
        !merged,
        "the same field changed two ways is not merged silently"
    );
    let text = s.read(&format!("ticket/items/{id}.toml"));
    assert!(
        text.contains("Title from a") && text.contains("Title from b"),
        "both sides are kept in the markers"
    );
}
