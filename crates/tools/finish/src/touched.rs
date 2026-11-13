//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Which crates a branch's diff touches.

use std::collections::BTreeSet;

/// The crate a repository path belongs to: the directory under `mock/crates/` or `mock/tools/`. A path that is not
/// inside a crate has none.
pub fn crate_of(path: &str) -> Option<&str> {
    let parts: Vec<&str> = path.split('/').collect();
    if parts.first() != Some(&"mock") || parts.len() < 4 {
        return None;
    }
    let at = match parts[1] {
        "crates" | "tools" => 1,
        _ => return None,
    };
    // a file directly in the crates directory is no crate's
    (parts.len() > at + 2).then(|| parts[at + 1])
}

/// The crates the paths belong to.
pub fn touched(paths: &[String]) -> BTreeSet<String> {
    paths
        .iter()
        .filter_map(|p| crate_of(p))
        .map(str::to_owned)
        .collect()
}
