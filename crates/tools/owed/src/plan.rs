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
use ticket_model::Store;
use ticket_model::rules::slugify;

use crate::fixme_markers::Marker;
use crate::group::{self, Class, How, Item};

/// Who `owed` is, in the moves it makes.
pub const BY: &str = "owed";

/// Who holds what `owed` files: the coordinator, whose agents evaluate it. Never the maintainer, who
/// is put a question only by an agent that has read the code and the canon.
pub const HOLDER: &str = "coord";

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
// FIXME(pack_lints_and_tools_read_the_mock_directory_as_mock): the mock directory is read as `mock`, not as the one `mockspace.toml` declares
/// per-crate documents and sources. Its marker is left as it is.
fn gated(rel: &str) -> bool {
    rel.starts_with("mock/crates/")
}

/// What the task for an answered question says: the answer, the picks and the options verbatim.
fn answered_what(store: &Store, question: &str, locations: &[(String, String)]) -> String {
    let t = &store.tickets[question];
    let mv = answered(store, question);
    let mut out = format!(
        "Markers cite the question `{question}`, which was answered. Filed by `mock owed`; the markers cite this ticket and it closes when none is left.\n\nThe question: {}\n",
        t.one("title")
    );
    if !t.many("options").is_empty() {
        out.push_str("\nOptions it was put with:\n");
        for o in t.many("options") {
            out.push_str(&format!("- {o}\n"));
        }
    }
    if let Some(m) = mv {
        if !m.picks.is_empty() {
            out.push_str(&format!("\nPicks: {}\n", m.picks.join("; ")));
        }
        if m.gives("answer") {
            out.push_str(&format!("\nAnswer, verbatim: {}\n", m.answer));
        }
    }
    out.push_str("\nAn agent is to apply the answer at each marker below: settle the marker where the answer decides it, or drop it where it names nothing owed. This task files no question of its own.");
    out.push_str(&format!("\n\n{COVERS}\n"));
    for (path, said) in locations {
        out.push_str(&format!("- {path}: {said}\n"));
    }
    out
}

fn unique_id(title: &str, store: &Store, taken: &mut BTreeSet<String>) -> String {
    let base = slugify(title);
    let mut id = base.clone();
    let mut n = 2;
    while store.tickets.contains_key(&id) || taken.contains(&id) {
        id = format!("{base}_{n}");
        n += 1;
    }
    taken.insert(id.clone());
    id
}

fn cut(text: &str, n: usize) -> String {
    if text.chars().count() <= n {
        return text.to_string();
    }
    let mut out: String = text.chars().take(n).collect();
    if let Some(i) = out.rfind(' ') {
        out.truncate(i);
    }
    out
}

/// Words a title does not end on: cut from the end of a title that was cut from a sentence.
const DANGLING: [&str; 26] = [
    "a", "an", "the", "to", "for", "of", "and", "or", "but", "is", "are", "in", "on", "at", "by",
    "with", "that", "so", "as", "its", "it", "be", "from", "this", "which", "than",
];

/// A title without the words and marks it was left hanging on when it was cut from a sentence.
fn tidy(text: &str) -> String {
    let mut words: Vec<&str> = text.split_whitespace().collect();
    loop {
        let Some(last) = words.last() else {
            break;
        };
        let bare = last
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        if bare.is_empty() || DANGLING.contains(&bare.as_str()) {
            words.pop();
        } else {
            break;
        }
    }
    let joined = words.join(" ");
    let joined = joined
        .trim_end_matches([',', ';', ':', '-', '(', '"'])
        .to_string();
    if joined.is_empty() { text.to_string() } else { joined }
}

#[cfg(test)]
pub(crate) fn tidy_for_test(text: &str) -> String {
    tidy(text)
}

/// What the ticket of a group is called.
fn title(g: &group::Group, items: &[Item]) -> String {
    let lead = &items[g.items[0]];
    let one_area = g.items.iter().all(|&i| items[i].area == lead.area);
    match (&g.class, g.how) {
        (Class::Seam(tag), _) => format!("Placeholders tagged {tag}"),
        (class, How::Rest) if g.items.len() > 1 => {
            format!("{}: {} {}", lead.area, g.items.len(), class.plural())
        },
        _ => {
            let said = tidy(&cut(&group::gist(&lead.reason), 70));
            if one_area { format!("{}: {said}", lead.area) } else { said }
        },
    }
}

fn what_of(g: &group::Group, items: &[Item], locations: &[(String, String)]) -> String {
    let areas: BTreeSet<&str> = g.items.iter().map(|&i| items[i].area.as_str()).collect();
    let areas = areas.into_iter().collect::<Vec<_>>().join(", ");
    let n = g.items.len();
    let mut out = format!(
        "{} in {areas}, {n} marker{}. Filed by `mock owed`; the markers cite this ticket and it closes when none is left.",
        match &g.class {
            Class::Seam(tag) => format!("Placeholders tagged {tag}"),
            c => format!("The {}", c.plural()),
        },
        if n == 1 { "" } else { "s" }
    );
    if g.class.owes_a_decision() {
        out.push_str("\n\nThe canon is silent on these, or the marker says the maintainer decides.");
    }
    out.push_str(EVALUATE);
    out.push_str(&format!("\n\n{COVERS}\n"));
    for (path, said) in locations {
        out.push_str(&format!("- {path}: {said}\n"));
    }
    out
}

fn candidate_title(c: &Candidate) -> String {
    let heading = c
        .text
        .lines()
        .find_map(|l| l.strip_prefix("# "))
        .map(|h| h.trim().to_string());
    let name = c.rel.rsplit('/').next().unwrap_or(&c.rel);
    let name = name.strip_suffix(".md").unwrap_or(name).to_string();
    format!("Ratify the candidate {}", cut(&heading.unwrap_or(name), 70))
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
                if answered(store, id).is_some() && !gated(&file.rel) {
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
                area:   group::area(&file.rel),
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
            group::subject_of(areas.iter().next().map_or("", String::as_str), inp.prefix, inp.subjects)
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
