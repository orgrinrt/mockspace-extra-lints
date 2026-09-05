//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Rulings recording a human's authority with no words of theirs behind them.
//!
//! `says` is required and `quote` is not, which inverts the trust order: a row
//! can carry somebody's restatement of the person it claims to record, pass
//! every schema check, and be mechanically indistinguishable from one that was
//! invented. One corpus's first port landed four of exactly that shape, and one
//! of them governed when anything became canon at all, its only record being an
//! agent's sentence reporting which option was taken.
//!
//! # A tool rather than a lint
//!
//! Carried across as a lint at first, which was wrong, and the finder's own
//! words say why: not an error in the row, because sometimes the corpus
//! genuinely holds no verbatim and the row is the best available record of a
//! real call. What the finding says is that the hole is in the corpus and
//! somebody should know where. That is the contract's `NoFailingCase`: an
//! inventory with no pass line. The evidence it was already one is that the
//! check guarding it pinned six row names rather than asserting empty, and a
//! pinned list of names is the invented threshold the contract calls worse than
//! no gate, because people defend numbers.
//!
//! # What generalising it across two corpora changed
//!
//! It was two tools. One corpus audited this under this name; another audited
//! the same property under a different one, and neither cited the other.
//!
//! **The exclusion is the part that had to stop being assumed.** Where a
//! ratification can be recorded as reached by experts rather than by the person
//! whose words are at issue, such a row never passed through them: the experts
//! propose, a coordinator gates, and the row carries that judgement instead, so
//! there is no verbatim to have lost and reporting it would report the
//! mechanism. That is true of the corpus the exclusion came from and is
//! **not a property of the concept**: a corpus declaring no such field makes no
//! such distinction, and every one of its rulings is in scope.
//!
//! So the field is consulted where it exists and its absence excludes nothing.
//! Assuming otherwise would silently shrink the population on every corpus that
//! does not declare it, which reads exactly like a clean report.
//!
//! **What stays out by construction.** A proposal carries no `quote`, because
//! there are no words but the panel's and `says` holds them, so reading that
//! namespace would report the namespace rather than a hole.

use mockspace_lint_rules::tool::{ArgSpec, NotALint, Outcome, Tool, ToolContext, ToolReport};

/// The namespace whose rows claim a human's authority.
///
/// Shared by every corpus this has been run against. A corpus that names it
/// differently gets an inconclusive report rather than a clean one, which is
/// the honest answer: the tool examined nothing.
const RULING: &str = "ruling";

/// The field recording who ratified, where a corpus records that at all.
const RATIFIED_BY: &str = "ratified_by";

/// The ratification route that never passed through the person in question.
const NOT_THEIRS: &str = "experts";

pub struct RulingsWithNoVerbatim;

impl Tool for RulingsWithNoVerbatim {
    fn name(&self) -> &'static str {
        "rulings-with-no-verbatim"
    }

    fn description(&self) -> &'static str {
        "rulings resting on somebody's restatement rather than on the words themselves"
    }

    fn not_a_lint(&self) -> NotALint {
        NotALint::NoFailingCase
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[ArgSpec {
            name:        "slug",
            required:    false,
            description: "report one ruling in full, by its slug, before quoting it",
        }]
    }

    fn help(&self) -> &'static str {
        "With no argument: every `ruling` row that carries no `quote`, so what \
         stands behind it is somebody's restatement rather \
         than the words themselves. A row here is not a defect. Sometimes the \
         corpus genuinely holds no verbatim and the row is the best available \
         record of a real call, and the reason for that sits in `note` where a \
         reader has to go and read it.\n\n\
         What the list is for is knowing where the holes are. Two repairs close \
         one: quote the source, or write into `note` that the corpus holds no \
         verbatim and what it holds instead. Which of the two applies is a \
         judgement about what the corpus holds, which is why nothing here \
         gates.\n\n\
         Where the corpus records a `ratified_by` of `experts`, that row is out \
         of scope: the experts propose and a coordinator gates, so there is no \
         verbatim to have lost. A corpus that declares no such field makes no \
         such distinction and every ruling in it is in scope."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let rows = ctx.registry.rows_in(RULING);
        if rows.is_empty() {
            return ToolReport::inconclusive(
                "no `ruling` rows are declared, so this says nothing about whether any of \
                 them rests on the words it claims to record",
            );
        }
        match ctx.args.first().copied() {
            Some(slug) => one(ctx, rows, slug),
            None => all(ctx, rows),
        }
    }
}

/// Whether the row carries the words themselves.
///
/// One question, about one field, and nothing else folded into it. The
/// exclusion below is a separate question with a separate answer, and a reader
/// who wants to know whether there is a quote is asking this one.
#[must_use]
pub fn carries_verbatim(ctx: &ToolContext<'_>, q: &str) -> bool {
    ctx.registry
        .field(q, "quote")
        .is_some_and(|v| !v.trim().is_empty())
}

/// Whether the row is outside what this tool asks about at all.
///
/// A ruling stamped by converging experts was never resting on somebody's
/// restatement of op, because it does not rest on op. So it is not a hole, and
/// it is also not a row that carries his words: those are two different things
/// and this answers only the first.
///
/// The lookup is a lookup rather than an assumption: a corpus not declaring the
/// field returns `None` and excludes nothing, which is what makes this the same
/// question on a corpus that never had the concept.
#[must_use]
pub fn outside_the_question(ctx: &ToolContext<'_>, q: &str) -> bool {
    ctx.registry.field(q, RATIFIED_BY) == Some(NOT_THEIRS)
}

/// Whether a row rests on somebody's restatement rather than on the words.
///
/// The report's population: in scope, and carrying no quote.
#[must_use]
pub fn has_no_verbatim(ctx: &ToolContext<'_>, q: &str) -> bool {
    !outside_the_question(ctx, q) && !carries_verbatim(ctx, q)
}

fn all(ctx: &ToolContext<'_>, rows: &[String]) -> ToolReport {
    let holes: Vec<&String> = rows.iter().filter(|q| has_no_verbatim(ctx, q)).collect();
    let total = rows.len();
    if holes.is_empty() {
        return ToolReport::reported(
            format!("every one of the {total} rulings carries the words behind it."),
            total,
        );
    }
    let mut s = format!(
        "{} of {total} rulings rest on somebody's restatement.\n\n",
        holes.len()
    );
    for q in &holes {
        let slug = q.rsplit("::").next().unwrap_or(q);
        let reason = ctx
            .registry
            .field(q, "note")
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(|n| n.chars().take(90).collect::<String>());
        match reason {
            Some(n) => s.push_str(&format!("  {slug}\n      note: {n}\n")),
            None => s.push_str(&format!("  {slug}\n      no note either\n")),
        }
    }
    s.push_str(
        "\nA row here is not a defect and nothing fails. It says the hole is in the corpus \
         and where to look. Two repairs close one: quote the source, or write into `note` \
         that the corpus holds no verbatim and what it holds instead. The ones already \
         carrying a note are the ones somebody has looked at.\n",
    );
    ToolReport {
        outcome: Outcome::Clean {
            examined: total,
        },
        output:  s,
    }
}

fn one(ctx: &ToolContext<'_>, rows: &[String], key: &str) -> ToolReport {
    let Some(q) = rows
        .iter()
        .find(|q| *q == key || q.rsplit("::").next() == Some(key))
    else {
        return ToolReport::inconclusive(format!(
            "no `ruling` row matches `{key}`, so this is a statement about the spelling \
             rather than about the canon. `rulings-with-no-verbatim` with no argument lists \
             every slug."
        ));
    };
    let mut s = format!("{}\n", q.rsplit("::").next().unwrap_or(q));
    // Two lines, because they are two facts and one of them used to stand in
    // for both: a row stamped by experts was printed as carrying the words
    // whether or not it had a quote, and the empty `quote` field was visible
    // two lines below saying otherwise. This view is for reading a row before
    // quoting it, which is exactly when a wrong yes costs something.
    s.push_str(&format!(
        "\n  carries the words themselves: {}\n",
        if carries_verbatim(ctx, q) { "yes" } else { "no" }
    ));
    if outside_the_question(ctx, q) {
        s.push_str(
            "  and it is not asked to: `ratified_by` is `experts`, so the row rests on their\n  \
             converging rather than on op, and it is left out of the report.\n",
        );
    }
    // Written as a filter rather than a let-chain: this pack is edition 2021
    // and the corpus it was ported from is 2024.
    for field in ["rung", RATIFIED_BY, "says", "quote", "note", "provenance"] {
        if let Some(v) = ctx
            .registry
            .field(q, field)
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            s.push_str(&format!("\n  {field}:\n    {v}\n"));
        }
    }
    ToolReport {
        outcome: Outcome::Clean {
            examined: 1,
        },
        output:  s,
    }
}
