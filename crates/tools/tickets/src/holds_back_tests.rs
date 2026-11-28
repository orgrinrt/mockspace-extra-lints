//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `holds_back`: a ticket naming the kinds of ticket nothing takes next while it is open. When it
//! closes, the work it held back is free again.

use crate::testkit::Scratch;
use crate::{load, view};

const LATER: &str = "2026-10-23T13:00:00Z";

fn next_ids(s: &Scratch) -> Vec<String> {
    let store = load(&s.root).unwrap();
    let mut ids: Vec<String> = view::next(
        &store,
        &planning::Plan::default(),
        &view::Filter::default(),
        &crate::testkit::Branches::default(),
    )
    .into_iter()
    .map(|r| r.id)
    .collect();
    ids.sort();
    ids
}

#[test]
fn an_open_ticket_holding_back_tasks_and_jobs_keeps_every_one_out_of_next_until_it_closes() {
    let s = Scratch::new();
    let wall = s.add_as("A wall", "task", Some("ready"), "", &[]).unwrap();
    let roof = s.add_as("A roof", "task", Some("backlog"), "", &[]).unwrap();
    // control first: with no gate both are free work, so a missing hold would show here
    let mut free = vec![wall.clone(), roof.clone()];
    free.sort();
    assert_eq!(next_ids(&s), free);
    let gate = s
        .add_as("The upstream is ready", "task", Some("waiting"), "op", &[(
            "holds_back",
            &["task", "job"],
        )])
        .unwrap();
    assert_eq!(next_ids(&s), Vec::<String>::new(), "an open gate holds back every task");
    s.mv(&gate, "done", None, LATER).unwrap();
    assert_eq!(next_ids(&s), free, "a closed gate holds nothing back");
}

#[test]
fn a_gate_holds_back_only_the_kinds_it_names() {
    let s = Scratch::new();
    let wall = s.add_as("A wall", "task", Some("ready"), "", &[]).unwrap();
    s.add_as("Jobs wait", "task", Some("waiting"), "op", &[("holds_back", &["job"])])
        .unwrap();
    assert_eq!(next_ids(&s), [wall], "a task passes a gate on jobs");
}

#[test]
fn a_kind_no_registry_has_is_refused_in_holds_back() {
    let s = Scratch::new();
    assert!(
        s.add_as("Gate", "task", Some("waiting"), "op", &[("holds_back", &["chore"])])
            .is_err(),
        "holds_back names ticket kinds"
    );
}
