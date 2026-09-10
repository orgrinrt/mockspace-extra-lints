//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What reaches each row of a demand namespace, and at what authority.
//!
//! A demand namespace is one written from outside the canon, enumerating what
//! the project owes rather than what it has already said. A row nothing reaches
//! is invisible to every check that walks what the canon already covers, which
//! is why the question is asked from this side.
//!
//! # Why one tool rather than one per corpus
//!
//! Two corpora had grown their own answer to this with different names, neither
//! citing the other, and a fix to one was a fix to one. The question is the
//! same wherever it is asked, and the thing the two differ on is which
//! namespace holds the demand, which is an argument. So the namespace is an
//! argument and there is one implementation.
//!
//! # A tool rather than a lint
//!
//! A row nothing reaches is not a defect. It is the state of unfinished work,
//! and most of the population sits there legitimately while a canon is being
//! written. Gating on a count would invent a deadline nobody set, and an
//! invented threshold is worse than no gate, because people defend numbers. So
//! this is `no-failing-case`, and what it does is make the population visible
//! and ordered so the reading happens on purpose.
//!
//! The namespace being required does not make it `takes-a-question`. The
//! contract's own words are that a configured default would be a different
//! check, and here it would not: a corpus with one demand namespace wants that
//! namespace every time. What the argument carries is which corpus this is,
//! not what the person wants to know.
//!
//! # Authority is the rung, and the namespace is not a proxy for it
//!
//! The namespace a row sits in says what kind of claim it is. It does not say
//! whether the claim governs, and whether it governs is the whole of what a
//! coverage tier is about. A ruling carries a rung, and reading the namespace
//! alone reads every rung as met.
//!
//! It is wrong in the other direction too. A proposal is what would be met if
//! a ruling stamped it, and `ratifies` is the stamp, so a proposal a ratified
//! ruling names there carries that ruling's authority. Reading the naming
//! namespace alone files it as proposed forever.
//!
//! # What generalising it across two corpora had to get right
//!
//! Every one of these is a case where assuming reads as working.
//!
//! A rung vocabulary is the corpus's own. One corpus spells four rungs and
//! another spells two, so a tier whose rung nothing spells simply has no
//! population, which is the honest answer rather than a miscount. A rung this
//! cannot read lands at `unsettled` rather than being dropped, because dropping
//! it would report the row as unreached and dropping is the direction nobody
//! checks.
//!
//! A namespace a corpus does not declare contributes nothing, and that is a
//! lookup rather than an assumption: `rows_in` on an absent namespace is empty,
//! so the tier is empty and no arm has to know which corpus it is running on.
//!
//! A namespace this does not recognise is named rather than dropped. The three
//! it tiers are the ones whose authority it can read; a fourth carrying the
//! same edge is real, is somebody's work, and setting a tier from it would be
//! inventing an authority nobody declared. So it is printed under the row and
//! sets nothing, which keeps it visible without letting it count.
//!
//! The field is not searched for. It is named after the namespace, per the
//! convention that a field typed as a namespace carries that namespace's name.
//! A corpus spelling it differently gets a report saying nothing reaches
//! anything, which is exactly what a corpus where nothing has answered anything
//! yet looks like, so the report says when no edge was read at all and names
//! both readings. It does not refuse: the second reading is the ordinary early
//! state of a canon being written, and only the schema separates the two, which
//! a tool is handed no parsed copy of.
//!
//! Preconditions are collected from every namespace rather than from a fixed
//! list of them. A fixed list is a fact about one corpus, and the field is its
//! own evidence: a row carrying `precondition_for` is establishing one whatever
//! namespace it sits in.
//!
//! A row carrying `retired` has been struck, and it is read the same way, from
//! whatever namespace it sits in. A corpus that keeps a struck row whole, so the
//! slugs citing it still land somewhere, would otherwise have the tool count
//! what it withdrew: a struck proposal's edges read as live, and a stamp over
//! one reads as met. So a struck row sets no tier, stamps nothing and
//! establishes no precondition, and each edge it carried is printed under the
//! row it names rather than dropped, because a row whose only namers were
//! struck and a row nobody has looked at print the same `nothing`. A stamp does
//! not revive a struck proposal, and the line for one names every live ruling
//! still stamping it, since that stamp now points at a claim the corpus
//! withdrew. A struck ruling's stamp is printed under each row the proposal it
//! stamped names, which is where the tier it withdrew would have shown.
//!
//! A demand row carrying the field is struck itself and owed nothing. It is
//! still walked and tallied, because what reaches it is a fact about the corpus
//! either way, and the report marks it so it does not read as outstanding work.
//! A corpus spelling the field differently has its struck rows counted as live,
//! which is the flattering direction, so the field's name is a fact this states
//! rather than one it searches for.

use std::collections::{BTreeMap, BTreeSet};

use mockspace_lint_rules::RegistryView;

mod report;
mod struck;

pub use report::{Coverage, anything_carries};
pub use struck::{StruckEdge, Through, struck, struck_demand};

/// The namespace carrying a rung and a stamp.
///
/// Shared by every corpus this has been run against. One that names it
/// differently loses the tiering rather than the report: its rows land under
/// the unrecognised heading, named and counted as reaching nothing, which says
/// plainly that the tool could not read their authority.
const RULING: &str = "ruling";

/// The namespace holding what would be met if a ruling stamped it.
const PROPOSAL: &str = "proposal";

/// The namespace recording a route tried and closed.
const RETIREMENT: &str = "retirement";

/// The one rung at which a ruling governs and at which its stamp is a stamp.
const RATIFIED: &str = "ratified";

/// The field a ruling names its stamped proposals in.
const RATIFIES: &str = "ratifies";

/// The field a row establishing a precondition carries.
const PRECONDITION_FOR: &str = "precondition_for";

/// The field a struck row carries, naming the retirement that struck it.
const RETIRED: &str = "retired";

/// What is printed for a ruling carrying no readable rung.
///
/// `rung` is required in both schemas this has been run against, so this should
/// be unreachable on a loaded registry. It is rendered rather than assumed away
/// because the tier it produces is a weak one, and a reader owed an explanation
/// of why a row landed there is owed the reason rather than a blank.
const NO_RUNG: &str = "(absent)";

/// What kind of row an edge came from.
///
/// A named kind rather than a tier beside the namespace, because a ruling
/// contributes its rung and a proposal contributes whether a ratified ruling
/// stamped it, and neither of those is a property of the namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    Ruling,
    Proposal,
    Retirement,
}

/// The namespaces whose authority this can read, and how.
const EDGES: [(&str, Edge); 3] = [
    (RULING, Edge::Ruling),
    (PROPOSAL, Edge::Proposal),
    (RETIREMENT, Edge::Retirement),
];

/// How far a demand row has got, from the typed edges and the rung.
///
/// The order is the ranking: a later variant is never reported where an earlier
/// one holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reach {
    /// A ruling at `ratified` names it, or a proposal such a ruling stamped
    /// names it. The only tier that is met.
    Ratified,
    /// A ruling at `in_force` names it: enforced independently of convergence,
    /// which is not the ratification route.
    InForce,
    /// A ruling at `stated` names it. Direction, and an ack rather than a
    /// ruling, so it binds without being canon.
    Stated,
    /// A proposal nothing has stamped names it. Proposed rather than met.
    Proposed,
    /// A ruling names it at a rung that settles nothing, or at one this cannot
    /// read at all.
    Unsettled,
    /// Only retirements name it: a route toward it was tried and closed.
    RouteClosed,
    /// Nothing this can tier names it at all.
    Nothing,
}

impl Reach {
    /// The word used in a report.
    ///
    /// Four of the seven are rung values spelled as a schema spells them, so a
    /// reader holding a report can grep `mockspace.toml` for the word and land
    /// on the field description that defines it. Three cannot be grepped for.
    /// `unsettled` stands over every rung that settles nothing plus any rung
    /// this cannot read, which is why it is not spelled after one of them.
    /// `route-closed` stands over the retirement namespace. `nothing` is this
    /// file's own word for absence and no schema word stands behind it.
    #[must_use]
    pub fn word(self) -> &'static str {
        match self {
            Self::Ratified => "ratified",
            Self::InForce => "in_force",
            Self::Stated => "stated",
            Self::Proposed => "proposed",
            Self::Unsettled => "unsettled",
            Self::RouteClosed => "route-closed",
            Self::Nothing => "nothing",
        }
    }

    /// Whether anything constructive reaches it.
    ///
    /// A match over every tier rather than a chain of not-equals, and the
    /// difference is not cosmetic: the not-equals form names the tiers to
    /// exclude, so a tier added later joins the unanswered side silently and a
    /// row that had just gained an answer would be reported as having none.
    #[must_use]
    pub fn answered(self) -> bool {
        match self {
            Self::Ratified | Self::InForce | Self::Stated | Self::Proposed => true,
            Self::Unsettled | Self::RouteClosed | Self::Nothing => false,
        }
    }
}

/// Every tier, strongest first, in one place.
pub const TIERS: [Reach; 7] = [
    Reach::Ratified,
    Reach::InForce,
    Reach::Stated,
    Reach::Proposed,
    Reach::Unsettled,
    Reach::RouteClosed,
    Reach::Nothing,
];

/// The slug half of a `namespace::slug`.
fn slug(qualified: &str) -> &str {
    qualified.rsplit("::").next().unwrap_or(qualified)
}

/// The namespace half of a `namespace::slug`.
fn namespace_of(qualified: &str) -> &str {
    qualified.split_once("::").map_or(qualified, |(ns, _)| ns)
}

/// The entries of a list field.
///
/// The engine joins a `string[]` with `", "` before a tool sees it, so the
/// split is that separator. Every field read here holds slugs, which carry no
/// comma, so nothing is lost.
fn list<'a>(reg: &'a RegistryView, q: &str, field: &str) -> Vec<&'a str> {
    reg.field(q, field)
        .map(|v| {
            v.split(", ")
                .map(str::trim)
                .filter(|e| !e.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// The retirement a row names as having struck it, where it names one.
///
/// A blank value is read as absent, since a field present and empty has named
/// nothing and striking on it would withdraw a claim on no one's word.
fn struck_by<'a>(reg: &'a RegistryView, q: &str) -> Option<&'a str> {
    reg.field(q, RETIRED)
        .map(str::trim)
        .filter(|r| !r.is_empty())
}

/// A ruling's rung as written, or `(absent)`.
fn rung<'a>(reg: &'a RegistryView, q: &str) -> &'a str {
    reg.field(q, "rung")
        .map(str::trim)
        .filter(|r| !r.is_empty())
        .unwrap_or(NO_RUNG)
}

/// The tier a rung puts a ruling at.
///
/// Written as a match over the words rather than a lookup keyed on a corpus's
/// declared value list, because the value list is the corpus's and this has to
/// answer for a corpus that spells fewer of them. A word this does not know
/// lands at `Unsettled`, which is the weak end and the safe direction: it says
/// the row is named by something whose authority could not be read, rather than
/// claiming the row is met.
fn tier_of_rung(r: &str) -> Reach {
    match r {
        RATIFIED => Reach::Ratified,
        "in_force" => Reach::InForce,
        "stated" => Reach::Stated,
        _ => Reach::Unsettled,
    }
}

/// Which proposals a ratified ruling has stamped, and which ruling stamped each.
///
/// Keyed by the proposal's slug, because `ratifies` holds slugs. The rung is
/// checked here rather than taken on trust: a stamp from anything below
/// `ratified` is a defect a gate catches, and a measurement that assumed the
/// gate had run would report the proposal as canon on exactly the row the gate
/// exists to catch.
#[must_use]
pub fn stamps(reg: &RegistryView) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for q in reg.rows_in(RULING) {
        if rung(reg, q) != RATIFIED || struck_by(reg, q).is_some() {
            continue;
        }
        for named in list(reg, q, RATIFIES) {
            out.entry(named.to_string()).or_default().push(q.clone());
        }
    }
    out
}

/// What each demand row has reached, and what got it there.
///
/// The second half of each entry is what the report prints under the tier: the
/// qualified row, plus the rung for a ruling and the stamping ruling for a
/// stamped proposal.
#[must_use]
pub fn reach(reg: &RegistryView, demand: &str) -> BTreeMap<String, (Reach, Vec<String>)> {
    let stamped = stamps(reg);
    let mut out: BTreeMap<String, (Reach, Vec<String>)> = reg
        .rows_in(demand)
        .iter()
        .map(|q| (slug(q).to_string(), (Reach::Nothing, Vec::new())))
        .collect();

    for (ns, edge) in EDGES {
        for q in reg.rows_in(ns) {
            if struck_by(reg, q).is_some() {
                continue; // `struck` names it instead
            }
            let (tier, by) = match edge {
                Edge::Ruling => {
                    let r = rung(reg, q);
                    (tier_of_rung(r), format!("{q}   (rung = {r})"))
                },
                Edge::Proposal => {
                    match stamped.get(slug(q)) {
                        Some(by) => {
                            (
                                Reach::Ratified,
                                format!("{q}   (stamped by {})", by.join(", ")),
                            )
                        },
                        None => (Reach::Proposed, q.clone()),
                    }
                },
                Edge::Retirement => (Reach::RouteClosed, q.clone()),
            };
            for named in list(reg, q, demand) {
                let Some(entry) = out.get_mut(named) else {
                    continue; // a slug naming no demand row is a lint's report
                };
                entry.0 = entry.0.min(tier);
                entry.1.push(by.clone());
            }
        }
    }
    out
}

/// Rows naming a demand row from a namespace whose authority cannot be read.
///
/// Never a tier and never counted as coverage. The three namespaces above are
/// the ones whose authority this understands; a fourth carrying the same edge
/// is somebody's real work, and tiering it would be inventing an authority
/// nobody declared. Dropping it is worse: the edge exists, and a report that
/// says nothing names the row when something does is wrong in the direction a
/// reader cannot see.
#[must_use]
pub fn also_named_by(reg: &RegistryView, demand: &str) -> BTreeMap<String, Vec<String>> {
    let known: BTreeSet<&str> = EDGES.iter().map(|(ns, _)| *ns).collect();
    let mut out: BTreeMap<String, Vec<String>> = reg
        .rows_in(demand)
        .iter()
        .map(|q| (slug(q).to_string(), Vec::new()))
        .collect();
    let namespaces: Vec<&str> = reg.namespaces().collect();
    for ns in namespaces {
        if known.contains(ns) || ns == demand {
            continue;
        }
        for q in reg.rows_in(ns) {
            if struck_by(reg, q).is_some() {
                continue; // `struck` names it instead
            }
            for named in list(reg, q, demand) {
                if let Some(entry) = out.get_mut(named) {
                    entry.push(q.clone());
                }
            }
        }
    }
    out
}

/// Preconditions somebody has established for each demand row.
///
/// Never a tier and never counted as coverage. Collected from every namespace
/// rather than from a named list of them, because the field is its own
/// evidence and a list of source namespaces is a fact about one corpus.
#[must_use]
pub fn preconditions(reg: &RegistryView, demand: &str) -> BTreeMap<String, Vec<String>> {
    let mut out: BTreeMap<String, Vec<String>> = reg
        .rows_in(demand)
        .iter()
        .map(|q| (slug(q).to_string(), Vec::new()))
        .collect();
    let namespaces: Vec<&str> = reg.namespaces().collect();
    for ns in namespaces {
        for q in reg.rows_in(ns) {
            if struck_by(reg, q).is_some() {
                continue; // `struck` names it instead
            }
            for named in list(reg, q, PRECONDITION_FOR) {
                if let Some(entry) = out.get_mut(named) {
                    entry.push(q.clone());
                }
            }
        }
    }
    out
}

/// How many demand rows sit at each tier.
#[must_use]
pub fn tally(reg: &RegistryView, demand: &str) -> BTreeMap<&'static str, usize> {
    let mut out = BTreeMap::new();
    for tier in TIERS {
        out.insert(tier.word(), 0);
    }
    for (_, (tier, _)) in reach(reg, demand) {
        *out.entry(tier.word()).or_insert(0) += 1;
    }
    out
}
