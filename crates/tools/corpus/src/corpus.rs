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

/// The root op's captured words are shown under. It sits outside the repository, so the path a
/// report prints for one of them starts here rather than at the mock directory.
pub const OP_ROOT: &str = "op-responses";

/// The environment variable naming the directory of op's captured words, where it is not where
/// [`op_words_dir`] would look.
pub const OP_WORDS_VAR: &str = "KASKI_OP_WORDS";

/// Where op's captured words sit relative to a workspace.
const OP_WORDS_IN_WORKSPACE: &str = ".data/op-responses";

/// The workspace repository's name, for the layout where it is cloned beside this one.
const WORKSPACE_REPO: &str = "clause-dev";

/// Directories never descended into, whatever root they sit under.
const SKIP_DIRS: &[&str] = &["target", ".git", "node_modules"];

/// Extensions worth reading. Everything else is skipped and counted.
///
/// Images, tables of numbers and rendered pages are what most of the research tree is by file
/// count, and none of them carries a sentence a question could match.
const KEEP_EXT: &[&str] = &["md", "txt", "toml", "rs", "cjs", "js", "sh", "wgsl", "slang"];

/// The directory of op's captured words, if one can be found.
///
/// A directory named by `named` (the caller reads [`OP_WORDS_VAR`]) wins, and one naming nothing
/// finds nothing rather than falling back, so a mistyped name is a missing root a report names
/// rather than a different directory read in silence. Without a name: `.data/op-responses` in the
/// repository root or any directory above it, which is op's layout, where this repository is a
/// clone inside the workspace; then `clause-dev/.data/op-responses` beside the repository root or any
/// directory above it, which is a helper's, where the two are cloned side by side.
///
/// A stand-in until the tools consolidate into one place for every project and the workspace says
/// where its data is.
#[must_use]
pub fn op_words_dir(repo_root: &Path, named: Option<&str>) -> Option<PathBuf> {
    if let Some(n) = named.filter(|n| !n.trim().is_empty()) {
        let p = PathBuf::from(n);
        return p.is_dir().then_some(p);
    }
    for dir in repo_root.ancestors() {
        let inside = dir.join(OP_WORDS_IN_WORKSPACE);
        if inside.is_dir() {
            return Some(inside);
        }
        let beside = dir.join(WORKSPACE_REPO).join(OP_WORDS_IN_WORKSPACE);
        if beside.is_dir() {
            return Some(beside);
        }
    }
    None
}

/// The loaded corpus, plus what it skipped and why.
pub struct Corpus {
    pub docs:       Vec<Doc>,
    /// Roots named above that do not exist on disk.
    pub missing:    Vec<String>,
    /// Files found and not read, by extension. A reader deciding whether to trust an empty result
    /// wants this.
    pub skipped:    BTreeMap<String, usize>,
    pub unreadable: usize,
    /// Whether op's captured words were read. Their absence is said in every report, because a
    /// search over the research alone reports nothing about what op said.
    pub op_read:    bool,
    /// Captures of op's words held out because they never name the project asked about. His words
    /// on another project weigh as his and are not about this one, so counted rather than searched.
    pub elsewhere:  usize,
    /// The project the captures were scoped to, if any.
    pub about:      Option<String>,
}

impl Corpus {
    /// The research and the rounds under `mock_dir`, without op's captured words.
    #[must_use]
    pub fn load(mock_dir: &Path) -> Self {
        Self::load_with(mock_dir, None)
    }

    /// The research and the rounds under `mock_dir`, and op's captured words from `op_words` where it
    /// names a directory.
    #[must_use]
    pub fn load_with(mock_dir: &Path, op_words: Option<&Path>) -> Self {
        Self::load_scoped(mock_dir, op_words, None)
    }

    fn load_scoped(mock_dir: &Path, op_words: Option<&Path>, project: Option<&str>) -> Self {
        let mut c = Self {
            docs:       Vec::new(),
            missing:    Vec::new(),
            skipped:    BTreeMap::new(),
            unreadable: 0,
            op_read:    false,
            elsewhere:  0,
            about:      project.map(str::to_string),
        };
        for root in ROOTS {
            let dir = mock_dir.join(root);
            if dir.is_dir() {
                c.walk(&dir, Path::new(root), None);
            } else {
                c.missing.push((*root).to_string());
            }
        }
        if let Some(op) = op_words.filter(|p| p.is_dir()) {
            c.walk(op, Path::new(OP_ROOT), project);
            c.op_read = true;
        }
        c.docs.sort_by(|a, b| a.path.cmp(&b.path));
        c
    }

    /// As [`Self::load_with`], reading only the captures of op's words that name `project`.
    ///
    /// The captures are every project's, and on a word common to several a capture about another one
    /// outranks this project's own notes at op's weight. A capture naming the project anywhere is read
    /// whole. A stand-in for a capture saying which project it is about.
    #[must_use]
    pub fn load_about(mock_dir: &Path, op_words: Option<&Path>, project: &str) -> Self {
        Self::load_scoped(mock_dir, op_words, Some(project))
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
    /// and a count over a corpus with a root absent, or with op's words unread, is a smaller claim
    /// wearing the same words.
    #[must_use]
    pub fn caveat(&self) -> Option<String> {
        let mut parts = Vec::new();
        if !self.missing.is_empty() {
            parts.push(format!(
                "roots absent from disk and therefore not searched: {}",
                self.missing.join(", ")
            ));
        }
        if !self.op_read {
            parts.push(format!(
                "op's captured words were not read: no `{OP_WORDS_IN_WORKSPACE}` was found above the \
                 repository or in a `{WORKSPACE_REPO}` beside it, and `{OP_WORDS_VAR}` names none"
            ));
        }
        if self.elsewhere > 0 {
            parts.push(format!(
                "{} of op's captures name no `{}` and were held out as about another project",
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
