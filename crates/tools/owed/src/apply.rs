//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Writing what a plan says: the tickets first, then the markers that cite them, then the closings.
//!
//! The order is the safe one. A run that stops after the tickets leaves markers a second run finds
//! listed on them and rewrites to cite them, and a run that stopped after rewriting would leave
//! markers citing a ticket that does not exist.

use std::collections::BTreeMap;
use std::path::Path;

use ticket_model::Store;
use tickets::write::{self, AddRequest, MoveRequest};

use crate::fixme_markers;
use crate::plan::{BY, File, Plan};

/// What a run did, in counts.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Done {
    pub tasks:   usize,
    pub covered: usize,
    pub cited:   usize,
    pub closed:  usize,
}

/// Writes the plan under `repo`, with the tickets in `registry` held by `holder`, nobody where it is empty,
/// as of `at`.
///
/// # Errors
///
/// What the ticket writer or the file system refused.
pub fn apply(
    repo: &Path,
    registry: &Path,
    store: &Store,
    files: &[File],
    plan: &Plan,
    holder: &str,
    at: &str,
) -> Result<Done, String> {
    let mut done = Done::default();
    for t in &plan.new {
        let mut fields = BTreeMap::new();
        fields.insert("what".to_string(), vec![t.what.clone()]);
        if !t.subjects.is_empty() {
            fields.insert(
                "related".to_string(),
                t.subjects.iter().map(|s| format!("subject::{s}")).collect(),
            );
        }
        write::add_with(store, registry, &AddRequest {
            title: t.title.clone(),
            id: Some(t.id.clone()),
            kind: "task".to_string(),
            state: None,
            holder: holder.to_string(),
            by: BY.to_string(),
            at: at.to_string(),
            why: format!("filed by mock owed for {} marker(s)", t.covers.len().max(1)),
            fields,
        })?;
        done.tasks += 1;
        done.covered += t.covers.len();
    }

    // every marker to rewrite, by file: those of the new tickets and those an earlier run listed
    let mut cites: BTreeMap<usize, Vec<(usize, &str)>> = BTreeMap::new();
    for t in &plan.new {
        for &(f, m) in &t.covers {
            cites.entry(f).or_default().push((m, t.id.as_str()));
        }
    }
    for c in &plan.cites {
        cites
            .entry(c.file)
            .or_default()
            .push((c.marker, c.id.as_str()));
    }
    for (f, list) in cites {
        let file = &files[f];
        let at: Vec<(&fixme_markers::Marker, &str)> =
            list.iter().map(|&(m, id)| (&file.markers[m], id)).collect();
        let text = fixme_markers::rewrite(&file.text, &at);
        let path = repo.join(&file.rel);
        std::fs::write(&path, text).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        done.cited += list.len();
    }

    for c in &plan.closes {
        write::move_ticket(registry, &MoveRequest {
            ticket: c.id.clone(),
            to: c.to.to_string(),
            release: true,
            by: BY.to_string(),
            at: at.to_string(),
            why: c.why.clone(),
            ..Default::default()
        })?;
        done.closed += 1;
    }
    Ok(done)
}
