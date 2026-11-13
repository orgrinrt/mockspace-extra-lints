//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Where the cited lines are read from.
//!
//! A [`Source`] answers a file's lines at a revision and the files a revision holds. [`Git`] reads a
//! clone through `git show` and `git ls-tree` and keeps what it read, so a file cited by a thousand rows
//! is read once; the tests hand the comparison a source held in memory instead.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The lines of the files a set of rows quotes.
pub trait Source {
    /// The lines of `path` at `rev`, each without its line feed, or nothing where the revision holds
    /// no such file.
    fn lines(&mut self, rev: &str, path: &str) -> Option<&[String]>;

    /// Every file `rev` holds, by its path from the root.
    fn paths(&mut self, rev: &str) -> Vec<String>;
}

/// A text split into lines the way a quote is: on the line feed, a carriage return kept with its line
/// so it reads as trailing space, and no empty last line for a text that ends in a line feed.
#[must_use]
pub fn split(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = text.split('\n').map(str::to_owned).collect();
    if lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

/// A clone of the repository the rows quote, read through git.
pub struct Git {
    root:  PathBuf,
    files: HashMap<(String, String), Option<Vec<String>>>,
}

impl Git {
    /// The clone at `root`.
    #[must_use]
    pub fn at(root: &Path) -> Self {
        Self {
            root:  root.to_path_buf(),
            files: HashMap::new(),
        }
    }

    /// Whether `root` is a clone git can read.
    #[must_use]
    pub fn readable(root: &Path) -> bool {
        Command::new("git")
            .arg("-C")
            .arg(root)
            .args(["rev-parse", "--git-dir"])
            .output()
            .is_ok_and(|o| o.status.success())
    }
}

impl Source for Git {
    fn lines(&mut self, rev: &str, path: &str) -> Option<&[String]> {
        let root = &self.root;
        self.files
            .entry((rev.to_owned(), path.to_owned()))
            .or_insert_with(|| {
                let out = Command::new("git")
                    .arg("-C")
                    .arg(root)
                    .arg("show")
                    .arg(format!("{rev}:{path}"))
                    .output()
                    .ok()?;
                out.status
                    .success()
                    .then(|| split(&String::from_utf8_lossy(&out.stdout)))
            })
            .as_deref()
    }

    fn paths(&mut self, rev: &str) -> Vec<String> {
        Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args(["ls-tree", "-r", "--name-only", rev])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }
}
