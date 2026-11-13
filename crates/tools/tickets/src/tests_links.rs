//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Recording `branch`, `pr`, `session` and `needs` on a ticket already added: each path that
//! writes, and each case the rules refuse, with the file shown unchanged after the refusal.

use kaski_tickets::rules;

use crate::args;
use crate::testkit::Scratch;

const T1: &str = "2026-10-23T11:00:00Z";

fn file(s: &Scratch, id: &str) -> String {
    s.read(&format!("ticket/items/{id}.toml"))
}

fn moves(s: &Scratch, id: &str) -> usize {
    crate::load(&s.root).unwrap().moves_of(id).count()
}

fn clean(s: &Scratch) {
    let store = crate::load(&s.root).unwrap();
    assert!(
        rules::check_store(&store).is_empty(),
        "{:?}",
        rules::check_store(&store)
    );
}

/// The refusal says `message`, and the ticket's file is the text it was before.
fn refused(s: &Scratch, id: &str, before: &str, got: Result<String, String>, message: &str) {
    let err = got.unwrap_err();
    assert!(err.contains(message), "{err}");
    assert_eq!(file(s, id), before, "a refusal writes nothing");
}

#[test]
fn links_alone_are_recorded_in_the_row_and_no_move_is_appended() {
    let s = Scratch::new();
    let dep = s.add("Dep", &[]).unwrap();
    let id = s.add("Work", &[("what", &["the aim"])]).unwrap();
    let made = s
        .links(&id, "", None, &[
            ("session", &["session_abc"]),
            ("branch", &["work/work"]),
            ("repos", &["orgrinrt/kaski#7", "orgrinrt/kaski#9"]),
            ("needs", &[&dep]),
        ])
        .unwrap();
    assert_eq!(made, "", "no move, so no move id");
    let store = crate::load(&s.root).unwrap();
    let t = &store.tickets[&id];
    assert_eq!(t.one("session"), "session_abc");
    assert_eq!(t.one("branch"), "work/work");
    assert_eq!(t.many("repos"), ["orgrinrt/kaski#7", "orgrinrt/kaski#9"]);
    assert_eq!(t.many("needs"), [dep.as_str()]);
    assert_eq!(t.one("what"), "the aim", "the rest of the row is as it was");
    assert_eq!(moves(&s, &id), 1, "the first move is the only one");
    assert_eq!(store.now_of(&id).unwrap().to, "backlog");
    clean(&s);
}

#[test]
fn a_job_is_started_with_its_branch_and_session_in_one_call() {
    let s = Scratch::new();
    let id = s
        .add_as("A job", "job", None, "", &[("brief", &["briefs/a.md"])])
        .unwrap();
    // the control: the same move without the branch is refused, and writes nothing
    let before = file(&s, &id);
    refused(
        &s,
        &id,
        &before,
        s.links(&id, "doing", Some("helper"), &[("session", &["session_x"])]),
        "needs its `branch` filled",
    );
    let made = s
        .links(&id, "doing", Some("helper"), &[
            ("branch", &["work/a"]),
            ("session", &["session_x"]),
        ])
        .unwrap();
    assert!(!made.is_empty());
    let store = crate::load(&s.root).unwrap();
    let now = store.now_of(&id).unwrap();
    assert_eq!((now.to.as_str(), now.holder.as_str()), ("doing", "helper"));
    assert_eq!(store.tickets[&id].one("branch"), "work/a");
    assert_eq!(store.tickets[&id].one("session"), "session_x");
    assert_eq!(store.tickets[&id].one("brief"), "briefs/a.md");
    clean(&s);
}

#[test]
fn a_job_without_a_brief_is_started_with_the_brief_in_the_same_move() {
    let s = Scratch::new();
    let id = s.add_as("A job", "job", None, "", &[]).unwrap();
    // the control: without the brief the start is refused
    let before = file(&s, &id);
    refused(
        &s,
        &id,
        &before,
        s.links(&id, "doing", Some("helper"), &[("branch", &["work/a"])]),
        "needs its `brief` filled",
    );
    let req = args::move_(&[
        &id,
        "doing",
        "--by",
        "tester",
        "--holder",
        "helper",
        "--branch",
        "work/a",
        "--brief",
        "briefs/job-a.md",
    ])
    .unwrap();
    assert_eq!(req.links["brief"], ["briefs/job-a.md"]);
    crate::write::move_ticket(&s.root, &req).unwrap();
    let store = crate::load(&s.root).unwrap();
    assert_eq!(store.tickets[&id].one("brief"), "briefs/job-a.md");
    assert_eq!(store.now_of(&id).unwrap().to, "doing");
    clean(&s);
}

#[test]
fn a_value_replaces_and_an_empty_one_clears() {
    let s = Scratch::new();
    let a = s.add("A", &[]).unwrap();
    let b = s.add("B", &[]).unwrap();
    let id = s
        .add("Work", &[
            ("branch", &["work/old"]),
            ("repos", &["o/r#1"]),
            ("needs", &[&a]),
        ])
        .unwrap();
    s.links(&id, "", None, &[
        ("branch", &["work/new"]),
        ("repos", &["o/r#2"]),
        ("needs", &[&b]),
    ])
    .unwrap();
    let store = crate::load(&s.root).unwrap();
    let t = &store.tickets[&id];
    assert_eq!(t.many("branch"), ["work/new"]);
    assert_eq!(t.many("repos"), ["o/r#2"]);
    assert_eq!(t.many("needs"), [b.as_str()]);
    s.links(&id, "", None, &[
        ("branch", &[]),
        ("repos", &[]),
        ("needs", &[]),
    ])
    .unwrap();
    let store = crate::load(&s.root).unwrap();
    let t = &store.tickets[&id];
    assert!(
        t.many("branch").is_empty() && t.many("repos").is_empty() && t.many("needs").is_empty()
    );
    assert_eq!(t.one("title"), "Work");
    assert!(
        !file(&s, &id).contains("work/"),
        "the lines are gone and not blanked"
    );
    clean(&s);
}

#[test]
fn a_hand_written_row_keeps_its_comments_and_its_multiline_arrays() {
    let s = Scratch::new();
    let a = s.add("A", &[]).unwrap();
    let id = s.add("Hand", &[("needs", &[&a])]).unwrap();
    let path = format!("ticket/items/{id}.toml");
    let text = file(&s, &id).replace(
        &format!("needs = [\"{a}\"]\n"),
        &format!("# why it waits\nneeds = [\n  \"{a}\",\n]\n"),
    );
    std::fs::write(s.root.join(&path), text).unwrap();
    s.links(&id, "", None, &[
        ("needs", &[]),
        ("session", &["session_h"]),
    ])
    .unwrap();
    let after = file(&s, &id);
    assert!(after.contains("# why it waits"), "{after}");
    assert!(!after.contains("needs"), "{after}");
    assert!(after.contains("session = \"session_h\""), "{after}");
    assert_eq!(moves(&s, &id), 1);
    clean(&s);
}

#[test]
fn only_the_named_ticket_in_a_shared_file_is_rewritten() {
    let s = Scratch::new();
    let first = s.add("First", &[("branch", &["work/first"])]).unwrap();
    let second = s.add("Second", &[("branch", &["work/second"])]).unwrap();
    let one = file(&s, &first);
    let two = file(&s, &second);
    std::fs::write(
        s.root.join("ticket/items/both.toml"),
        format!("{one}\n{two}"),
    )
    .unwrap();
    std::fs::remove_file(s.root.join(format!("ticket/items/{first}.toml"))).unwrap();
    std::fs::remove_file(s.root.join(format!("ticket/items/{second}.toml"))).unwrap();
    s.links(&second, "", None, &[("branch", &["work/changed"])])
        .unwrap();
    let store = crate::load(&s.root).unwrap();
    assert_eq!(store.tickets[&first].one("branch"), "work/first");
    assert_eq!(store.tickets[&second].one("branch"), "work/changed");
}

#[test]
fn an_unknown_ticket_is_refused() {
    let s = Scratch::new();
    let err = s
        .links("ghost", "", None, &[("session", &["s"])])
        .unwrap_err();
    assert!(err.contains("`ghost` is no ticket"), "{err}");
}

#[test]
fn a_need_that_names_no_ticket_the_ticket_itself_or_a_cycle_is_refused() {
    let s = Scratch::new();
    let a = s.add("A", &[]).unwrap();
    let b = s.add("B", &[("needs", &[&a])]).unwrap();
    let before_a = file(&s, &a);
    let before_b = file(&s, &b);
    refused(
        &s,
        &a,
        &before_a,
        s.links(&a, "", None, &[("needs", &["ghost"])]),
        "`needs` names `ghost`, which is no ticket",
    );
    refused(
        &s,
        &a,
        &before_a,
        s.links(&a, "", None, &[("needs", &[&a])]),
        "names the ticket itself",
    );
    refused(
        &s,
        &a,
        &before_a,
        s.links(&a, "", None, &[("needs", &[&b])]),
        "would go round",
    );
    // the control: the same edit with a need that is fine goes through
    s.links(&a, "", None, &[("needs", &[])]).unwrap();
    assert_eq!(file(&s, &b), before_b);
    let c = s.add("C", &[]).unwrap();
    s.links(&a, "", None, &[("needs", &[&c])]).unwrap();
    clean(&s);
}

#[test]
fn a_need_the_current_state_does_not_meet_is_refused() {
    let s = Scratch::new();
    let dep = s.add("Dep", &[]).unwrap();
    let id = s.add("Held", &[]).unwrap();
    s.mv(&id, "doing", Some("someone"), T1).unwrap();
    let before = file(&s, &id);
    refused(
        &s,
        &id,
        &before,
        s.links(&id, "", None, &[("needs", &[&dep])]),
        "waits on",
    );
    s.mv(&dep, "done", None, T1).unwrap();
    s.links(&id, "", None, &[("needs", &[&dep])]).unwrap();
    clean(&s);
}

#[test]
fn a_required_link_is_not_cleared_from_a_held_job() {
    let s = Scratch::new();
    let id = s
        .add_as("Job", "job", Some("doing"), "helper", &[
            ("brief", &["briefs/j.md"]),
            ("branch", &["work/j"]),
        ])
        .unwrap();
    let before = file(&s, &id);
    refused(
        &s,
        &id,
        &before,
        s.links(&id, "", None, &[("branch", &[])]),
        "needs its `branch` filled",
    );
    s.links(&id, "", None, &[("session", &["session_j"])])
        .unwrap();
    clean(&s);
}

#[test]
fn what_belongs_to_a_move_is_refused_without_a_state_and_so_is_asking_for_nothing() {
    let s = Scratch::new();
    let id = s.add("Plain", &[]).unwrap();
    let before = file(&s, &id);
    refused(
        &s,
        &id,
        &before,
        s.links(&id, "", Some("x"), &[("session", &["s"])]),
        "belong to a move",
    );
    refused(
        &s,
        &id,
        &before,
        s.links(&id, "", None, &[]),
        "name a state to move it to, or a link",
    );
}

#[test]
fn the_flags_are_read_into_links_and_a_state_is_optional() {
    let r =
        args::move_(&["w", "--session", "s1", "--needs", "a, b", "--branch", "work/w"]).unwrap();
    assert_eq!((r.ticket.as_str(), r.to.as_str()), ("w", ""));
    assert_eq!(r.links["session"], ["s1"]);
    assert_eq!(r.links["needs"], ["a", "b"]);
    assert_eq!(r.links["branch"], ["work/w"]);
    let r = args::move_(&["w", "doing", "--holder", "h", "--branch", ""]).unwrap();
    assert_eq!(r.to, "doing");
    assert!(r.links["branch"].is_empty(), "an empty value is a clear");
    let r = args::move_(&["w", "done"]).unwrap();
    assert!(r.links.is_empty());
    assert!(
        args::move_(&["w"])
            .unwrap_err()
            .contains("a state to move to, or a link")
    );
    assert!(args::move_(&[]).is_err());
    assert!(args::move_(&["w", "a", "b"]).is_err());
}

#[test]
fn a_task_is_promoted_to_a_job_by_its_kind_tenets_and_subjects_alone() {
    // A backlog task becomes a job in one call that records its kind and the canon rows it derives from.
    // Mutation: the `kind` link dropped from the link table, so the flag is read and nothing is written.
    let s = Scratch::new();
    let id = s.add("Backlog item", &[("what", &["the aim"])]).unwrap();
    let before = moves(&s, &id);
    let made = s
        .links(&id, "", None, &[
            ("kind", &["job"]),
            ("related", &["tenet::t_a", "tenet::t_b", "subject::engine"]),
        ])
        .unwrap();
    assert_eq!(made, "", "no move is made");
    let store = crate::load(&s.root).unwrap();
    let t = &store.tickets[&id];
    assert_eq!(t.one("kind"), "job");
    assert_eq!(t.many("related"), [
        "tenet::t_a",
        "tenet::t_b",
        "subject::engine"
    ]);
    assert_eq!(t.one("what"), "the aim", "the rest of the row is as it was");
    assert_eq!(moves(&s, &id), before, "promoting appends no move");
    clean(&s);
}

#[test]
fn a_promoted_ticket_is_judged_as_a_job_in_the_state_it_stands_in() {
    // A held ticket becoming a job needs the brief and the branch the kind requires, in the same call.
    // Mutation: the kind link recorded after the rules judged the old kind, so this is accepted unbriefed.
    let s = Scratch::new();
    let id = s
        .add_as("Held", "task", Some("doing"), "helper", &[])
        .unwrap();
    let before = file(&s, &id);
    refused(
        &s,
        &id,
        &before,
        s.links(&id, "", None, &[("kind", &["job"])]),
        "needs its `brief` filled",
    );
    s.links(&id, "", None, &[
        ("kind", &["job"]),
        ("brief", &["briefs/j.md"]),
        ("branch", &["work/j"]),
    ])
    .unwrap();
    assert_eq!(
        crate::load(&s.root).unwrap().tickets[&id].one("kind"),
        "job"
    );
    clean(&s);
}

#[test]
fn a_kind_that_is_no_ticket_kind_is_refused_and_writes_nothing() {
    let s = Scratch::new();
    let id = s.add("Plain", &[]).unwrap();
    let before = file(&s, &id);
    refused(
        &s,
        &id,
        &before,
        s.links(&id, "", None, &[("kind", &["nonsense"])]),
        "which is no ticket kind",
    );
}

#[test]
fn promoting_again_replaces_the_related_rows_and_an_empty_value_clears_them() {
    let s = Scratch::new();
    let id = s.add("Work", &[("related", &["tenet::t_a"])]).unwrap();
    s.links(&id, "", None, &[("related", &["tenet::t_b", "tenet::t_c"])])
        .unwrap();
    assert_eq!(
        crate::load(&s.root).unwrap().tickets[&id].many("related"),
        ["tenet::t_b", "tenet::t_c"]
    );
    s.links(&id, "", None, &[("related", &[])]).unwrap();
    assert!(
        crate::load(&s.root).unwrap().tickets[&id]
            .many("related")
            .is_empty()
    );
    clean(&s);
}

#[test]
fn the_kind_and_related_flags_are_read_into_links() {
    let r = args::move_(&["w", "--kind", "job", "--related", "tenet::a, subject::engine"]).unwrap();
    assert_eq!((r.ticket.as_str(), r.to.as_str()), ("w", ""));
    assert_eq!(r.links["kind"], ["job"]);
    assert_eq!(r.links["related"], ["tenet::a", "subject::engine"]);
}
