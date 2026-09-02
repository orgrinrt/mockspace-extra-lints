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

use std::collections::{BTreeMap, BTreeSet};

use mockspace_lint_rules::tool::{ArgSpec, NotALint, Outcome, Tool, ToolContext, ToolReport};
use mockspace_lint_rules::RegistryView;

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
        if rung(reg, q) != RATIFIED {
            continue;
        }
        for named in list(reg, q, RATIFIES) {
            out.entry(named.to_string()).or_default().push(q.clone());
        }
    }
    out
}

/// Whether any row anywhere carries the field naming this namespace.
///
/// Reported rather than refused, and the difference is the whole of what this
/// is for. A corpus recording the relation under another name and a corpus
/// where nothing has answered anything yet produce the identical report, and
/// the second is the ordinary early state of a canon being written, so refusing
/// it would refuse the case the tool exists to serve.
///
/// The first draft here did refuse it, as an inconclusive verdict, on the
/// reasoning that a clean report cannot tell the two apart. That reasoning is
/// right and the remedy was wrong: the two are told apart by the schema, which
/// a tool is handed no parsed copy of, so nothing the registry holds can decide
/// it. What is left is to say so in the report and let a reader decide, which
/// costs a line and claims nothing.
#[must_use]
pub fn anything_carries(reg: &RegistryView, field: &str) -> bool {
    reg.namespaces()
        .flat_map(|ns| reg.rows_in(ns))
        .any(|q| reg.field(q, field).is_some_and(|v| !v.trim().is_empty()))
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
            let (tier, by) = match edge {
                Edge::Ruling => {
                    let r = rung(reg, q);
                    (tier_of_rung(r), format!("{q}   (rung = {r})"))
                },
                Edge::Proposal => match stamped.get(slug(q)) {
                    Some(by) => (
                        Reach::Ratified,
                        format!("{q}   (stamped by {})", by.join(", ")),
                    ),
                    None => (Reach::Proposed, q.clone()),
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

pub struct Coverage;

impl Tool for Coverage {
    fn name(&self) -> &'static str {
        "coverage"
    }

    fn description(&self) -> &'static str {
        "what reaches each row of a demand namespace, by the rung that reaches it"
    }

    fn not_a_lint(&self) -> NotALint {
        NotALint::NoFailingCase
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[
            ArgSpec {
                name: "namespace",
                required: true,
                description: "the demand namespace to measure, whichever this corpus calls it",
            },
            ArgSpec {
                name: "slug",
                required: false,
                description: "report one row of it in full rather than all of them",
            },
        ]
    }

    fn help(&self) -> &'static str {
        "With a namespace: every row of it, by the tier the typed edges and the \
         rung put it at, with the rows that got it there. With a slug after it: \
         that row alone, in full.\n\n\
         The tier is the authority of what reaches it, never the namespace the \
         row sits in. `ratified` is the one tier that is met: a ruling at that \
         rung governs, and so does a proposal such a ruling stamped through \
         `ratifies`, which is what a stamp is for. `in_force` is enforced \
         without having gone through convergence. `stated` is direction and an \
         ack rather than a ruling. `proposed` is a proposal nobody has stamped, \
         which is proposed rather than met. `unsettled` is a ruling at a rung \
         that settles nothing, or one whose rung could not be read. \
         `route-closed` means only a retirement names it: a way to it was tried \
         and is known not to work, which is not the same as nobody having \
         looked.\n\n\
         A rung vocabulary belongs to the corpus, so a tier no rung here spells \
         simply holds nobody, and a namespace this corpus does not declare \
         contributes nothing. A ruling's rung is printed beside it and a stamped \
         proposal names the ruling that stamped it, so a `ratified` line reached \
         through the stamp can be checked rather than taken.\n\n\
         A row named from a namespace whose authority cannot be read is printed \
         under it and sets no tier, because tiering it would invent an authority \
         nobody declared. Preconditions are reported beside the tiers and never \
         folded into them: a precondition is a dependency somebody established, \
         so it leaves a row further from met rather than nearer, and a row with \
         four of them and no answer is the worst-placed one here rather than the \
         best-attended.\n\n\
         Nothing here fails. An unanswered row is the state of unfinished work \
         rather than a defect, and gating on a count would invent a deadline \
         nobody set."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let Some(&demand) = ctx.args.first() else {
            // Unreachable through the engine, which refuses a missing required
            // argument before `run`. Answered anyway rather than indexed into,
            // because a direct caller is a caller.
            return ToolReport::inconclusive(
                "no namespace was named, so this examined nothing. Name the namespace \
                 holding the demand side.",
            );
        };
        let rows = ctx.registry.rows_in(demand);
        if rows.is_empty() {
            return ToolReport::inconclusive(format!(
                "no `{demand}` rows are declared, so there is no demand side to measure. \
                 A namespace with no rows and a namespace nothing answers are the same \
                 empty output and the opposite meaning."
            ));
        }
        match ctx.args.get(1).copied() {
            Some(key) => one(ctx.registry, demand, rows, key),
            None => all(ctx.registry, demand, rows),
        }
    }
}

fn all(reg: &RegistryView, demand: &str, _rows: &[String]) -> ToolReport {
    let reached = reach(reg, demand);
    let others = also_named_by(reg, demand);
    let pre = preconditions(reg, demand);
    let counts = tally(reg, demand);
    let total = reached.len();

    let mut s = format!("{total} `{demand}` rows.\n\n");
    for tier in TIERS {
        s.push_str(&format!(
            "  {:<13} {}\n",
            tier.word(),
            counts.get(tier.word()).copied().unwrap_or(0)
        ));
    }
    s.push_str(
        "\n`ratified` is the only tier that is met. The rest are degrees of not yet, \
         ordered by\nhow far each has got, and a ruling's rung is printed beside it.\n",
    );

    // The tally above reads strongest first, as a ladder from met downward. The
    // body reads the other way, so a reader looking for work finds it at the
    // top. Both orders carry meaning and they are deliberately opposite, which
    // is why the heading says which way round this one is.
    s.push_str(&format!("\nBy {demand}, weakest first:\n\n"));
    let mut ordered: Vec<(&String, &(Reach, Vec<String>))> = reached.iter().collect();
    ordered.sort_by(|a, b| b.1.0.cmp(&a.1.0).then_with(|| a.0.cmp(b.0)));
    for (id, (tier, by)) in ordered {
        let deps = pre.get(id).map_or(0, Vec::len);
        let mark = match deps {
            0 => String::new(),
            1 => "   (1 precondition against it)".to_string(),
            n => format!("   ({n} preconditions against it)"),
        };
        s.push_str(&format!("  {:<13} {id}{mark}\n", tier.word()));
        for who in by {
            s.push_str(&format!("                  {who}\n"));
        }
        for who in others.get(id).map(Vec::as_slice).unwrap_or(&[]) {
            s.push_str(&format!(
                "                  {who}, from a namespace this cannot tier, so it sets none\n"
            ));
        }
    }

    let closed: Vec<&String> = reached
        .iter()
        .filter(|(_, (tier, _))| *tier == Reach::RouteClosed)
        .map(|(id, _)| id)
        .collect();
    if !closed.is_empty() {
        s.push_str(&format!(
            "\n{} row(s) are named only by a retirement: {closed:?}. The row is open and \
             one way to it is known not to work, which is not the same as nobody having \
             looked, and reads identically on a flat list.\n",
            closed.len()
        ));
    }

    if !anything_carries(reg, demand) {
        s.push_str(&format!(
            "\nNo row in any namespace carries a `{demand}` field, so no edge was read at \
             all. Either nothing has answered anything yet, which is an ordinary early \
             state, or this corpus records the relation under another name, in which case \
             every line above is about a relation this did not look for.\n"
        ));
    }

    let stuck: Vec<&String> = reached
        .iter()
        .filter(|(_, (tier, _))| !tier.answered())
        .filter(|(id, _)| pre.get(*id).is_some_and(|on| !on.is_empty()))
        .map(|(id, _)| id)
        .collect();
    if !stuck.is_empty() {
        s.push_str(&format!(
            "\n{} row(s) are answered by nothing and carry an established precondition: \
             {stuck:?}. Each is further from met than a row nobody has looked at, rather \
             than nearer.\n",
            stuck.len()
        ));
    }

    ToolReport {
        outcome: Outcome::Clean { examined: total },
        output: s,
    }
}

fn one(reg: &RegistryView, demand: &str, _rows: &[String], wanted: &str) -> ToolReport {
    let reached = reach(reg, demand);
    let Some((tier, by)) = reached.get(wanted) else {
        return ToolReport::inconclusive(format!(
            "no `{demand}` row matches `{wanted}`, so this is a statement about the \
             spelling rather than about the corpus. `coverage {demand}` with no slug \
             lists every one."
        ));
    };
    let pre = preconditions(reg, demand);
    let others = also_named_by(reg, demand);
    let mut s = format!("{wanted}\n\n  tier: {}\n", tier.word());
    let q = format!("{demand}::{wanted}");
    // Written as a filter rather than a let-chain: this pack is edition 2021
    // and the corpus it was ported from is 2024.
    for field in ["what", "says", "why", "note"] {
        if let Some(v) = reg
            .field(&q, field)
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            s.push_str(&format!("\n  {field}:\n    {v}\n"));
        }
    }
    s.push('\n');
    match by.len() {
        0 => s.push_str("  Nothing names it.\n"),
        _ => {
            s.push_str("  Named by:\n");
            for who in by {
                s.push_str(&format!("    {who}\n"));
            }
        },
    }
    if let Some(on) = others.get(wanted).filter(|on| !on.is_empty()) {
        s.push_str(
            "\n  Also named from a namespace this cannot tier, so these set no tier:\n",
        );
        for who in on {
            s.push_str(&format!("    {who} ({})\n", namespace_of(who)));
        }
    }
    if let Some(on) = pre.get(wanted).filter(|on| !on.is_empty()) {
        s.push_str(&format!(
            "\n  {} established precondition(s), which leave it further from met \
             rather than nearer:\n",
            on.len()
        ));
        for who in on {
            s.push_str(&format!("    {who}\n"));
        }
    }
    ToolReport {
        outcome: Outcome::Clean { examined: 1 },
        output: s,
    }
}
