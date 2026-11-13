//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The files of the repository a citation or a word can sit in, and the tier each belongs to.
//!
//! This file declares no lint. It is the one owner of which files are read and which tier a path is in,
//! so the lint `a-retired-word-stays-out` and the `canon` tool, which reads it too, cannot
//! disagree about either. It reads through the [`View`] it is handed, a lint's own or one a tool builds over
//! a reading of the repository.
//!
//! The tiers are the ones the workflow names: design, code, research, rounds, content, and the canon
//! itself. Generated documents, build output and the agent files generated from templates are not read,
//! since each only repeats what one of the tiers already says.

use std::path::{Path, PathBuf};

use mockspace::View;

use crate::layout::Layout;

/// Which tier a path sits in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    /// A design template under the mock directory.
    Design,
    /// Rust under the mock directory.
    Code,
    /// A research note.
    Research,
    /// A design round, open or archived.
    Rounds,
    /// A file under one of the content directories the project names.
    Content,
    /// Another registry row.
    Canon,
    /// Anything else read, such as the mockspace config.
    Other,
}

impl Tier {
    /// The tier's name as the verbs print it.
    pub fn label(self) -> &'static str {
        match self {
            Self::Design => "design",
            Self::Code => "code",
            Self::Research => "research",
            Self::Rounds => "rounds",
            Self::Content => "content",
            Self::Canon => "canon",
            Self::Other => "other",
        }
    }
}

/// Directories never read, by name, wherever they sit.
const SKIPPED_ANYWHERE: [&str; 3] = [".git", "target", "node_modules"];

/// Directories never read at the repository root: generated documents and generated agent files.
const SKIPPED_AT_ROOT: [&str; 3] = ["docs", ".claude", ".github"];

/// The extensions of text a citation can sit in.
const READ: [&str; 6] = ["md", "tmpl", "rs", "toml", "slang", "wgsl"];

/// The tier of a file, by its path relative to the repository root, in the repository laid out as
/// `layout` says.
pub fn tier_of(rel: &Path, layout: &Layout) -> Tier {
    let s = rel.to_string_lossy().replace('\\', "/");
    if layout.in_content(&s).is_some() {
        return Tier::Content;
    }
    let Some(m) = layout.under_mock(&s) else {
        return Tier::Other;
    };
    if m.starts_with("registry/") {
        Tier::Canon
    } else if m.starts_with("design_rounds/") {
        Tier::Rounds
    } else if m.starts_with("research/") {
        Tier::Research
    } else if m.ends_with(".md.tmpl") {
        Tier::Design
    } else if m.ends_with(".rs") {
        Tier::Code
    } else {
        Tier::Other
    }
}

/// Every file read, relative to `repo`, sorted.
pub fn files(view: &View<'_>, repo: &Path) -> Vec<PathBuf> {
    let mut out = crate::tree_walk::walk_files(
        view,
        repo,
        |rel, name| {
            SKIPPED_ANYWHERE.contains(&name)
                || (rel.as_os_str().is_empty() && SKIPPED_AT_ROOT.contains(&name))
        },
        |path| {
            path.extension()
                .is_some_and(|e| READ.contains(&e.to_string_lossy().as_ref()))
        },
    );
    for path in &mut out {
        *path = path.strip_prefix(repo).unwrap_or(path).to_path_buf();
    }
    out
}
