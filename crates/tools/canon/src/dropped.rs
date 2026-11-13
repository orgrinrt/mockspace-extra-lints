//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The words a replaced tenet used that the tenets replacing it do not, and how many lines of designs,
//! code and content still name each.
//!
//! A tenet that is replaced is a concept the canon changed, and a rename (a grid that became a frame) is
//! the shape of change a design most easily survives: the old word stays in the prose and the new row
//! is cited beside it. Which words a replaced tenet retired is recorded in `rules_out`, a canon edit the
//! maintainer ratifies. This is the triage under it: it reads the replaced row and its successors, names
//! the words only the old one carried, and counts where they still stand, so the person ratifying starts
//! from the words with lines behind them and not from every replaced row at once.
//! It reports candidates and gates nothing: a word dropped in a rewording ("thinks", "thinking") is
//! counted the same as one that was retired.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use crate::canon_tree::{Tier, files, tier_of};
use crate::retired_words::{scanned, words};
use crate::rows::Tenet;

/// Words shorter than this are connectives and numerals, and are not read as concepts.
const LEAST: usize = 4;

/// How many candidate words a listing shows for one tenet.
pub const SHOWN_WORDS: usize = 6;

/// A word named by more than this share of the design files, once there are enough of them to say, is
/// part of how designs are written and not a concept: `which`, `after`, `every`.
const COMMON_SHARE: usize = 4;

/// Fewer design files than this say nothing about which words are common.
const COMMON_FLOOR: usize = 8;

/// The lines naming a word, by tier.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub design:       usize,
    pub code:         usize,
    pub content:      usize,
    /// The design files naming it, which is what tells a concept from a common word.
    pub design_files: usize,
}

impl Counts {
    pub const fn total(self) -> usize {
        self.design + self.code + self.content
    }
}

/// What the scanned files say, word by word.
#[derive(Debug, Default)]
pub struct Corpus {
    words:        HashMap<String, Counts>,
    design_files: usize,
}

impl Corpus {
    /// The lines naming `word`, or nothing where none does or where it is a common word of designs.
    pub fn named(&self, word: &str) -> Option<Counts> {
        let c = self.words.get(word).copied().filter(|c| c.total() > 0)?;
        let common =
            self.design_files >= COMMON_FLOOR && c.design_files * COMMON_SHARE > self.design_files;
        (!common).then_some(c)
    }
}

/// For every word of a scanned file, the lines naming it, each line counted once however often it says it.
pub fn word_counts(repo: &Path) -> Corpus {
    let mut out = Corpus::default();
    let reading = mockspace::reading::Reading::new(repo);
    let view = reading.view();
    for rel in files(&view, repo) {
        if !scanned(&rel) {
            continue;
        }
        let tier = tier_of(&rel);
        let Some(body) = view.text(repo.join(&rel)) else {
            continue;
        };
        let mut in_file: HashSet<String> = HashSet::new();
        for line in body.lines() {
            let on_line: HashSet<String> = words(line).into_iter().collect();
            for w in on_line {
                let c = out.words.entry(w.clone()).or_default();
                match tier {
                    Tier::Design => c.design += 1,
                    Tier::Code => c.code += 1,
                    Tier::Content => c.content += 1,
                    _ => {},
                }
                in_file.insert(w);
            }
        }
        if tier == Tier::Design {
            out.design_files += 1;
            for w in in_file {
                if let Some(c) = out.words.get_mut(&w) {
                    c.design_files += 1;
                }
            }
        }
    }
    out
}

/// The words of a tenet's id, `what` and keywords: everything it says about itself.
fn carried(t: &Tenet<'_>) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = words(t.id).into_iter().collect();
    out.extend(words(t.what));
    for k in &t.keywords {
        out.extend(words(k));
    }
    out
}

/// The words in the id of `t` that none of the tenets replacing it carries in its id, `what` or keywords,
/// longer than a connective and made of letters.
///
/// Only the id is read for the dropped word. An id names the concept the tenet is about, and its `what`
/// carries the prose around it, so reading the `what` too turns every rewording into a candidate
/// ("either", "across"). A successor's `because` is left out of what it keeps: a row that replaces
/// another explains the change in the old words, and those words are what this is looking for.
pub fn dropped(all: &[Tenet<'_>], t: &Tenet<'_>) -> Vec<String> {
    let mut kept: BTreeSet<String> = BTreeSet::new();
    for s in t
        .superseded_by
        .iter()
        .filter_map(|s| all.iter().find(|n| n.id == *s))
    {
        kept.extend(carried(s));
    }
    words(t.id)
        .into_iter()
        .filter(|w| w.len() >= LEAST && w.chars().all(char::is_alphabetic) && !kept.contains(w))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// The dropped words of `t` that some line still names, the most named first.
pub fn still_named(all: &[Tenet<'_>], t: &Tenet<'_>, corpus: &Corpus) -> Vec<(String, Counts)> {
    let mut out: Vec<(String, Counts)> = dropped(all, t)
        .into_iter()
        .filter_map(|w| {
            let c = corpus.named(&w)?;
            Some((w, c))
        })
        .collect();
    out.sort_by(|a, b| b.1.total().cmp(&a.1.total()).then_with(|| a.0.cmp(&b.0)));
    out
}

/// `word design/code/content` for a listing.
pub fn show(w: &str, c: Counts) -> String {
    format!("{w} {}/{}/{}", c.design, c.code, c.content)
}
