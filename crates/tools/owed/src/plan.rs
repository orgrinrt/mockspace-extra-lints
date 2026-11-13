//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What a run does, worked out before anything is written: the tickets to file, the markers to
//! rewrite to cite one, and the tickets to close.
//!
//! The tickets `owed` filed are known by their first move, made by `owed`, and each lists the
//! markers it covers in its `what` as a path and the first words of the marker. That list is how a
//! second run finds a marker an earlier run filed but did not get to rewrite, and agrees with it.

use std::collections::{BTreeMap, BTreeSet};

use mockspace_registry::is_valid_slug;
use registry_lints::layout::Layout;
use ticket_model::Store;
use ticket_model::rules::slugify;

use crate::fixme_markers::Marker;
use crate::group::{self, Class, Item};

mod wording;
#[cfg(test)]
pub(crate) use wording::tidy_for_test;
use wording::{answered_what, candidate_title, cut, title, unique_id, what_of};

/// Who `owed` is, in the moves it makes.
pub const BY: &str = "owed";

/// The key of `[tools.owed]` naming who holds what `owed` files: whoever's agents evaluate it, such as a
/// coordinator's channel, and never the maintainer, who is put a question only by an agent that has read
/// the code and the canon. What is filed is held by nobody where the table names no one.
pub const HOLDER: &str = "holder";

/// The heading of the list of markers in a ticket's `what`.
pub const COVERS: &str = "Covers:";

/// What a task filed by `owed` says to the agent that takes it. A question written from a marker
/// alone has neither the context nor the options a reader of the code and the canon gives it, so
/// `owed` files none, and the agent files the question it finds there is.
const EVALUATE: &str = "\n\nAn agent is to evaluate each marker below: read the code at its place and the canon (`mock canon find`, `mock canon subject`), then do one of three things. Put a question to the maintainer, with a subject and real options, where the canon is silent and the gap is the maintainer's to settle. Settle the marker, where the code or the canon already answers it. Drop the marker, where it names nothing owed. This task files no question of its own, and the line of each marker is where it stood when the task was filed.";

/// The line that names the file a candidate task is about.
pub const CANDIDATE: &str = "Candidate file: ";

/// What a task for a candidate file says to the agent that takes it.
const RATIFY: &str = "\n\nAn agent is to read the candidate against the canon and the code, and then to put it to the maintainer as a question with a subject and the options ratify, ratify changed and refuse, or to settle it without the maintainer where the canon already answers.";

/// A file with markers in it.
pub struct File {
    pub rel:     String,
    pub text:    String,
    pub markers: Vec<Marker>,
}

/// A candidate file.
pub struct Candidate {
    pub rel:  String,
    pub text: String,
}

/// What a run reads.
pub struct Inputs<'a> {
    /// The live files with markers, after `--only`.
    pub files:          &'a [File],
    /// The candidate files, after `--only`.
    pub candidates:     &'a [Candidate],
    /// Every candidate file there is, whatever `--only` says.
    pub all_candidates: &'a BTreeSet<String>,
    pub store:          &'a Store,
    pub subjects:       &'a BTreeSet<String>,
    /// The project's `crate_prefix`, which a crate's subject is named without.
    pub prefix:         &'a str,
    /// Where the mock tree and the content sit, which a marker's area is named by.
    pub layout:         &'a Layout,
    /// Nothing was left out by `--only`, so a ticket none of whose markers is found is closed.
    pub full:           bool,
}

/// A ticket to file.
#[derive(Debug, Clone)]
pub struct NewTicket {
    pub id:       String,
    pub title:    String,
    pub what:     String,
    pub subjects: Vec<String>,
    /// The markers it covers, as (index into files, index into that file's markers).
    pub covers:   Vec<(usize, usize)>,
}

/// A marker to rewrite to cite a ticket.
#[derive(Debug, Clone)]
pub struct Cite {
    pub file:   usize,
    pub marker: usize,
    pub id:     String,
}

/// A ticket to close.
#[derive(Debug, Clone)]
pub struct Close {
    pub id:  String,
    pub to:  &'static str,
    pub why: String,
}

/// Everything a run does.
#[derive(Debug, Default)]
pub struct Plan {
    pub new:    Vec<NewTicket>,
    pub cites:  Vec<Cite>,
    pub closes: Vec<Close>,
    /// Markers that already cite a ticket.
    pub cited:  usize,
}

impl Plan {
    pub fn is_empty(&self) -> bool {
        self.new.is_empty() && self.cites.is_empty() && self.closes.is_empty()
    }
}

/// The tickets `owed` filed: those whose first move it made.
fn filed_by_owed(store: &Store) -> BTreeSet<String> {
    let mut first: BTreeMap<&str, &str> = BTreeMap::new();
    for m in &store.moves {
        first.entry(m.ticket.as_str()).or_insert(m.by.as_str());
    }
    first
        .into_iter()
        .filter(|(_, by)| *by == BY)
        .map(|(t, _)| t.to_string())
        .collect()
}

/// A listed path without the line the marker stood on when it was listed. The line moves with every
/// edit above the marker, so what a rerun matches by is the path and the words, and a list written
/// before lines were listed reads the same.
fn without_line(path: &str) -> &str {
    match path.rsplit_once(':') {
        Some((p, n)) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => p,
        _ => path,
    }
}

/// The (path, key) lines of a ticket's list of markers, and the candidate file it is about.
fn listed(what: &str) -> (Vec<(String, String)>, Option<String>) {
    let mut lines = Vec::new();
    let mut candidate = None;
    let mut in_list = false;
    for line in what.lines() {
        if let Some(path) = line.strip_prefix(CANDIDATE) {
            candidate = Some(path.trim().to_string());
        } else if line.trim() == COVERS {
            in_list = true;
        } else if in_list
            && let Some((path, said)) = line.strip_prefix("- ").and_then(|l| l.split_once(": "))
        {
            lines.push((without_line(path).to_string(), group::key(said)));
        }
    }
    (lines, candidate)
}

fn open(store: &Store, id: &str) -> bool {
    store.state_of(id).is_some_and(|s| !s.closed)
}

/// The move that answered a closed ticket: its newest move, where that one carries an answer or
/// picks. A ticket closed with neither is not answered.
fn answered<'a>(store: &'a Store, id: &str) -> Option<&'a ticket_model::Move> {
    let now = store.now_of(id)?;
    let closed = store.states.get(&now.to).is_some_and(|s| s.closed);
    (closed && (now.gives("answer") || now.gives("picks"))).then_some(now)
}

/// Whether a file sits under the phase gate, so that editing it needs a round of its own: the
/// per-crate documents and sources under the mock directory. Its marker is left as it is.
fn gated(rel: &str, layout: &Layout) -> bool {
    layout.under_mock(rel).is_some_and(|m| m.starts_with("crates/"))
}

/// Works out what a run does.
pub fn plan(inp: &Inputs<'_>) -> Plan {
    let store = inp.store;
    let owed = filed_by_owed(store);
    // the markers an open ticket of ours lists, and the candidates any of ours is about
    let mut listed_by: BTreeMap<(String, String), String> = BTreeMap::new();
    let mut candidates_filed: BTreeSet<String> = BTreeSet::new();
    let mut open_marker_tickets: BTreeSet<String> = BTreeSet::new();
    let mut open_candidate_tickets: BTreeMap<String, String> = BTreeMap::new();
    for id in &owed {
        let (lines, candidate) = listed(store.tickets[id].one("what"));
        if let Some(path) = candidate {
            candidates_filed.insert(path.clone());
            if open(store, id) {
                open_candidate_tickets.insert(id.clone(), path);
            }
        }
        if open(store, id) && !lines.is_empty() {
            open_marker_tickets.insert(id.clone());
            for l in lines {
                listed_by.entry(l).or_insert_with(|| id.clone());
            }
        }
    }

    let mut out = Plan::default();
    let mut cited_ids: BTreeSet<String> = BTreeSet::new();
    let mut items: Vec<Item> = Vec::new();
    let mut places: Vec<(usize, usize)> = Vec::new();
    let mut answered_markers: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
    for (f, file) in inp.files.iter().enumerate() {
        for (m, marker) in file.markers.iter().enumerate() {
            let named = marker
                .tag
                .as_deref()
                .filter(|t| is_valid_slug(t) && store.tickets.contains_key(*t));
            if let Some(id) = named {
                out.cited += 1;
                cited_ids.insert(id.to_string());
                if answered(store, id).is_some() && !gated(&file.rel, inp.layout) {
                    answered_markers
                        .entry(id.to_string())
                        .or_default()
                        .push((f, m));
                }
                continue;
            }
            if let Some(id) = listed_by.get(&(file.rel.clone(), group::key(&marker.reason))) {
                cited_ids.insert(id.clone());
                out.cites.push(Cite {
                    file:   f,
                    marker: m,
                    id:     id.clone(),
                });
                continue;
            }
            items.push(Item {
                rel:    file.rel.clone(),
                class:  Class::of(marker.tag.as_deref()),
                reason: marker.reason.clone(),
                area:   group::area(&file.rel, inp.layout),
                line:   marker.line,
            });
            places.push((f, m));
        }
    }

    // the one task of a seam tag: markers of the tag found later cite the ticket already filed for it
    let seam_tickets: BTreeMap<String, String> = open_marker_tickets
        .iter()
        .map(|id| (store.tickets[id].one("title").to_string(), id.clone()))
        .collect();
    let mut taken = BTreeSet::new();
    for g in group::group(&items) {
        let title = title(&g, &items);
        let mut order = g.items.clone();
        order.sort_unstable();
        if let (Class::Seam(_), Some(id)) = (&g.class, seam_tickets.get(&title)) {
            for &i in &order {
                out.cites.push(Cite {
                    file:   places[i].0,
                    marker: places[i].1,
                    id:     id.clone(),
                });
            }
            cited_ids.insert(id.clone());
            continue;
        }
        let locations: Vec<(String, String)> = order
            .iter()
            .map(|&i| {
                (
                    format!("{}:{}", items[i].rel, items[i].line),
                    group::gist(&items[i].reason),
                )
            })
            .collect();
        let areas: BTreeSet<String> = order.iter().map(|&i| items[i].area.clone()).collect();
        let subjects: Vec<String> = if areas.len() == 1 {
            group::subject_of(areas.iter().next().map_or("", String::as_str), inp.prefix, inp.subjects, inp.layout)
                .into_iter()
                .collect()
        } else {
            Vec::new()
        };
        let id = unique_id(&title, store, &mut taken);
        cited_ids.insert(id.clone());
        out.new.push(NewTicket {
            id,
            title,
            what: what_of(&g, &items, &locations),
            subjects,
            covers: order.iter().map(|&i| places[i]).collect(),
        });
    }

    for (question, at) in &answered_markers {
        let title = format!(
            "Markers citing the closed {}",
            store.tickets[question].one("title")
        );
        let base = slugify(&title);
        let existing = owed
            .iter()
            .find(|id| id.starts_with(&base) && open(store, id));
        if let Some(task) = existing {
            for &(file, marker) in at {
                out.cites.push(Cite {
                    file,
                    marker,
                    id: task.clone(),
                });
            }
            cited_ids.insert(task.clone());
            continue;
        }
        let locations: Vec<(String, String)> = at
            .iter()
            .map(|&(f, m)| {
                let marker = &inp.files[f].markers[m];
                (
                    format!("{}:{}", inp.files[f].rel, marker.line),
                    group::gist(&marker.reason),
                )
            })
            .collect();
        out.new.push(NewTicket {
            id: unique_id(&title, store, &mut taken),
            title,
            what: answered_what(store, question, &locations),
            subjects: Vec::new(),
            covers: at.clone(),
        });
    }

    for c in inp.candidates {
        if candidates_filed.contains(&c.rel) {
            continue;
        }
        let title = candidate_title(c);
        let first = c
            .text
            .split("\n\n")
            .map(str::trim)
            .find(|p| !p.is_empty() && !p.starts_with('#'))
            .map_or(String::new(), |p| cut(&p.replace('\n', " "), 400));
        out.new.push(NewTicket {
            id: unique_id(&title, store, &mut taken),
            title,
            what: format!("{CANDIDATE}{}\n\n{first}{RATIFY}", c.rel),
            subjects: Vec::new(),
            covers: Vec::new(),
        });
    }

    if inp.full {
        for id in &open_marker_tickets {
            if !cited_ids.contains(id) {
                let question = store.tickets[id].one("kind") == "question";
                out.closes.push(Close {
                    id:  id.clone(),
                    to:  if question { "dropped" } else { "done" },
                    why: "no placeholder marker cites it any more".to_string(),
                });
            }
        }
        for (id, path) in &open_candidate_tickets {
            if !inp.all_candidates.contains(path) {
                out.closes.push(Close {
                    id:  id.clone(),
                    to:  "dropped",
                    why: format!("the candidate file {path} is gone"),
                });
            }
        }
    }
    out
}
