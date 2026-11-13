//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What git says of a name a ticket mentions: held by the trunk once and held no more.

use kaski_tickets::Tree;

use crate::landing::Git;

/// The directories a crate named alone could have been in.
const CRATE_DIRS: [&str; 1] = ["mock/crates"];

/// The paths a name stands for: a crate in each directory crates live in, a path as it is.
fn paths(name: &str) -> Vec<String> {
    if name.contains('/') {
        vec![name.to_string()]
    } else {
        CRATE_DIRS.iter().map(|d| format!("{d}/{name}")).collect()
    }
}

impl Git<'_> {
    fn held(&self, trunk: &str, path: &str) -> bool {
        self.git(&["cat-file", "-e", &format!("{trunk}:{path}")])
            .is_some_and(|o| o.status.success())
    }

    /// The newest commit before the trunk that deleted something at `path`, as its short sha and subject.
    fn deleted(&self, trunk: &str, path: &str) -> Option<String> {
        let out = self.git(&[
            "--literal-pathspecs",
            "log",
            trunk,
            "--diff-filter=D",
            "--format=%h %s",
            "-1",
            "--",
            path,
        ])?;
        let line = String::from_utf8_lossy(&out.stdout).trim().to_string();
        (out.status.success() && !line.is_empty()).then_some(line)
    }
}

impl Tree for Git<'_> {
    fn gone(&self, name: &str) -> Option<String> {
        if let Some(said) = self.gone.borrow().get(name) {
            return said.clone();
        }
        let said = self.trunk().and_then(|trunk| {
            let all = paths(name);
            if all.iter().any(|p| self.held(&trunk, p)) {
                return None;
            }
            all.iter().find_map(|p| self.deleted(&trunk, p))
        });
        self.gone
            .borrow_mut()
            .insert(name.to_string(), said.clone());
        said
    }
}
