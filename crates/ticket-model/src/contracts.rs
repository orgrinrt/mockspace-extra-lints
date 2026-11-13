//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The contracts a ticket changes: its `contracts` field, each value a crate and a type joined by `::`,
//! and the question of which open tickets hold one. The crate names no crate of the workspace, so whether
//! a path's crate exists is the tool's to ask.

use crate::model::{Fields, Store, Ticket};

/// The field of a ticket row that names the contracts it changes.
pub const FIELD: &str = "contracts";

fn is_crate(s: &str) -> bool {
    s.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_type(s: &str) -> bool {
    s.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Whether `path` is a crate in the dashed lowercase spelling of its directory and a type as Rust spells
/// it, joined by `::`, and nothing else.
///
/// # Errors
///
/// A sentence naming the value.
pub fn shape(path: &str) -> Result<(), String> {
    match path.split_once("::") {
        Some((krate, ty)) if is_crate(krate) && is_type(ty) => Ok(()),
        _ => {
            Err(format!(
                "`{FIELD}` names `{path}`, which is not a crate and a type joined by `::`"
            ))
        },
    }
}

/// Holds every value of the `contracts` field in `fields` to `shape`.
///
/// # Errors
///
/// The sentence for the first value that fails it.
pub fn check_fields(fields: &Fields) -> Result<(), String> {
    fields
        .get(FIELD)
        .into_iter()
        .flatten()
        .filter(|v| !v.is_empty())
        .try_for_each(|v| shape(v))
}

/// Whether a contract a ticket holds is the one asked for: the same path, or, where the ask is a bare
/// type name, a path ending in that type.
fn holds(contract: &str, asked: &str) -> bool {
    contract == asked || (!asked.contains("::") && contract.rsplit("::").next() == Some(asked))
}

impl Store {
    /// The open tickets whose `contracts` hold `path`, in id order. A path with no `::` is a bare type
    /// name and matches the type in any crate. A ticket is open in a state that does not close it.
    pub fn changing<'a>(&'a self, path: &'a str) -> impl Iterator<Item = &'a Ticket> + 'a {
        self.tickets.values().filter(move |t| {
            self.state_of(&t.id).is_some_and(|s| !s.closed)
                && t.many(FIELD).iter().any(|c| holds(c, path))
        })
    }
}
