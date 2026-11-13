//! This repository's index, as the query tools ask it.
//!
//! The index itself is muisti's: its documents, their classes and days, the ranking and the copy
//! folding. What is this crate's is which directories feed it, where the canon's rows sit, and what a
//! tool is told when the index could not be had, which is said to every question and never read
//! as an empty answer.

use std::path::Path;

/// The index, under the names the tools that ask it use.
pub use muisti::Error as IndexError;
pub use muisti::repo::RepoIndex as Index;
use muisti::repo::cache::{cache_root, index_file};
use muisti::repo::{DocClass, RepoIndex, Root, RowSpec};

use crate::corpus::{MAINTAINER_ROOT, ROOTS};
use crate::maintainer::{Maintainer, unread};

/// The class a directory under the mock dir is read as: the rounds are design, the rest research.
const fn class_of(shown: &str) -> DocClass {
    // A match over the corpus's own two fixed roots, which are the corpus's vocabulary and not
    // content.
    match shown.as_bytes() {
        b"design_rounds" => DocClass::Design,
        _ => DocClass::Research,
    }
}

/// The directories the index reads: the mock dir's research and rounds, and the maintainer's
/// captured words where `maintainer_words` names a directory that exists.
#[must_use]
pub fn roots(mock_dir: &Path, maintainer_words: Option<&Path>) -> Vec<Root> {
    let mut out: Vec<Root> = ROOTS
        .iter()
        .map(|r| {
            Root {
                dir:         mock_dir.join(r),
                shown:       (*r).to_string(),
                class:       class_of(r),
                only_naming: None,
            }
        })
        .collect();
    let registry = mock_dir.join(REGISTRY_ROOT);
    if registry.is_dir() {
        out.push(Root {
            dir:         registry,
            shown:       REGISTRY_ROOT.to_string(),
            class:       DocClass::Canon,
            only_naming: None,
        });
    }
    if let Some(dir) = maintainer_words.filter(|d| d.is_dir()) {
        out.push(Root {
            dir:         dir.to_path_buf(),
            shown:       MAINTAINER_ROOT.to_string(),
            class:       DocClass::Human,
            // The captures may cover several projects; a capture that never names this one is not read.
            only_naming: Some(registry_lints::project::Project::of(mock_dir).name),
        });
    }
    out
}

/// An index brought up to date, and what could not be done to get it.
pub struct Ready {
    pub index: RepoIndex,
    /// What is owed to the reader of an answer: a cache that could not be opened and was replaced
    /// by one in memory, a refresh that failed and left the index as it stood, the maintainer's
    /// words left unread.
    pub notes: Vec<String>,
}

/// The repository's index, opened in the user cache, or in memory where that cannot be had, and
/// refreshed from the roots. Never fails: a failure is a note, and an index that holds nothing
/// says so through its own report.
///
/// The maintainer's captured words are read from the directory `maintainer` names, and their
/// quoted lines lifted under the names it gives. A tool that does not ask about them passes `None`.
#[must_use]
pub fn ready(mock_dir: &Path, repo_root: &Path, maintainer: Option<&Maintainer>) -> Ready {
    let words = maintainer.and_then(|m| m.words_dir(repo_root));
    let names = maintainer.map_or(&[][..], Maintainer::names);
    ready_at(cache_root().as_deref(), mock_dir, repo_root, words.as_deref(), names)
}

/// [`ready`] with the cache directory given, so a test keeps its index out of the user's.
#[must_use]
pub fn ready_at(
    cache: Option<&Path>,
    mock_dir: &Path,
    repo_root: &Path,
    maintainer_words: Option<&Path>,
    maintainer_names: &[String],
) -> Ready {
    let mut notes = Vec::new();
    if !maintainer_words.is_some_and(Path::is_dir) {
        notes.push(unread(maintainer_words));
    }
    let opened = cache
        .map(|c| index_file(c, repo_root))
        .ok_or_else(|| "there is no user cache directory".to_string())
        .and_then(|file| {
            RepoIndex::open_shared(&file).map_err(|e| format!("{}: {e}", file.db().display()))
        });
    let mut index = match opened {
        Ok(i) => i,
        Err(why) => {
            notes.push(format!("the index is not kept on disk: {why}"));
            match RepoIndex::in_memory() {
                Ok(i) => i,
                Err(e) => {
                    notes.push(format!("the index could not be started: {e}"));
                    return Ready {
                        index: RepoIndex::in_memory().expect("an in-memory index is always made"),
                        notes,
                    };
                },
            }
        },
    };
    index.set_row_spec(Some(registry_spec()));
    index.set_speakers(maintainer_names.iter().cloned());
    if let Err(e) = index.refresh(&roots(mock_dir, maintainer_words), Some(repo_root)) {
        notes.push(format!(
            "the refresh failed and the index stands as it was: {e}"
        ));
    }
    Ready {
        index,
        notes,
    }
}

/// The registry directory under the mock dir, read as canon: its files are documents and its rows
/// and edges are the index's.
pub const REGISTRY_ROOT: &str = "registry";

/// Which row fields hold a bare id, and of which namespace.
///
/// These are this registry's conventions: `cites`, `needs`, `part_of` and `superseded_by` name a row
/// of the same namespace; `ticket` and `tickets` name tickets and `subject` a subject.
#[must_use]
pub fn registry_spec() -> RowSpec {
    RowSpec::new(&["cites", "needs", "part_of", "superseded_by"], &[
        ("ticket", "ticket"),
        ("tickets", "ticket"),
        ("subject", "subject"),
    ])
    .with_vocabulary(&["keywords"])
}

/// A word near the one asked: a form the documents hold, or a synonym the registry's keywords give.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Near {
    pub word:    String,
    /// Whether it comes from a keyword list rather than from the documents' own words.
    pub synonym: bool,
    /// The rows that give a synonym; zero for a form.
    pub rows:    usize,
}

/// The forms and the synonyms of each word of a question, at most `limit` of each kind per word.
#[must_use]
pub fn near_words(index: &RepoIndex, question: &str, limit: usize) -> Vec<Near> {
    let mut out: Vec<Near> = Vec::new();
    for term in muisti::repo::ask::terms_of(question) {
        let _ = index.word_forms(&term, limit, |w| {
            if !out.iter().any(|n| n.word == w) {
                out.push(Near {
                    word:    w,
                    synonym: false,
                    rows:    0,
                });
            }
        });
        let _ = index.synonyms_of(&term, limit, |w, rows| {
            if !out.iter().any(|n| n.word == w) {
                out.push(Near {
                    word: w,
                    synonym: true,
                    rows,
                });
            }
        });
    }
    out
}

/// A path the index shows for a document, as the repository spells it for git.
#[must_use]
pub fn repo_path(mock_dir: &Path, repo_root: &Path, shown: &str) -> String {
    let below = mock_dir.strip_prefix(repo_root).unwrap_or(mock_dir);
    let below = below.to_string_lossy().replace('\\', "/");
    if below.is_empty() { shown.to_string() } else { format!("{below}/{shown}") }
}

#[cfg(test)]
mod tests;

/// A document the index found for a question, as a tool lists it.
#[derive(Clone, Debug)]
pub struct Found {
    pub path:    String,
    pub line:    usize,
    pub class:   String,
    pub excerpt: String,
    /// The index's ranking score, class weight included.
    pub score:   f64,
}

/// The best `limit` documents for a question, in the index's own order.
#[must_use]
pub fn find(index: &RepoIndex, question: &str, limit: usize) -> Vec<Found> {
    let report = index.search(&muisti::repo::Query {
        text: question,
        expand: &[],
        phrase: false,
        limit,
        today: crate::day::today(),
    });
    report
        .hits
        .into_iter()
        .map(|h| {
            Found {
                path:    h.path,
                line:    h.line,
                class:   format!("{:?}", h.class).to_lowercase(),
                excerpt: h.excerpt,
                score:   h.score,
            }
        })
        .collect()
}

/// The question a subject asks of the index: its name as words, then its own keywords.
#[must_use]
pub fn question_of(subject: &str, keywords: &[String]) -> String {
    let mut out = subject.replace('_', " ");
    for k in keywords {
        out.push(' ');
        out.push_str(k);
    }
    out
}

/// Whether a question has any word long and uncommon enough to search. One that has none was not
/// asked, and its silence says nothing of the corpus.
#[must_use]
pub fn askable(question: &str) -> bool {
    !crate::terms::words(question).is_empty()
        && muisti::repo::ask::terms_of(question).next().is_some()
}

/// How a hit was reached, beyond its class and day, for every tool that prints one: whether it is
/// the speaker's own words lifted out of a note, and which keywords of the document the question
/// reached it through where the text itself did not hold the words. Empty for a plain text match in
/// a document's own writing.
#[must_use]
pub fn found_by(h: &muisti::repo::RepoHit) -> String {
    let mut out = String::new();
    if h.quoted {
        out.push_str(", the maintainer's words quoted in this note");
    }
    if !h.text && !h.keywords.is_empty() {
        out.push_str(&format!(", keywords: {}", h.keywords.join(" ")));
    }
    if !h.associated.is_empty() {
        out.push_str(&format!(", tied to: {}", h.associated.join(" ")));
    }
    out
}
