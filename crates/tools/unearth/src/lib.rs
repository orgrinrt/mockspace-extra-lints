//! What the research already says about a subject, which no other tool can see.
//!
//! Every other tool here reads the registry, which is a small part of what a
//! project has written. The rest is the research tree: the argument, the
//! evidence and the panels' own conclusions. A tool asked whether a subject has
//! been settled that reads only the registry answers from the index rather than
//! from the book, answers confidently, and is believed.
//!
//! A subject is easily reported absent from a corpus that specifies it in full,
//! because the corpus calls it by other words than the question does, and an
//! empty `already-said` answer then licenses asking a question the research has
//! settled.
//!
//! **The registry is the expansion, not the answer.** A question first finds the
//! rows that carry it, takes their `keywords`, and asks the corpus in those
//! words as well as its own. That is what the keyword field was added for and it
//! is the one thing a plain grep structurally cannot do.
//!
//! **A tool because it takes a question**, and because a subject the research
//! does not carry is an ordinary and often correct state.

use std::collections::{BTreeMap, BTreeSet};

use corpus::day::today;
use corpus::{Query, Source};
use mockspace::tool::{ArgSpec, ConfigKey, NotALint, Purpose, Tool, ToolContext, ToolParams, ToolReport};
use muisti::repo::{Query as AskQuery, RepoIndex};

mod render;

use render::{render_corpus, render_question, render_registry, render_verdict};

/// Fields that are citations rather than prose. A hit in one says a document or a
/// row was named, not that a subject was discussed: `provenance` names the note a
/// row came from, and `cites`, `superseded_by` and `subject` name other rows.
///
/// **They are excluded from the text search and read for something else.** A
/// provenance entry names the file a row came from, so the set of them is the
/// set of research this registry has actually absorbed, and the difference
/// between that set and what a search returns is the transcription nobody has
/// done. That difference is the whole output of this tool.
const NOT_PROSE: &[&str] = &["provenance", "cites", "superseded_by", "subject", "related"];

/// How many documents a report shows before it stops being read.
///
/// The confidence cut in the ranker does most of the work; this is the backstop
/// for a question so broad that a fifth of the corpus clears it. A reader given
/// sixty files has been handed the problem back.
const SHOWN: usize = 5;

/// How many registry keywords may join the question.
///
/// A cap rather than a filter on quality, because quality is not decidable here:
/// a keyword is a word somebody thought worth searching for, and the tool cannot
/// tell which of a row's fifteen bears on this question. What it can do is prefer
/// the rows that carry most of the question and stop at a number where the
/// ranking still discriminates.
const EXPANSION: usize = 24;

/// The tool, holding where the project keeps the maintainer's captured words.
#[derive(Default)]
pub struct Unearth {
    maintainer: corpus::Maintainer,
}

impl Tool for Unearth {
    fn name(&self) -> &'static str {
        "unearth"
    }

    fn description(&self) -> &'static str {
        "what the research corpus says about a subject, ranked, with the registry's own keywords \
         folded into the question"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Check(NotALint::TakesAQuestion)
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[
            ArgSpec {
                name:        "subject",
                required:    true,
                description: "the subject to look for, in whatever words come to hand",
            },
            ArgSpec {
                name:        "uncited",
                required:    false,
                description: "as the last word: show only documents no row has ever cited",
            },
        ]
    }

    fn config_keys(&self) -> &[ConfigKey] {
        corpus::maintainer::KEYS
    }

    fn configure(&mut self, params: &ToolParams) {
        self.maintainer.configure(params);
    }

    fn help(&self) -> &'static str {
        "Run this before concluding that anything is missing. It reads the whole \
         research tree, expands the question through the keywords of every \
         registry row that carries it, and ranks what it finds. The line naming \
         terms that matched nothing is what separates a real absence from a \
         question asked in the wrong words."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        run_with(ctx, &|| {
            corpus::repo::ready(ctx.mock_dir, ctx.repo_root, Some(&self.maintainer))
        })
    }
}

/// The run, with the index made only once a question has been found to ask, so a call with no
/// subject pays no refresh and a test hands over an index of planted files.
fn run_with(ctx: &ToolContext<'_>, make: &dyn Fn() -> corpus::repo::Ready) -> ToolReport {
    {
        // A trailing `uncited` is a mode rather than a word of the question.
        // Taken off the end rather than as a flag, because the argument surface
        // here is positional and a subject genuinely ending in that word is not
        // a question anybody asks.
        let mut args: Vec<&str> = ctx.args.to_vec();
        let only_uncited = args.last().is_some_and(|a| *a == "uncited");
        if only_uncited {
            args.pop();
        }
        let subject = args.join(" ");
        if subject.trim().is_empty() {
            return ToolReport::inconclusive(if only_uncited {
                "only `uncited` was given, which is the mode rather than the subject, so \
                 nothing was searched"
            } else {
                "no subject given, so nothing was searched"
            });
        }

        answer(ctx, &make(), &subject, only_uncited)
    }
}

/// The question asked of an index already made, so a test hands it one over planted files.
fn answer(
    ctx: &ToolContext<'_>,
    ready: &corpus::repo::Ready,
    subject: &str,
    only_uncited: bool,
) -> ToolReport {
    {
        // Pass one: the registry, both as an answer and as the source of the
        // words the index will be asked in. How rare a keyword is in the
        // documents is asked of the index, because the alternative was an
        // alphabetical tie-break that decided the whole expansion.
        let mut query = Query::new(subject);
        let (rows, keywords, binding) = registry_pass(ctx, &ready.index, &query);
        for k in &keywords {
            query.push(k.clone(), Source::Keyword);
        }
        let want = if only_uncited { SHOWN * 4 } else { SHOWN };
        let mut found = ready.index.search(&AskQuery {
            text:   subject,
            expand: &keywords,
            phrase: false,
            limit:  want,
            today:  today(),
        });
        found.caveats.splice(0 .. 0, ready.notes.iter().cloned());
        if found.indexed == 0 {
            return ToolReport::inconclusive(format!(
                "no documents are indexed under {}, so this establishes nothing about the \
                 corpus. {}",
                ctx.mock_dir.display(),
                found.caveats.join(" ")
            ));
        }
        // A question of nothing but common words expands into no terms at all,
        // so no document is opened and the silence that follows is about the
        // question rather than about the corpus. It is reported rather than
        // answered, because the verdict at the foot of this file reads a short
        // dead-term line as evidence that an absence is real, and an unasked
        // question produces no dead-term line whatever. `unearth "what does it
        // do with all of this"` searched nothing over four thousand documents
        // and came back with the most confident absence this tool can print.
        if query.terms.is_empty() {
            let why = if query.dropped.is_empty() {
                "nothing in it is long enough to search".to_string()
            } else {
                format!(
                    "every word of it is too common to carry a subject: {}",
                    query.dropped.join(", ")
                )
            };
            return ToolReport::inconclusive(format!(
                "`{subject}` was never asked: {why}. The {} indexed documents were not read \
                 and nothing here says whether they carry it. Ask again in a word the subject \
                 itself would use.",
                found.indexed
            ));
        }
        // Whether the research carries the subject at all, taken before the
        // mode's filter runs. `uncited` narrows what is shown and not what is
        // there, and the verdict is about what is there.
        let carried = !found.hits.is_empty();

        let cited = cited_files(ctx);
        // In this mode the ranking is taken four deep and then cut here, so
        // `filtered` is the hits dropped for being cited, which is what the
        // mode is for.
        let mut filtered = 0;
        if only_uncited {
            let before = found.hits.len();
            found.hits.retain(|h| !cited.contains_key(&h.path));
            filtered = before - found.hits.len();
            found.hits.truncate(SHOWN);
        }
        let canon_day = canon_changed(ctx, &ready.index, &binding);
        let mut out = String::new();
        render_question(&mut out, &query, &keywords, &rows);
        render_registry(&mut out, &marked(ctx, &rows));
        render_corpus(&mut out, &found, &cited, filtered, canon_day);
        render_verdict(&mut out, &rows, &found, &cited, carried);

        ToolReport::reported(out, found.indexed)
    }
}

/// The day the canon last changed on this subject: the latest commit to any file that holds a
/// row binding it. A research note older than this was written before the canon last spoke.
fn canon_changed(ctx: &ToolContext<'_>, index: &RepoIndex, binding: &[String]) -> Option<i64> {
    binding
        .iter()
        .filter_map(|q| index.row(q).ok().flatten())
        .map(|r| r.path)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter_map(|file| {
            index.changed_day(&corpus::repo::repo_path(ctx.mock_dir, ctx.repo_root, &file))
        })
        .max()
}

/// Each row as the report names it, a superseded tenet followed by what supersedes it.
///
/// A superseded tenet stays whole in the canon so its replacement is read with it in
/// view. Listed bare beside the rows that bind, it reads as one of them.
fn marked(ctx: &ToolContext<'_>, rows: &[String]) -> Vec<String> {
    rows.iter()
        .map(|q| {
            match ctx.registry.field(q, "superseded_by").map(str::trim) {
                Some(s) if !s.is_empty() => {
                    format!("{q}  superseded by {}", s.trim_matches(['[', ']', '"']))
                },
                _ => q.clone(),
            }
        })
        .collect()
}

/// Rows carrying the subject, and every keyword those rows declare.
///
/// The keywords come from the rows that matched, not from the whole registry. A
/// question about licensing should not be expanded with the vocabulary of the
/// boot chain, and expanding from everything is how a search stops
/// discriminating.
fn registry_pass(
    ctx: &ToolContext<'_>,
    index: &RepoIndex,
    probe: &Query,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    // The same expansion the corpus gets, applied to rows. Matching the raw
    // phrase as a substring was the first version and it failed on the one
    // question this tool was built for: no row contains the string "two
    // surfaces data store", so nothing matched, nothing was harvested, and the
    // corpus search ran with no registry vocabulary at all. A row is a document
    // like any other and is matched like one.
    let mut scored: Vec<(usize, String, Vec<String>)> = Vec::new();

    let namespaces: Vec<String> = ctx.registry.namespaces().map(str::to_string).collect();
    for ns in &namespaces {
        for q in ctx.registry.rows_in(ns) {
            let Some(row) = ctx.registry.row(q) else {
                continue;
            };
            let mut prose = q.replace('_', " ");
            let mut declared: Vec<String> = Vec::new();
            for (name, value) in row {
                if NOT_PROSE.contains(&name.as_str()) {
                    continue;
                }
                if name == "keywords" {
                    // A `string[]` arrives as one rendered value, so both
                    // separators are split and the quoting stripped.
                    for k in value.split(['\n', ',']) {
                        let k = k.trim().trim_matches(['"', '[', ']', ' ']);
                        if k.len() > 2 {
                            declared.push(k.to_lowercase());
                        }
                    }
                }
                prose.push(' ');
                prose.push_str(value);
            }
            let prose = prose.to_lowercase();
            let hits = probe
                .terms
                .iter()
                .filter(|t| !Query::find(&t.stem, &prose).is_empty())
                .count();
            if hits > 0 {
                scored.push((hits, q.clone(), declared));
            }
        }
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let rows: Vec<String> = scored.iter().map(|(_, q, _)| q.clone()).collect();

    // Keywords come only from the rows that carry most of the question. A row
    // matching one common word of it is not about the subject, and harvesting
    // from every such row is how a search stops discriminating: on this
    // registry, taking keywords from every partial match produced ninety terms
    // and a ranking indistinguishable from a word count.
    let best = scored.first().map_or(0, |(h, ..)| *h);
    let floor = best.saturating_sub(1).max(1);
    let binding: Vec<String> = scored
        .iter()
        .filter(|(h, ..)| *h >= floor)
        .map(|(_, q, _)| q.clone())
        .collect();
    // Ranked by how much of the question their own row carries, then capped.
    // Uncapped, one broad row's keyword list put `cars`, `guitars` and `kitchen
    // appliances` into a search about a data store, and ninety terms is a
    // ranking that no longer discriminates.
    let mut ranked: Vec<(usize, String)> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (hits, _, declared) in &scored {
        if *hits < floor {
            continue;
        }
        for k in declared {
            if seen.insert(k.clone()) {
                ranked.push((*hits, k.clone()));
            }
        }
    }
    // How many documents carry a keyword, asked of the same index the ranking
    // will use. A phrase is not a key there, so it is measured on its rarest
    // word: `mere aggregation` is as rare as `aggregation` at worst, and taking
    // the rarest is the closest single-key answer available without scanning.
    let df = |k: &str| {
        corpus::terms::words(k)
            .iter()
            .map(|w| index.doc_frequency(w).unwrap_or(0))
            .min()
            .unwrap_or(0)
    };
    (rows, rank_keywords(ranked, df, EXPANSION), binding)
}

/// Which of the harvested keywords are worth a slot in the question.
///
/// **Ordered by the row's own hits first, then by how much a keyword
/// discriminates.** The row score stays the primary key because a keyword from a
/// row carrying two words of the question is about the subject, while a rare word
/// from a row that barely matched is a coincidence.
///
/// The tie-break is what this fixes. It was alphabetical, and on a one-word
/// subject every matching row ties at one hit, so the tie-break decided the whole
/// expansion and the cap fell wherever the alphabet ran out. A licensing question
/// searched `aggregate`, `bundling` and `copyleft`, and never searched `gpl`,
/// `licence`, `mpl` or `spdx`, which sit on the same rows and are the words the
/// subject is actually argued in.
///
/// **Rarest first among the terms the corpus carries, and terms it carries
/// nowhere go last.** Rarity is the discriminating power the ranker already
/// weighs, so this picks the keywords that will decide a placement rather than
/// the ones that will match everything. Sorting purely by rarity would fill every
/// slot with terms present in no document, which searches for nothing.
fn rank_keywords(
    mut candidates: Vec<(usize, String)>,
    df: impl Fn(&str) -> usize,
    cap: usize,
) -> Vec<String> {
    let mut ranked: Vec<(usize, usize, String)> = candidates
        .drain(..)
        .map(|(hits, k)| {
            let d = df(&k);
            // A term in no document is dead: last, and ordered among its own
            // kind by the alphabet so a run is reproducible.
            (hits, if d == 0 { usize::MAX } else { d }, k)
        })
        .collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    ranked.truncate(cap);
    ranked.into_iter().map(|(_, _, k)| k).collect()
}

/// Every research file the registry cites, and how many rows cite it.
///
/// **The join that makes this tool answer a question rather than list files.** A
/// document the registry already cites has been read by somebody who wrote a row
/// from it; one it has never cited is material this canon has not absorbed, and
/// telling those apart is the difference between a search and a worklist.
///
/// A citation is `root::segment::segment::#anchor`, and the roots are declared in
/// `mockspace.toml` rather than handed to a tool, so both shapes are tried and
/// whichever names a real document wins. That mirrors what `provenance-sweep`
/// does with the same problem, and for the same reason: a second copy of the
/// root table here would drift from the first with nothing reporting it.
fn cited_files(ctx: &ToolContext<'_>) -> BTreeMap<String, usize> {
    let mut cited: BTreeMap<String, usize> = BTreeMap::new();
    for ns in ctx.registry.namespaces().collect::<Vec<_>>() {
        for q in ctx.registry.rows_in(ns) {
            let Some(row) = ctx.registry.row(q) else {
                continue;
            };
            for (name, value) in row {
                if !NOT_PROSE.contains(&name.as_str()) {
                    continue;
                }
                for entry in value.split(['\n', ',']) {
                    if let Some(path) =
                        corpus::corpus::citation_path(entry, |p| ctx.mock_dir.join(p).is_file())
                    {
                        *cited.entry(path).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    cited
}

mockspace::lint_pack! {
    tools: [Unearth::default()],
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_hits;

#[cfg(test)]
mod tests_corpus;
