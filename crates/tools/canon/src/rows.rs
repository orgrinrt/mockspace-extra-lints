//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Reading tenets and subjects out of the flattened registry a tool is handed.
//!
//! The view joins a list field's entries with `", "`, which costs nothing here: every list read is of
//! ids or keywords, and neither carries the separator.

use std::collections::BTreeMap;

use mockspace::RegistryView;
use registry_lints::standing::Standing;

/// The separator the engine joins a list field with.
const JOIN: &str = ", ";

/// One tenet, as the verbs read it.
pub struct Tenet<'a> {
    /// Its id, the slug every citation names.
    pub id:            &'a str,
    /// Whether the maintainer has ratified it, holds it for himself or demoted it. Apart from
    /// `superseded_by`: a superseded row does not bind whatever its standing.
    pub standing:      Standing,
    /// Who stamped it provisionally, empty where nobody did. A stamp is never a ratification and changes
    /// nothing about whether the row binds.
    pub stamped_by:    &'a str,
    /// The stamp: what was stamped, why, where it is recorded and the question ticket asking the
    /// maintainer, empty where there is none.
    pub stamp:         &'a str,
    /// The subject it names, where it names one.
    pub subject:       Option<&'a str>,
    /// The tenets that replace it, empty while it binds.
    pub superseded_by: Vec<&'a str>,
    /// The tenets its words name.
    pub cites:         Vec<&'a str>,
    /// The tenets it links in `related`, where its words do not name them.
    pub links:         Vec<&'a str>,
    /// The words somebody would search for to find it.
    pub keywords:      Vec<&'a str>,
    /// The words and names it rules out of designs, code and content, empty where it rules out none.
    pub rules_out:     Vec<&'a str>,
    /// The commitment.
    pub what:          &'a str,
    /// The reasoning, empty where there is none.
    pub because:       &'a str,
}

impl Tenet<'_> {
    /// Whether it still binds: nothing supersedes it and its standing binds.
    pub fn binds(&self) -> bool {
        !self.replaced() && self.standing.binds()
    }

    /// Whether it carries a provisional stamp, which keeps its standing and is never a ratification.
    pub fn stamped(&self) -> bool {
        !self.stamped_by.is_empty() || !self.stamp.is_empty()
    }

    /// Whether another tenet replaces it, which is what a supersession chain follows. A demoted row is
    /// not replaced: it binds nothing and has no heir.
    pub fn replaced(&self) -> bool {
        !self.superseded_by.is_empty()
    }
}

/// Whether `find <word>` returns the tenet: the word, lowercased, is inside its id, a keyword, its
/// `what` or its `because`.
pub fn carries(t: &Tenet<'_>, needle: &str) -> bool {
    t.id.contains(needle)
        || t.keywords.iter().any(|k| k.to_lowercase().contains(needle))
        || t.what.to_lowercase().contains(needle)
        || t.because.to_lowercase().contains(needle)
}

/// The tenets `related <id>` shows beside `id` through `cites` and its tenet links in `related`, either direction: what
/// it cites or links, and what cites or links it. Each once, `id` itself never.
pub fn joined_to<'a>(all: &[Tenet<'a>], id: &str) -> Vec<&'a str> {
    let mut out: Vec<&'a str> = Vec::new();
    for t in all {
        let outgoing = t.id == id;
        let incoming = t.cites.contains(&id) || t.links.contains(&id);
        if outgoing {
            for n in t.cites.iter().chain(&t.links) {
                if *n != id && !out.contains(n) {
                    out.push(n);
                }
            }
        }
        if incoming && t.id != id && !out.contains(&t.id) {
            out.push(t.id);
        }
    }
    out
}

/// The entries of a list field, or nothing where it is absent or blank.
fn list<'a>(reg: &'a RegistryView, q: &str, field: &str) -> Vec<&'a str> {
    reg.field(q, field)
        .map(|v| {
            v.split(JOIN)
                .map(str::trim)
                .filter(|e| !e.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// The slugs a relation field holds that point into one namespace: an item written
/// `<namespace>::<slug>` is a link to that row, and an item of another namespace or of more segments
/// (a citation) is not one.
fn linked<'a>(reg: &'a RegistryView, q: &str, field: &str, namespace: &str) -> Vec<&'a str> {
    list(reg, q, field)
        .into_iter()
        .filter_map(|item| {
            let (ns, row) = item.split_once("::")?;
            (ns == namespace && !row.contains("::")).then_some(row)
        })
        .collect()
}

/// A scalar field, trimmed, or the empty string where it is absent.
fn text<'a>(reg: &'a RegistryView, q: &str, field: &str) -> &'a str {
    reg.field(q, field).map_or("", str::trim)
}

/// The slug half of `namespace::slug`.
fn slug(qualified: &str) -> &str {
    qualified.rsplit("::").next().unwrap_or(qualified)
}

/// Every tenet, in id order.
pub fn tenets(reg: &RegistryView) -> Vec<Tenet<'_>> {
    reg.rows_in("tenet")
        .iter()
        .map(|q| {
            Tenet {
                id:            slug(q),
                standing:      Standing::of(text(reg, q, "standing")),
                stamped_by:    text(reg, q, "stamped_by"),
                stamp:         text(reg, q, "stamp"),
                subject:       reg
                    .field(q, "subject")
                    .map(str::trim)
                    .filter(|s| !s.is_empty()),
                superseded_by: list(reg, q, "superseded_by"),
                cites:         list(reg, q, "cites"),
                links:         linked(reg, q, "related", "tenet"),
                keywords:      list(reg, q, "keywords"),
                rules_out:     list(reg, q, "rules_out"),
                what:          text(reg, q, "what"),
                because:       text(reg, q, "because"),
            }
        })
        .collect()
}

/// Every subject's id and sentence, in id order.
pub fn subjects(reg: &RegistryView) -> Vec<(&str, &str)> {
    reg.rows_in("subject")
        .iter()
        .map(|q| (slug(q), text(reg, q, "what")))
        .collect()
}

/// Each superseded tenet with the tenets that bind in its place, a replacement that is itself superseded
/// followed to the end of its chain, each heir once and a chain that loops dropped. A tenet that binds is
/// not a key.
pub fn heirs(all: &[Tenet<'_>]) -> BTreeMap<String, Vec<String>> {
    fn walk<'a>(all: &[Tenet<'a>], id: &'a str, seen: &mut Vec<&'a str>, out: &mut Vec<String>) {
        if seen.contains(&id) {
            return;
        }
        seen.push(id);
        match all.iter().find(|t| t.id == id) {
            Some(t) if t.replaced() => {
                for next in &t.superseded_by {
                    walk(all, next, seen, out);
                }
            },
            _ => {
                if !out.iter().any(|o| o == id) {
                    out.push(id.to_string());
                }
            },
        }
    }
    all.iter()
        .filter(|t| t.replaced())
        .map(|t| {
            let mut out = Vec::new();
            let mut seen = vec![t.id];
            for next in &t.superseded_by {
                walk(all, next, &mut seen, &mut out);
            }
            (t.id.to_string(), out)
        })
        .collect()
}
