//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What a placeholder marker says, and which markers say one thing.
//!
//! A marker is grouped by the words after its colon, never by where it stands: markers of one kind
//! whose content words overlap by at least `SIMILAR` share a ticket, a marker with too few words to
//! name a gap shares only with the same words in its crate, and the rest of a crate's markers of one
//! kind share one ticket between them. A marker whose tag names a seam joins the one ticket of that
//! tag. The design is `mock/WORKFLOW.md.tmpl`, "Work items".

use std::collections::{BTreeMap, BTreeSet};

/// The share of content words two markers have in common for them to be one gap: the size of what
/// they share over the size of what either has.
pub const SIMILAR: f64 = 0.45;

/// Fewer content words than this and a reason names no gap by itself.
pub const TOO_FEW: usize = 3;

/// How many words of a reason a ticket lists for the marker it covers.
const GIST_WORDS: usize = 10;

/// How long the listed words may be.
const GIST_CHARS: usize = 100;

/// What a marker is, by its tag.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    /// The canon is silent on it: a decision is owed.
    CanonSilent,
    /// Its tag says the maintainer must settle it: a decision is owed.
    Maintainer,
    /// A stand-in for what is to come.
    StandIn,
    /// No tag.
    Unmarked,
    /// A tag that names a seam, and every marker of the tag is one task.
    Seam(String),
}

impl Class {
    /// The class of a marker from what stands between its parentheses.
    pub fn of(tag: Option<&str>) -> Class {
        let Some(tag) = tag.map(str::trim).filter(|t| !t.is_empty()) else {
            return Class::Unmarked;
        };
        match tag.to_ascii_lowercase().replace(['-', '_'], " ").as_str() {
            "canon silent" => Class::CanonSilent,
            "maintainer" => Class::Maintainer,
            "stand in" | "standin" => Class::StandIn,
            _ => Class::Seam(tag.to_string()),
        }
    }

    /// Whether the markers of this class owe a decision of the maintainer or of the canon, which the task says.
    pub fn owes_a_decision(&self) -> bool {
        matches!(self, Class::CanonSilent | Class::Maintainer)
    }

    /// What the markers of the class are called, in the plural.
    pub fn plural(&self) -> String {
        match self {
            Class::CanonSilent => "canon silences".to_string(),
            Class::Maintainer => "points for the maintainer".to_string(),
            Class::StandIn => "stand-ins".to_string(),
            Class::Unmarked => "unmarked placeholders".to_string(),
            Class::Seam(tag) => format!("placeholders tagged {tag}"),
        }
    }
}

/// The words that carry no gap: said of everything, so sharing them says nothing.
const STOP: [&str; 34] = [
    "the", "and", "for", "with", "that", "this", "from", "are", "not", "but", "its", "has", "have",
    "will", "when", "then", "than", "into", "out", "over", "per", "any", "all", "can", "may",
    "use", "used", "uses", "via", "yet", "now", "one", "two", "was",
];

/// The content words of a reason: lowercase runs of letters and digits of three or more characters,
/// without the words that are said of everything.
pub fn words(reason: &str) -> BTreeSet<String> {
    reason
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.chars().count() >= 3 && !STOP.contains(w))
        .map(String::from)
        .collect()
}

/// What two sets share over what either holds.
pub fn jaccard(a: &BTreeSet<String>, b: &BTreeSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let shared = a.intersection(b).count();
    shared as f64 / (a.len() + b.len() - shared) as f64
}

/// The first words of a reason, as a ticket lists the marker it covers. Cutting it again gives the
/// same words, which is what lets a listed line be matched back to the marker it came from.
pub fn gist(reason: &str) -> String {
    let mut out = String::new();
    for w in reason.split_whitespace().take(GIST_WORDS) {
        let next = if out.is_empty() { w.len() } else { out.len() + 1 + w.len() };
        if next > GIST_CHARS {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(w);
    }
    if out.is_empty() { "(no reason given)".to_string() } else { out }
}

/// What identifies a marker within its file across a move of its line: its listed words, lowercase.
pub fn key(reason: &str) -> String {
    gist(reason).to_lowercase()
}

/// The part of the repository a path is in, as a ticket names it: a crate by its directory, the
/// content, or the kind of tooling.
pub fn area(rel: &str) -> String {
    let segs: Vec<&str> = rel.split('/').collect();
    if let Some(i) = segs.iter().position(|s| *s == "crates")
        && let Some(name) = segs.get(i + 1).filter(|_| i + 2 < segs.len())
    {
        return (*name).to_string();
    }
    match segs.as_slice() {
        ["content", pack, ..] if segs.len() > 2 => format!("content/{pack}"),
        ["content", ..] => "content".to_string(),
        // FIXME(pack_lints_and_tools_read_the_mock_directory_as_mock): the mock directory is read as `mock`, not as the one `mockspace.toml` declares
        ["mock", "tools", name, ..] if segs.len() > 3 => format!("tool {name}"),
        ["mock", "lints", ..] => "lints".to_string(),
        ["mock", "agent", ..] => "agent text".to_string(),
        ["mock", "benches", ..] => "benches".to_string(),
        ["mock", _] => "mock design".to_string(),
        [first, ..] if segs.len() > 1 => (*first).to_string(),
        _ => "root".to_string(),
    }
}

/// The canon subject a path is about, where one of the registry's `subject` rows is named for it:
/// the crate without the project's `crate_prefix` and its dash, the content, or the workflow for the tooling.
pub fn subject_of(area: &str, prefix: &str, subjects: &BTreeSet<String>) -> Option<String> {
    let guess = if let Some(rest) = area.strip_prefix(prefix).and_then(|r| r.strip_prefix('-')) {
        rest.replace('-', "_")
    } else if area.starts_with("content") {
        "content".to_string()
    } else if area.starts_with("tool ")
        || matches!(area, "lints" | "agent text" | "mock design" | "benches")
    {
        "workflow".to_string()
    } else {
        area.replace('-', "_")
    };
    subjects.contains(&guess).then_some(guess)
}

/// One uncited marker, as the grouping sees it.
#[derive(Debug, Clone)]
pub struct Item {
    pub rel:    String,
    pub class:  Class,
    pub reason: String,
    pub area:   String,
    /// The line the marker is on, from 1, as it stands when the task is filed.
    pub line:   usize,
}

/// Why a set of markers share a ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum How {
    /// The tag names a seam.
    Seam,
    /// The words overlap.
    Similar,
    /// The same few words in the same crate.
    Stock,
    /// Nothing else joins them: a crate's markers of one kind.
    Rest,
}

/// Markers that share one ticket, as indices into the items given.
#[derive(Debug, Clone)]
pub struct Group {
    pub class: Class,
    pub how:   How,
    pub items: Vec<usize>,
}

/// Groups the items. The same items in the same order always group the same way.
pub fn group(items: &[Item]) -> Vec<Group> {
    let mut by_class: BTreeMap<Class, Vec<usize>> = BTreeMap::new();
    for (i, it) in items.iter().enumerate() {
        by_class.entry(it.class.clone()).or_default().push(i);
    }
    let mut out = Vec::new();
    for (class, idx) in by_class {
        if matches!(class, Class::Seam(_)) {
            out.push(Group {
                class,
                how: How::Seam,
                items: idx,
            });
            continue;
        }
        // a cluster is led by its first marker, and a marker joins the first cluster whose leader it
        // overlaps, so a chain of near-matches cannot pull two unlike gaps into one
        let mut clusters: Vec<(BTreeSet<String>, Vec<usize>)> = Vec::new();
        let mut stock: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
        for &i in &idx {
            let w = words(&items[i].reason);
            if w.len() < TOO_FEW {
                stock
                    .entry((items[i].area.clone(), key(&items[i].reason)))
                    .or_default()
                    .push(i);
                continue;
            }
            match clusters
                .iter_mut()
                .find(|(lead, _)| jaccard(lead, &w) >= SIMILAR)
            {
                Some((_, members)) => members.push(i),
                None => clusters.push((w, vec![i])),
            }
        }
        let mut rest: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut joined: Vec<(How, Vec<usize>)> = Vec::new();
        for (_, members) in clusters {
            if members.len() > 1 {
                joined.push((How::Similar, members));
            } else {
                rest.entry(items[members[0]].area.clone())
                    .or_default()
                    .extend(members);
            }
        }
        for (_, members) in stock {
            if members.len() > 1 {
                joined.push((How::Stock, members));
            } else {
                rest.entry(items[members[0]].area.clone())
                    .or_default()
                    .extend(members);
            }
        }
        for (_, mut members) in rest {
            members.sort_unstable();
            joined.push((How::Rest, members));
        }
        joined.sort_by_key(|(_, m)| m[0]);
        out.extend(joined.into_iter().map(|(how, items)| {
            Group {
                class: class.clone(),
                how,
                items,
            }
        }));
    }
    out
}

#[cfg(test)]
mod tests;
