//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Where tenets are named outside the registry's own rows, by tier.
//!
//! Which files are read and which tier a path is in is `mock/lints/canon_tree.rs`, which the lint
//! `a-retired-word-stays-out` reads as well; this is the scan for citations over them.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub use crate::canon_tree::{Tier, files, tier_of};
use crate::names::named;

/// One line naming a tenet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    /// The tier it sits in.
    pub tier: Tier,
    /// The file, relative to the repository root.
    pub path: PathBuf,
    /// Its line number, from one.
    pub line: usize,
}

/// Every line in the tree naming a tenet in `known`, by the tenet it names.
///
/// A registry row's own `id = "..."` line declares the tenet rather than naming it, and is not counted.
pub fn mentions<'a>(repo: &Path, known: &BTreeSet<&'a str>) -> BTreeMap<&'a str, Vec<Line>> {
    let mut out: BTreeMap<&'a str, Vec<Line>> = BTreeMap::new();
    let reading = mockspace::reading::Reading::new(repo);
    let view = reading.view();
    for rel in files(&view, repo) {
        let Some(body) = view.text(repo.join(&rel)) else {
            continue;
        };
        let tier = tier_of(&rel);
        for (n, text) in body.lines().enumerate() {
            if tier == Tier::Canon && text.trim_start().starts_with("id = ") {
                continue;
            }
            for id in named(text, known) {
                out.entry(id).or_default().push(Line {
                    tier,
                    path: rel.clone(),
                    line: n + 1,
                });
            }
        }
    }
    out
}
