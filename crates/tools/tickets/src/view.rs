//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The tickets as the board would list them: each with its state and holder, filtered, and ranked by
//! how much it unblocks.

use std::collections::BTreeSet;

use planning::Plan;
use ticket_model::model::Store;

use crate::landing::{Trunk, landed};

/// Which tickets to list. An empty field matches all.
#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub state:  Vec<String>,
    pub holder: Vec<String>,
    pub kind:   Vec<String>,
    /// Epoch ids; `none` matches the tickets that are members of no epoch.
    pub epoch:  Vec<String>,
    /// Tags; a ticket matches when it holds at least one of them, each matched whole.
    pub tag:    Vec<String>,
}

/// One ticket as the list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub id:       String,
    pub state:    String,
    pub holder:   String,
    pub title:    String,
    /// The epoch the ticket is a member of, empty where it is in none.
    pub epoch:    String,
    /// The open tickets, counted through their needs, that cannot start until this one is done.
    pub unblocks: usize,
}

fn matches(want: &[String], got: &str) -> bool {
    want.is_empty() || want.iter().any(|w| w == got)
}

/// The open tickets that wait, directly or through others, on `ticket`.
fn unblocked_by(store: &Store, ticket: &str) -> usize {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut stack = vec![ticket];
    while let Some(t) = stack.pop() {
        for d in store.dependents(t) {
            let open = store.state_of(&d.id).is_some_and(|s| !s.closed);
            if open && seen.insert(d.id.as_str()) {
                stack.push(d.id.as_str());
            }
        }
    }
    seen.len()
}

/// Every ticket that passes `filter`, in board order: by the state's column, then rank, then when it
/// was added, then id.
pub fn list(store: &Store, plan: &Plan, filter: &Filter) -> Vec<Row> {
    let members = plan.members_now();
    let epoch_of = |id: &str| -> String {
        members
            .get(id)
            .map(|(span, _)| span.as_str())
            .filter(|span| {
                plan.row("plan_span", span)
                    .is_some_and(|s| s.one("planned") == "true")
            })
            .unwrap_or_default()
            .to_string()
    };
    let mut rows: Vec<(i64, i64, String, Row)> = Vec::new();
    for t in store.tickets.values() {
        let Some(now) = store.now_of(&t.id) else {
            continue;
        };
        if !matches(&filter.state, &now.to)
            || !matches(&filter.holder, &now.holder)
            || !matches(&filter.kind, t.one("kind"))
            || !(filter.tag.is_empty() || t.many("tags").iter().any(|g| filter.tag.contains(g)))
            || !matches(
                &filter.epoch,
                &if epoch_of(&t.id).is_empty() { "none".to_string() } else { epoch_of(&t.id) },
            )
        {
            continue;
        }
        let column = store.states.get(&now.to).map_or(i64::MAX, |s| s.column);
        let rank = t.rank().unwrap_or(i64::MAX);
        let added = store
            .moves_of(&t.id)
            .next()
            .map(|m| m.at.clone())
            .unwrap_or_default();
        rows.push((column, rank, added, Row {
            id:       t.id.clone(),
            state:    now.to.clone(),
            holder:   now.holder.clone(),
            title:    t.one("title").to_string(),
            epoch:    epoch_of(&t.id),
            unblocks: unblocked_by(store, &t.id),
        }));
    }
    rows.sort_by(|a, b| (a.0, a.1, &a.2, &a.3.id).cmp(&(b.0, b.1, &b.2, &b.3.id)));
    rows.into_iter().map(|r| r.3).collect()
}

/// What to take next: the open tickets nobody holds and nothing blocks, the one that unblocks the
/// most open work first, then the one added earliest. A job whose branch has landed on `trunk` is not
/// free: its work is done and its ticket missed the move.
pub fn next(store: &Store, plan: &Plan, filter: &Filter, trunk: &impl Trunk) -> Vec<Row> {
    let mut rows: Vec<Row> = list(store, plan, filter)
        .into_iter()
        .filter(|r| {
            let free = store
                .states
                .get(&r.state)
                .is_some_and(|s| !s.closed && !s.needs_holder);
            let blocked = store.tickets.get(&r.id).is_some_and(|t| {
                t.many("needs")
                    .iter()
                    .any(|n| !store.state_of(n).is_some_and(|s| s.satisfies))
            });
            free && !blocked
                && r.holder.is_empty()
                && superseded_by(store, &r.id).is_empty()
                && landed(store, &r.id, trunk).is_none()
        })
        .collect();
    rows.sort_by(|a, b| b.unblocks.cmp(&a.unblocks));
    rows
}

/// The open tickets among those `id` names in `superseded_by`: while one stands, the work of `id` is on
/// a path going away, so it is never taken next. A closed superseder leaves it as it was.
pub fn superseded_by<'a>(store: &'a Store, id: &str) -> Vec<&'a str> {
    store.tickets.get(id).map_or_else(Vec::new, |t| {
        t.many("superseded_by")
            .iter()
            .filter(|s| !store.state_of(s).is_some_and(|st| st.closed))
            .map(String::as_str)
            .collect()
    })
}

/// A row as a line: `id state holder title`, the holder `-` where there is none.
pub fn line(r: &Row, with_unblocks: bool) -> String {
    let holder = if r.holder.is_empty() { "-" } else { r.holder.as_str() };
    let mut s = format!("{} {} {} {}", r.id, r.state, holder, r.title);
    if !r.epoch.is_empty() {
        s.push_str(&format!(" [epoch {}]", r.epoch));
    }
    if with_unblocks {
        s.push_str(&format!(" (unblocks {})", r.unblocks));
    }
    s
}

/// The open tickets that change `path`, in board order, as `list` shows them. A path with no `::` is a
/// bare type name and matches the type in any crate.
pub fn changing(store: &Store, plan: &Plan, path: &str) -> Vec<Row> {
    let held: BTreeSet<&str> = store.changing(path).map(|t| t.id.as_str()).collect();
    list(store, plan, &Filter::default())
        .into_iter()
        .filter(|r| held.contains(r.id.as_str()))
        .collect()
}

fn who(holder: &str) -> &str {
    if holder.is_empty() { "-" } else { holder }
}

/// The tracks that pass `filter` by their track ticket, in the board order of `list`, one block each: the
/// track's id, state, holder and `shows` (and how many of its first steps wait on another track, where
/// any do), then its steps in order with state, holder and title, and under a step a line for each wait.
/// The count of blocks goes with the text.
pub fn tracks(store: &Store, plan: &Plan, filter: &Filter) -> (String, usize) {
    let all = ticket_model::tracks::of(store);
    let mut out = String::new();
    let mut count = 0;
    for row in list(store, plan, filter) {
        let Some(t) = all.iter().find(|t| t.id == row.id) else {
            continue;
        };
        count += 1;
        out.push_str(&format!(
            "track {} {} {} shows: {}",
            t.id,
            t.state,
            who(&t.holder),
            t.shows
        ));
        match t.first_steps_waiting() {
            0 => {},
            n => out.push_str(&format!(" [first steps waiting: {n}]")),
        }
        out.push('\n');
        for (i, s) in t.steps.iter().enumerate() {
            out.push_str(&format!(
                "  {} {} {} {} {}\n",
                i + 1,
                s.id,
                s.state,
                who(&s.holder),
                s.title
            ));
            for w in &s.waits {
                let track = w
                    .track
                    .as_deref()
                    .map_or("no track".to_string(), |x| format!("track {x}"));
                out.push_str(&format!(
                    "    waits on {} ({track}) {} {}\n",
                    w.on,
                    w.state,
                    who(&w.holder)
                ));
            }
        }
    }
    (out, count)
}
