//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The tickets that may have stopped being true, given as lists and never as refusals.
//!
//! A ticket may name a deleted crate only as history, and a parent may be left open on purpose, so
//! what is found here is read by a person and decides nothing. What the tree says is handed in as a
//! `Tree`; the rows are the `Store`.

use crate::model::Store;
use crate::rules::Finding;

/// What the tree and git say of a name.
pub trait Tree {
    /// The commit that deleted `name` where the tree held it once and holds it no more, and nothing
    /// where it is there still or never was.
    fn gone(&self, name: &str) -> Option<String>;
}

/// The names in a text that could be a crate or a path: a `kaski-` and a lowercase dashed word, or a
/// word holding a `/`. Each once, in the order first met.
pub fn names_in(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for raw in text.split(|c: char| !(c.is_ascii_alphanumeric() || "_./-".contains(c))) {
        let w = raw.trim_matches(|c| c == '.' || c == '-' || c == '/');
        // an absolute path and one leaving the repository name nothing the tree holds
        if w.is_empty() || w.contains("://") || raw.starts_with('/') || raw.contains("..") {
            continue;
        }
        if (is_crate(w) || w.contains('/')) && !out.iter().any(|o| o == w) {
            out.push(w.to_string());
        }
    }
    out
}

fn is_crate(w: &str) -> bool {
    w.strip_prefix("kaski-").is_some_and(|rest| {
        !rest.is_empty()
            && rest.split('-').all(|p| {
                !p.is_empty()
                    && p.chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            })
    })
}

fn closed(store: &Store, id: &str) -> Option<bool> {
    store.state_of(id).map(|s| s.closed)
}

/// Every open ticket whose title or `what` names what the tree held once and holds no more, with the
/// commit that deleted each.
pub fn gone_names(store: &Store, tree: &impl Tree) -> Vec<Finding> {
    let mut out = Vec::new();
    for t in store.tickets.values() {
        if closed(store, &t.id) != Some(false) {
            continue;
        }
        let text = format!("{}\n{}", t.one("title"), t.one("what"));
        let gone: Vec<String> = names_in(&text)
            .into_iter()
            .filter_map(|n| tree.gone(&n).map(|by| format!("`{n}` (deleted by {by})")))
            .collect();
        if !gone.is_empty() {
            out.push(Finding {
                row:     format!("ticket::{}", t.id),
                message: format!(
                    "may be settled, it names what the tree lost: {}",
                    gone.join(", ")
                ),
            });
        }
    }
    out
}

/// An open parent whose children and needs are all closed, and a closed parent with an open child.
/// An open ticket with no child is never named.
pub fn left_open(store: &Store) -> Vec<Finding> {
    let mut out = Vec::new();
    for t in store.tickets.values() {
        let Some(is_closed) = closed(store, &t.id) else {
            continue;
        };
        let children: Vec<&str> = store
            .tickets
            .values()
            .filter(|c| c.one("part_of") == t.id)
            .map(|c| c.id.as_str())
            .collect();
        if children.is_empty() {
            continue;
        }
        let open_children: Vec<&str> = children
            .iter()
            .copied()
            .filter(|c| closed(store, c) == Some(false))
            .collect();
        if is_closed && !open_children.is_empty() {
            out.push(Finding {
                row:     format!("ticket::{}", t.id),
                message: format!(
                    "is closed while its child `{}` is open",
                    open_children.join("`, `")
                ),
            });
        } else if !is_closed
            && open_children.is_empty()
            && t.many("needs")
                .iter()
                .all(|n| closed(store, n) == Some(true))
        {
            out.push(Finding {
                row:     format!("ticket::{}", t.id),
                message: format!(
                    "is open while all {} of its children and everything it needs are closed",
                    children.len()
                ),
            });
        }
    }
    out
}

/// An open ticket whose every subject the tree lost, with each name and the commit that deleted it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Closable {
    pub ticket: String,
    pub kind:   String,
    pub gone:   Vec<(String, String)>,
}

/// The open tickets all of whose names the tree lost, that no work hangs from: no open child through
/// `part_of`, no open ticket that `needs` it, a kind not closed by an answer, and nobody working it: a
/// state that needs a holder is a ticket in hand, while a holder named on a backlog row is only whose
/// ticket it is filed under.
/// A name the tree still holds, or never held, is a subject the ticket still has, so such a ticket stays
/// on the list `gone_names` gives.
pub fn closable(store: &Store, tree: &impl Tree) -> Vec<Closable> {
    let open = |id: &str| closed(store, id) == Some(false);
    let mut out = Vec::new();
    for t in store.tickets.values() {
        let by_answer = store
            .kinds
            .get(t.one("kind"))
            .is_none_or(|k| !k.satisfied_by.is_empty());
        // in hand is a state that needs somebody working it; a holder named on a backlog row only says
        // whose ticket it is filed under, which is nearly every open ticket of the real registry
        let held = store.state_of(&t.id).is_some_and(|s| s.needs_holder);
        if !open(&t.id) || by_answer || held {
            continue;
        }
        let names = names_in(&format!("{}\n{}", t.one("title"), t.one("what")));
        let gone: Option<Vec<(String, String)>> = names
            .iter()
            .map(|n| tree.gone(n).map(|by| (n.clone(), by)))
            .collect();
        let Some(gone) = gone.filter(|g| !g.is_empty()) else {
            continue;
        };
        let has_open_child = store
            .tickets
            .values()
            .any(|c| c.one("part_of") == t.id && open(&c.id));
        let has_open_dependent = store.dependents(&t.id).any(|d| open(&d.id));
        if has_open_child || has_open_dependent {
            continue;
        }
        out.push(Closable {
            ticket: t.id.clone(),
            kind: t.one("kind").to_string(),
            gone,
        });
    }
    out
}

/// The state a kind's tickets are closed to without being satisfied: the kind's one closed state that
/// does not satisfy. No state's id is written here, so a kind that names its states differently is
/// served as it is.
///
/// # Errors
///
/// When the kind is no kind, or has no such state or more than one.
pub fn closing_state(store: &Store, kind: &str) -> Result<String, String> {
    let k = store
        .kinds
        .get(kind)
        .ok_or_else(|| format!("`{kind}` is no kind of ticket"))?;
    let mut found = k.states.iter().filter(|s| {
        store
            .states
            .get(*s)
            .is_some_and(|st| st.closed && !st.satisfies)
    });
    match (found.next(), found.next()) {
        (Some(only), None) => Ok(only.clone()),
        (None, _) => {
            Err(format!(
                "kind `{kind}` has no closed state that does not satisfy"
            ))
        },
        _ => {
            Err(format!(
                "kind `{kind}` has several closed states that do not satisfy"
            ))
        },
    }
}

/// The why of a move that closes a ticket for what the tree lost: each name with the commit that deleted it.
pub fn why(c: &Closable) -> String {
    let names: Vec<String> = c
        .gone
        .iter()
        .map(|(n, by)| format!("`{n}` (deleted by {by})"))
        .collect();
    format!(
        "its only subject is gone from the tree: {}",
        names.join(", ")
    )
}

/// How many findings the two lists give.
pub fn stale_count(store: &Store, tree: &impl Tree) -> usize {
    gone_names(store, tree).len() + left_open(store).len()
}
