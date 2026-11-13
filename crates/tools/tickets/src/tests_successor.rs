//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! A move made `--for` another ticket: what the registry holds a successor to, and that a move
//! refused for it writes nothing. The markers it hands on are the `ticket-move` tool's.

use crate::testkit::Scratch;
use crate::tests_tags::{added, file, moved, moves};
use crate::{args, load, write};

fn state_of(s: &Scratch, id: &str) -> String {
    load(&s.root)
        .unwrap()
        .state_of(id)
        .map(|st| st.id.clone())
        .unwrap_or_default()
}

fn two(s: &Scratch) -> (String, String) {
    (
        added(s, &["Old work"]).unwrap(),
        added(s, &["New work"]).unwrap(),
    )
}

/// The registry as the file system holds it, so that a refusal can be shown to have written nothing.
fn bytes(s: &Scratch, ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| file(s, id)).collect()
}

#[test]
fn for_is_a_flag_of_a_move_and_names_the_successor() {
    let r = args::move_(&["old", "done", "--for", "new", "--by", "tester"]).unwrap();
    assert_eq!(r.successor.as_deref(), Some("new"));
    let r = args::move_(&["old", "done", "--by", "tester"]).unwrap();
    assert_eq!(r.successor, None, "the control: none given is none");
}

#[test]
fn a_move_for_an_open_ticket_is_made_and_the_state_is_the_new_one() {
    let s = Scratch::new();
    let (old, new) = two(&s);
    let made = moved(&s, &[&old, "done", "--for", &new]).unwrap();
    assert!(!made.is_empty(), "a state makes a move");
    assert_eq!(state_of(&s, &old), "done");
    assert_eq!(state_of(&s, &new), "backlog", "the successor is not moved");
}

#[test]
fn a_successor_that_is_no_ticket_is_refused_and_nothing_is_written() {
    let s = Scratch::new();
    let (old, new) = two(&s);
    let before = bytes(&s, &[&old, &new]);
    let e = moved(&s, &[&old, "done", "--for", "no_such_ticket"]).unwrap_err();
    assert!(
        e.contains("`no_such_ticket`") && e.contains("no ticket"),
        "{e}"
    );
    assert_eq!(bytes(&s, &[&old, &new]), before);
    assert_eq!(state_of(&s, &old), "backlog");
}

#[test]
fn a_ticket_cannot_be_its_own_successor() {
    let s = Scratch::new();
    let (old, new) = two(&s);
    let before = bytes(&s, &[&old, &new]);
    let e = moved(&s, &[&old, "done", "--for", &old]).unwrap_err();
    assert!(e.contains("itself"), "{e}");
    assert_eq!(bytes(&s, &[&old, &new]), before);
    assert_eq!(moves(&s, &old), 1);
}

#[test]
fn a_successor_that_is_closed_is_refused_since_a_marker_citing_it_would_be_refused_too() {
    let s = Scratch::new();
    let (old, new) = two(&s);
    moved(&s, &[&new, "done"]).unwrap();
    let before = bytes(&s, &[&old, &new]);
    let e = moved(&s, &[&old, "done", "--for", &new]).unwrap_err();
    assert!(e.contains("closed"), "{e}");
    assert_eq!(bytes(&s, &[&old, &new]), before);
    assert_eq!(state_of(&s, &old), "backlog");
}

#[test]
fn a_successor_without_a_state_is_refused_since_it_belongs_to_a_move() {
    let s = Scratch::new();
    let (old, new) = two(&s);
    let before = bytes(&s, &[&old, &new]);
    let e = moved(&s, &[&old, "--tags", "x", "--for", &new]).unwrap_err();
    assert!(e.contains("--for") && e.contains("belong"), "{e}");
    assert_eq!(
        bytes(&s, &[&old, &new]),
        before,
        "the tag is not written either"
    );
}

#[test]
fn an_empty_successor_is_refused_and_not_read_as_none() {
    let s = Scratch::new();
    let (old, _) = two(&s);
    let before = bytes(&s, &[&old]);
    let e = moved(&s, &[&old, "done", "--for", ""]).unwrap_err();
    assert!(
        e.contains("--for") && e.contains("given none"),
        "the empty name is told apart from an unknown one: {e}"
    );
    assert!(!e.contains("no ticket"), "{e}");
    assert_eq!(bytes(&s, &[&old]), before);
}

#[test]
fn a_move_refused_for_its_own_reason_is_refused_with_a_good_successor_too() {
    // the control: the successor is fine and the state is not one the kind declares
    let s = Scratch::new();
    let (old, new) = two(&s);
    let before = bytes(&s, &[&old, &new]);
    assert!(
        write::move_ticket(
            &s.root,
            &args::move_(&[
                &old,
                "limbo",
                "--for",
                &new,
                "--by",
                "tester",
                "--at",
                "2026-10-23T11:00:00Z"
            ])
            .unwrap()
        )
        .is_err()
    );
    assert_eq!(bytes(&s, &[&old, &new]), before);
}
