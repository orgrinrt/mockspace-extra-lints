//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Which placeholder markers a merge in progress carries in from its parents.
//!
//! While a merge is uncommitted its tree holds the markers of both sides, so a lint reading the tree
//! judges what the merge did not write. A marker the same file already held, with the same tag and the
//! same words, at `HEAD` or at `MERGE_HEAD` is the parent's: it passed or failed its own gate there, and
//! the merge neither adds nor changes it. A marker found in neither is the merge's own, and so is one
//! whose words the resolution changed.
//!
//! Git is asked through the view: the answer is the repository's state when the reading first asked
//! it, and the question is in the unit's trace, so a saved finding replays only while git answers the
//! same.

use std::collections::BTreeSet;
use std::path::Path;

use mockspace::View;

use crate::fixme_markers::{self, Kind};

/// What a marker is by, apart from where it sits: its tag and its words.
pub type Identity = (Option<String>, String);

/// The output of `git` run in `repo`, where it succeeded and printed text.
pub fn git(view: &View<'_>, repo: &Path, args: &[&str]) -> Option<String> {
    let answer = view.git_in(repo, args);
    if !answer.ok() {
        return None;
    }
    String::from_utf8(answer.stdout().to_vec()).ok()
}

/// Whether a merge is in progress in the repository at `repo`.
pub fn in_progress(view: &View<'_>, repo: &Path) -> bool {
    git(view, repo, &["rev-parse", "-q", "--verify", "MERGE_HEAD"]).is_some()
}

/// The markers `rel` held at `HEAD` and at `MERGE_HEAD`, read as a file of kind `kind`.
///
/// A parent that does not hold the file contributes none.
pub fn carried(view: &View<'_>, repo: &Path, rel: &str, kind: Kind) -> BTreeSet<Identity> {
    let mut out = BTreeSet::new();
    for rev in ["HEAD", "MERGE_HEAD"] {
        let Some(text) = git(view, repo, &["show", &format!("{rev}:{rel}")]) else {
            continue;
        };
        for m in fixme_markers::scan(&text, kind) {
            out.insert(identity(&m));
        }
    }
    out
}

/// A marker's identity.
pub fn identity(m: &fixme_markers::Marker) -> Identity {
    (
        m.tag.as_deref().map(|t| t.trim().to_string()),
        m.reason.clone(),
    )
}
