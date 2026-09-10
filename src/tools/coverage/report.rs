//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What the tool prints, over what the walk in the parent module found.
//!
//! The walk decides tiers and the report decides nothing, so it is kept apart.
//! Every tier and every edge printed here comes out of a map the walk returned.
//! The report reads the registry itself for two things only: whether any row
//! carries the demand field at all, and a row's own prose fields when that one
//! row is printed in full.

use mockspace_lint_rules::RegistryView;
use mockspace_lint_rules::tool::{ArgSpec, NotALint, Outcome, Tool, ToolContext, ToolReport};

use super::{
    RETIRED,
    RETIREMENT,
    Reach,
    TIERS,
    Through,
    also_named_by,
    namespace_of,
    preconditions,
    reach,
    struck,
    struck_demand,
    tally,
};

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
                name:        "namespace",
                required:    true,
                description: "the demand namespace to measure, whichever this corpus calls it",
            },
            ArgSpec {
                name:        "slug",
                required:    false,
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
         folded into them: a precondition names something a row cannot be met \
         without, and the field does not say whether that is an obstacle still \
         standing or a dependency already in place, so it moves no row nearer met \
         and a row with four of them and no answer is still answered by nothing.\n\n\
         A row carrying `retired` has been struck. It sets no tier, stamps \
         nothing and establishes no precondition, and each edge it carried is \
         printed under the row it names, so a row whose only namers were struck \
         does not read as one nobody has looked at. A struck ruling's stamp is \
         printed under each row the proposal it stamped names, and a struck \
         proposal's line names any live ruling still stamping it. A demand row \
         carrying `retired` itself is marked as owed nothing, and is still \
         tallied at the tier its edges reach.\n\n\
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
    let gone = struck(reg, demand);
    let owed_nothing = struck_demand(reg, demand);
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
        let mut mark = match deps {
            0 => String::new(),
            1 => "   (1 precondition against it)".to_string(),
            n => format!("   ({n} preconditions against it)"),
        };
        if let Some(r) = owed_nothing.get(id) {
            mark.push_str(&format!(
                "   (struck by {RETIREMENT}::{r}, so it is owed nothing)"
            ));
        }
        s.push_str(&format!("  {:<13} {id}{mark}\n", tier.word()));
        for who in by {
            s.push_str(&format!("                  {who}\n"));
        }
        for who in others.get(id).map(Vec::as_slice).unwrap_or(&[]) {
            s.push_str(&format!(
                "                  {who}, from a namespace this cannot tier, so it sets no tier\n"
            ));
        }
        for edge in gone.get(id).map(Vec::as_slice).unwrap_or(&[]) {
            s.push_str(&format!("                  {}\n", edge.line(demand)));
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

    // An answer withdrawn, and never a precondition or a stamp: a struck
    // precondition was a dependency rather than something that reached the row,
    // and a withdrawn stamp leaves its proposal reaching the row still.
    let withdrawn: Vec<&String> = reached
        .iter()
        .filter(|(_, (tier, _))| *tier == Reach::Nothing)
        .filter(|(id, _)| !owed_nothing.contains_key(*id))
        .filter(|(id, _)| {
            gone.get(*id).is_some_and(|on| {
                on.iter()
                    .any(|e| matches!(e.through, Through::Answer { .. }))
            })
        })
        .map(|(id, _)| id)
        .collect();
    if !withdrawn.is_empty() {
        s.push_str(&format!(
            "\n{} row(s) reach nothing and were answered by rows since struck: {withdrawn:?}. \
             What reached each was withdrawn rather than never written, and a flat list \
             reads the two identically.\n",
            withdrawn.len()
        ));
    }

    if !owed_nothing.is_empty() {
        let rows: Vec<&String> = owed_nothing.keys().collect();
        s.push_str(&format!(
            "\n{} `{demand}` row(s) carry `{RETIRED}` themselves: {rows:?}. Each is struck \
             and owed nothing, is marked so above, and is still tallied at the tier its \
             edges reach, since what reaches a row is a fact about the corpus either way.\n",
            rows.len()
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
        .filter(|(id, _)| !owed_nothing.contains_key(*id))
        .filter(|(id, _)| pre.get(*id).is_some_and(|on| !on.is_empty()))
        .map(|(id, _)| id)
        .collect();
    if !stuck.is_empty() {
        s.push_str(&format!(
            "\n{} row(s) are answered by nothing and carry an established precondition: \
             {stuck:?}. Each has something established about what it cannot be met \
             without and nothing that meets it, and the field does not say whether what \
             it depends on is in place.\n",
            stuck.len()
        ));
    }

    ToolReport {
        outcome: Outcome::Clean {
            examined: total,
        },
        output:  s,
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
    if let Some(r) = struck_demand(reg, demand).get(wanted) {
        s.push_str(&format!(
            "  struck by {RETIREMENT}::{r}, so it is owed nothing\n"
        ));
    }
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
        0 => s.push_str("  No live row this can tier names it.\n"),
        _ => {
            s.push_str("  Named by:\n");
            for who in by {
                s.push_str(&format!("    {who}\n"));
            }
        },
    }
    if let Some(on) = others.get(wanted).filter(|on| !on.is_empty()) {
        s.push_str("\n  Also named from a namespace this cannot tier, so these set no tier:\n");
        for who in on {
            s.push_str(&format!("    {who} ({})\n", namespace_of(who)));
        }
    }
    if let Some(on) = struck(reg, demand)
        .remove(wanted)
        .filter(|on| !on.is_empty())
    {
        s.push_str("\n  Also named by rows since struck:\n");
        for edge in on {
            s.push_str(&format!("    {}\n", edge.line(demand)));
        }
    }
    if let Some(on) = pre.get(wanted).filter(|on| !on.is_empty()) {
        s.push_str(&format!(
            "\n  {} established precondition(s), which say what it cannot be met \
             without and count nothing toward meeting it:\n",
            on.len()
        ));
        for who in on {
            s.push_str(&format!("    {who}\n"));
        }
    }
    ToolReport {
        outcome: Outcome::Clean {
            examined: 1,
        },
        output:  s,
    }
}
