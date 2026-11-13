//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Where a consuming repository keeps its mock tree and its content, as one value every lint and tool
//! of the workflow reads paths by. This file declares no lint.
//!
//! The mock directory is the one `mockspace.toml` declares as `mock_dir`, which the engine hands a lint
//! and a tool as `mock_dir` beside the repository root, so nothing here reads it from the config. The
//! content directories are the repository's own and the engine knows nothing of them: a project names
//! them with `content_dirs` in the table of each lint and tool that reads them, `[lints.<name>]` as a
//! comma separated value and `[tools.<name>]` as a list, and none is read where it names none. A path
//! is relative to the repository root and `/` separated.

use std::collections::HashMap;
use std::path::Path;

use mockspace::tool::{ConfigKey, ToolParams};


/// The key naming the repository's content directories.
pub const CONTENT_DIRS: &str = "content_dirs";

/// [`CONTENT_DIRS`] as a tool declares it.
pub const CONTENT_DIRS_KEY: ConfigKey = ConfigKey {
    name:        CONTENT_DIRS,
    description: "the directories at the repository root that hold the project's content packs, read as the content tier; none when unset",
};

/// The content directories a project named in one lint's or tool's table.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ContentDirs(Vec<String>);

impl ContentDirs {
    /// The directories given, each trimmed of a trailing `/`.
    #[must_use]
    pub fn new<S: AsRef<str>>(dirs: &[S]) -> Self {
        Self(
            dirs.iter()
                .map(|d| d.as_ref().trim().trim_end_matches('/').to_owned())
                .filter(|d| !d.is_empty())
                .collect(),
        )
    }

    /// What a lint's `[lints.<name>]` table set, a comma separated value.
    pub fn configure_lint(&mut self, params: &HashMap<String, String>) {
        if let Some(v) = params.get(CONTENT_DIRS) {
            *self = Self::new(&v.split(',').collect::<Vec<_>>());
        }
    }

    /// What a tool's `[tools.<name>]` table set, a list.
    pub fn configure_tool(&mut self, params: &ToolParams) {
        if let Some(v) = params.list(CONTENT_DIRS) {
            *self = Self::new(v);
        }
    }

    /// The directories, in the order named.
    #[must_use]
    pub fn dirs(&self) -> &[String] {
        &self.0
    }
}

/// The places of one repository: its mock directory and its content directories.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    /// The mock directory relative to the repository root, empty where it is the root.
    mock:    String,
    content: ContentDirs,
}

impl Layout {
    /// A layout whose mock directory is `mock`, relative to the root.
    #[must_use]
    pub fn new(mock: &str, content: ContentDirs) -> Self {
        Self {
            mock: mock.trim_matches('/').to_owned(),
            content,
        }
    }

    /// The layout of the repository at `repo_root` whose mock directory is `mock_dir`, as the engine
    /// hands both.
    #[must_use]
    pub fn of(mock_dir: &Path, repo_root: &Path, content: &ContentDirs) -> Self {
        Self::new(&mock_rel(mock_dir, repo_root), content.clone())
    }

    /// The mock directory relative to the root, empty where it is the root.
    #[must_use]
    pub fn mock(&self) -> &str {
        &self.mock
    }

    /// `rel` under the mock directory, relative to the root.
    #[must_use]
    pub fn in_mock(&self, rel: &str) -> String {
        if self.mock.is_empty() {
            rel.to_owned()
        } else {
            format!("{}/{rel}", self.mock)
        }
    }

    /// The content directories, in the order the project named them.
    #[must_use]
    pub fn content(&self) -> &[String] {
        self.content.dirs()
    }

    /// A path relative to the root, relative to the mock directory instead, where it lies under it.
    #[must_use]
    pub fn under_mock<'a>(&self, rel: &'a str) -> Option<&'a str> {
        if self.mock.is_empty() {
            Some(rel)
        } else {
            rel.strip_prefix(self.mock.as_str())?.strip_prefix('/')
        }
    }

    /// The content directory a path relative to the root lies in, and the path relative to it.
    #[must_use]
    pub fn in_content<'a>(&self, rel: &'a str) -> Option<(&str, &'a str)> {
        self.content.dirs().iter().find_map(|d| {
            let rest = rel.strip_prefix(d.as_str())?.strip_prefix('/')?;
            Some((d.as_str(), rest))
        })
    }
}


/// The mock directory relative to the root, empty where it is the root or not under it.
#[must_use]
pub fn mock_rel(mock_dir: &Path, repo_root: &Path) -> String {
    mock_dir
        .strip_prefix(repo_root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default()
}
#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
