//! Two rows stating the same rule in different words.
//!
//! The previous canon's worst failure mode was not a wrong claim, it was two
//! claims about one thing that nobody could tell apart. A later reader cites
//! whichever they found, a later editor changes one of them, and the two drift
//! apart with nothing anywhere saying they were ever the same rule.
//!
//! A registry makes the pair findable rather than fixing it: rows sharing a
//! subject, saying overlapping things, are candidates. The grouping field is a
//! tenet's `subject`.
//!
//! **A tool because every hit needs reading.** Two rows about one subject
//! *should* share vocabulary, and the design deliberately splits a general rule
//! from the arm that applies it. So a high overlap is a question, not a defect,
//! and there is no failing case that should stop a commit.
//!
//! The measure is deliberately crude and stated rather than tuned: the share of
//! one statement's distinctive words that appear in the other. It is a shortlist
//! for a person, and a cleverer score would only make the shortlist look like a
//! verdict.

use std::collections::BTreeSet;

use mockspace::tool::{ArgSpec, NotALint, Purpose, Tool, ToolContext, ToolReport};

/// Words carrying no subject. Not a stop-word list from anywhere: each member was
/// measured as occurring in most statements of one canon of 315 tenets. `same` is
/// in 115 statements, `rather` 86, `already` 76, `only` 77, `each` 88, and none
/// of them says which statement it is. `player` and `world` are as common and
/// stay, because they do.
const COMMON: &[&str] = &[
    "the", "a", "an", "and", "or", "of", "to", "is", "it", "that", "this", "in", "on", "for",
    "with", "as", "by", "at", "from", "not", "no", "never", "always", "what", "which", "than",
    "then", "so", "but", "its", "be", "are", "was", "were", "has", "have", "had", "any", "every",
    "one", "two", "may", "must", "can", "does", "do", "same", "rather", "already", "only",
    "each", "other", "their", "them", "where",
];

/// How much overlap earns a place on the shortlist, as a percentage.
///
/// **Measured, not picked, and the measurement below is the current one.** The
/// numbers this comment carried were taken over 338 same-topic pairs, said the
/// distribution topped out at 50, and concluded that a threshold above 50 could
/// never fire. That was a corpus a tenth of this one and it stopped being true
/// without the comment changing, which is the same defect as a stale count in a
/// report: it read as evidence because it named one.
///
/// Over the current registry, comparing across namespaces rather than within
/// each: **5518 same-topic pairs, 28 at or above 40, and the top is 82%**. So
/// the old ceiling of 50 is wrong by more than thirty points and a threshold
/// picked under it would have been picked against the wrong shape.
///
/// It is a knob on a shortlist rather than a threshold on a verdict, which is
/// why a stale number here was survivable, and the number sits here so a reader
/// can move it against their own corpus rather than inherit one measured on
/// somebody else's. Re-measure before moving it.
///
/// Over one canon of 296 binding tenets: 1987 same-subject
/// pairs with 11 at or above 40, and 54946 pairs across every subject with 51. The
/// eleven read as a rule beside the arm that applies it, which is the question this
/// shortlist is for, so the number is kept.
const THRESHOLD: usize = 40;

/// The fewest distinctive words a statement needs before it joins the pool.
///
/// A share is of the smaller statement, so a short one needs very few shared
/// words to clear the threshold: at four, two words is 50%. **The reviewer that
/// found the namespace-scoped comparison measured the denominator misbehaving
/// below about six**, and this floor is at four, so the band between them is
/// real and unmeasured. It is not raised here because nothing has swept it, and
/// every hit now prints the denominator it was computed over so a reader can see
/// which side of that band a pair sits on rather than being told.
const POOL_FLOOR: usize = 4;

/// How small a denominator has to be before it is worth counting separately in
/// the report. The reviewer's number, carried as what it is: an observation
/// about where the measure gets soft, not a threshold anything gates on.
const SOFT_DENOMINATOR: usize = 6;

pub struct ParallelStatements;

/// The words of `text` that say which statement it is: none of [`COMMON`], none of three letters or
/// fewer, and not the project's own name, which a project's statements name as often as `same`.
fn distinctive(text: &str, project: &str) -> BTreeSet<String> {
    let project = project.to_lowercase();
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 3)
        .filter(|w| !COMMON.contains(w) && *w != project)
        .map(str::to_string)
        .collect()
}

/// The field rows are grouped by: two rows are compared when they name one subject.
const GROUP: &str = "subject";

/// The argument that puts every binding statement in one group, whatever its subject.
///
/// A project's subjects may be a first binning rather than a settled one, so one rule written into two
/// subjects is a pair a per-subject comparison cannot see.
const ACROSS: &str = "across";

/// The fields a row states its content in, the first one present being read. A field no row carries
/// costs nothing.
const STATEMENT: &[&str] = &["what", "says", "does", "use"];

impl Tool for ParallelStatements {
    fn name(&self) -> &'static str {
        "parallel-statements"
    }

    fn description(&self) -> &'static str {
        "binding tenets that may be stating one rule twice, in different words"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Check(NotALint::NoFailingCase)
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[ArgSpec {
            name:        "scope",
            required:    false,
            description: "a subject, to compare within it alone, or `across` to compare every pair \
                          whatever its subject",
        }]
    }

    fn help(&self) -> &'static str {
        "Each hit is a pair and the share of the smaller statement's distinctive words the other \
         also carries. Two rows about one subject are supposed to share vocabulary, so a hit is a \
         question: are these one rule written twice, or a rule and the arm that applies it? A \
         superseded tenet is left out, since what replaced it keeps its intent by design."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let scope = ctx.args.first().copied();
        let across = scope == Some(ACROSS);
        let only = scope.filter(|s| *s != ACROSS);
        let project = registry_lints::project::Project::of(ctx.mock_dir).name;
        let mut out = String::new();
        let mut pairs = 0usize;
        let mut flagged = 0usize;

        // One pool across every namespace. Rows of different namespaces can state one thing, and a
        // comparison inside each namespace cannot see it.
        let namespaces: Vec<String> = ctx.registry.namespaces().map(str::to_string).collect();
        let mut rows: Vec<(String, String, BTreeSet<String>)> = Vec::new();
        let mut silent = 0usize;
        let mut ungrouped = 0usize;
        let mut too_short = 0usize;
        let mut superseded = 0usize;
        for ns in &namespaces {
            for q in ctx.registry.rows_in(ns) {
                let Some(says) = STATEMENT.iter().find_map(|f| ctx.registry.field(q, f)) else {
                    silent += 1;
                    continue;
                };
                // No longer binding, and what replaced it keeps its intent and much of its
                // wording by design, so every supersession would be reported as a pair.
                if ctx
                    .registry
                    .field(q, "superseded_by")
                    .is_some_and(|s| !s.trim_matches(['[', ']', '"', ' ']).is_empty())
                {
                    superseded += 1;
                    continue;
                }
                let group = ctx.registry.field(q, GROUP).unwrap_or("").to_string();
                if only.is_some_and(|o| o != group) {
                    continue;
                }
                let group = if across { ACROSS.to_string() } else { group };
                if group.is_empty() {
                    ungrouped += 1;
                    continue;
                }
                let w = distinctive(says, &project);
                if w.len() >= POOL_FLOOR {
                    rows.push((q.clone(), group, w));
                } else {
                    too_short += 1;
                }
            }
        }

        // Flagged pairs resting on a denominator small enough that the share is coarse. Counted
        // rather than filtered: the pair may still be the one worth reading.
        let mut soft = 0usize;
        for (i, j, share) in same_group_pairs(&rows) {
            pairs += 1;
            if share >= THRESHOLD {
                flagged += 1;
                let (ref qa, _, ref wa) = rows[i];
                let (ref qb, _, ref wb) = rows[j];
                let smaller = wa.len().min(wb.len());
                if smaller < SOFT_DENOMINATOR {
                    soft += 1;
                }
                let mut common: Vec<&String> = wa.intersection(wb).collect();
                common.sort();
                let shared = common.len();
                common.truncate(8);
                out.push_str(&format!(
                    "{qa}\n{qb}\n  {share}%, {shared} of {smaller}: {}\n\n",
                    common
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }

        if pairs == 0 {
            return ToolReport::inconclusive(format!(
                "no two binding rows share a {GROUP} and both say something, so nothing was \
                 compared. A clean report here would be about the corpus's shape rather than about \
                 whether it repeats itself."
            ));
        }

        let grouping = if across {
            "pairs across every subject".to_string()
        } else if let Some(o) = only {
            format!("pairs within the subject `{o}`")
        } else {
            format!("same-{GROUP} pairs")
        };
        out.push_str(&format!(
            "\n{flagged} of {pairs} {grouping} share at least {THRESHOLD}% of the smaller \
             statement's distinctive words.\n"
        ));
        // What was not compared, because a total over a corner reads as a total.
        out.push_str(&format!(
            "  {superseded} superseded rows were left out, no longer binding; {ungrouped} rows state \
             something and carry no {GROUP}, so nothing grouped them; {silent} rows state nothing \
             at all; {too_short} say something in fewer than {POOL_FLOOR} distinctive words, which \
             is the floor to enter the pool.\n"
        ));
        out.push_str(&format!(
            "  {soft} of the {flagged} rest on fewer than {SOFT_DENOMINATOR} distinctive words, \
             where the share is coarse: at {POOL_FLOOR}, the floor to enter the pool at all, two \
             shared words is 50%. Every hit prints its denominator.\n"
        ));
        if ungrouped > 0 && !across {
            out.push_str(&format!(
                "  The rows with no {GROUP} are the eras, the ecotypes and the gear rows. `across` \
                 compares them with everything else.\n"
            ));
        }
        ToolReport::reported(out, pairs)
    }
}

/// Every pair sharing a group, with the share of the smaller statement the
/// other also carries.
///
/// **Over one pool, whatever namespace each row came from.** Compared within each
/// namespace alone, a ruling and a proposal about the same topic would never be
/// compared, which is the commonest way a canon repeats itself: a proposal
/// ratified into a ruling and left standing beside it.
fn same_group_pairs(rows: &[(String, String, BTreeSet<String>)]) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    for i in 0 .. rows.len() {
        for j in (i + 1) .. rows.len() {
            let (_, ref ta, ref wa) = rows[i];
            let (_, ref tb, ref wb) = rows[j];
            if ta != tb {
                continue;
            }
            let shared = wa.intersection(wb).count();
            let smaller = wa.len().min(wb.len());
            out.push((i, j, shared * 100 / smaller));
        }
    }
    out
}

#[cfg(test)]
mod tests;

mockspace::lint_pack! {
    tools: [ParallelStatements],
}
