//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The order of a ticket's moves and the rank a ticket is added with: a move behind the newest is
//! refused, moves of one second stay as written, and a rank is a whole number or no rank.

use crate::testkit::Scratch;
use crate::view;

const T1: &str = "2026-10-23T11:00:00Z";
const T2: &str = "2026-10-23T12:00:00Z";

#[test]
fn a_move_earlier_than_the_newest_is_refused_and_writes_nothing() {
    let s = Scratch::new();
    let id = s.add("Raced", &[]).unwrap();
    s.mv(&id, "done", None, T2).unwrap();
    let file = format!("ticket/items/{id}.toml");
    let before = s.read(&file);
    // the later time was written first, so this one is behind the ticket's state
    let err = s.mv(&id, "ready", None, T1).unwrap_err();
    assert!(err.contains(T1) && err.contains(T2), "{err}");
    assert_eq!(
        s.read(&file),
        before,
        "a refused move leaves the file as it was"
    );
    let store = crate::load(&s.root).unwrap();
    assert_eq!(store.now_of(&id).unwrap().to, "done");
    assert_eq!(store.moves_of(&id).count(), 2);
}

#[test]
fn moves_of_one_second_are_made_and_the_last_written_is_the_state() {
    let s = Scratch::new();
    let id = s.add("Tied", &[]).unwrap();
    let mut written = Vec::new();
    // the ids carry a nonce, so across many ties an id would put some last that were written first
    for to in ["ready", "backlog", "ready", "backlog", "ready", "backlog", "ready", "backlog"] {
        written.push(s.mv(&id, to, None, T1).unwrap());
    }
    let store = crate::load(&s.root).unwrap();
    let ids: Vec<&str> = store.moves_of(&id).skip(1).map(|m| m.id.as_str()).collect();
    assert_eq!(ids, written.iter().map(String::as_str).collect::<Vec<_>>());
    assert_eq!(store.now_of(&id).unwrap().to, "backlog");
}

#[test]
fn a_rank_that_is_no_whole_number_is_refused_and_no_ticket_is_made() {
    let s = Scratch::new();
    for rank in ["soon", "1.5", "", "3 4"] {
        let err = s.add("Ranked", &[("rank", &[rank])]).unwrap_err();
        assert!(
            err.contains("rank") && err.contains("whole number"),
            "{rank:?}: {err}"
        );
    }
    assert!(
        !s.root.join("ticket/items").exists()
            || std::fs::read_dir(s.root.join("ticket/items"))
                .unwrap()
                .next()
                .is_none(),
        "a refused add leaves no file"
    );
    assert!(crate::load(&s.root).unwrap().tickets.is_empty());
}

#[test]
fn a_rank_is_written_as_a_number_in_its_plain_form_and_read_back_as_one() {
    let s = Scratch::new();
    let id = s.add("Plain", &[("rank", &["007"])]).unwrap();
    let neg = s.add("Negative", &[("rank", &["-2"])]).unwrap();
    assert!(
        s.read(&format!("ticket/items/{id}.toml"))
            .contains("\nrank = 7\n")
    );
    let store = crate::load(&s.root).unwrap();
    assert_eq!(store.tickets[&id].rank(), Some(7));
    assert_eq!(store.tickets[&neg].rank(), Some(-2));
    let order: Vec<String> = view::list(
        &store,
        &kaski_planning::Plan::default(),
        &view::Filter::default(),
    )
    .into_iter()
    .map(|r| r.id)
    .collect();
    assert_eq!(order, [neg, id], "the board orders by the number");
}
