//! This repository's index, as the search asks it.
//!
//! The index itself is muisti's: its documents, their classes and days, the ranking and the copy
//! folding. What is this crate's is which files feed it, where the index of a checkout is kept, and
//! what a question is told when the index is not there or is behind.
//!
//! A question never builds the index. It opens the index of its checkout read only, says how far
//! behind it is, and where it is absent or behind starts a refresh in the background and answers
//! from what stands. The refresh is the only writer.

use std::path::{Path, PathBuf};

pub use muisti::Error as IndexError;
pub use muisti::repo::RepoIndex as Index;
use muisti::repo::{Behind, RepoIndex, RowSpec};

pub use self::listing::{Listing, REGISTRY_ROOT, class_of, list};
pub use self::place::{background, index_path, sibling_index};
use crate::maintainer::{Maintainer, unread};

mod listing;
mod place;

/// An index opened for a question, how far behind its files it is, and what the reader is owed.
pub struct Opened {
    /// The index, where one is built for this checkout.
    pub index:  Option<RepoIndex>,
    /// How far behind the files the index is; empty where it is current or absent.
    pub behind: Behind,
    /// What is owed to the reader of an answer: an index not built, one behind, a listing that
    /// failed, the maintainer's words left unread. One sentence each.
    pub notes:  Vec<String>,
}

/// The index of the checkout at `repo_root`, opened read only. Where it is absent or behind its
/// files, `start` is handed the repository's root to bring it up to date in the background, and the
/// answer is from the index as it stands.
#[must_use]
pub fn open(
    mock_dir: &Path,
    repo_root: &Path,
    maintainer: Option<&Maintainer>,
    start: &dyn Fn(&Path),
) -> Opened {
    match index_path(repo_root) {
        Ok(path) => open_at(&path, mock_dir, repo_root, maintainer, start),
        Err(e) => {
            Opened {
                index:  None,
                behind: Behind::default(),
                notes:  vec![format!("this checkout has no place for an index: {e}")],
            }
        },
    }
}

/// [`open`] with the index's file given.
#[must_use]
pub fn open_at(
    path: &Path,
    mock_dir: &Path,
    repo_root: &Path,
    maintainer: Option<&Maintainer>,
    start: &dyn Fn(&Path),
) -> Opened {
    let words = maintainer.and_then(|m| m.words_dir(repo_root));
    let mut notes = Vec::new();
    if let Some(m) = maintainer
        && !words.as_deref().is_some_and(Path::is_dir)
    {
        notes.push(unread(m.named(), words.as_deref()));
    }
    let mut index = match RepoIndex::open_reader(path) {
        Ok(i) => i,
        Err(e) => {
            start(repo_root);
            notes.push(format!(
                "this checkout's index is not built ({e}); a refresh was started in the background, \
                 so ask again once it ends"
            ));
            return Opened {
                index: None,
                behind: Behind::default(),
                notes,
            };
        },
    };
    let listing = list(mock_dir, repo_root, words.as_deref());
    notes.extend(listing.notes);
    // Measured against what a refresh would record, so a changed speaker list or row spec is behind.
    index.set_row_spec(Some(registry_spec()));
    index.set_speakers(
        maintainer
            .map_or(&[][..], Maintainer::names)
            .iter()
            .cloned(),
    );
    let behind = match index.behind(&listing.roots, Some(repo_root)) {
        Ok(b) => b,
        Err(e) => {
            notes.push(format!(
                "how far the index is behind could not be read: {e}"
            ));
            Behind::default()
        },
    };
    if !behind.is_empty() {
        start(repo_root);
        notes.push(format!(
            "the index is behind by {} file{} ({} new, {} changed, {} gone) and {} commit{}{}; a \
             refresh was started in the background, and this answer is from the index as it stood",
            behind.len(),
            if behind.len() == 1 { "" } else { "s" },
            behind.missing.len(),
            behind.moved.len(),
            behind.gone.len(),
            behind.commits,
            if behind.commits == 1 { "" } else { "s" },
            behind
                .definition
                .iter()
                .chain(behind.history.iter())
                .map(|d| format!(", and {d}"))
                .collect::<String>()
        ));
    }
    Opened {
        index: Some(index),
        behind,
        notes,
    }
}

/// The index of the checkout at `repo_root` as a question asks it: where it exists it is brought
/// current first over what moved, and the note says what that read; where it does not, nothing is
/// built in the question's path, `start` is handed the root to build it in the background, and the
/// question is told so, as [`open`] tells it.
#[must_use]
pub fn current(
    mock_dir: &Path,
    repo_root: &Path,
    maintainer: Option<&Maintainer>,
    start: &dyn Fn(&Path),
) -> Opened {
    match index_path(repo_root) {
        Ok(path) => current_at(&path, mock_dir, repo_root, maintainer, start),
        Err(_) => open(mock_dir, repo_root, maintainer, start),
    }
}

/// [`current`] with the index's file given.
#[must_use]
pub fn current_at(
    path: &Path,
    mock_dir: &Path,
    repo_root: &Path,
    maintainer: Option<&Maintainer>,
    start: &dyn Fn(&Path),
) -> Opened {
    let refreshed = path
        .is_file()
        .then(|| refresh_at(path, mock_dir, repo_root, maintainer));
    let mut opened = open_at(path, mock_dir, repo_root, maintainer, start);
    match refreshed {
        Some(Ok(Some(done))) => {
            let mut s = format!(
                "the index was brought current first: {} added, {} changed, {} removed, {} \
                 commits read",
                done.added, done.changed, done.removed, done.commits
            );
            for c in &done.caveats {
                s.push_str(&format!("; {c}"));
            }
            opened.notes.insert(0, s);
        },
        Some(Ok(None)) => {
            opened.notes.insert(
                0,
                "another refresh holds the index, so this answer is from the index as it stood"
                    .to_string(),
            );
        },
        Some(Err(e)) => {
            opened
                .notes
                .insert(0, format!("the index could not be brought current: {e}"));
        },
        None => {},
    }
    opened
}

/// Bring the index of the checkout at `repo_root` up to date: a new one starts as a copy of the
/// clone's, and only the files whose token moved are read. `Ok(None)` where another refresh holds
/// the index, which is that refresh's to finish.
///
/// # Errors
/// What kept the index from being opened or refreshed, as one sentence.
pub fn refresh(
    mock_dir: &Path,
    repo_root: &Path,
    maintainer: Option<&Maintainer>,
) -> Result<Option<muisti::repo::Refreshed>, String> {
    let path = index_path(repo_root)?;
    if !path.exists()
        && let Some(sibling) = sibling_index(repo_root).filter(|s| s.is_file())
    {
        // A seed that fails leaves a refresh from nothing, which is slower and as right.
        let _ = RepoIndex::seed(&sibling, &path);
    }
    refresh_at(&path, mock_dir, repo_root, maintainer)
}

/// [`refresh`] with the index's file given.
///
/// # Errors
/// As [`refresh`].
pub fn refresh_at(
    path: &Path,
    mock_dir: &Path,
    repo_root: &Path,
    maintainer: Option<&Maintainer>,
) -> Result<Option<muisti::repo::Refreshed>, String> {
    let mut index = match RepoIndex::open_within(path, std::time::Duration::from_millis(200)) {
        Ok(i) => i,
        Err(IndexError::Busy(_)) => return Ok(None),
        Err(e) => return Err(format!("{}: {e}", path.display())),
    };
    let words = maintainer.and_then(|m| m.words_dir(repo_root));
    index.set_row_spec(Some(registry_spec()));
    index.set_speakers(
        maintainer
            .map_or(&[][..], Maintainer::names)
            .iter()
            .cloned(),
    );
    let listing = list(mock_dir, repo_root, words.as_deref());
    index
        .refresh(&listing.roots, Some(repo_root))
        .map(Some)
        .map_err(|e| format!("the refresh failed and the index stands as it was: {e}"))
}

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

/// A path the index shows for a document, which is the path from the repository's root.
#[must_use]
pub fn repo_path(shown: &str) -> PathBuf {
    PathBuf::from(shown)
}

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
                class:   h.class.label().to_string(),
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

#[cfg(test)]
mod tests;
