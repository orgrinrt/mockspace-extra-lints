//! The Slang sources of a repository.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::path::Path;

use mockspace_lint_rules::{Item, View};

use super::{SlangFile, read};
use crate::repo_scope::{crate_of, paths, record_dirs};

/// A `.slang` file of the repository, read.
#[derive(Debug, Clone)]
pub struct SlangSource {
    /// The path from the repository root, with forward slashes.
    pub rel:        String,
    /// The crate the file sits in, or `workspace`.
    pub crate_name: String,
    pub text:       String,
    pub file:       SlangFile,
}

impl SlangSource {
    /// The file name without its extension: the name a module of that file is imported by.
    pub fn stem(&self) -> &str {
        let name = self.rel.rsplit('/').next().unwrap_or(&self.rel);
        name.strip_suffix(".slang").unwrap_or(name)
    }

    /// Whether the file sits in the `shaders/` directory of its crate, which is where a crate keeps the
    /// shaders it ships and the modules it offers. A file elsewhere, a test probe among them, is
    /// neither.
    pub fn shipped(&self) -> bool {
        let parts: Vec<&str> = self.rel.split('/').collect();
        parts.len() >= 2 && parts[parts.len() - 2] == "shaders"
    }
}

/// One `.slang` file of the repository, named and not yet read: where it is and the crate it sits in.
///
/// An item of [`View::each`], saved under the crate and the path together, since the crate is
/// worked out from the workspace's crates and no trace of the file holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlangPath {
    /// The path from the repository root, with forward slashes.
    pub rel:        String,
    /// The crate the file sits in, or `workspace`.
    pub crate_name: String,
}

impl Item for SlangPath {
    fn key(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{}\0{}", self.crate_name, self.rel))
    }
}

/// The `.slang` files under `root`, in path order, but for the record directories under `mock_dir`,
/// each with the crate it sits in. Nothing is read of them but the listings.
pub fn slang_paths(
    view: &View<'_>,
    root: &Path,
    mock_dir: &Path,
    all_crates: &BTreeSet<String>,
) -> Vec<SlangPath> {
    let record = record_dirs(root, mock_dir);
    paths(view, root, &record, |name, _| name.ends_with(".slang"))
        .into_iter()
        .map(|rel| SlangPath {
            crate_name: crate_of(&rel, all_crates),
            rel,
        })
        .collect()
}

/// The file `path` names, read through `view`, `None` where it cannot be read as text.
pub fn read_source(view: &View<'_>, root: &Path, path: &SlangPath) -> Option<SlangSource> {
    let text = view.text(root.join(&path.rel))?.to_string();
    Some(SlangSource {
        crate_name: path.crate_name.clone(),
        file: read(&text),
        rel: path.rel.clone(),
        text,
    })
}

/// Every readable `.slang` file under `root`, in path order, but for the record directories under
/// `mock_dir`, each read through the view.
pub fn sources(view: &View<'_>, root: &Path, mock_dir: &Path, all_crates: &BTreeSet<String>) -> Vec<SlangSource> {
    slang_paths(view, root, mock_dir, all_crates)
        .iter()
        .filter_map(|p| read_source(view, root, p))
        .collect()
}
