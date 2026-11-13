//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Walking the corpus, and the collection the walk builds.
//!
//! Split from `doc` because that module answers what a document is and this one answers which
//! documents there are.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::doc::Doc;

/// The roots under the mock directory, named rather than discovered.
///
/// A discovery rule walking everything under `mock/` would search the registry and the crates'
/// designs, which the registry tools already read, and report them back as research. What is
/// discovered is which of these exist, and one that does not is reported rather than silently
/// contributing nothing.
pub const ROOTS: &[&str] = &["research", "design_rounds"];

/// The root the maintainer's captured words are shown under. It sits outside the repository, so the
/// path a report prints for one of them starts here rather than at the mock directory.
pub const MAINTAINER_ROOT: &str = "maintainer-words";

/// Directories never descended into, whatever root they sit under.
const SKIP_DIRS: &[&str] = &["target", ".git", "node_modules"];

/// Extensions worth reading. Everything else is skipped and counted.
///
/// Images, tables of numbers and rendered pages are what most of the research tree is by file
/// count, and none of them carries a sentence a question could match.
const KEEP_EXT: &[&str] = &["md", "txt", "toml", "rs", "cjs", "js", "sh", "wgsl", "slang"];

/// The loaded corpus, plus what it skipped and why.
pub struct Corpus {
    pub docs:        Vec<Doc>,
    /// Roots named above that do not exist on disk.
    pub missing:     Vec<String>,
    /// Files found and not read, by extension. A reader deciding whether to trust an empty result
    /// wants this.
    pub skipped:     BTreeMap<String, usize>,
    pub unreadable:  usize,
    /// Whether the maintainer's captured words were read. Their absence is said in every report,
    /// because a search over the research alone reports nothing about what the maintainer said.
    pub words_read:  bool,
    /// The directory named for the maintainer's words, where one was named, read or not.
    pub words_named: Option<PathBuf>,
    /// Captures of the maintainer's words held out because they never name the project asked
    /// about. Words on another project weigh as the maintainer's and are not about this one, so
    /// they are counted rather than searched.
    pub elsewhere:   usize,
    /// The project the captures were scoped to, if any.
    pub about:       Option<String>,
}

impl Corpus {
    /// The research and the rounds under `mock_dir`, without the maintainer's captured words.
    #[must_use]
    pub fn load(mock_dir: &Path) -> Self {
        Self::load_with(mock_dir, None)
    }

    /// The research and the rounds under `mock_dir`, and the maintainer's captured words from
    /// `maintainer_words` where it names a directory.
    #[must_use]
    pub fn load_with(mock_dir: &Path, maintainer_words: Option<&Path>) -> Self {
        Self::load_scoped(mock_dir, maintainer_words, None)
    }

    fn load_scoped(
        mock_dir: &Path,
        maintainer_words: Option<&Path>,
        project: Option<&str>,
    ) -> Self {
        let mut c = Self {
            docs:        Vec::new(),
            missing:     Vec::new(),
            skipped:     BTreeMap::new(),
            unreadable:  0,
            words_read:  false,
            words_named: maintainer_words.map(Path::to_path_buf),
            elsewhere:   0,
            about:       project.map(str::to_string),
        };
        for root in ROOTS {
            let dir = mock_dir.join(root);
            if dir.is_dir() {
                c.walk(&dir, Path::new(root), None);
            } else {
                c.missing.push((*root).to_string());
            }
        }
        if let Some(words) = maintainer_words.filter(|p| p.is_dir()) {
            c.walk(words, Path::new(MAINTAINER_ROOT), project);
            c.words_read = true;
        }
        c.docs.sort_by(|a, b| a.path.cmp(&b.path));
        c
    }

    /// As [`Self::load_with`], reading only the captures of the maintainer's words that name
    /// `project`.
    ///
    /// The captures may cover several projects, and on a word common to several a capture about
    /// another one would outrank this project's own notes at the maintainer's weight. A capture
    /// naming the project anywhere is read whole. A stand-in for a capture saying which project it
    /// is about.
    #[must_use]
    pub fn load_about(mock_dir: &Path, maintainer_words: Option<&Path>, project: &str) -> Self {
        Self::load_scoped(mock_dir, maintainer_words, Some(project))
    }

    /// Every readable file under `dir`, each shown under `shown` joined with its path below `dir`.
    /// Where `about` names a project, a file whose text never names it is counted in [`Self::elsewhere`]
    /// and not kept.
    fn walk(&mut self, dir: &Path, shown: &Path, about: Option<&str>) {
        let mut stack = vec![dir.to_path_buf()];
        while let Some(d) = stack.pop() {
            let Ok(entries) = std::fs::read_dir(&d) else {
                self.unreadable += 1;
                continue;
            };
            for e in entries.flatten() {
                let p = e.path();
                let name = e.file_name().to_string_lossy().to_string();
                if p.is_dir() {
                    if !SKIP_DIRS.contains(&name.as_str()) {
                        stack.push(p);
                    }
                    continue;
                }
                let ext = p
                    .extension()
                    .and_then(|x| x.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if !KEEP_EXT.contains(&ext.as_str()) {
                    *self
                        .skipped
                        .entry(if ext.is_empty() { "(none)".to_string() } else { ext })
                        .or_default() += 1;
                    continue;
                }
                let rel = p.strip_prefix(dir).unwrap_or(&p);
                match Doc::load(&shown.join(rel), &p) {
                    Some(doc) if about.is_some_and(|a| !doc.lower.contains(&a.to_lowercase())) => {
                        self.elsewhere += 1;
                    },
                    Some(doc) => self.docs.push(doc),
                    None => self.unreadable += 1,
                }
            }
        }
    }

    /// How many documents were loaded. A zero here means every finding from this corpus is void, and
    /// the caller says so rather than reporting no hits.
    #[must_use]
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    /// How many words the corpus holds, counted.
    #[must_use]
    pub fn words(&self) -> usize {
        self.docs
            .iter()
            .map(|d| d.lower.split_whitespace().count())
            .sum()
    }

    /// What was not read, as one sentence, or nothing where nothing was missed.
    ///
    /// Every tool holding a corpus prints this. A document count is a claim about what was searched,
    /// and a count over a corpus with a root absent, or with the maintainer's words unread, is a
    /// smaller claim wearing the same words.
    #[must_use]
    pub fn caveat(&self) -> Option<String> {
        let mut parts = Vec::new();
        if !self.missing.is_empty() {
            parts.push(format!(
                "roots absent from disk and therefore not searched: {}",
                self.missing.join(", ")
            ));
        }
        if !self.words_read {
            parts.push(crate::maintainer::unread(self.words_named.as_deref()));
        }
        if self.elsewhere > 0 {
            parts.push(format!(
                "{} of the maintainer's captures name no `{}` and were held out as about another \
                 project",
                self.elsewhere,
                self.about.as_deref().unwrap_or_default()
            ));
        }
        if self.unreadable > 0 {
            parts.push(format!("{} files could not be read", self.unreadable));
        }
        if parts.is_empty() {
            return None;
        }
        Some(format!("  {}.\n", parts.join("; ")))
    }
}

/// Where a provenance citation points, given a way to ask what exists.
///
/// A citation is `root::segment::segment::#anchor` and the roots are declared in
/// `mockspace.toml`, which a tool is handed no parsed copy of. So both shapes
/// are tried and whichever names a real document wins: the root's own directory
/// under `research/`, and `research/` itself, which is what the panel root is.
/// Keeping a second copy of the root table in a tool is what would drift.
///
/// `exists` is passed in rather than hitting the filesystem here, so the
/// resolution is testable without planting a tree, and so a caller that already
/// knows its corpus can answer from that instead of stat-ing thousands of paths.
#[must_use]
pub fn citation_path(entry: &str, exists: impl Fn(&str) -> bool) -> Option<String> {
    let entry = entry.trim().trim_matches(['"', '[', ']', ' ']);
    let parts: Vec<&str> = entry.split("::").collect();
    // Three at minimum: a root, at least one path segment, and an anchor. Fewer
    // is not a citation and is left alone rather than guessed at.
    if parts.len() < 3 {
        return None;
    }
    let root = parts[0];
    let tail = parts[1 .. parts.len() - 1].join("/");
    for base in [format!("research/{root}"), "research".to_string()] {
        let candidate = format!("{base}/{tail}.md");
        if exists(&candidate) {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests;
