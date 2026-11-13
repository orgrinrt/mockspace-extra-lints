//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Which crates a branch's diff touches.

use std::collections::BTreeSet;

use registry_lints::layout::Layout;

/// The crate a repository path belongs to: the directory under `crates/` or `tools/` of the mock
/// directory `layout` names. A path that is not inside a crate has none.
pub fn crate_of<'a>(path: &'a str, layout: &Layout) -> Option<&'a str> {
    let parts: Vec<&str> = layout.under_mock(path)?.split('/').collect();
    if parts.len() < 3 || !matches!(parts[0], "crates" | "tools") {
        return None;
    }
    Some(parts[1])
}

/// The crates the paths belong to.
pub fn touched(paths: &[String], layout: &Layout) -> BTreeSet<String> {
    paths
        .iter()
        .filter_map(|p| crate_of(p, layout))
        .map(str::to_owned)
        .collect()
}
