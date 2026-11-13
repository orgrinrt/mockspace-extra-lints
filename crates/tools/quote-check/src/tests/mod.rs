//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The comparison, the locator and the coverage, each against a source held in memory, and every
//! verdict beside the near case that must not reach it.

mod bevy;
mod compare;
mod coverage;
mod locator;
mod options;

use std::collections::HashMap;

use crate::source::{Source, split};

/// A source held in memory: files by revision and path, and the paths of each revision.
#[derive(Default)]
pub(crate) struct Memory {
    files: HashMap<(String, String), Vec<String>>,
}

impl Memory {
    /// Adds `text` as `path` at `rev`.
    pub(crate) fn with(mut self, rev: &str, path: &str, text: &str) -> Self {
        self.files
            .insert((rev.to_owned(), path.to_owned()), split(text));
        self
    }
}

impl Source for Memory {
    fn lines(&mut self, rev: &str, path: &str) -> Option<&[String]> {
        self.files
            .get(&(rev.to_owned(), path.to_owned()))
            .map(Vec::as_slice)
    }

    fn paths(&mut self, rev: &str) -> Vec<String> {
        let mut p: Vec<String> = self
            .files
            .keys()
            .filter(|(r, _)| r == rev)
            .map(|(_, p)| p.clone())
            .collect();
        p.sort();
        p
    }
}
