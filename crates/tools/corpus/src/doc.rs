//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The corpus as documents, and what kind each one is.
//!
//! The registry is a few hundred rows. The research notes, the round transcripts and op's captured
//! words beside it are many times that, and they hold the argument and most of the answers. A tool
//! that reads only the rows answers from a fraction of the material and answers confidently, which is
//! the failure this crate exists to remove.

use std::path::{Path, PathBuf};

/// What a document is, which decides how much a hit in it is worth.
///
/// Read off where the document sits rather than its contents or its name. kamu, where this came from,
/// read op's files off a naming convention, and found a name is not provenance: a document named
/// for op outranked his own words. Here op's words are what sits under the root they were captured
/// to, `op-responses/`, and nothing else is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// A capture of op's own words. The highest tier there is.
    Op,
    /// A research note, or the data and lists a round of research kept beside it.
    Notes,
    /// A design round's topic or changelist.
    Round,
    /// A sketch: code written to try something, under `research/`. It establishes one thing and
    /// says so at length, and the length is what wins a naive search.
    Sketch,
}

impl Kind {
    /// The multiplier a hit in this kind of document carries.
    ///
    /// Not a ranking of quality. A ranking of where a settled answer lives, which is what somebody
    /// searching the corpus is looking for.
    #[must_use]
    pub fn weight(self) -> f64 {
        match self {
            Self::Op => 1.9,
            Self::Notes | Self::Round => 1.0,
            Self::Sketch => 0.6,
        }
    }

    /// The word a report prints.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Op => "op",
            Self::Notes => "notes",
            Self::Round => "round",
            Self::Sketch => "sketch",
        }
    }

    /// Classify one path, relative to the corpus: `op-responses/`, `research/` or `design_rounds/`
    /// first.
    #[must_use]
    pub fn of(path: &Path) -> Self {
        let full = path.to_string_lossy().to_lowercase();
        if full.starts_with("op-responses/") {
            return Self::Op;
        }
        if full.starts_with("design_rounds/") || full.contains("/design_rounds/") {
            return Self::Round;
        }
        let ext = path
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or_default()
            .to_lowercase();
        if SKETCH_EXT.contains(&ext.as_str()) {
            return Self::Sketch;
        }
        Self::Notes
    }
}

/// The extensions a sketch is written in. Everything else read under `research/` is a note.
pub const SKETCH_EXT: &[&str] = &["rs", "cjs", "js", "sh", "wgsl", "slang"];

/// One document, loaded.
pub struct Doc {
    /// What a reader is shown and opens: relative to the mock directory, or under `op-responses/`
    /// for a capture of op's words, which sits outside it.
    pub path:  PathBuf,
    /// Where it was read from, so a line can be read back in the case it was written in.
    pub file:  PathBuf,
    pub kind:  Kind,
    /// Lowercased once. Every search is case-insensitive and doing it per term
    /// per document was most of the first version's runtime.
    pub lower: String,
    /// Line offsets into `lower`, so a hit reports a line number without a
    /// second pass over the text.
    lines:     Vec<(usize, usize)>,
    /// Byte ranges that are headings, and ranges that are blockquotes. A hit
    /// inside one is worth more, because a heading names the subject and a
    /// blockquote in this corpus is nearly always somebody being quoted.
    heads:     Vec<(usize, usize)>,
    quotes:    Vec<(usize, usize)>,
}

impl Doc {
    /// Load one file. `None` where it cannot be read or is not text worth
    /// searching, which is reported by the caller rather than swallowed.
    #[must_use]
    pub fn load(shown: &Path, file: &Path) -> Option<Self> {
        let raw = std::fs::read_to_string(file).ok()?;
        // A file with no line breaks at all in two million words is a data
        // blob rather than prose, and searching it produces one enormous hit.
        if raw.len() > 4_000_000 {
            return None;
        }
        let lower = raw.to_lowercase();
        let mut lines = Vec::new();
        let mut heads = Vec::new();
        let mut quotes = Vec::new();
        let mut at = 0usize;
        for line in lower.split_inclusive('\n') {
            let end = at + line.len();
            lines.push((at, end));
            let t = line.trim_start();
            if t.starts_with('#') {
                heads.push((at, end));
            } else if t.starts_with("> ") || t == ">\n" || t == ">" {
                quotes.push((at, end));
            }
            at = end;
        }
        Some(Self {
            path: shown.to_path_buf(),
            file: file.to_path_buf(),
            kind: Kind::of(shown),
            lower,
            lines,
            heads,
            quotes,
        })
    }

    /// Which line a byte offset falls on, one-indexed.
    #[must_use]
    pub fn line_of(&self, at: usize) -> usize {
        match self.lines.binary_search_by(|(s, e)| {
            if at < *s {
                std::cmp::Ordering::Greater
            } else if at >= *e {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        }) {
            Ok(i) => i + 1,
            Err(i) => i + 1,
        }
    }

    /// The text of the line an offset falls on, trimmed.
    #[must_use]
    pub fn line_text(&self, at: usize) -> &str {
        let i = self.line_of(at).saturating_sub(1);
        self.lines
            .get(i)
            .map_or("", |(s, e)| self.lower[*s .. *e].trim_end())
    }

    /// What a hit at this offset is worth, positionally.
    ///
    /// Three tiers and no more. A finer scale would be inventing precision the
    /// corpus does not have: what is genuinely different is a heading, which
    /// declares the subject, and a blockquote, which in this corpus is somebody
    /// being quoted and usually op.
    #[must_use]
    pub fn position_weight(&self, at: usize) -> f64 {
        if in_range(&self.heads, at) {
            3.0
        } else if in_range(&self.quotes, at) {
            2.0
        } else {
            1.0
        }
    }
}

fn in_range(ranges: &[(usize, usize)], at: usize) -> bool {
    ranges
        .binary_search_by(|(s, e)| {
            if at < *s {
                std::cmp::Ordering::Greater
            } else if at >= *e {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}
