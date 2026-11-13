//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Where a row says its quote comes from: one or more ranges of lines, each of one file.
//!
//! A locator is written `<path>:<first>-<last>`, several joined by `, `, and a path may be led by
//! `<branch>:`, which names where the lines were read and is otherwise informational, since the row's
//! revision decides what is read. A bare file name, with no directory in it, sits under the base the
//! run names; a path with a directory in it is from the source's root.

/// A range of lines, numbered from one, first and last both included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lines {
    first: usize,
    last:  usize,
}

impl Lines {
    /// The range, or nothing where it starts at nought or ends before it starts.
    #[must_use]
    pub const fn new(first: usize, last: usize) -> Option<Self> {
        if first == 0 || last < first {
            None
        } else {
            Some(Self {
                first,
                last,
            })
        }
    }

    /// The first line.
    #[must_use]
    pub const fn first(self) -> usize {
        self.first
    }

    /// The last line.
    #[must_use]
    pub const fn last(self) -> usize {
        self.last
    }

    /// Whether `line` falls inside the range.
    #[must_use]
    pub const fn holds(self, line: usize) -> bool {
        line >= self.first && line <= self.last
    }
}

/// One cited range: the path it is read from, as the source names it, and its lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cited {
    /// The path from the source's root, the base already applied.
    pub path:   String,
    /// The branch the locator says it was read on, where it says one.
    pub branch: Option<String>,
    /// The lines.
    pub lines:  Lines,
}

impl Cited {
    /// A bare file name: no branch and no directory, which `--base` puts under its directory and a run
    /// with no base reads at the clone's root.
    pub fn is_bare(&self) -> bool {
        self.branch.is_none() && !self.path.contains('/')
    }
}

/// Why a locator could not be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unreadable {
    /// The field is empty.
    Empty,
    /// A part names no line range, or one that is not two numbers in order.
    Range(String),
}

/// Reads a locator into its ranges, in the order written, putting `base` before a bare file name.
pub fn parse(text: &str, base: &str) -> Result<Vec<Cited>, Unreadable> {
    let text = text.trim();
    if text.is_empty() {
        return Err(Unreadable::Empty);
    }
    text.split(", ")
        .map(|part| one(part.trim(), base))
        .collect()
}

fn one(part: &str, base: &str) -> Result<Cited, Unreadable> {
    let bad = || Unreadable::Range(part.to_owned());
    let (place, range) = part.rsplit_once(':').ok_or_else(bad)?;
    let (first, last) = range.split_once('-').ok_or_else(bad)?;
    let (first, last) = (
        first.parse().map_err(|_| bad())?,
        last.parse().map_err(|_| bad())?,
    );
    let lines = Lines::new(first, last).ok_or_else(bad)?;
    let (branch, path) = match place.split_once(':') {
        Some((branch, path)) => (Some(branch.to_owned()), path.to_owned()),
        None => (None, place.to_owned()),
    };
    if path.is_empty() {
        return Err(bad());
    }
    // A path led by a branch is from the repository root, as the namespace's `doc` field says, so a
    // file at the root (`mockspace.toml`) is not put under the base like a seed document's bare name.
    let path = if branch.is_some() || path.contains('/') || base.is_empty() {
        path
    } else {
        format!("{}/{path}", base.trim_end_matches('/'))
    };
    Ok(Cited {
        path,
        branch,
        lines,
    })
}
