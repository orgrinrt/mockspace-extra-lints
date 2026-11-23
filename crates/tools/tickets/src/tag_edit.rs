//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! An edit of a ticket's tags, for `--add-tags` and `--remove-tags`.

/// An edit of a ticket's tags against the ones it holds: the held ones less `remove`, then `add` not held.
#[derive(Debug, Clone, Default)]
pub struct TagEdit {
    pub add:    Vec<String>,
    pub remove: Vec<String>,
}

impl TagEdit {
    /// Whether it changes nothing.
    pub fn is_empty(&self) -> bool {
        self.add.is_empty() && self.remove.is_empty()
    }

    /// The list that results from `held`, in the order held and then added.
    pub fn applied(&self, held: &[String]) -> Vec<String> {
        let mut out: Vec<String> = held
            .iter()
            .filter(|t| !self.remove.contains(t))
            .cloned()
            .collect();
        for t in &self.add {
            if !out.contains(t) {
                out.push(t.clone());
            }
        }
        out
    }
}
