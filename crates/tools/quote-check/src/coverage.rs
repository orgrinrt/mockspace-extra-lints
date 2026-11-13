//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Which of the source's lines no row quotes.
//!
//! A port that means to hold a document whole leaves nothing of it outside a row, and a port that
//! holds part of it should say which part. The union of every row's ranges is taken per path over
//! every revision the rows cite, since a port reading two revisions of one tree cites a file at
//! whichever it read it at; a file is then measured at the revision most of its rows cite.

use std::collections::{BTreeMap, BTreeSet};

use crate::locator::Lines;
use crate::source::Source;

/// The files a revision holds that are read as text, by extension.
const TEXT: &[&str] = &["md", "toml", "tmpl", "rs", "slang", "txt", "json", "yaml", "yml"];

/// What no row holds.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Coverage {
    /// Cited files with lines that carry words no range covers: path, those lines, the file's lines
    /// that carry words.
    pub partial: Vec<(String, usize, usize)>,
    /// Text files a cited revision holds that no row cites at any revision: path and lines.
    pub uncited: Vec<(String, usize)>,
    /// Cited files every line of which, among those that carry words, some range covers.
    pub whole:   usize,
}

/// Measures `cited`, every row's revision and ranges, against the files under `within`.
pub fn measure(
    source: &mut impl Source,
    cited: &[(String, String, Lines)],
    within: &str,
) -> Coverage {
    let mut ranges: BTreeMap<&str, Vec<Lines>> = BTreeMap::new();
    let mut revs: BTreeMap<&str, BTreeMap<&str, usize>> = BTreeMap::new();
    for (rev, path, lines) in cited {
        ranges.entry(path.as_str()).or_default().push(*lines);
        *revs
            .entry(path.as_str())
            .or_default()
            .entry(rev.as_str())
            .or_default() += 1;
    }
    let mut out = Coverage::default();
    for (path, by_rev) in &revs {
        let rev = by_rev
            .iter()
            .max_by_key(|(_, n)| **n)
            .map(|(r, _)| *r)
            .unwrap_or_default();
        let Some(lines) = source.lines(rev, path) else { continue };
        let worded: Vec<usize> = (1 ..= lines.len())
            .filter(|n| !lines[n - 1].trim().is_empty())
            .collect();
        let held = &ranges[path];
        let open = worded
            .iter()
            .filter(|n| !held.iter().any(|r| r.holds(**n)))
            .count();
        if open == 0 {
            out.whole += 1;
        } else {
            out.partial.push(((*path).to_owned(), open, worded.len()));
        }
    }
    let every_rev: BTreeSet<&str> = cited.iter().map(|(r, ..)| r.as_str()).collect();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for rev in every_rev {
        for path in source.paths(rev) {
            let text = path
                .rsplit_once('.')
                .is_some_and(|(_, ext)| TEXT.contains(&ext));
            if !text
                || !path.starts_with(within)
                || ranges.contains_key(path.as_str())
                || !seen.insert(path.clone())
            {
                continue;
            }
            let n = source.lines(rev, &path).map_or(0, <[String]>::len);
            out.uncited.push((path, n));
        }
    }
    out.partial
        .sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    out.uncited
        .sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    out
}
