//! Which crates to read, and their declarations.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use mockspace_lint_rules::{CrateSourceFile, DeclaredType, declared_types};

use crate::repo_files::{WORKSPACE, crate_of, rust_files};

/// The source of every named crate (or of every crate, when `only` is `None`), as the crate-relative
/// files `declared_types` reads: what sits under the crate's own `src/`.
pub fn sources(
    root: &Path,
    mock_dir: &Path,
    all_crates: &BTreeSet<String>,
    only: Option<&BTreeSet<String>>,
) -> Sources {
    let mut by_crate: BTreeMap<String, Vec<CrateSourceFile>> = BTreeMap::new();
    for file in rust_files(root, mock_dir, all_crates, &[]) {
        if file.crate_name == WORKSPACE || only.is_some_and(|set| !set.contains(&file.crate_name)) {
            continue;
        }
        let mut parts = file.rel.split('/');
        parts.find(|part| *part == file.crate_name);
        let inside: Vec<&str> = parts.collect();
        if inside.first() != Some(&"src") {
            continue;
        }
        by_crate
            .entry(file.crate_name)
            .or_default()
            .push(CrateSourceFile {
                rel_path: PathBuf::from(inside.join("/")),
                text:     file.text,
            });
    }
    by_crate
}

/// The source files of each crate by name, each file's path relative to its crate.
pub type Sources = BTreeMap<String, Vec<CrateSourceFile>>;

/// Every type the crates declare outside test code.
pub fn declarations(sources: &Sources) -> Vec<DeclaredType> {
    sources
        .iter()
        .flat_map(|(name, files)| declared_types(name, files))
        .collect()
}

/// The crates a branch touched against `against`: what the branch committed since it left it, and
/// what is changed in the tree now. `Err` carries what git said when it could not answer.
pub fn touched(
    root: &Path,
    all_crates: &BTreeSet<String>,
    against: &str,
) -> Result<BTreeSet<String>, String> {
    let mut crates = BTreeSet::new();
    for args in [vec!["diff", "--name-only", &format!("{against}...HEAD")], vec![
        "diff",
        "--name-only",
        "HEAD",
    ]] {
        let out = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(&args)
            .output()
            .map_err(|e| format!("could not run git: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "`git {}` failed: {}",
                args.join(" "),
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        for path in String::from_utf8_lossy(&out.stdout).lines() {
            let name = crate_of(path, all_crates);
            if name != WORKSPACE {
                crates.insert(name);
            }
        }
    }
    Ok(crates)
}

/// The crates, outside `touched`, that declare a type some crate in `touched` names: a word of its
/// source that is also the name of a type declared elsewhere.
pub fn named_from(
    types: &[DeclaredType],
    sources: &Sources,
    touched: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut words: BTreeSet<&str> = BTreeSet::new();
    for (name, files) in sources {
        if touched.contains(name) {
            words.extend(files.iter().flat_map(|f| {
                f.text
                    .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .filter(|w| !w.is_empty())
            }));
        }
    }
    types
        .iter()
        .filter(|t| !touched.contains(&t.crate_name) && words.contains(t.name.as_str()))
        .map(|t| t.crate_name.clone())
        .collect()
}
