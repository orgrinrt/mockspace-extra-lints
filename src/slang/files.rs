//! The Slang sources of a repository.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::{SlangFile, read};
use crate::repo_files::{crate_of, paths, record_dirs};

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

/// Every readable `.slang` file under `root`, in path order, but for the record directories under
/// `mock_dir`.
pub fn sources(root: &Path, mock_dir: &Path, all_crates: &BTreeSet<String>) -> Vec<SlangSource> {
    let record = record_dirs(root, mock_dir);
    paths(root, &record, |name, _| name.ends_with(".slang"))
        .into_iter()
        .filter_map(|rel| {
            let text = fs::read_to_string(root.join(&rel)).ok()?;
            Some(SlangSource {
                crate_name: crate_of(&rel, all_crates),
                file: read(&text),
                rel,
                text,
            })
        })
        .collect()
}
