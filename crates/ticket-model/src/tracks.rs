//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Tracks: a ticket that carries `shows` is a line of work ending in a result, the tickets `part_of` it
//! are its steps, and a step that needs a ticket outside the track which does not yet meet the edge waits
//! on it. Asked of the registry and never written, so it cannot drift from it. No kind's id is read.

use std::collections::BTreeSet;

use crate::model::{Store, Ticket};

/// The field of a ticket row that makes it a track: the result the line of work ends in.
pub const SHOWS: &str = "shows";

/// A `needs` edge of a step that leaves its track for a ticket not yet meeting it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wait {
    pub on:     String,
    /// The track the ticket waited on is a step of, none where it is a step of no track.
    pub track:  Option<String>,
    pub state:  String,
    pub holder: String,
}

/// One step of a track, with the waits it has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub id:     String,
    pub state:  String,
    pub holder: String,
    pub title:  String,
    /// Whether the step needs no step of its own track.
    pub first:  bool,
    pub waits:  Vec<Wait>,
}

/// A track, its steps in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub id:     String,
    pub state:  String,
    pub holder: String,
    pub shows:  String,
    pub steps:  Vec<Step>,
}

impl Track {
    /// The first steps that wait on somebody outside the track: what a coordinator reads before dispatching.
    pub fn first_steps_waiting(&self) -> usize {
        self.steps
            .iter()
            .filter(|s| s.first && !s.waits.is_empty())
            .count()
    }
}

fn is_track(t: &Ticket) -> bool {
    !t.one(SHOWS).is_empty()
}

/// The board order of a ticket: its state's column, then rank, then id.
fn board_key(store: &Store, t: &Ticket) -> (i64, i64, String) {
    (
        store.state_of(&t.id).map_or(i64::MAX, |s| s.column),
        t.rank().unwrap_or(i64::MAX),
        t.id.clone(),
    )
}

fn waits_of(store: &Store, step: &Ticket, steps: &BTreeSet<&str>) -> Vec<Wait> {
    step.many("needs")
        .iter()
        .filter(|n| !steps.contains(n.as_str()))
        .filter(|n| !store.state_of(n).is_some_and(|s| s.satisfies))
        .filter_map(|n| {
            let now = store.now_of(n)?;
            let track = store
                .tickets
                .get(n.as_str())
                .map(|t| t.one("part_of"))
                .filter(|p| store.tickets.get(*p).is_some_and(is_track))
                .map(str::to_string);
            Some(Wait {
                on: n.clone(),
                track,
                state: now.to.clone(),
                holder: now.holder.clone(),
            })
        })
        .collect()
}

/// The steps of the track `id`, each after every step of the track it needs and otherwise in board order.
fn ordered<'a>(store: &'a Store, id: &str) -> Vec<&'a Ticket> {
    let mut left: Vec<&Ticket> = store
        .tickets
        .values()
        .filter(|t| t.one("part_of") == id)
        .collect();
    left.sort_by_key(|t| board_key(store, t));
    let members: BTreeSet<&str> = left.iter().map(|t| t.id.as_str()).collect();
    let mut done: BTreeSet<&str> = BTreeSet::new();
    let mut out = Vec::with_capacity(left.len());
    while !left.is_empty() {
        // `needs` is acyclic where the rules hold; should it not be, the earliest step goes first
        let at = left
            .iter()
            .position(|t| {
                t.many("needs")
                    .iter()
                    .all(|n| !members.contains(n.as_str()) || done.contains(n.as_str()))
            })
            .unwrap_or(0);
        let t = left.remove(at);
        done.insert(t.id.as_str());
        out.push(t);
    }
    out
}

/// Every track the registry holds, in id order, with its steps and their waits.
pub fn of(store: &Store) -> Vec<Track> {
    store
        .tickets
        .values()
        .filter(|t| is_track(t))
        .map(|t| {
            let steps = ordered(store, &t.id);
            let ids: BTreeSet<&str> = steps.iter().map(|s| s.id.as_str()).collect();
            let now = store.now_of(&t.id);
            Track {
                id:     t.id.clone(),
                state:  now.map_or_else(String::new, |m| m.to.clone()),
                holder: now.map_or_else(String::new, |m| m.holder.clone()),
                shows:  t.one(SHOWS).to_string(),
                steps:  steps
                    .iter()
                    .map(|s| {
                        let now = store.now_of(&s.id);
                        Step {
                            id:     s.id.clone(),
                            state:  now.map_or_else(String::new, |m| m.to.clone()),
                            holder: now.map_or_else(String::new, |m| m.holder.clone()),
                            title:  s.one("title").to_string(),
                            first:  s.many("needs").iter().all(|n| !ids.contains(n.as_str())),
                            waits:  waits_of(store, s, &ids),
                        }
                    })
                    .collect(),
            }
        })
        .collect()
}
