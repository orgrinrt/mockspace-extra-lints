//! The files of a repository, for the lints that read tests wherever they sit.
//!
//! A crate lint is handed `src/**` of one crate, which leaves out an integration test under
//! `tests/` and any file that sits beside the crates, so a lint about tests walks the repository
//! itself. This is the one walk, taken through a [`View`]: it skips what is never authored (build
//! output, vendored sources, hidden directories) and names each file's crate by the first path
//! component that is one.

use std::collections::BTreeSet;
use std::path::Path;

use mockspace_lint_rules::{Entry, View, glob_match_anchored};

/// Directory names never entered, wherever they sit.
const SKIPPED: &[&str] = &["target", "vendor", "node_modules"];

/// The directories under the mock directory that are the record of what was once true: a probe or
/// a sketch in them is kept as it was written, and no lint about live code reads it.
const RECORD: &[&str] = &["design_rounds", "research"];

/// The crate name a file is given when no component of its path is a crate.
pub(crate) const WORKSPACE: &str = "workspace";

/// A `.rs` file of the repository, read.
pub(crate) struct RustFile {
    /// The path from the repository root, with forward slashes.
    pub rel: String,
    /// The crate the file sits in, or [`WORKSPACE`].
    pub crate_name: String,
    pub text: String,
}

/// Every readable `.rs` file under `root`, in path order, but for the record directories under
/// `mock_dir` and the paths matching one of the `skip` globs, each read through the view.
pub(crate) fn rust_files(
    view: &View<'_>,
    root: &Path,
    mock_dir: &Path,
    all_crates: &BTreeSet<String>,
    skip: &[String],
) -> Vec<RustFile> {
    let record = record_dirs(root, mock_dir);
    let mut out = Vec::new();
    for rel in paths(view, root, &record, |name, rel| {
        name.ends_with(".rs") && !skip.iter().any(|g| glob_match_anchored(g, rel))
    }) {
        let Some(text) = view.text(root.join(&rel)) else {
            continue;
        };
        out.push(RustFile {
            crate_name: crate_of(&rel, all_crates),
            rel,
            text: text.to_string(),
        });
    }
    out
}

/// Every file under `root` whose path from the root matches one of `globs`, in path order.
pub(crate) fn matching(view: &View<'_>, root: &Path, globs: &[String]) -> Vec<String> {
    if globs.is_empty() {
        return Vec::new();
    }
    paths(view, root, &[], |_, rel| globs.iter().any(|g| glob_match_anchored(g, rel)))
}

/// The record directories under `mock_dir`, as paths from `root`: the ones no walk enters.
pub(crate) fn record_dirs(root: &Path, mock_dir: &Path) -> Vec<String> {
    RECORD
        .iter()
        .filter_map(|d| mock_dir.join(d).strip_prefix(root).ok().map(slashed))
        .collect()
}

/// The crate a repository-relative path sits in.
pub(crate) fn crate_of(rel: &str, all_crates: &BTreeSet<String>) -> String {
    rel.split('/')
        .find(|c| all_crates.contains(*c))
        .unwrap_or(WORKSPACE)
        .to_string()
}

/// The paths under `root` that `accept` takes, given the file name and the path from the root, in
/// path order, without entering a hidden or skipped directory or one of the `skip` paths.
pub(crate) fn paths(
    view: &View<'_>,
    root: &Path,
    skip: &[String],
    accept: impl Fn(&str, &str) -> bool,
) -> Vec<String> {
    let rel_of = |path: &Path| {
        path.strip_prefix(root)
            .or_else(|_| path.strip_prefix(view.root()))
            .ok()
            .map(slashed)
    };
    let skipped = |path: &Path, entry: &Entry| {
        entry.is_dir()
            && (entry.name().starts_with('.')
                || SKIPPED.contains(&entry.name())
                || rel_of(path).is_some_and(|rel| skip.contains(&rel)))
    };
    let mut out: Vec<String> = view
        .walk(root, skipped)
        .iter()
        .filter_map(|path| {
            let rel = rel_of(path)?;
            let name = path.file_name()?.to_str()?;
            accept(name, &rel).then_some(rel)
        })
        .collect();
    out.sort();
    out
}

/// A path with forward slashes, whatever the platform.
fn slashed(p: &Path) -> String {
    p.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
#[path = "repo_scope_tests.rs"]
mod tests;
