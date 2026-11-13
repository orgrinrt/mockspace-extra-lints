//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The sweep that closes the tickets whose only subject the tree lost.

use kaski_tickets::Tree;

use crate::close_gone::{Sweep, close_gone};
use crate::testkit::Scratch;

struct Lost;

impl Tree for Lost {
    fn gone(&self, name: &str) -> Option<String> {
        (name == "kaski-render").then(|| "ef19a8e0c".to_string())
    }
}

const AT: &str = "2026-11-08T10:00:00Z";

fn sweep<'a>(successor: Option<&'a str>, cited: &'a dyn Fn(&str) -> bool) -> Sweep<'a> {
    Sweep {
        by: "tester",
        at: AT,
        successor,
        cited,
    }
}

fn uncited(_: &str) -> bool {
    false
}

fn state(s: &Scratch, id: &str) -> String {
    let store = crate::load(&s.root).unwrap();
    store.now_of(id).map(|m| m.to.clone()).unwrap_or_default()
}

#[test]
fn a_ticket_naming_only_a_lost_crate_is_dropped_with_the_why_and_one_naming_a_live_crate_is_not() {
    let s = Scratch::new();
    let gone = s.add("Port kaski-render", &[]).unwrap();
    let kept = s.add("Move kaski-render into kaski-frame", &[]).unwrap();
    let out = close_gone(&s.root, &Lost, &sweep(None, &uncited)).unwrap();
    assert_eq!(out.closed, [gone.clone()]);
    assert_eq!(state(&s, &gone), "dropped");
    assert_eq!(state(&s, &kept), "backlog");
    let store = crate::load(&s.root).unwrap();
    let why = &store.now_of(&gone).unwrap().why;
    assert!(
        why.contains("kaski-render") && why.contains("ef19a8e0c"),
        "{why}"
    );
}

#[test]
fn a_second_sweep_closes_nothing() {
    let s = Scratch::new();
    s.add("Port kaski-render", &[]).unwrap();
    close_gone(&s.root, &Lost, &sweep(None, &uncited)).unwrap();
    let again = close_gone(&s.root, &Lost, &sweep(None, &uncited)).unwrap();
    assert!(
        again.closed.is_empty() && again.left.is_empty(),
        "{again:?}"
    );
}

#[test]
fn a_ticket_another_swept_ticket_needs_is_closed_in_the_same_sweep() {
    let s = Scratch::new();
    let a = s.add("Port kaski-render", &[]).unwrap();
    let b = s
        .add("Then finish kaski-render", &[("needs", &[&a])])
        .unwrap();
    let out = close_gone(&s.root, &Lost, &sweep(None, &uncited)).unwrap();
    assert_eq!(
        out.closed,
        [b.clone(), a.clone()],
        "the dependent closes first"
    );
    assert_eq!(state(&s, &a), "dropped");
    assert_eq!(state(&s, &b), "dropped");
}

#[test]
fn a_ticket_a_live_marker_cites_is_left_open_and_named_unless_a_successor_takes_the_marker() {
    let s = Scratch::new();
    let a = s.add("Port kaski-render", &[]).unwrap();
    let next = s.add("Replacement", &[]).unwrap();
    let cited = |id: &str| id == a;
    let left = close_gone(&s.root, &Lost, &sweep(None, &cited)).unwrap();
    assert_eq!(left.left, [a.clone()]);
    assert!(left.closed.is_empty());
    assert_eq!(state(&s, &a), "backlog");
    let handed = close_gone(&s.root, &Lost, &sweep(Some(&next), &cited)).unwrap();
    assert_eq!(handed.closed, [a.clone()]);
    assert_eq!(state(&s, &a), "dropped");
}

#[test]
fn the_successor_is_never_swept_even_where_it_names_the_lost_crate() {
    let s = Scratch::new();
    let old = s.add("Port kaski-render", &[]).unwrap();
    let next = s.add("Port kaski-render again", &[]).unwrap();
    let out = close_gone(&s.root, &Lost, &sweep(Some(&next), &uncited)).unwrap();
    assert_eq!(out.closed, [old]);
    assert_eq!(state(&s, &next), "backlog");
}

#[test]
fn a_successor_that_is_no_ticket_is_refused_by_name_and_nothing_is_closed() {
    let s = Scratch::new();
    let a = s.add("Port kaski-render", &[]).unwrap();
    let (done, said) = close_gone(&s.root, &Lost, &sweep(Some("nothing"), &uncited)).unwrap_err();
    assert!(done.closed.is_empty(), "{done:?}");
    assert!(said.contains(&a) && said.contains("nothing"), "{said}");
    assert_eq!(state(&s, &a), "backlog");
}

#[test]
fn a_held_ticket_is_left_to_the_list() {
    let s = Scratch::new();
    let held = s.add("Port kaski-render", &[]).unwrap();
    s.mv(&held, "doing", Some("helper"), AT).unwrap();
    let out = close_gone(&s.root, &Lost, &sweep(None, &uncited)).unwrap();
    assert!(out.closed.is_empty(), "{out:?}");
    assert_eq!(state(&s, &held), "doing");
}
