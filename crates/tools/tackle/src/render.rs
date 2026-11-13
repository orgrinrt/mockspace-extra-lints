//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Writing the picked subject up for whoever has to act on it.
//!
//! Split from the tool because deciding what to tackle and writing the brief are two jobs.
//! Everything here takes a decision already made and turns it into text, so nothing in this module
//! can change which subject comes back.

use std::collections::BTreeMap;

use mockspace::tool::ToolContext;

use crate::score;

pub(crate) fn render_table(out: &mut String, ranked: &[score::Scored], corpus_read: usize) {
    out.push_str("what the canon owes, weighed\n\n");
    out.push_str("  score  thin unrd movg open isld   rows  sups  open  subject\n");
    for s in ranked {
        out.push_str(&format!(
            "  {:.3}  {:.2} {:.2} {:.2} {:.2} {:.2}   {:4}  {:4}  {:4}  {}{}\n",
            s.score,
            s.signals[0],
            s.signals[1],
            s.signals[2],
            s.signals[3],
            s.signals[4],
            s.raw.rows,
            s.raw.superseded,
            s.raw.gaps,
            s.topic,
            if s.raw.about_work { "  (about the work, never picked)" } else { "" }
        ));
    }
    out.push_str("\n  thin: how little binds.  unrd: research per binding row.\n");
    out.push_str(
        "  movg: share superseded.  open: rows marking something left open.  isld: rows joined to \
         no other tenet.\n",
    );
    if corpus_read == 0 {
        out.push_str(
            "\n  NO RESEARCH WAS READ, so `unrd` is zero everywhere and this ranking is missing the \
             signal it was rebuilt for. Run `mock corpus` to find out why.\n",
        );
    } else {
        out.push_str(&format!(
            "\n  {corpus_read} documents of the corpus were weighed.\n"
        ));
    }
    out.push('\n');
}

/// The namespace relation table, reported rather than ranked.
///
/// It was a second phase, reached only when every topic was healthy, which meant
/// a real hole in the schema waited behind a queue that never emptied. It is a
/// fact about the registry rather than about a subject, so it is printed every
/// time and a reader can act on it whenever they like.
pub(crate) fn render_namespaces(out: &mut String, ctx: &ToolContext<'_>) {
    let namespaces: Vec<String> = ctx.registry.namespaces().map(str::to_string).collect();
    let mut rows_in: BTreeMap<&str, usize> = BTreeMap::new();
    let mut out_edges: BTreeMap<&str, usize> = BTreeMap::new();
    for ns in &namespaces {
        rows_in.insert(ns, ctx.registry.rows_in(ns).len());
        out_edges.entry(ns).or_insert(0);
    }
    for ns in &namespaces {
        for q in ctx.registry.rows_in(ns) {
            for r in ctx.registry.referrers(q) {
                if let Some(from) = r.split_once("::").map(|(n, _)| n)
                    && let Some(c) = out_edges.get_mut(from)
                {
                    *c += 1;
                }
            }
        }
    }
    out.push_str("and whether the parts reference each other\n\n");
    let mut silent = Vec::new();
    for ns in &namespaces {
        let rows = rows_in[ns.as_str()];
        let edges = out_edges[ns.as_str()];
        out.push_str(&format!("  {rows:5} rows  {edges:5} edges out   {ns}\n"));
        if rows > 0 && edges == 0 {
            silent.push(ns.clone());
        }
    }
    if !silent.is_empty() {
        out.push_str(&format!(
            "\n  Nothing leaves {}. Either the namespace declares no relation field, \
             which is a hole in the schema that no row work closes, or it declares one \
             nobody has used, which is row work. The engine computes edges from field \
             types and does not hand tools the types, so read the schema to find out \
             which, and say so when you fix it. A namespace that is a leaf on purpose \
             is also possible and nothing here can tell.\n",
            silent.join(", ")
        ));
    }
    out.push('\n');
}

pub(crate) fn render_brief(
    out: &mut String,
    ctx: &ToolContext<'_>,
    pick: &score::Scored,
    namers: usize,
) {
    let what = ctx
        .registry
        .field(&format!("subject::{}", pick.topic), "what")
        .unwrap_or("no `what` on the subject row");
    let driver = score::dominant(pick);

    out.push_str(&format!(
        "TACKLE NEXT: {}\n\nplaced at {:.3} by `{driver}`, with {} binding of the {namers} rows that \
         name a subject.\n\n{what}\n\n",
        pick.topic, pick.score, pick.raw.rows
    ));

    // The brief differs by what put it here, which is the whole reason the signal is named.
    match driver {
        "unread" => out.push_str(
            "GO AND READ, BEFORE YOU DERIVE ANYTHING.\n\n\
             This is thin because what was said is in the research, the rounds or op's captures and \
             never reached the canon. `mock unearth <subject> uncited` ranks that material and shows \
             what no row cites yet; `mock already-said <term>` says which rows already carry a word.\n\n\
             Where op's words settle something, it becomes a tenet in this subject's file citing them \
             by heading, as the workflow says. Where the research argues something op has not said, \
             it is a question for op, put with what the tools returned.\n\n",
        ),
        "moving" => out.push_str(
            "THIS SUBJECT IS STILL MOVING.\n\n\
             Much of what was written here has been superseded. Read each replacement beside what it \
             replaced with `mock canon related <id>`, check `mock parallel-statements <subject>` for a \
             binding row still saying what a superseded one said, and check with `mock already-said` \
             whether op's latest words settle what is still open.\n\n",
        ),
        "gaps" => out.push_str(
            "THE HOLES ARE ALREADY NAMED.\n\n\
             This subject's rows mark more left open than any other's, so somebody has done the \
             finding already. Read them. A hole the research or op's captures already close is a \
             transcription, found with `mock unearth`; one they do not close is a question for op.\n\n",
        ),
        "islands" => out.push_str(
            "THE ROWS HERE ARE JOINED TO NOTHING.\n\n\
             The subject is written and its tenets neither cite nor are cited by another, so it reads \
             as a pile of answers rather than a design. `mock canon related <id>` and `mock canon \
             citers <id>` show what each touches. A relation goes into a tenet's words first; `cites` \
             follows the words.\n\n",
        ),
        _ => out.push_str(
            "LITTLE BINDS HERE.\n\n\
             Start with `mock unearth <subject>`, which reads the research, the rounds and op's \
             captures rather than the rows, because thin here does not always mean unthought. What op \
             has said becomes tenets citing his words. What nobody has asked him is researched wide \
             first, by category before variant, and then put to him as a question with the options \
             and what the tools returned.\n\n",
        ),
    }

    out.push_str(
        "A tenet rests on op's words, so the work ends in rows citing what he said or in questions \
         for him, never in a row resting on a helper's judgement. A list of what nobody has decided \
         about this subject is not a result; it restates this output.\n",
    );
}
