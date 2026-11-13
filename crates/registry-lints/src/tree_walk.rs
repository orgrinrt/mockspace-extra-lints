//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The one walk of a directory, and the one list of the directories under one.
//!
//! The lints and the tools each carried their own recursion for the files of an extension and their
//! own `read_dir` for the crates under `mock/crates`. This file declares no lint. It uses `std` only
//! and carries no test that needs the lints' testkit, so a tool including it by path, as `canon`
//! includes `canon_tree.rs`, needs nothing more.

use std::path::{Path, PathBuf};

/// Every file under `dir`, at any depth, whose extension is `ext` (without the dot), sorted. A directory
/// named in `skip` is not entered. A `dir` that is not there gives nothing, which is how a crate without
/// a `tests` directory is read.
pub fn with_extension(dir: &Path, ext: &str, skip: &[&str]) -> Vec<PathBuf> {
    walk_files(
        dir,
        |_, name| skip.contains(&name),
        |path| path.extension().is_some_and(|e| e == ext),
    )
}

/// Every file under `dir`, at any depth, that `keep` admits, sorted. `skip` is asked of each directory
/// before it is entered, with its path relative to `dir` and its name. A `dir` that is not there gives
/// nothing.
pub fn walk_files(
    dir: &Path,
    skip: impl Fn(&Path, &str) -> bool,
    keep: impl Fn(&Path) -> bool,
) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(dir, Path::new(""), &skip, &keep, &mut out);
    out.sort();
    out
}

fn walk(
    root: &Path,
    rel: &Path,
    skip: &impl Fn(&Path, &str) -> bool,
    keep: &impl Fn(&Path) -> bool,
    out: &mut Vec<PathBuf>,
) {
    let Ok(entries) = std::fs::read_dir(root.join(rel)) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let child = rel.join(&name);
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if !skip(rel, &name) {
                walk(root, &child, skip, keep, out);
            }
        } else if kind.is_file() && keep(&child) {
            out.push(root.join(child));
        }
    }
}

/// The directories directly under `dir`, as `(name, path)`, sorted by name. Files beside them are not
/// listed, and a `dir` that is not there gives nothing. The crates are `subdirs(&mock_dir.join("crates"))`.
pub fn subdirs(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
        .collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let d = std::env::temp_dir().join(format!(
            "kaski-tree-walk-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("a scratch directory");
        d
    }

    fn write(dir: &Path, rel: &str, body: &str) {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().expect("a parent")).expect("a directory");
        std::fs::write(p, body).expect("a file");
    }

    fn names(found: &[PathBuf], under: &Path) -> Vec<String> {
        found
            .iter()
            .map(|p| {
                p.strip_prefix(under)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    #[test]
    fn the_files_of_an_extension_are_found_at_any_depth_and_sorted() {
        let d = scratch("depth");
        write(&d, "b.rs", "");
        write(&d, "a/deep/c.rs", "");
        write(&d, "a/d.toml", "");
        write(&d, "e.rs.bak", "");
        let found = with_extension(&d, "rs", &[]);
        assert_eq!(names(&found, &d), ["a/deep/c.rs", "b.rs"]);
        assert_eq!(names(&with_extension(&d, "toml", &[]), &d), ["a/d.toml"]);
    }

    #[test]
    fn a_skipped_directory_is_not_entered_and_the_control_it_is_otherwise() {
        let d = scratch("skip");
        write(&d, "src/lib.rs", "");
        write(&d, "src/tests/t.rs", "");
        write(&d, "src/fixtures/f.rs", "");
        let all = with_extension(&d, "rs", &[]);
        assert_eq!(all.len(), 3, "{all:?}");
        let skipped = with_extension(&d, "rs", &["tests", "fixtures"]);
        assert_eq!(names(&skipped, &d), ["src/lib.rs"]);
    }

    #[test]
    fn a_directory_that_is_not_there_gives_nothing() {
        let d = scratch("absent");
        assert!(with_extension(&d.join("nope"), "rs", &[]).is_empty());
        assert!(subdirs(&d.join("nope")).is_empty());
    }

    #[test]
    fn a_skip_sees_where_a_directory_sits_so_a_root_only_name_is_skipped_at_the_root_alone() {
        let d = scratch("root-only");
        write(&d, "target/a.rs", "");
        write(&d, "src/target/b.rs", "");
        let found = walk_files(
            &d,
            |rel, name| rel.as_os_str().is_empty() && name == "target",
            |p| p.extension().is_some_and(|e| e == "rs"),
        );
        assert_eq!(names(&found, &d), ["src/target/b.rs"]);
    }

    #[test]
    fn the_directories_under_one_are_listed_by_name_and_the_files_beside_them_are_not() {
        let d = scratch("subdirs");
        write(&d, "kaski-b/Cargo.toml", "");
        write(&d, "kaski-a/Cargo.toml", "");
        write(&d, "README.md", "");
        let found: Vec<String> = subdirs(&d).into_iter().map(|(n, _)| n).collect();
        assert_eq!(found, ["kaski-a", "kaski-b"]);
    }
}
