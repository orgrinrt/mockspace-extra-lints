//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! A superseded tenet named by a ticket, followed to the row that binds in its place.
//!
//! The crate reads no canon: it takes the `Heirs` the tool that reads the tenets hands it. A ticket
//! about the demotion keeps the old id on purpose, so it is returned as it is.

use std::collections::BTreeMap;

use crate::model::{Store, Ticket};
use crate::rules::Finding;

/// Each superseded row with the rows that bind in its place.
pub type Heirs = BTreeMap<String, Vec<String>>;

/// The tag a ticket about a demotion carries.
pub const DEMOTED_TAG: &str = "demoted-row";

/// A ticket's links after the heirs replace the superseded rows in them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repointed {
    pub related: Vec<String>,
    pub tenets:  Vec<String>,
    /// The superseded rows left in place because they have several heirs.
    pub left:    Vec<String>,
    /// Whether any entry was replaced.
    pub changed: bool,
}

fn norm(s: &str) -> String {
    let mut out = String::from("_");
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    if !out.ends_with('_') {
        out.push('_');
    }
    out
}

/// Whether the ticket is about the demotion of `row`: it carries the tag, or the row is in its title.
fn about_demotion(t: &Ticket, row: &str) -> bool {
    t.many("tags").iter().any(|g| g == DEMOTED_TAG) || norm(t.one("title")).contains(&norm(row))
}

/// The row an entry names: `tenet::<row>` or a bare id.
fn row_of(entry: &str) -> &str {
    entry.strip_prefix("tenet::").unwrap_or(entry)
}

/// The superseded rows a ticket's links name that its being about a demotion does not excuse, each
/// with its heirs.
fn naming<'a>(t: &Ticket, heirs: &'a Heirs) -> Vec<(&'a str, &'a [String])> {
    let mut out: Vec<(&str, &[String])> = Vec::new();
    for field in ["related", "tenets"] {
        for e in t.many(field) {
            if let Some((old, new)) = heirs.get_key_value(row_of(e))
                && !about_demotion(t, old)
                && !out.iter().any(|(o, _)| *o == old)
            {
                out.push((old.as_str(), new.as_slice()));
            }
        }
    }
    out
}

fn rewrite(t: &Ticket, field: &str, heirs: &Heirs, changed: &mut bool) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for e in t.many(field) {
        let row = row_of(e);
        let kept = match heirs.get(row) {
            Some(new) if new.len() == 1 && !about_demotion(t, row) => {
                *changed = true;
                if e.starts_with("tenet::") {
                    format!("tenet::{}", new[0])
                } else {
                    new[0].clone()
                }
            },
            _ => e.clone(),
        };
        if !out.contains(&kept) {
            out.push(kept);
        }
    }
    out
}

/// The `related` and `tenets` a ticket should hold: each superseded row with exactly one heir replaced
/// by it, deduplicated keeping the first, and the rows with several heirs named in `left`.
pub fn repoint(t: &Ticket, heirs: &Heirs) -> Repointed {
    let mut changed = false;
    let related = rewrite(t, "related", heirs, &mut changed);
    let tenets = rewrite(t, "tenets", heirs, &mut changed);
    let left = naming(t, heirs)
        .into_iter()
        .filter(|(_, new)| new.len() != 1)
        .map(|(old, _)| old.to_string())
        .collect();
    Repointed {
        related,
        tenets,
        left,
        changed,
    }
}

/// Every ticket still naming a superseded row it is not about, with the heirs.
pub fn left_naming(store: &Store, heirs: &Heirs) -> Vec<Finding> {
    store
        .tickets
        .values()
        .filter_map(|t| {
            let named: Vec<String> = naming(t, heirs)
                .into_iter()
                .map(|(old, new)| {
                    if new.is_empty() {
                        format!("`{old}` (superseded by nothing that binds)")
                    } else {
                        format!("`{old}` (superseded, now `{}`)", new.join("`, `"))
                    }
                })
                .collect();
            (!named.is_empty()).then(|| {
                Finding {
                    row:     format!("ticket::{}", t.id),
                    message: format!("names {}", named.join(", ")),
                }
            })
        })
        .collect()
}
