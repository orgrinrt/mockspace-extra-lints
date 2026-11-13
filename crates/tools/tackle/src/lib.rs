//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What to work on next, chosen by weighing every signal the canon carries.
//!
//! Work drifts toward whatever is cheapest to decide, and nothing reports the drift because every
//! unit produces real output. Ported from kamu, where op's objection to ranking on one number was
//! that it should be a weighed complex scoring instead of one or the other, with all things playing
//! into it.
//!
//! Five signals per subject: how little binds in it, how much research sits behind it per row
//! already written, how much of it has been superseded, how many of its rows mark something left
//! open, and how many of its rows neither cite nor are cited by another tenet. [`score`] carries
//! the weights and why each is there.
//!
//! kamu's third signal was the share of a topic an expert proposed rather than op ratified. Every
//! kaski tenet is op's, so that signal has nothing to measure here; the nearest kaski measure is how
//! much of a subject has been superseded, which says it is still moving. kamu's fourth read a `gap`
//! field no kaski row carries; here it reads the words a tenet marks something open with.
//!
//! A tool because it is a ranking with weights nobody has justified. They are judgements, stated in
//! the source and printed in the output.

mod render;
mod score;

use std::collections::BTreeMap;

use mockspace::tool::{ArgSpec, NotALint, Purpose, Tool, ToolContext, ToolReport};

use crate::render::{render_brief, render_namespaces, render_table};

/// Subjects that are about how kaski is built rather than about the game. Scored, printed, never
/// picked, for the reason in [`score::PICKABLE`].
const ABOUT_THE_WORK: &[&str] = &["workflow"];

/// The words a tenet marks something left open with, matched in its `what` and `because` ignoring
/// case. Measured over the canon when this was ported: `not yet` in 7 statements, `left open` 4, `is
/// open` 3, `open question` 2, `stand-in` 2. `later` (22) and `until` (23) are left out, being as
/// often a sequence as a hole. A stand-in for a field saying what a row leaves open.
const OPEN_MARKS: &[&str] = &["not yet", "left open", "is open", "open question", "stand-in"];

/// How many documents of research count toward a subject's corpus mass.
const CORPUS_DEPTH: usize = 10;

/// How many of a subject's own keywords join its corpus question.
const SUBJECT_TERMS: usize = 12;

pub struct Tackle;

impl Tool for Tackle {
    fn name(&self) -> &'static str {
        "tackle"
    }

    fn description(&self) -> &'static str {
        "the subject most owed work, weighed across five signals, with the reason it was picked"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Check(NotALint::NoFailingCase)
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[ArgSpec {
            name:        "next",
            required:    true,
            description: "the only verb; picks one subject and says what it is owed",
        }]
    }

    fn help(&self) -> &'static str {
        "Weighs every subject on how little binds in it, how much research sits unabsorbed behind \
         it, how much of it is superseded, how many of its rows mark something left open, and how \
         many of its rows cite or are cited by nothing. Prints all five per subject, then names one \
         and says which signal placed it, because the brief differs: a thin subject wants research \
         and a question for op, and a subject with unread research wants somebody to go and read it."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        run_with(ctx, &|| {
            corpus::repo::ready(ctx.mock_dir, ctx.repo_root, None)
        })
    }
}

/// The run, with the index made once the registry has been found worth ranking, so a test hands
/// over an index of planted files.
fn run_with(ctx: &ToolContext<'_>, make: &dyn Fn() -> corpus::repo::Ready) -> ToolReport {
    {
        if let Some(first) = ctx.args.first()
            && *first != "next"
        {
            return ToolReport::inconclusive(format!(
                "`{first}` is not a verb here. `mock tackle next` is the whole surface, and a second \
                 verb would be a second opinion about what is thin."
            ));
        }

        let subjects: Vec<String> = ctx
            .registry
            .rows_in("subject")
            .iter()
            .map(|q| q.split_once("::").map_or(q.clone(), |(_, s)| s.to_string()))
            .collect();
        if subjects.is_empty() {
            return ToolReport::inconclusive(
                "no subject rows, so there is nothing to rank and this establishes nothing",
            );
        }

        let (mut raws, vocab, namers, blind) = gather(ctx, &subjects);
        if namers == 0 {
            return ToolReport::inconclusive(
                "no row anywhere names a subject, so every subject reads as equally thin and the \
                 ranking below would be alphabetical noise",
            );
        }

        // The corpus, once, for every subject.
        let ready = make();
        let corpus_read = ready.index.len().unwrap_or(0);
        // Subjects whose name and vocabulary are all words too common to search. Their `unrd` is
        // zero because no document was opened rather than because no research sits behind them.
        let mut unasked: Vec<String> = Vec::new();
        if corpus_read > 0 {
            for (subject, terms) in &vocab {
                let words = corpus::repo::question_of(subject, terms);
                if !corpus::repo::askable(&words) {
                    unasked.push(subject.clone());
                    continue;
                }
                let found = corpus::repo::find(&ready.index, &words, CORPUS_DEPTH);
                if let Some(r) = raws.get_mut(subject) {
                    r.corpus = found.iter().map(|h| h.score).sum();
                }
            }
        }

        let ranked = score::rank(&raws);
        let mut out = String::new();
        render_table(&mut out, &ranked, corpus_read);
        if !unasked.is_empty() {
            out.push_str(&format!(
                "\n  The research was not asked about {}: every word of the subject and its keywords \
                 is too common to search, so `unrd` above is zero for want of a question rather than \
                 for want of research.\n",
                unasked.join(", ")
            ));
        }
        for note in &ready.notes {
            out.push_str(&format!("\n{note}\n"));
        }
        out.push_str(&coverage_line(namers, &blind));
        render_namespaces(&mut out, ctx);

        let Some(pick) = ranked.iter().find(|s| score::PICKABLE(s)) else {
            return ToolReport::inconclusive(
                "every subject here is about the work rather than the game, so there is nothing to \
                 pick and the table above is the whole answer",
            );
        };
        render_brief(&mut out, ctx, pick, namers);
        ToolReport::reported(out, ranked.len())
    }
}

/// Every measurement the registry can supply, plus each subject's own vocabulary.
///
/// One walk over every namespace. A row is attributed to the subject its `subject` field names; a
/// superseded row counts as the subject moving rather than as something written.
fn gather(
    ctx: &ToolContext<'_>,
    subjects: &[String],
) -> (
    BTreeMap<String, score::Raw>,
    BTreeMap<String, Vec<String>>,
    usize,
    BTreeMap<String, usize>,
) {
    let mut raws: BTreeMap<String, score::Raw> = subjects
        .iter()
        .map(|t| {
            (t.clone(), score::Raw {
                about_work: ABOUT_THE_WORK.contains(&t.as_str()),
                ..score::Raw::default()
            })
        })
        .collect();
    let mut vocab: BTreeMap<String, Vec<String>> =
        subjects.iter().map(|t| (t.clone(), Vec::new())).collect();
    let mut namers = 0usize;
    // Per namespace, the rows carrying no `subject`.
    let mut blind: BTreeMap<String, usize> = BTreeMap::new();

    // How many other tenets each tenet is joined to, in either direction: the tenets it cites or is
    // superseded by, and the tenets citing it or superseded by it. The `subject` a tenet names is
    // not a relation between tenets, and every tenet carries one, so counting it would make every
    // row look connected.
    let mut edges: BTreeMap<String, usize> = BTreeMap::new();
    for target in ctx.registry.rows_in("tenet") {
        for source in ctx.registry.referrers(target) {
            if source.starts_with("tenet::") {
                *edges.entry(source.clone()).or_insert(0) += 1;
                *edges.entry(target.clone()).or_insert(0) += 1;
            }
        }
    }

    // The subject rows' own words come first, so a subject with no rows still has a question to
    // ask the corpus.
    for t in subjects {
        let q = format!("subject::{t}");
        let mut terms = Vec::new();
        if let Some(what) = ctx.registry.field(&q, "what") {
            terms.extend(corpus::terms::words(what));
        }
        if let Some(kw) = ctx.registry.field(&q, "keywords") {
            terms.extend(split_keywords(kw));
        }
        vocab.insert(t.clone(), terms);
    }

    for ns in ctx.registry.namespaces().collect::<Vec<_>>() {
        for q in ctx.registry.rows_in(ns) {
            let Some(v) = ctx.registry.field(q, "subject") else {
                // Counted rather than skipped silently: these are the rows every column is blind
                // to. The subject rows themselves are what is named and cannot name themselves.
                if ns != "subject" {
                    *blind.entry(ns.to_string()).or_insert(0) += 1;
                }
                continue;
            };
            let t = v.trim().trim_matches(['"', '[', ']']);
            let Some(r) = raws.get_mut(t) else {
                continue;
            };
            namers += 1;
            if ctx
                .registry
                .field(q, "superseded_by")
                .is_some_and(|s| !s.trim_matches(['"', '[', ']', ' ']).is_empty())
            {
                r.superseded += 1;
                continue;
            }
            r.rows += 1;
            let words = ["what", "because"]
                .iter()
                .filter_map(|f| ctx.registry.field(q, f))
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            if OPEN_MARKS.iter().any(|m| words.contains(m)) {
                r.gaps += 1;
            }
            r.edges += edges.get(q).copied().unwrap_or(0);
            if let (Some(kw), Some(bag)) = (ctx.registry.field(q, "keywords"), vocab.get_mut(t)) {
                bag.extend(split_keywords(kw));
            }
        }
    }

    // Capped and deduplicated. A subject with forty rows carries hundreds of keywords, and a
    // question in hundreds of words matches everything, which would make the corpus signal a
    // measure of subject size rather than of unabsorbed research.
    for terms in vocab.values_mut() {
        let mut seen = std::collections::BTreeSet::new();
        terms.retain(|t| seen.insert(t.clone()));
        terms.truncate(SUBJECT_TERMS);
    }
    (raws, vocab, namers, blind)
}

/// What the ranking could see, and what it could not.
///
/// A subject is attributed to a row through that row's `subject` field, so a
/// namespace declaring no such field is invisible to every signal here. That is
/// a hole in the schema rather than in this tool, and closing it is row work
/// nobody can do from inside a ranking, but reporting the ranking as though it
/// covered the registry is this tool's own fault and is what this fixes.
fn coverage_line(namers: usize, blind: &BTreeMap<String, usize>) -> String {
    let named: Vec<(&String, &usize)> = blind.iter().filter(|(_, n)| **n > 0).collect();
    if named.is_empty() {
        return format!("\n  Every one of the {namers} rows carries a subject and was weighed.\n");
    }
    let unseen: usize = named.iter().map(|(_, n)| **n).sum();
    let where_ = named
        .iter()
        .map(|(ns, n)| format!("{ns} {n}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "\n  Weighed over the {namers} rows that carry a subject. {unseen} more could not be \
         attributed to one and were not weighed at all: {where_}. A row with no subject is \
         invisible to every column above, so a subject whose material sits in those \
         namespaces reads as thinner here than it is.\n"
    )
}

/// A `string[]` field arrives as one rendered value, so both separators split.
fn split_keywords(raw: &str) -> Vec<String> {
    raw.split(['\n', ','])
        .map(|k| k.trim().trim_matches(['"', '[', ']', ' ']).to_lowercase())
        .filter(|k| k.len() > 2)
        .collect()
}

mockspace::lint_pack! {
    tools: [Tackle],
}

#[cfg(test)]
mod tests;
