//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! A job's branch names one job, and the ticket follows git: a second ticket on a branch is refused
//! where it is written, and an open ticket whose branch landed is named by the check and left out of
//! `next`, with the answer to git given by a table here and by a real repository in the last tests.

use std::path::Path;
use std::process::Command;

use kaski_tickets::rules;

use crate::args::{self, Verb};
use crate::landing::{self, Git, Landing, Trunk};
use crate::testkit::{Branches, Scratch};
use crate::view;

fn job(s: &Scratch, title: &str, branch: &str, holder: &str, state: &str) -> String {
    s.add_as(title, "job", Some(state), holder, &[
        ("branch", &[branch]),
        ("brief", &["briefs/job.md"]),
    ])
    .unwrap()
}

#[test]
fn a_second_ticket_on_a_branch_is_refused_when_added_and_writes_no_file() {
    let s = Scratch::new();
    job(&s, "First", "work/shared", "", "backlog");
    let err = s
        .add_as("Second", "job", Some("backlog"), "", &[
            ("branch", &["work/shared"]),
            ("brief", &["briefs/job.md"]),
        ])
        .unwrap_err();
    assert!(
        err.contains("work/shared") && err.contains("holds already"),
        "{err}"
    );
    assert_eq!(crate::load(&s.root).unwrap().tickets.len(), 1);
}

#[test]
fn a_branch_another_ticket_holds_is_refused_when_recorded_and_the_file_is_as_it_was() {
    let s = Scratch::new();
    let a = job(&s, "First", "work/a", "", "backlog");
    let b = job(&s, "Second", "work/b", "", "backlog");
    let file = format!("ticket/items/{b}.toml");
    let before = s.read(&file);
    let err = s
        .links(&b, "", None, &[("branch", &["work/a"])])
        .unwrap_err();
    assert!(err.contains(&format!("`{a}`")), "{err}");
    assert_eq!(s.read(&file), before, "a refusal writes nothing");
    // keeping its own branch is not a second holder
    s.links(&b, "", None, &[("branch", &["work/b"])]).unwrap();
}

#[test]
fn a_start_with_a_branch_another_holds_is_refused_whole() {
    let s = Scratch::new();
    job(&s, "First", "work/a", "", "backlog");
    let b = job(&s, "Second", "work/b", "", "backlog");
    let err = s
        .links(&b, "doing", Some("helper"), &[("branch", &["work/a"])])
        .unwrap_err();
    assert!(err.contains("holds already"), "{err}");
    let store = crate::load(&s.root).unwrap();
    assert_eq!(store.now_of(&b).unwrap().to, "backlog", "it did not start");
}

#[test]
fn an_unheld_ticket_whose_branch_is_on_the_trunk_is_landed_and_named_by_the_check() {
    let s = Scratch::new();
    let id = job(&s, "Merged long ago", "work/old", "", "backlog");
    let store = crate::load(&s.root).unwrap();
    let on = Branches::default().with("work/old", Landing::Reachable("abc1234".into()));
    assert_eq!(
        landing::landed(&store, &id, &on).as_deref(),
        Some("abc1234")
    );
    let found = landing::findings(&store, &on);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].row, format!("ticket::{id}"));
    assert!(
        found[0].message.contains("work/old")
            && found[0].message.contains("abc1234")
            && found[0].message.contains("`backlog`"),
        "{found:?}"
    );
}

#[test]
fn a_held_ticket_is_landed_only_by_a_merge_and_not_by_a_branch_the_trunk_merely_reaches() {
    let s = Scratch::new();
    let id = job(&s, "Just started", "work/new", "helper", "doing");
    let store = crate::load(&s.root).unwrap();
    let cut = Branches::default().with("work/new", Landing::Reachable("t1p".into()));
    assert_eq!(
        landing::landed(&store, &id, &cut),
        None,
        "cut from the trunk, not committed to"
    );
    assert!(landing::findings(&store, &cut).is_empty());
    let merged = Branches::default().with("work/new", Landing::Merged("m3rge".into()));
    assert_eq!(
        landing::landed(&store, &id, &merged).as_deref(),
        Some("m3rge")
    );
    assert_eq!(landing::findings(&store, &merged).len(), 1);
}

/// A move written by hand, as an older tool wrote them, into a state that needs a holder with none named.
fn doing_without_a_holder(s: &Scratch, id: &str) {
    s.append(
        &format!("ticket/items/{id}.toml"),
        &format!(
            "\n[[ticket_move]]\nid = \"m_hand\"\nticket = \"{id}\"\nto = \"doing\"\nby = \"tester\"\nat = \
             \"2026-10-23T15:00:00Z\"\n"
        ),
    );
}

#[test]
fn a_ticket_in_a_state_that_needs_a_holder_is_held_whether_or_not_its_move_names_one() {
    let s = Scratch::new();
    let id = job(&s, "Just cut", "work/cut", "helper", "backlog");
    doing_without_a_holder(&s, &id);
    let store = crate::load(&s.root).unwrap();
    assert_eq!(store.state_of(&id).unwrap().id, "doing");
    assert!(
        store.now_of(&id).unwrap().holder.is_empty(),
        "the fixture names no holder"
    );
    let cut = Branches::default().with("work/cut", Landing::Reachable("t1p".into()));
    assert_eq!(
        landing::landed(&store, &id, &cut),
        None,
        "cut from the trunk and not committed to, in doing"
    );
    assert!(landing::findings(&store, &cut).is_empty());
    let merged = Branches::default().with("work/cut", Landing::Merged("m3rge".into()));
    assert_eq!(
        landing::landed(&store, &id, &merged).as_deref(),
        Some("m3rge")
    );
}

#[test]
fn a_branch_with_work_on_it_and_a_branch_git_does_not_have_are_not_landed() {
    let s = Scratch::new();
    let open = job(&s, "Live", "work/live", "", "backlog");
    let absent = job(&s, "Absent", "work/absent", "", "backlog");
    let store = crate::load(&s.root).unwrap();
    let on = Branches::default().with("work/live", Landing::Open);
    assert_eq!(landing::landed(&store, &open, &on), None);
    assert_eq!(landing::landed(&store, &absent, &on), None);
    assert!(landing::findings(&store, &on).is_empty());
}

#[test]
fn a_closed_ticket_and_a_ticket_with_no_branch_are_never_named() {
    let s = Scratch::new();
    let done = job(&s, "Done", "work/done", "", "backlog");
    s.mv(&done, "done", None, "2026-10-23T11:00:00Z").unwrap();
    s.add("No branch", &[]).unwrap();
    let store = crate::load(&s.root).unwrap();
    let on = Branches::default().with("work/done", Landing::Merged("sha".into()));
    assert!(landing::findings(&store, &on).is_empty());
}

/// A move of `id` to `to` that lets go of the holder, with `links` recorded in the same call.
fn release(s: &Scratch, id: &str, to: &str, links: &[(&str, &[&str])]) -> Result<String, String> {
    crate::write::move_ticket(&s.root, &crate::write::MoveRequest {
        ticket: id.to_string(),
        to: to.to_string(),
        links: links
            .iter()
            .map(|(k, v)| {
                (
                    (*k).to_string(),
                    v.iter().map(|x| (*x).to_string()).collect(),
                )
            })
            .collect(),
        release: true,
        by: "tester".to_string(),
        at: "2026-10-23T14:00:00Z".to_string(),
        ..Default::default()
    })
}

#[test]
fn a_ticket_released_to_the_backlog_no_longer_holds_the_branch_it_was_doing() {
    let s = Scratch::new();
    let id = job(&s, "Started, never built", "work/stale", "helper", "doing");
    release(&s, &id, "backlog", &[]).unwrap();
    let store = crate::load(&s.root).unwrap();
    assert_eq!(
        store.tickets[&id].one("branch"),
        "",
        "the release cleared the branch"
    );
    let on = Branches::default().with("work/stale", Landing::Merged("a3808ff".into()));
    assert_eq!(
        landing::landed(&store, &id, &on),
        None,
        "another job's landing on the old branch is not this ticket's"
    );
    assert!(landing::findings(&store, &on).is_empty());
}

#[test]
fn a_move_that_keeps_the_work_held_or_closes_it_keeps_the_branch() {
    let s = Scratch::new();
    let waits = job(&s, "Waits", "work/waits", "helper", "doing");
    s.mv(&waits, "waiting", None, "2026-10-23T11:00:00Z")
        .unwrap();
    let ends = job(&s, "Ends", "work/ends", "helper", "doing");
    release(&s, &ends, "done", &[]).unwrap();
    let store = crate::load(&s.root).unwrap();
    assert_eq!(store.tickets[&waits].one("branch"), "work/waits");
    assert_eq!(store.tickets[&ends].one("branch"), "work/ends");
}

#[test]
fn a_release_that_names_a_branch_records_that_one() {
    let s = Scratch::new();
    let id = job(&s, "Handed on", "work/first", "helper", "doing");
    release(&s, &id, "ready", &[("branch", &["work/second"])]).unwrap();
    let store = crate::load(&s.root).unwrap();
    assert_eq!(store.tickets[&id].one("branch"), "work/second");
}

#[test]
fn next_leaves_out_a_job_whose_branch_landed_and_keeps_the_rest() {
    let s = Scratch::new();
    let landed = job(&s, "Landed", "work/landed", "", "backlog");
    let fresh = job(&s, "Fresh", "work/fresh", "", "backlog");
    let store = crate::load(&s.root).unwrap();
    let on = Branches::default()
        .with("work/landed", Landing::Reachable("abc1234".into()))
        .with("work/fresh", Landing::Open);
    let ids: Vec<String> = view::next(
        &store,
        &kaski_planning::Plan::default(),
        &view::Filter::default(),
        &on,
    )
    .into_iter()
    .map(|r| r.id)
    .collect();
    assert_eq!(ids, [fresh.clone()]);
    let all: Vec<String> = view::next(
        &store,
        &kaski_planning::Plan::default(),
        &view::Filter::default(),
        &Branches::default(),
    )
    .into_iter()
    .map(|r| r.id)
    .collect();
    assert!(
        all.contains(&landed) && all.contains(&fresh),
        "git unknown: {all:?}"
    );
}

#[test]
fn the_branch_and_landed_verbs_take_one_word_and_the_rest_take_none() {
    assert_eq!(
        args::tickets(&["branch", "work/x"]).unwrap().verb,
        Verb::Branch("work/x".into())
    );
    assert_eq!(
        args::tickets(&["landed", "job_x"]).unwrap().verb,
        Verb::Landed("job_x".into())
    );
    for bad in [&["branch"][..], &["landed", "a", "b"], &["next", "work/x"], &["check", "x"]] {
        assert!(args::tickets(bad).is_err(), "{bad:?}");
    }
}

pub(crate) fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t", "-c", "commit.gpgsign=false"])
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

pub(crate) fn commit(dir: &Path, name: &str) {
    std::fs::write(dir.join(name), name).unwrap();
    git(dir, &["add", name]);
    git(dir, &["commit", "-q", "-m", name]);
}

/// A repository whose `dev` has a landed branch (merged with a merge commit), a branch the trunk
/// reaches without a merge, and a branch with work the trunk lacks.
fn repository(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("tickets-git-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "dev"]);
    commit(&dir, "base");
    git(&dir, &["branch", "work/cut"]);
    git(&dir, &["switch", "-q", "-c", "work/landed"]);
    commit(&dir, "landed-work");
    git(&dir, &["switch", "-q", "-c", "work/open", "dev"]);
    commit(&dir, "open-work");
    git(&dir, &["switch", "-q", "dev"]);
    git(&dir, &[
        "merge",
        "-q",
        "--no-ff",
        "-m",
        "land",
        "work/landed",
    ]);
    dir
}

#[test]
fn git_tells_a_merged_branch_a_reached_one_an_open_one_and_one_it_lacks() {
    let dir = repository("tells");
    let repo = Git::at(&dir);
    assert!(
        matches!(repo.landing("work/landed"), Landing::Merged(_)),
        "merged"
    );
    assert!(
        matches!(repo.landing("work/cut"), Landing::Reachable(_)),
        "reached"
    );
    assert_eq!(repo.landing("work/open"), Landing::Open);
    assert_eq!(repo.landing("work/never"), Landing::Unknown);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_repository_with_no_dev_has_no_trunk_to_ask() {
    let dir = std::env::temp_dir().join(format!("tickets-git-other-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-q", "-b", "other"]);
    commit(&dir, "base");
    git(&dir, &["branch", "work/x"]);
    assert_eq!(Git::at(&dir).landing("work/x"), Landing::Unknown);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_merge_sha_is_the_merge_commit_and_not_the_branch_tip() {
    let dir = repository("sha");
    let Landing::Merged(sha) = Git::at(&dir).landing("work/landed") else {
        panic!("not merged");
    };
    let out = Command::new("git")
        .arg("-C")
        .arg(&dir)
        .args(["rev-parse", "dev"])
        .output()
        .unwrap();
    assert_eq!(
        sha,
        String::from_utf8_lossy(&out.stdout).trim(),
        "dev is the merge"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_hand_edit_that_puts_two_tickets_on_one_branch_is_found_by_the_rules_the_makers_use() {
    let s = Scratch::new();
    job(&s, "First", "work/x", "", "backlog");
    s.append(
        "ticket/items/second.toml",
        "[[ticket]]\nid = \"second\"\ntitle = \"Second\"\nkind = \"job\"\nbranch = \"work/x\"\nbrief = \"b.md\"\n\n[[ticket_move]]\nid = \"m_second\"\nticket = \"second\"\nto = \"backlog\"\nby = \"tester\"\nat = \"2026-10-23T10:00:00Z\"\n",
    );
    let store = crate::load(&s.root).unwrap();
    let found = rules::check_store(&store);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.contains("holds already"));
}

#[test]
fn a_landed_branch_a_clone_never_fetched_is_read_from_the_remote_heads() {
    // 7 November: job_72_sky and job_75 had landed on 30 October and `tickets check` never named them, since a
    // clone fetches `dev` alone and their branches read as unknown.
    // Mutation: the remote heads not asked, so `work/landed` reads `Unknown`.
    let remote = repository("remote-heads");
    let clone = std::env::temp_dir().join(format!("tickets-git-clone-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&clone);
    let out = Command::new("git")
        .args(["clone", "-q", "--single-branch", "--branch", "dev"])
        .arg(&remote)
        .arg(&clone)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let git = Git::at(&clone);
    assert!(
        matches!(git.landing("work/landed"), Landing::Merged(_)),
        "{:?}",
        git.landing("work/landed")
    );
    // control: work the trunk lacks is never read as landed, whether its objects came with the clone or not
    assert!(matches!(
        git.landing("work/open"),
        Landing::Open | Landing::Unknown
    ));
    let _ = std::fs::remove_dir_all(&clone);
    let _ = std::fs::remove_dir_all(&remote);
}
