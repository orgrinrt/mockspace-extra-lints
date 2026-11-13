//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `ticket-move --close-gone`: every open ticket whose only subject the tree lost is moved to its kind's
//! closing state.
//!
//! Which tickets those are and what their kind closes to is the ticket model crate's `closable` and
//! `closing_state`; this is only the write, one move per ticket, each judged by the same rules as any
//! other move. The markers that cite a closed ticket are the caller's, which hands them on to the
//! successor, so this takes the question of whether a ticket is cited as a function.

use std::path::Path;

use ticket_model::{Tree, stale};

use crate::write::{MoveRequest, move_ticket};

/// What a sweep is told besides the registry and the tree.
pub struct Sweep<'a> {
    pub by:        &'a str,
    pub at:        &'a str,
    /// The open ticket that takes over the markers citing each ticket closed, where one is named.
    pub successor: Option<&'a str>,
    /// Whether a live marker cites this ticket. Closing a cited ticket with no successor would make the
    /// lint on markers refuse them on every branch that merges the move, so it is left open.
    pub cited:     &'a dyn Fn(&str) -> bool,
}

/// What a sweep did: how many open tickets it looked at, the ids closed, in the order closed, and the ids
/// left open for a live marker.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Swept {
    /// The open tickets the first pass read, which is not the number closed (ticket_move_close_gone_reports_the_number_it_closed_as).
    pub examined: usize,
    pub closed:   Vec<String>,
    pub left:     Vec<String>,
}

/// Closes every closable ticket, and then every ticket that closing them made closable, until a pass
/// closes none. Every ticket is tried; a refusal is returned with what was done, so a sweep is never
/// half-silent.
pub fn close_gone(
    registry: &Path,
    tree: &impl Tree,
    sweep: &Sweep<'_>,
) -> Result<Swept, (Swept, String)> {
    let mut done = Swept::default();
    let mut refused: Vec<String> = Vec::new();
    let mut tried: Vec<String> = Vec::new();
    loop {
        let store = crate::load(registry).map_err(|e| (done.clone(), e))?;
        if tried.is_empty() {
            done.examined = store
                .tickets
                .keys()
                .filter(|id| store.state_of(id).is_some_and(|s| !s.closed))
                .count();
        }
        let next: Vec<_> = stale::closable(&store, tree)
            .into_iter()
            .filter(|c| !tried.contains(&c.ticket) && Some(c.ticket.as_str()) != sweep.successor)
            .collect();
        if next.is_empty() {
            break;
        }
        for c in next {
            tried.push(c.ticket.clone());
            if sweep.successor.is_none() && (sweep.cited)(&c.ticket) {
                done.left.push(c.ticket);
                continue;
            }
            let to = match stale::closing_state(&store, &c.kind) {
                Ok(to) => to,
                Err(e) => {
                    refused.push(format!("`{}`: {e}", c.ticket));
                    continue;
                },
            };
            let request = MoveRequest {
                ticket: c.ticket.clone(),
                to,
                by: sweep.by.to_string(),
                at: sweep.at.to_string(),
                why: stale::why(&c),
                successor: sweep.successor.map(str::to_string),
                ..MoveRequest::default()
            };
            match move_ticket(registry, &request) {
                Ok(_) => done.closed.push(c.ticket),
                Err(e) => refused.push(format!("`{}`: {e}", c.ticket)),
            }
        }
    }
    if refused.is_empty() { Ok(done) } else { Err((done, refused.join("\n"))) }
}
