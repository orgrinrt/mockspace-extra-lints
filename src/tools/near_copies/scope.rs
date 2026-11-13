//! Which crates to read, and their declarations.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use mockspace_lint_rules::{CrateSourceFile, DeclaredType, Reading, View, declared_types};

use crate::repo_scope::{WORKSPACE, crate_of};

/// The source of every crate, read from the directories the engine holds source packages in and
/// the way its lints read a crate: what sits under the crate's own `src/`, `src/lib.rs` first. A
/// crate found under two of the directories is read from the first. Every file is held by
/// `reading`, which the result borrows.
pub fn sources<'r>(reading: &'r Reading, src_dirs: &[PathBuf], all_crates: &BTreeSet<String>) -> Sources<'r> {
    let mut by_crate: Sources<'r> = BTreeMap::new();
    for dir in src_dirs {
        for name in all_crates {
            if by_crate.contains_key(name) {
                continue;
            }
            let files = reading.crate_sources(&dir.join(name));
            if !files.is_empty() {
                by_crate.insert(name.clone(), files);
            }
        }
    }
    by_crate
}

/// The source files of each crate by name, each file's path relative to its crate.
pub type Sources<'r> = BTreeMap<String, &'r [CrateSourceFile<'r>]>;

/// Every type the crates declare outside test code.
pub fn declarations(sources: &Sources<'_>) -> Vec<DeclaredType> {
    sources
        .iter()
        .flat_map(|(name, files)| {
            let pairs: Vec<(&Path, &str)> = files.iter().map(|f| (f.rel_path, f.text)).collect();
            declared_types(name, &pairs)
        })
        .collect()
}

/// The crates a branch touched against `against`: what the branch committed since it left it, and
/// what is changed in the tree now. `Err` carries what git said when it could not answer.
pub fn touched(
    view: &View<'_>,
    root: &Path,
    all_crates: &BTreeSet<String>,
    against: &str,
) -> Result<BTreeSet<String>, String> {
    let mut crates = BTreeSet::new();
    // Committed since the branch left the trunk, changed in the tree now, and written and not yet
    // added: a crate a helper has just made is touched before git tracks it.
    let range = format!("{against}...HEAD");
    for args in [
        vec!["diff", "--name-only", range.as_str()],
        vec!["diff", "--name-only", "HEAD"],
        vec!["ls-files", "--others", "--exclude-standard"],
    ] {
        let out = view.git_in(root, &args);
        if !out.started() {
            return Err("could not run git".to_string());
        }
        if !out.ok() {
            return Err(format!(
                "`git {}` failed: {}",
                args.join(" "),
                String::from_utf8_lossy(out.stderr()).trim()
            ));
        }
        for path in String::from_utf8_lossy(out.stdout()).lines() {
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
    sources: &Sources<'_>,
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
