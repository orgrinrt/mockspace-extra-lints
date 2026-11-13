//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `mock corpus`, and the search over the research, the rounds and the maintainer's captured words
//! that `already-said`, `unearth` and `tackle` link.
//!
//! The registry is a few hundred rows. The research notes, the round transcripts and the
//! maintainer's captured words hold the argument, the evidence and most of the answers, and a tool
//! reading only the rows answers from a fraction of the material, confidently, and is believed. A
//! tool asked whether something was already settled has to say what the research held, or the same
//! question goes back to the maintainer again.
//!
//! # The shape
//!
//! A question expands before it is asked: the phrase, its content words, their forms, and the words
//! the registry already associates with the subject. Then every document is scored on several axes
//! rather than counted, and every axis is reported per hit so a ranking is arguable. See
//! [`score::rank`].
//!
//! # What it does not do
//!
//! It does not read the registry. The caller does that and hands the words over, which keeps this
//! testable against a directory of markdown and keeps the registry's shape out of a scorer that has
//! no business knowing it.

pub mod corpus;
pub mod day;
pub mod doc;
pub mod index;
pub mod maintainer;
pub mod repo;
pub mod score;
pub mod terms;

use std::path::Path;

use mockspace::reading::Reading;

pub use corpus::Corpus;
pub use doc::{Doc, Kind};
pub use index::Index;
pub use maintainer::Maintainer;
pub use score::{Hit, Search, rank};
pub use terms::{Query, Source};

/// The corpus a tool asks: the research and the rounds under `mock_dir`, and those of the
/// maintainer's captured words that name the project, its `project_name`, from the directory
/// `maintainer` names.
#[must_use]
pub fn load(mock_dir: &Path, repo_root: &Path, maintainer: &Maintainer) -> Corpus {
    let words = maintainer.words_dir(repo_root);
    Corpus::load_about(
        mock_dir,
        words.as_deref(),
        &registry_lints::project::Project::of(&Reading::new(repo_root).view(), mock_dir).name,
    )
}

// --- the tool face ---------------------------------------------------------------
//
// One tool, and it is the crate's own subject: what is in the corpus. Everything above is a library
// `already-said`, `unearth` and `tackle` link, and none of them can answer this, because a report
// about the search is not a search.

use mockspace::tool::{ArgSpec, ConfigKey, NotALint, Purpose, Tool, ToolContext, ToolParams, ToolReport};

/// What the corpus holds, so an empty search result can be believed or not.
///
/// The question this answers is the one nobody asks until a tool has already misled them: was the
/// material even read? A search reporting nothing over four documents and one reporting nothing over
/// four hundred are different claims, and the output of a search does not distinguish them.
#[derive(Default)]
pub struct CorpusReport {
    maintainer: Maintainer,
}

impl Tool for CorpusReport {
    fn name(&self) -> &'static str {
        "corpus"
    }

    fn description(&self) -> &'static str {
        "what the research corpus contains, by kind, and what was not read"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Check(NotALint::NoFailingCase)
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[]
    }

    fn config_keys(&self) -> &[ConfigKey] {
        maintainer::KEYS
    }

    fn configure(&mut self, params: &ToolParams) {
        self.maintainer.configure(params);
    }

    fn help(&self) -> &'static str {
        "The inventory `already-said`, `unearth` and `tackle` search. Run it when a search comes \
         back empty and the emptiness matters: a root missing from disk, the maintainer's captured \
         words not read, or an extension nobody thought to read, is the difference between an \
         absence and a blind spot."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let corpus = load(ctx.mock_dir, ctx.repo_root, &self.maintainer);
        report(&corpus, ctx.mock_dir)
    }
}

/// The report over a loaded corpus, apart from where it was loaded from, so it can be tested over a
/// corpus with and without the maintainer's words whatever the machine running the test has.
fn report(corpus: &Corpus, mock_dir: &Path) -> ToolReport {
    if corpus.is_empty() {
        return ToolReport::inconclusive(format!(
            "nothing was read under {}. Every search over this corpus is void until that is \
             explained.",
            mock_dir.display()
        ));
    }

    let mut by_kind: std::collections::BTreeMap<&str, (usize, usize)> =
        std::collections::BTreeMap::new();
    for d in &corpus.docs {
        let e = by_kind.entry(d.kind.label()).or_default();
        e.0 += 1;
        e.1 += d.lower.len();
    }

    let mut out = String::from("WHAT IS THERE\n\n");
    for (kind, (n, bytes)) in &by_kind {
        out.push_str(&format!("  {n:>5} documents  {bytes:>9} bytes   {kind}\n"));
    }
    let total: usize = by_kind.values().map(|(n, _)| n).sum();
    out.push_str(&format!(
        "\n  {total} documents, {} words.\n",
        corpus.words()
    ));

    out.push_str("\nWHAT WAS NOT READ\n\n");
    if corpus.missing.is_empty() {
        out.push_str("  every declared root exists\n");
    } else {
        out.push_str(&format!(
            "  roots absent from disk: {}\n",
            corpus.missing.join(", ")
        ));
    }
    if corpus.words_read {
        out.push_str("  the maintainer's captured words were read\n");
        if corpus.elsewhere > 0 {
            out.push_str(&format!(
                "  {} of the maintainer's captures name no `{}` and were held out as about another \
                 project\n",
                corpus.elsewhere,
                corpus.about.as_deref().unwrap_or_default()
            ));
        }
    } else {
        out.push_str(&format!(
            "  {}\n",
            maintainer::unread(corpus.words_named.as_deref())
        ));
    }
    if corpus.skipped.is_empty() {
        out.push_str("  no files skipped for their extension\n");
    } else {
        // Ranked and capped. A report nobody reads to the end is where a real blind spot hides.
        let mut by_count: Vec<(&String, &usize)> = corpus.skipped.iter().collect();
        by_count.sort_by(|a, b| b.1.cmp(a.1));
        let total: usize = corpus.skipped.values().sum();
        for (ext, n) in by_count.iter().take(6) {
            out.push_str(&format!("  {n:>5} files with extension {ext}\n"));
        }
        if by_count.len() > 6 {
            let shown: usize = by_count.iter().take(6).map(|(_, n)| **n).sum();
            out.push_str(&format!(
                "  {:>5} files across {} other extensions\n",
                total - shown,
                by_count.len() - 6
            ));
        }
    }
    if corpus.unreadable > 0 {
        out.push_str(&format!("  {} could not be read\n", corpus.unreadable));
    }
    ToolReport::reported(out, corpus.len())
}

mockspace::lint_pack! {
    tools: [CorpusReport::default()],
}

// `run` itself. Everything under `doc`, `terms`, `index` and `score` is tested against the pieces,
// and nothing else exercises what `mock corpus` prints.

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use mockspace::tool::Outcome;

    use super::{Corpus, ToolReport, report};

    /// A corpus on disk. Cleared on construction and again on drop, so a run that panics does not
    /// leave the next one reading its files.
    struct Planted(PathBuf);

    impl Planted {
        fn new(name: &str) -> Self {
            let dir =
                std::env::temp_dir().join(format!("corpus-report-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        /// Relative to the mock directory, so a test names the root it means.
        fn file(&self, rel: &str, body: &str) -> &Self {
            let p = self.0.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, body).unwrap();
            self
        }
    }

    impl Drop for Planted {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn run(mock: &Path) -> ToolReport {
        report(&Corpus::load(mock), mock)
    }

    fn run_with(mock: &Path, op: &Path) -> ToolReport {
        report(&Corpus::load_with(mock, Some(op)), mock)
    }

    #[test]
    fn a_corpus_that_read_nothing_is_inconclusive_and_names_the_directory() {
        // The arm that matters most, because the alternative is a clean report over nothing, which
        // reads as "the subject is not discussed".
        let p = Planted::new("empty");
        let r = run(&p.0);
        let Outcome::Inconclusive {
            reason,
        } = &r.outcome
        else {
            panic!("an unread corpus passed as clean: {:?}", r.output);
        };
        assert!(
            reason.contains(&p.0.display().to_string()),
            "the reason says which directory was walked: {reason}"
        );

        // THE CONTROL. One file is the difference between void and thin.
        p.file("research/a.md", "one word");
        assert!(matches!(run(&p.0).outcome, Outcome::Clean { .. }));
    }

    #[test]
    fn the_word_count_is_counted_rather_than_derived_from_the_bytes() {
        let p = Planted::new("words");
        let body = "supercalifragilistic ".repeat(40);
        p.file("research/a.md", &body);
        let out = run(&p.0).output;
        assert!(out.contains("1 documents, 40 words."), "{out}");
    }

    #[test]
    fn an_absent_root_is_named_and_a_complete_set_says_so() {
        let p = Planted::new("roots");
        p.file("research/a.md", "prose");
        let out = run(&p.0).output;
        assert!(
            out.contains("roots absent from disk: design_rounds"),
            "{out}"
        );

        // THE CONTROL. A line printed whatever the state carries nothing.
        p.file("design_rounds/c.md", "prose");
        let out = run(&p.0).output;
        assert!(out.contains("every declared root exists"), "{out}");
        assert!(!out.contains("absent from disk"), "{out}");
    }

    #[test]
    fn the_maintainer_s_words_are_counted_under_their_own_kind_or_said_to_be_unread() {
        let p = Planted::new("maintainer");
        p.file("research/a/NOTES.md", "prose")
            .file("words/202610040900_on-the-project.md", "the maintainer's words");
        let out = run(&p.0).output;
        assert!(
            out.contains("the maintainer's captured words were not read: name their directory"),
            "{out}"
        );
        assert!(!out.contains("   maintainer\n"), "{out}");

        // A directory named and absent is said to be absent, and nothing is counted under the kind.
        let out = run_with(&p.0, &p.0.join("no-such-words")).output;
        assert!(out.contains("no-such-words`, named for them, is not a directory"), "{out}");
        assert!(!out.contains("   maintainer\n"), "{out}");

        // THE CONTROL. Read, they are counted under their kind and the line says so.
        let out = run_with(&p.0, &p.0.join("words")).output;
        assert!(out.contains("the maintainer's captured words were read"), "{out}");
        let words_line = out
            .lines()
            .find(|l| l.ends_with("   maintainer"))
            .unwrap_or_default();
        assert!(words_line.contains("    1 documents"), "{out}");
    }

    #[test]
    fn the_skipped_extension_list_is_capped_at_six_and_the_rest_are_counted() {
        // Two subtractions in one branch, over a list of thirty one-off
        // extensions from fetched source trees. Nobody reads to the end of that
        // list, which is where a real blind spot would sit, so the remainder
        // has to be right.
        let p = Planted::new("skipped");
        p.file("research/a.md", "prose");
        for (ext, n) in [
            ("aaa", 8),
            ("bbb", 7),
            ("ccc", 6),
            ("ddd", 5),
            ("eee", 4),
            ("fff", 3),
            ("ggg", 2),
            ("hhh", 1),
        ] {
            for i in 0 .. n {
                p.file(&format!("research/skip/{ext}{i}.{ext}"), "x");
            }
        }
        let out = run(&p.0).output;
        // Thirty-six skipped, thirty-three across the six shown, so three left
        // across two extensions.
        assert!(
            out.contains("3 files across 2 other extensions"),
            "the remainder and the extension count are both off-by-one hazards: {out}"
        );
        assert!(out.contains("extension aaa"), "{out}");
        assert!(out.contains("extension fff"), "{out}");
        assert!(
            !out.contains("extension ggg"),
            "the seventh is folded into the remainder rather than printed: {out}"
        );
    }
}
