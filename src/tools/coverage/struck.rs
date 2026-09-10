//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What a strike withdrew, and the demand rows struck themselves.
//!
//! Kept apart from the walk because it is the walk's mirror: every edge the
//! live functions in the parent module skip on a struck row lands here
//! instead, so the two halves are read against each other rather than as one.

use std::collections::{BTreeMap, BTreeSet};

use mockspace_lint_rules::RegistryView;

use super::{
    EDGES,
    PRECONDITION_FOR,
    PROPOSAL,
    RATIFIED,
    RATIFIES,
    RETIREMENT,
    RULING,
    list,
    rung,
    slug,
    stamps,
    struck_by,
};

/// Which field carried an edge a strike withdrew, and what else it bears.
///
/// Three kinds and they withdraw different things, so a report reading them as
/// one would count a withdrawn dependency as a withdrawn answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Through {
    /// The demand field: the struck row answered the demand row. For a struck
    /// proposal, `stamped_by` holds every live ratified ruling still stamping
    /// it, which is a stamp pointing at a claim the corpus withdrew.
    Answer {
        stamped_by: Vec<String>,
    },
    /// `precondition_for`: the struck row was a dependency somebody established.
    Precondition,
    /// `ratifies`, on the named proposal, which names the demand row: the
    /// struck row was a ratified ruling and its stamp is what the proposal's
    /// tier rested on.
    Stamp {
        proposal: String,
    },
}

/// One edge a strike withdrew from a demand row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StruckEdge {
    /// The struck row, qualified.
    pub row:     String,
    /// The retirement the struck row names as having struck it, by slug.
    pub by:      String,
    /// The field that carried the edge.
    pub through: Through,
}

impl StruckEdge {
    /// The line a report prints for it, saying what the strike withdrew.
    #[must_use]
    pub fn line(&self, demand: &str) -> String {
        let (row, by) = (&self.row, &self.by);
        let what = match &self.through {
            Through::Answer {
                stamped_by,
            } if stamped_by.is_empty() => format!("through `{demand}`, so it sets no tier"),
            Through::Answer {
                stamped_by,
            } => {
                format!(
                    "through `{demand}`, so it sets no tier even though {} still stamps it",
                    stamped_by.join(", ")
                )
            },
            Through::Precondition => {
                format!("through `{PRECONDITION_FOR}`, so it establishes nothing")
            },
            Through::Stamp {
                proposal,
            } => format!("through `{RATIFIES}` on {proposal}, so it stamps nothing"),
        };
        format!("{row}   (struck by {RETIREMENT}::{by}, {what})")
    }
}

/// Struck rows' edges onto each demand row.
///
/// Never a tier, never a precondition, never counted as coverage. For the
/// demand field and `precondition_for`, what this collects is what `reach`,
/// `also_named_by` and `preconditions` would have read off these rows had they
/// not been struck: the demand field wherever one of those reads it, and
/// `precondition_for` from every namespace. For `ratifies` it is the stamp a
/// struck ratified ruling put on a proposal, listed under each row that
/// proposal names, which is where `reach` would have printed it as the
/// proposal's stamper. A struck proposal's answer carries every live ruling
/// still stamping it.
#[must_use]
pub fn struck(reg: &RegistryView, demand: &str) -> BTreeMap<String, Vec<StruckEdge>> {
    let live_stamps = stamps(reg);
    let tiered: BTreeSet<&str> = EDGES.iter().map(|(ns, _)| *ns).collect();
    let mut out: BTreeMap<String, Vec<StruckEdge>> = reg
        .rows_in(demand)
        .iter()
        .map(|q| (slug(q).to_string(), Vec::new()))
        .collect();
    let namespaces: Vec<&str> = reg.namespaces().collect();
    for ns in namespaces {
        // `reach` reads the demand field off every namespace it tiers, whatever
        // the demand namespace is, and `also_named_by` reads it off every other
        // namespace but the demand's own. So the demand's own is read only when
        // it is one of the tiered three.
        let answers_read = tiered.contains(ns) || ns != demand;
        for q in reg.rows_in(ns) {
            let Some(by) = struck_by(reg, q) else {
                continue;
            };
            let mut edges: Vec<(&str, Through)> = Vec::new();
            if answers_read {
                let stamped_by = if ns == PROPOSAL {
                    live_stamps.get(slug(q)).cloned().unwrap_or_default()
                } else {
                    Vec::new()
                };
                for named in list(reg, q, demand) {
                    edges.push((named, Through::Answer {
                        stamped_by: stamped_by.clone(),
                    }));
                }
            }
            for named in list(reg, q, PRECONDITION_FOR) {
                edges.push((named, Through::Precondition));
            }
            // Only a ratified ruling's stamp was ever a stamp, as in `stamps`.
            if ns == RULING && rung(reg, q) == RATIFIED {
                for p in list(reg, q, RATIFIES) {
                    let proposal = format!("{PROPOSAL}::{p}");
                    for named in list(reg, &proposal, demand) {
                        edges.push((named, Through::Stamp {
                            proposal: proposal.clone(),
                        }));
                    }
                }
            }
            for (named, through) in edges {
                if let Some(entry) = out.get_mut(named) {
                    entry.push(StruckEdge {
                        row: q.clone(),
                        by: by.to_string(),
                        through,
                    });
                }
            }
        }
    }
    out
}

/// Demand rows carrying `retired` themselves, each with the retirement that
/// struck it.
///
/// A struck demand row is owed nothing. It is still walked and tallied at the
/// tier its edges reach, since what reaches it is a fact about the corpus
/// either way, and this is what lets the report mark it rather than list it as
/// outstanding work.
#[must_use]
pub fn struck_demand(reg: &RegistryView, demand: &str) -> BTreeMap<String, String> {
    reg.rows_in(demand)
        .iter()
        .filter_map(|q| struck_by(reg, q).map(|by| (slug(q).to_string(), by.to_string())))
        .collect()
}
