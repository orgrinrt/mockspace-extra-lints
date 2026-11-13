//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The words of what `owed` files: the title and the `what` of each ticket, and the id made from
//! the title.

use std::collections::BTreeSet;

use ticket_model::Store;
use ticket_model::rules::slugify;

use super::{COVERS, Candidate, EVALUATE, answered};
use crate::group::{self, Class, How, Item};

/// What the task for an answered question says: the answer, the picks and the options verbatim.
pub(super) fn answered_what(store: &Store, question: &str, locations: &[(String, String)]) -> String {
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

pub(super) fn unique_id(title: &str, store: &Store, taken: &mut BTreeSet<String>) -> String {
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

pub(super) fn cut(text: &str, n: usize) -> String {
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
pub(super) fn tidy(text: &str) -> String {
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
pub(super) fn title(g: &group::Group, items: &[Item]) -> String {
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

pub(super) fn what_of(g: &group::Group, items: &[Item], locations: &[(String, String)]) -> String {
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

pub(super) fn candidate_title(c: &Candidate) -> String {
    let heading = c
        .text
        .lines()
        .find_map(|l| l.strip_prefix("# "))
        .map(|h| h.trim().to_string());
    let name = c.rel.rsplit('/').next().unwrap_or(&c.rel);
    let name = name.strip_suffix(".md").unwrap_or(name).to_string();
    format!("Ratify the candidate {}", cut(&heading.unwrap_or(name), 70))
}
