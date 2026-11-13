//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `superseded_by`: a ticket naming the tickets that retire, port or replace its work. While one of
//! them is open it is not taken next; when they close it is as it was.

use crate::testkit::Scratch;
use crate::{args, load, view};

const LATER: &str = "2026-10-23T13:00:00Z";

fn next_ids(s: &Scratch) -> Vec<String> {
    let store = load(&s.root).unwrap();
    view::next(
        &store,
        &planning::Plan::default(),
        &view::Filter::default(),
        &crate::testkit::Branches::default(),
    )
    .into_iter()
    .map(|r| r.id)
    .collect()
}

#[test]
fn a_ticket_an_open_ticket_supersedes_is_not_taken_next_until_that_one_closes() {
    let s = Scratch::new();
    let port = s
        .add_as("The engine port", "task", Some("doing"), "alpha", &[])
        .unwrap();
    let old = s
        .add_as("The glTF scene", "task", Some("ready"), "", &[(
            "superseded_by",
            &[&port],
        )])
        .unwrap();
    // control first: without a superseder `old` is free work, so a missing skip would show here
    let lone = s.add_as("Lone", "task", Some("ready"), "", &[]).unwrap();
    assert_eq!(
        next_ids(&s),
        [lone.clone()],
        "an open superseder keeps `old` out of next"
    );
    let store = load(&s.root).unwrap();
    assert_eq!(view::superseded_by(&store, &old), [port.as_str()]);
    s.mv(&port, "done", None, LATER).unwrap();
    let mut after = next_ids(&s);
    after.sort();
    let mut want = vec![old.clone(), lone];
    want.sort();
    assert_eq!(
        after, want,
        "a closed superseder no longer holds `old` back"
    );
    assert!(view::superseded_by(&load(&s.root).unwrap(), &old).is_empty());
}

#[test]
fn superseded_by_is_written_by_add_and_by_a_move_and_read_from_both_flags() {
    let s = Scratch::new();
    let a = s.add("A", &[]).unwrap();
    let b = s.add("B", &[]).unwrap();
    let c = s.add("C", &[("superseded_by", &[&a])]).unwrap();
    s.links(&c, "", None, &[("superseded_by", &[&a, &b])])
        .unwrap();
    let store = load(&s.root).unwrap();
    let held = store.tickets.get(&c).unwrap().many("superseded_by");
    assert_eq!(
        held,
        [a.clone(), b.clone()],
        "a move records the whole list it is given"
    );

    let r = args::add(&["New", "--superseded-by", "x,y", "--by", "t", "--at", LATER]).unwrap();
    assert_eq!(r.fields.get("superseded_by").unwrap(), &[
        "x".to_string(),
        "y".to_string()
    ]);
    let m = args::move_(&["z", "--superseded-by", "x", "--by", "t", "--at", LATER]).unwrap();
    assert_eq!(m.links.get("superseded_by").unwrap(), &["x".to_string()]);
}
