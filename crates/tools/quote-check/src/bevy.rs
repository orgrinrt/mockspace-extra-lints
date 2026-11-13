//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Where a row of the `bevy` namespace says its quote comes from, and whether it can be read here.
//!
//! A row's `doc` lists the places Bevy says the fact, the first being the one the row rests on and the
//! quote being from that one. Three kinds of place are cited. A crate source in the cargo registry,
//! `<crate>-<version>/<path>:<first>-<last>`, is read from the registry's source directory. A blob of
//! Bevy's repository, `https://github.com/bevyengine/bevy/blob/<tag>/<path>#L<first>-L<last>`, is read
//! from a clone of Bevy at that tag. A page, a book chapter or a release post cited by a heading, has no
//! lines to compare, and is counted and listed as not checkable here rather than passed.
//!
//! A quote is an excerpt of what the row cites, so it is held when its lines, taken one by one with their
//! white space made one space, follow one another inside one cited range. The feature rows, whose quote is lines of a manifest, are blobs.

use std::path::{Path, PathBuf};

use crate::locator::{Cited, Lines};
use crate::source::{Source, split};

/// The first place a row's `doc` names, as far as it can be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Place {
    /// A file of a crate in the cargo registry: the directory `<crate>-<version>`, the path under it.
    Crate {
        dir:   String,
        cited: Cited,
    },
    /// A file of Bevy's repository at a tag.
    Blob {
        tag:   String,
        cited: Cited,
    },
}

/// The entries of a `doc` field. The list is joined with `, `, and an entry may hold a comma of its own
/// inside the heading in parentheses, so a piece that starts like neither kind of place continues the
/// entry before it.
#[must_use]
pub fn entries(doc: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for piece in doc.split(", ") {
        let starts = piece.starts_with("http://")
            || piece.starts_with("https://")
            || crate_place(piece).is_some();
        match out.last_mut() {
            Some(last) if !starts => {
                last.push_str(", ");
                last.push_str(piece);
            },
            _ => out.push(piece.to_owned()),
        }
    }
    out
}

/// The first place of a `doc`, where it names one that can be read.
#[cfg(test)]
pub fn first_place(doc: &str) -> Option<Place> {
    let first = entries(doc).into_iter().next()?;
    crate_place(&first).or_else(|| blob_place(&first))
}

/// `<crate>-<version>/<path>:<first>` or `:<first>-<last>`.
fn crate_place(entry: &str) -> Option<Place> {
    let (dir, rest) = entry.split_once('/')?;
    // the crate's name ends at the first dash that a digit follows: `bevy_ecs-0.20.0-rc.2`
    let at = dir
        .char_indices()
        .zip(dir.chars().skip(1))
        .find(|((_, c), next)| *c == '-' && next.is_ascii_digit())
        .map(|((i, _), _)| i)?;
    let name = &dir[.. at];
    if name.is_empty() || name.contains(['.', ':']) {
        return None;
    }
    let (path, range) = rest.rsplit_once(':')?;
    let range = range.split(' ').next()?;
    let (first, last) = range.split_once('-').unwrap_or((range, range));
    let lines = Lines::new(first.parse().ok()?, last.parse().ok()?)?;
    Some(Place::Crate {
        dir:   dir.to_owned(),
        cited: Cited {
            path: path.to_owned(),
            branch: None,
            lines,
        },
    })
}

/// `https://github.com/bevyengine/bevy/blob/<tag>/<path>#L<first>` or `#L<first>-L<last>`.
fn blob_place(entry: &str) -> Option<Place> {
    let rest = entry.strip_prefix("https://github.com/bevyengine/bevy/blob/")?;
    let rest = rest.split(" (").next()?;
    let (location, range) = rest.split_once("#L")?;
    let (tag, path) = location.split_once('/')?;
    let (first, last) = range.split_once("-L").unwrap_or((range, range));
    let lines = Lines::new(first.parse().ok()?, last.parse().ok()?)?;
    Some(Place::Blob {
        tag:   tag.to_owned(),
        cited: Cited {
            path: path.to_owned(),
            branch: None,
            lines,
        },
    })
}

/// The checkable places a `doc` opens with: the first, and each entry directly after it that is also a
/// crate source or a blob. Nothing is returned where the first entry is a page, since the quote is then
/// the page's words.
#[must_use]
pub fn leading_places(doc: &str) -> Vec<Place> {
    let mut out = Vec::new();
    for e in entries(doc) {
        match crate_place(&e).or_else(|| blob_place(&e)) {
            Some(p) => out.push(p),
            None => break,
        }
    }
    out
}

/// A line with its white space made one space, which is how lines are told equal here.
fn norm(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// How a row stood.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The quoted lines follow one another inside one cited range.
    Held,
    /// Every quoted line is in the cited ranges, though not as one run in one of them.
    Scattered,
    /// A quoted line is in none of the cited ranges: the first such line.
    Absent(String),
    /// A cited file is not in its source.
    NoFile(String),
    /// A cited range starts past the end of its file.
    PastEnd {
        path:  String,
        first: usize,
        lines: usize,
    },
    /// The source of the cited crate or tag is not here: a crate version the registry does not hold.
    NoSource(String),
    /// A blob, and no clone of Bevy was given.
    NeedsClone,
    /// A page: no lines to compare.
    NotLocal,
}

impl Outcome {
    /// The outcome's name, as the report spells it.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Held => "held",
            Self::Scattered => "scattered",
            Self::Absent(_) => "absent",
            Self::NoFile(_) => "no-file",
            Self::PastEnd {
                ..
            } => "past-end",
            Self::NoSource(_) => "no-source",
            Self::NeedsClone => "needs-clone",
            Self::NotLocal => "not-local",
        }
    }

    /// Whether the row's quote is wrong against what it cites, and not merely unread.
    #[must_use]
    pub const fn is_finding(&self) -> bool {
        matches!(
            self,
            Self::Absent(_) | Self::NoFile(_) | Self::PastEnd { .. }
        )
    }
}

/// The lines of one cited range, normalised and without the blank ones.
fn range_of(source: &mut impl Source, rev: &str, cited: &Cited) -> Result<Vec<String>, Outcome> {
    let Some(lines) = source.lines(rev, &cited.path) else {
        return Err(if source.paths(rev).is_empty() {
            Outcome::NoSource(rev.to_owned())
        } else {
            Outcome::NoFile(cited.path.clone())
        });
    };
    let first = cited.lines.first();
    if first > lines.len() {
        return Err(Outcome::PastEnd {
            path: cited.path.clone(),
            first,
            lines: lines.len(),
        });
    }
    let last = cited.lines.last().min(lines.len());
    Ok(lines[first - 1 .. last]
        .iter()
        .map(|l| norm(l))
        .filter(|l| !l.is_empty())
        .collect())
}

/// How a row's quote stands, reading a crate place from `crates` and a blob from `blobs` where there is
/// one. The quote is an excerpt of what the row cites, so it is held when its lines follow one another
/// inside one cited range.
pub fn check(
    doc: &str,
    quote: &str,
    crates: &mut impl Source,
    mut blobs: Option<&mut impl Source>,
) -> Outcome {
    let places = leading_places(doc);
    if places.is_empty() {
        return Outcome::NotLocal;
    }
    let mut ranges: Vec<Vec<String>> = Vec::new();
    let mut failure: Option<Outcome> = None;
    for p in &places {
        let got = match p {
            Place::Crate {
                dir,
                cited,
            } => range_of(crates, dir, cited),
            Place::Blob {
                tag,
                cited,
            } => {
                match blobs.as_deref_mut() {
                    Some(b) => range_of(b, tag, cited),
                    None => Err(Outcome::NeedsClone),
                }
            },
        };
        match got {
            Ok(r) => ranges.push(r),
            Err(e) => {
                failure.get_or_insert(e);
            },
        }
    }
    let want: Vec<String> = quote.lines().map(norm).filter(|l| !l.is_empty()).collect();
    if want.is_empty() {
        return Outcome::NotLocal;
    }
    if ranges
        .iter()
        .any(|r| r.windows(want.len()).any(|w| run_is_quoted(w, &want)))
    {
        return Outcome::Held;
    }
    if let Some(e) = failure {
        return e;
    }
    match want.iter().find(|l| !ranges.iter().any(|r| r.contains(l))) {
        // a quote of prose may be the words of a page the row also cites, which is not read here
        Some(l) if cites_a_page(doc) && !quote.trim_start().starts_with("//") => {
            let _ = l;
            Outcome::NotLocal
        },
        Some(l) => Outcome::Absent(l.clone()),
        None => Outcome::Scattered,
    }
}

/// The cargo registry's source directory, read as a [`Source`]: a revision is the directory
/// `<crate>-<version>` under it.
pub struct CrateDirs {
    root:  PathBuf,
    files: std::collections::HashMap<(String, String), Option<Vec<String>>>,
}

impl CrateDirs {
    /// The registry source directory at `root`.
    #[must_use]
    pub fn at(root: &Path) -> Self {
        Self {
            root:  root.to_path_buf(),
            files: std::collections::HashMap::new(),
        }
    }
}

impl Source for CrateDirs {
    fn lines(&mut self, rev: &str, path: &str) -> Option<&[String]> {
        let file = self.root.join(rev).join(path);
        self.files
            .entry((rev.to_owned(), path.to_owned()))
            .or_insert_with(|| std::fs::read_to_string(file).ok().map(|t| split(&t)))
            .as_deref()
    }

    fn paths(&mut self, rev: &str) -> Vec<String> {
        let base = self.root.join(rev);
        let mut out = Vec::new();
        walk(&base, &base, &mut out);
        out.sort();
        out
    }
}

/// The files under `dir`, by path from `base`.
fn walk(base: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(base, &p, out);
        } else if let Ok(rel) = p.strip_prefix(base) {
            out.push(rel.to_string_lossy().into_owned());
        }
    }
}

/// Whether a `doc` names a place that cannot be read here: a page cited by a heading, or an issue.
fn cites_a_page(doc: &str) -> bool {
    entries(doc)
        .iter()
        .any(|e| crate_place(e).or_else(|| blob_place(e)).is_none())
}

/// Whether `window`, lines of the source, is what `want` quotes: the inner lines whole, and the first and
/// the last only as far as they are quoted, since an excerpt may begin or end inside a line.
fn run_is_quoted(window: &[String], want: &[String]) -> bool {
    let last = want.len() - 1;
    window
        .iter()
        .zip(want)
        .enumerate()
        .all(|(i, (have, quoted))| {
            match (i == 0, i == last) {
                (true, true) => have.contains(quoted.as_str()),
                (true, false) => have.ends_with(quoted.as_str()),
                (false, true) => have.starts_with(quoted.as_str()),
                (false, false) => have == quoted,
            }
        })
}
