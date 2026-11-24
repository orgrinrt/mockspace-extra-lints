//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `mock search`: the one search over the repository, its documents and its registry rows, narrowed
//! by filters that compose.
//!
//! Every file of the checkout is in one index: the research, the rounds, the registry, the
//! maintainer's captured words the repository holds, the sketches, the source, and the
//! repository's own commits. A question brings an index that exists current first, over what
//! moved, and says what that read; it never builds one that does not exist, but starts the refresh
//! in the background and says so.
//!
//! **The registry is the expansion, not only an answer.** A question first finds the rows that
//! carry it, takes their `keywords`, and asks the documents in those words as well as its own. A
//! subject is easily reported absent from a corpus that specifies it in full, because the corpus
//! calls it by other words than the question does.
//!
//! **Every narrowing is a filter**: by class, by path, by what changed in a window of days, the
//! maintainer's own words, a phrase, by meaning, by registry namespace, subject or link, and the
//! documents no row cites; and newest first is a sort a flag asks for. The repository's own commits
//! are a class of hit beside its files, `history`, dated by their commit. A window or `--newest` with
//! no words lists what the filters keep, newest first.

use corpus::day::today;
use corpus::repo::Opened;
use corpus::{Query, Source};
use mockspace::tool::{
    ArgSpec,
    ConfigKey,
    NotALint,
    Purpose,
    Tool,
    ToolContext,
    ToolParams,
    ToolReport,
};
use muisti::repo::{Filter, Query as AskQuery, RepoIndex, Words};

mod filters;
mod registry;
mod render;
mod rows;
mod spec;
mod status;

use registry::{canon_changed, cited_files, marked, registry_pass};
use render::{render_corpus, render_question, render_registry, render_verdict};
pub use spec::{Mode, Spec};

/// The tool, holding where the project keeps the maintainer's captured words.
#[derive(Default)]
pub struct Search {
    maintainer: corpus::Maintainer,
}

impl Tool for Search {
    fn name(&self) -> &'static str {
        "search"
    }

    fn description(&self) -> &'static str {
        "the one search over the repository's documents and registry rows, narrowed by filters"
    }

    fn purpose(&self) -> Purpose {
        // Not `TakesAQuestion`: `--status`, `--refresh` and a search by row filters alone ask no words, and
        // the answer is the output in every case.
        Purpose::Check(NotALint::NoFailingCase)
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[ArgSpec {
            name:        "question",
            required:    false,
            description: "the words to look for, and any of the filters: `--class <c,..>` (human, \
                          canon, design, research, sketch, source, history: a commit), `--under \
                          <path>`, `--since <YYYY-MM-DD>`, `--until <YYYY-MM-DD>` and `--days <n>`, \
                          what changed in the window, files and commits alike, `--newest`, the \
                          last changed first, `--spoken`, `--phrase`, \
                          `--meaning`, `--uncited`, `--namespace <ns,..>`, `--subject <s>`, \
                          `--linked <ns::id>`, `--rows`, `--limit <n>`; `--status` says what the \
                          index holds and `--refresh` brings it up to date",
        }]
    }

    fn config_keys(&self) -> &[ConfigKey] {
        corpus::maintainer::KEYS
    }

    fn configure(&mut self, params: &ToolParams) {
        self.maintainer.configure(params);
    }

    fn value_flags(&self) -> &[&'static str] {
        spec::VALUE_FLAGS
    }

    fn help(&self) -> &'static str {
        "Run this before concluding that anything is missing, and before asking anybody. It reads \
         the repository's one index, research, rounds, registry, captured words, sketches and \
         source, expands the question through the keywords of every registry row that carries \
         it, and ranks what it finds. The line naming the words that matched nothing is what \
         separates a real absence from a question asked in the wrong words."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let spec = match Spec::parse(ctx.args) {
            Ok(s) => s,
            Err(e) => return ToolReport::inconclusive(e),
        };
        if spec.mode == Mode::Refresh {
            return status::refresh(ctx, &self.maintainer);
        }
        let opened = match corpus::repo::index_path(ctx.repo_root) {
            Ok(path) => {
                opened_for(
                    &spec,
                    &path,
                    ctx,
                    Some(&self.maintainer),
                    &corpus::repo::background,
                )
            },
            Err(_) => {
                corpus::repo::open(
                    ctx.mock_dir,
                    ctx.repo_root,
                    Some(&self.maintainer),
                    &corpus::repo::background,
                )
            },
        };
        answer(ctx, &spec, opened)
    }
}

/// The index at `path` as the search's mode asks for it. A question brings an index that exists
/// current first, over what moved, and never builds one that does not: that is started in the
/// background with `start` and said. `--status` only reads.
#[must_use]
pub fn opened_for(
    spec: &Spec,
    path: &std::path::Path,
    ctx: &ToolContext<'_>,
    maintainer: Option<&corpus::Maintainer>,
    start: &dyn Fn(&std::path::Path),
) -> Opened {
    match spec.mode {
        Mode::Status => corpus::repo::open_at(path, ctx.mock_dir, ctx.repo_root, maintainer, start),
        _ => corpus::repo::current_at(path, ctx.mock_dir, ctx.repo_root, maintainer, start),
    }
}

/// The search answered from an index already opened, so a test hands it one over planted files.
#[must_use]
pub fn answer(ctx: &ToolContext<'_>, spec: &Spec, opened: Opened) -> ToolReport {
    if spec.mode == Mode::Status {
        return status::report(ctx, &opened);
    }
    let listing = spec.question.trim().is_empty() && (spec.windowed() || spec.newest);
    if spec.question.trim().is_empty() && !spec.narrows_rows() && !listing {
        return ToolReport::inconclusive(
            "no question, no window and no row filter given, so nothing was searched",
        );
    }
    let Some(index) = &opened.index else {
        return ToolReport::inconclusive(opened.notes.join(" "));
    };
    let built = match filters::build(spec, index, ctx) {
        Ok(f) => f,
        Err(e) => return ToolReport::inconclusive(e),
    };
    let mut out = String::new();
    for n in &opened.notes {
        out.push_str(&format!("NOTE: {n}\n"));
    }
    if !opened.notes.is_empty() {
        out.push('\n');
    }
    if spec.rows || (spec.narrows_rows() && spec.question.trim().is_empty()) {
        return rows::answer(out, spec, index, &built);
    }
    if listing {
        return changed(out, ctx, spec, index, &built);
    }
    documents(out, ctx, spec, index, &built)
}

/// Every document the filters keep, newest first, with no words asked: what changed in a window,
/// files and commits alike.
fn changed(
    mut out: String,
    ctx: &ToolContext<'_>,
    spec: &Spec,
    index: &RepoIndex,
    built: &impl Filter,
) -> ToolReport {
    let found = index.newest(built, spec.limit, today());
    if found.indexed == 0 {
        return ToolReport::inconclusive(format!(
            "no documents are indexed for this checkout, so this establishes nothing about it. {}",
            found.caveats.join(" ")
        ));
    }
    render_corpus(&mut out, &found, &cited_files(ctx), 0, None, spec.limit);
    ToolReport::reported(out, found.indexed)
}

/// The documents that answer, with the rows that carry the question and the verdict.
fn documents(
    mut out: String,
    ctx: &ToolContext<'_>,
    spec: &Spec,
    index: &RepoIndex,
    built: &impl Filter,
) -> ToolReport {
    let subject = spec.question.as_str();
    let mut query = Query::new(subject);
    let (rows, keywords, binding) = registry_pass(ctx, index, &query);
    for k in &keywords {
        query.push(k.clone(), Source::Keyword);
    }
    let want = if spec.uncited { spec.limit * 4 } else { spec.limit };
    let ask = AskQuery {
        text:   subject,
        expand: &keywords,
        phrase: false,
        limit:  want,
        today:  today(),
    };
    let mut found = index.search_filtered(&ask, built, None::<&mut Words>);
    if found.indexed == 0 {
        return ToolReport::inconclusive(format!(
            "no documents are indexed for this checkout, so this establishes nothing about it. {}",
            found.caveats.join(" ")
        ));
    }
    // A question of nothing but common words expands into no terms at all, so no document is
    // opened and the silence that follows is about the question rather than the repository.
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
            "`{subject}` was never asked: {why}. The {} indexed documents were not read and \
             nothing here says whether they carry it. Ask again in a word the subject itself \
             would use.",
            found.indexed
        ));
    }
    // Whether the repository carries the subject at all, taken before `--uncited` drops the
    // cited documents: that filter narrows what is shown and not what is there.
    let carried = !found.hits.is_empty();
    let cited = cited_files(ctx);
    let mut filtered = 0;
    if spec.uncited {
        let before = found.hits.len();
        found.hits.retain(|h| !cited.contains_key(&h.path));
        filtered = before - found.hits.len();
        found.hits.truncate(spec.limit);
    }
    let canon_day = canon_changed(index, &binding);
    render_question(&mut out, &query, &keywords, &rows);
    render_registry(&mut out, &marked(ctx, &rows));
    render_corpus(&mut out, &found, &cited, filtered, canon_day, spec.limit);
    render_verdict(&mut out, &rows, &found, &cited, carried);
    ToolReport::reported(out, found.indexed)
}

mockspace::lint_pack! {
    tools: [Search::default()],
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_history;
