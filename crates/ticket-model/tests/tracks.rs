//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Tracks: a ticket holding `shows` is a track, the tickets `part_of` it are its steps, in the order of
//! their `needs` and then the board order, and a wait is a `needs` edge leaving the track for a ticket that
//! does not yet meet it.

mod common;

use common::{closed, open, store};
use mockspace_extra_ticket_model::tracks;

fn step_ids(t: &tracks::Track) -> Vec<&str> {
    t.steps.iter().map(|s| s.id.as_str()).collect()
}

#[test]
fn a_ticket_holding_shows_is_a_track_and_one_without_is_not() {
    let s = store(&[open("t", "shows = \"the sea moves under a ship\""), open("plain", "")]);
    let all = tracks::of(&s);
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].id, "t");
    assert_eq!(all[0].shows, "the sea moves under a ship");
}

#[test]
fn the_steps_of_a_track_are_the_tickets_part_of_it() {
    let s = store(&[
        open("t", "shows = \"x\""),
        open("a", "part_of = \"t\""),
        open("b", "part_of = \"t\""),
        open("other", "part_of = \"elsewhere\""),
    ]);
    let t = &tracks::of(&s)[0];
    assert_eq!(step_ids(t), ["a", "b"]);
}

#[test]
fn a_step_comes_after_every_step_it_needs_whatever_the_ids_say() {
    let s = store(&[
        open("t", "shows = \"x\""),
        open("a", "part_of = \"t\"\nneeds = [\"c\"]"),
        open("b", "part_of = \"t\"\nneeds = [\"a\"]"),
        open("c", "part_of = \"t\""),
    ]);
    assert_eq!(step_ids(&tracks::of(&s)[0]), ["c", "a", "b"]);
}

#[test]
fn where_needs_does_not_decide_the_board_order_does() {
    let s = store(&[
        open("t", "shows = \"x\""),
        open("a", "part_of = \"t\"\nrank = 3"),
        open("b", "part_of = \"t\"\nrank = 1"),
        open("c", "part_of = \"t\"\nrank = 2"),
    ]);
    assert_eq!(step_ids(&tracks::of(&s)[0]), ["b", "c", "a"]);
}

#[test]
fn a_need_leaving_the_track_for_an_open_ticket_is_a_wait_naming_where_it_is() {
    let s = store(&[
        open("t", "shows = \"x\""),
        open("u", "shows = \"y\""),
        open("a", "part_of = \"t\"\nneeds = [\"x1\", \"loose\"]"),
        open("x1", "part_of = \"u\""),
        open("loose", ""),
    ]);
    let t = tracks::of(&s).into_iter().find(|t| t.id == "t").unwrap();
    let a = &t.steps[0];
    assert_eq!(a.waits.len(), 2);
    let on_x1 = a.waits.iter().find(|w| w.on == "x1").unwrap();
    assert_eq!(on_x1.track.as_deref(), Some("u"));
    assert_eq!(on_x1.state, "backlog");
    let on_loose = a.waits.iter().find(|w| w.on == "loose").unwrap();
    assert_eq!(on_loose.track, None);
}

#[test]
fn a_need_already_met_or_inside_the_track_is_no_wait() {
    let s = store(&[
        open("t", "shows = \"x\""),
        open("a", "part_of = \"t\""),
        open("b", "part_of = \"t\"\nneeds = [\"a\", \"done_elsewhere\"]"),
        closed("done_elsewhere", ""),
    ]);
    let t = &tracks::of(&s)[0];
    assert!(t.steps.iter().all(|s| s.waits.is_empty()), "{t:?}");
}

#[test]
fn a_closed_step_stays_on_the_track_with_its_state() {
    let s = store(&[
        open("t", "shows = \"x\""),
        closed("a", "part_of = \"t\""),
        open("b", "part_of = \"t\""),
    ]);
    let t = &tracks::of(&s)[0];
    assert_eq!(t.steps.iter().find(|s| s.id == "a").unwrap().state, "done");
}

#[test]
fn the_first_steps_waiting_are_counted_and_a_later_step_is_not() {
    let s = store(&[
        open("t", "shows = \"x\""),
        open("u", "shows = \"y\""),
        open("blocker", "part_of = \"u\""),
        open("first_waiting", "part_of = \"t\"\nneeds = [\"blocker\"]"),
        open("first_free", "part_of = \"t\""),
        open(
            "later",
            "part_of = \"t\"\nneeds = [\"first_free\", \"blocker\"]",
        ),
    ]);
    let t = tracks::of(&s).into_iter().find(|t| t.id == "t").unwrap();
    assert_eq!(t.first_steps_waiting(), 1);
    assert!(!t.steps.iter().find(|s| s.id == "later").unwrap().first);
}

#[test]
fn a_track_with_no_steps_is_listed_with_none() {
    let s = store(&[open("t", "shows = \"x\"")]);
    let t = &tracks::of(&s)[0];
    assert!(t.steps.is_empty());
    assert_eq!(t.first_steps_waiting(), 0);
}
