//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The one walk of a directory, and the one list of the directories under one.
//!
//! The lints and the tools each carried their own recursion for the files of an extension and their
//! own `read_dir` for the crates under `mock/crates`. Both read through a [`View`]: a lint the one it
//! is handed, so every directory walked is in its trace, and a tool one of its own over a
//! [`Reading`](mockspace::reading::Reading) of the repository. This file declares no lint.

use std::path::{Component, Path, PathBuf};

use mockspace::View;

/// Every file under `dir`, at any depth, whose extension is `ext` (without the dot), sorted. A directory
/// named in `skip` is not entered. A `dir` that is not there gives nothing, which is how a crate without
/// a `tests` directory is read.
pub fn with_extension(view: &View<'_>, dir: &Path, ext: &str, skip: &[&str]) -> Vec<PathBuf> {
    walk_files(
        view,
        dir,
        |_, name| skip.contains(&name),
        |path| path.extension().is_some_and(|e| e == ext),
    )
}

/// Every file under `dir`, at any depth, that `keep` admits, sorted. `skip` is asked of each directory
/// before it is entered, with its path relative to `dir` and its name. A `dir` that is not there gives
/// nothing.
pub fn walk_files(
    view: &View<'_>,
    dir: &Path,
    skip: impl Fn(&Path, &str) -> bool,
    keep: impl Fn(&Path) -> bool,
) -> Vec<PathBuf> {
    let given = if dir.is_absolute() { dir.to_path_buf() } else { view.root().join(dir) };
    // The view answers with `.` and `..` folded away, so what it gives is taken against the folded
    // name and handed back under the name the caller gave.
    let base = folded(&given);
    let mut out: Vec<PathBuf> = view
        .walk(&base, |abs, entry| {
            let rel = abs.parent().and_then(|p| p.strip_prefix(&base).ok()).unwrap_or(Path::new(""));
            mockspace::reading::Entry::is_dir(entry) && skip(rel, entry.name())
        })
        .into_iter()
        .filter_map(|abs| abs.strip_prefix(&base).ok().filter(|rel| keep(rel)).map(|rel| given.join(rel)))
        .collect();
    out.sort();
    out
}

/// `path` with `.` and `..` folded away by name, as the view names what it walks.
fn folded(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {},
            Component::ParentDir => {
                out.pop();
            },
            other => out.push(other),
        }
    }
    out
}

/// The directories directly under `dir`, as `(name, path)`, sorted by name. Files beside them are not
/// listed, and a `dir` that is not there gives nothing. The crates are `subdirs(view, &mock_dir.join("crates"))`.
pub fn subdirs(view: &View<'_>, dir: &Path) -> Vec<(String, PathBuf)> {
    let Some(listing) = view.list(dir) else {
        return Vec::new();
    };
    let mut out: Vec<(String, PathBuf)> = listing
        .entries()
        .iter()
        .filter(|e| mockspace::reading::Entry::is_dir(e))
        .map(|e| (e.name().to_owned(), dir.join(e.name())))
        .collect();
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};

    use super::*;
    use mockspace::reading::Reading;

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
        let r = Reading::new(&d);
        let v = r.view();
        let found = with_extension(&v, &d, "rs", &[]);
        assert_eq!(names(&found, &d), ["a/deep/c.rs", "b.rs"]);
        assert_eq!(names(&with_extension(&v, &d, "toml", &[]), &d), ["a/d.toml"]);
    }

    #[test]
    fn a_skipped_directory_is_not_entered_and_the_control_it_is_otherwise() {
        let d = scratch("skip");
        write(&d, "src/lib.rs", "");
        write(&d, "src/tests/t.rs", "");
        write(&d, "src/fixtures/f.rs", "");
        let r = Reading::new(&d);
        let v = r.view();
        let all = with_extension(&v, &d, "rs", &[]);
        assert_eq!(all.len(), 3, "{all:?}");
        let skipped = with_extension(&v, &d, "rs", &["tests", "fixtures"]);
        assert_eq!(names(&skipped, &d), ["src/lib.rs"]);
    }

    #[test]
    fn a_directory_named_through_a_parent_step_is_walked_and_answered_under_that_name() {
        // Mutation: the walk is matched against the name as given, which the view folds, so nothing is found.
        let d = scratch("parent-step");
        write(&d, "a/x.rs", "");
        write(&d, "b/keep", "");
        let r = Reading::new(&d);
        let through = d.join("b/../a");
        assert_eq!(with_extension(&r.view(), &through, "rs", &[]), [through.join("x.rs")]);
    }

    #[test]
    fn a_directory_that_is_not_there_gives_nothing() {
        let d = scratch("absent");
        let r = Reading::new(&d);
        let v = r.view();
        assert!(with_extension(&v, &d.join("nope"), "rs", &[]).is_empty());
        assert!(subdirs(&v, &d.join("nope")).is_empty());
    }

    #[test]
    fn a_skip_sees_where_a_directory_sits_so_a_root_only_name_is_skipped_at_the_root_alone() {
        let d = scratch("root-only");
        write(&d, "target/a.rs", "");
        write(&d, "src/target/b.rs", "");
        let r = Reading::new(&d);
        let found = walk_files(
            &r.view(),
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
        let found: Vec<String> = subdirs(&Reading::new(&d).view(), &d).into_iter().map(|(n, _)| n).collect();
        assert_eq!(found, ["kaski-a", "kaski-b"]);
    }
}
