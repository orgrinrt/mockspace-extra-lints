//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Whether a quote is the lines it cites, and if not, how it differs.
//!
//! The comparison is a ladder, each rung forgiving one more kind of difference that leaves the words
//! as they were: space at the ends of lines and a carriage return, blank lines opening or closing the
//! range, the line a port writes between two ranges it joins, and runs of space or tab inside a line.
//! The first rung that holds is the verdict, so a quote is reported by the least it differs by, and a
//! quote whose words differ is a [`Verdict::Differs`] naming the first line where they do.

use crate::locator::{Cited, Unreadable};
use crate::source::{Source, split};

/// How a quote stands against the lines it cites.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The quote is the lines.
    Exact,
    /// Equal once space and a carriage return at the end of each line are dropped.
    Trailing,
    /// Equal once blank lines at the start and the end are dropped as well.
    EdgeBlank,
    /// Several ranges, equal once the separator lines between them and the blank lines are dropped.
    Joined,
    /// Equal once every run of white space in a line is one space and blank lines are dropped.
    Spacing,
    /// The words differ, first at `at`, counted from one over the lines that carry words.
    Differs {
        at:     usize,
        cited:  String,
        quoted: String,
    },
    /// A cited file is not at the revision.
    NoFile(String),
    /// A cited range runs past the end of its file.
    PastEnd {
        path:  String,
        last:  usize,
        lines: usize,
    },
    /// The locator could not be read.
    Locator(Unreadable),
}

impl Verdict {
    /// Whether the quote holds the cited words, whatever its quirks of space.
    #[must_use]
    pub const fn faithful(&self) -> bool {
        matches!(
            self,
            Self::Exact | Self::Trailing | Self::EdgeBlank | Self::Joined | Self::Spacing
        )
    }

    /// The verdict's name, as the report and the tests spell it.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Trailing => "trailing",
            Self::EdgeBlank => "edge-blank",
            Self::Joined => "joined",
            Self::Spacing => "spacing",
            Self::Differs {
                ..
            } => "differs",
            Self::NoFile(_) => "no-file",
            Self::PastEnd {
                ..
            } => "past-end",
            Self::Locator(_) => "locator",
        }
    }
}

/// Compares `quote` with the lines `cited` names at `rev`, `separator` being the line a port writes
/// between two joined ranges.
pub fn compare(
    source: &mut impl Source,
    rev: &str,
    cited: &[Cited],
    quote: &str,
    separator: &str,
) -> Verdict {
    let mut want: Vec<String> = Vec::new();
    for c in cited {
        let Some(lines) = source.lines(rev, &c.path) else {
            return Verdict::NoFile(c.path.clone());
        };
        // a range may name the line after the last, which a file ending without a line feed and a
        // range written to its end both produce
        if c.lines.first() > lines.len() + 1 || c.lines.last() > lines.len() + 1 {
            return Verdict::PastEnd {
                path:  c.path.clone(),
                last:  c.lines.last(),
                lines: lines.len(),
            };
        }
        let end = c.lines.last().min(lines.len());
        want.extend(lines[c.lines.first() - 1 .. end].iter().cloned());
    }
    let quoted = split(quote);
    ladder(&want, &quoted, cited.len() > 1, separator)
}

fn ladder(want: &[String], quoted: &[String], several: bool, separator: &str) -> Verdict {
    if want == quoted {
        return Verdict::Exact;
    }
    let trimmed =
        |v: &[String]| -> Vec<String> { v.iter().map(|l| l.trim_end().to_owned()).collect() };
    let (w, q) = (trimmed(want), trimmed(quoted));
    if w == q {
        return Verdict::Trailing;
    }
    let edged = |v: &[String]| -> Vec<String> {
        let start = v.iter().position(|l| !l.is_empty()).unwrap_or(v.len());
        let end = v
            .iter()
            .rposition(|l| !l.is_empty())
            .map_or(start, |e| e + 1);
        v[start .. end].to_vec()
    };
    if edged(&w) == edged(&q) {
        return Verdict::EdgeBlank;
    }
    let worded = |v: &[String], drop: Option<&str>| -> Vec<String> {
        v.iter()
            .filter(|l| !l.is_empty() && Some(l.as_str()) != drop)
            .cloned()
            .collect()
    };
    if several && worded(&w, None) == worded(&q, Some(separator)) {
        return Verdict::Joined;
    }
    let spaced = |v: &[String], drop: Option<&str>| -> Vec<String> {
        worded(v, drop)
            .iter()
            .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect()
    };
    let drop = several.then_some(separator);
    let (w, q) = (spaced(want, None), spaced(quoted, drop));
    if w == q {
        return Verdict::Spacing;
    }
    let at = w
        .iter()
        .zip(&q)
        .position(|(a, b)| a != b)
        .unwrap_or(w.len().min(q.len()));
    Verdict::Differs {
        at:     at + 1,
        cited:  w.get(at).cloned().unwrap_or_default(),
        quoted: q.get(at).cloned().unwrap_or_default(),
    }
}
