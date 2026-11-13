//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The lines a change adds or touches, read from git.
//!
//! A gate that lets debt stand has to say what is new, and a lint reading the whole tree cannot. The
//! change is what the work tree holds that the trunk does not: the diff of the work tree against a base,
//! and every file git does not track yet, whole. The base is where the change started, the merge base of
//! `HEAD` and the trunk; while a merge of work already on the trunk is being committed it is the trunk's
//! tip, because the commit being made holds everything the trunk has and the lines the merge brings in are
//! the trunk's. Where no merge base can be found, in a clone cut too shallow, the trunk's tip stands in,
//! which counts a line the trunk lacks and so errs towards refusing.
//!
//! A line a change touches is a line it adds: one it rewrites is a removed line and an added one, and the
//! added one is the line now in the file. A file git moves without changing a line touches none.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use mockspace::View;

use crate::merge_carried::{git, in_progress};

/// The refs the trunk is looked for under, the first that exists.
pub const TRUNKS: [&str; 2] = ["origin/dev", "dev"];

/// Which lines of one file a change touches.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Lines {
    /// Every line: the file is new to git.
    All,
    /// These lines, from one.
    Some(BTreeSet<usize>),
}

/// The lines of the files a change touches.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Touched {
    files: BTreeMap<String, Lines>,
}

impl Touched {
    /// Whether the change adds or touches line `line` (from one) of `path`, relative to the repository root.
    pub fn holds(&self, path: &str, line: usize) -> bool {
        match self.files.get(path) {
            Some(Lines::All) => true,
            Some(Lines::Some(lines)) => lines.contains(&line),
            None => false,
        }
    }

    /// The lines a unified diff with no context adds, per file. The hunk header says how many lines
    /// of each side follow, and those are counted off rather than recognised by their look, since an added
    /// line can read like a file header.
    pub fn from_diff(diff: &str) -> Self {
        let mut files: BTreeMap<String, Lines> = BTreeMap::new();
        let mut current: Option<String> = None;
        let (mut old, mut new, mut next) = (0usize, 0usize, 0usize);
        for line in diff.lines() {
            if old > 0 || new > 0 {
                match line.as_bytes().first() {
                    Some(b'-') => old = old.saturating_sub(1),
                    Some(b'+') => {
                        new = new.saturating_sub(1);
                        if let Some(Lines::Some(lines)) =
                            current.as_ref().and_then(|p| files.get_mut(p))
                        {
                            lines.insert(next);
                        }
                        next += 1;
                    },
                    _ => {},
                }
                continue;
            }
            if let Some(path) = line.strip_prefix("+++ ") {
                current = path
                    .trim_end_matches('\t')
                    .strip_prefix("b/")
                    .map(str::to_owned);
                if let Some(p) = &current {
                    files
                        .entry(p.clone())
                        .or_insert_with(|| Lines::Some(BTreeSet::new()));
                }
            } else if let Some(head) = line.strip_prefix("@@ ") {
                (old, next, new) = hunk(head).unwrap_or((0, 0, 0));
            }
        }
        Self {
            files,
        }
    }

    /// Adds every line of `path`, a file git does not track.
    fn add_whole(&mut self, path: &str) {
        self.files.insert(path.to_owned(), Lines::All);
    }
}

/// `(lines removed, first line added, lines added)` of a hunk header's `-12,3 +14,2 @@ ...`.
fn hunk(head: &str) -> Option<(usize, usize, usize)> {
    let mut parts = head.split_whitespace();
    let (_, old_count) = range(parts.next()?.strip_prefix('-')?)?;
    let (start, new_count) = range(parts.next()?.strip_prefix('+')?)?;
    Some((old_count, start, new_count))
}

/// `(start, count)` of `12,3`, the count being one where git leaves it out.
fn range(text: &str) -> Option<(usize, usize)> {
    match text.split_once(',') {
        Some((start, count)) => Some((start.parse().ok()?, count.parse().ok()?)),
        None => Some((text.parse().ok()?, 1)),
    }
}

/// What a change is: the lines it touches, and the commit it started from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub touched: Touched,
    pub base:    String,
}

/// The commit the change at `repo` started from, or `None` where there is no trunk to compare with.
pub fn base(view: &View<'_>, repo: &Path) -> Option<String> {
    let trunk = TRUNKS.iter().find_map(|name| {
        git(view, repo, &[
            "rev-parse",
            "-q",
            "--verify",
            &format!("{name}^{{commit}}"),
        ])
    })?;
    let trunk = trunk.trim().to_owned();
    if in_progress(view, repo) {
        let on_trunk = git(view, repo, &["rev-parse", "-q", "--verify", "MERGE_HEAD"])
            .map(|m| m.trim().to_owned())
            .is_some_and(|m| git(view, repo, &["merge-base", "--is-ancestor", &m, &trunk]).is_some());
        if on_trunk {
            return Some(trunk);
        }
    }
    Some(
        git(view, repo, &["merge-base", "HEAD", &trunk])
            .map(|b| b.trim().to_owned())
            .filter(|b| !b.is_empty())
            .unwrap_or(trunk),
    )
}

/// The change at `repo`, or what stopped it being read.
pub fn read(view: &View<'_>, repo: &Path) -> Result<Change, String> {
    let base = base(view, repo).ok_or_else(|| {
        "there is no `origin/dev` or `dev` to compare the change with; fetch `origin dev`"
            .to_owned()
    })?;
    // The prefixes are named so a user's `diff.noprefix` or `diff.mnemonicPrefix` cannot change what is read.
    let diff = git(view, repo, &[
        "-c",
        "core.quotepath=false",
        "diff",
        "--no-ext-diff",
        "--no-textconv",
        "--no-color",
        "--src-prefix=a/",
        "--dst-prefix=b/",
        "-U0",
        "-M",
        &base,
        "--",
    ])
    .ok_or_else(|| format!("`git diff {base}` failed in `{}`", repo.display()))?;
    let mut touched = Touched::from_diff(&diff);
    let untracked = git(view, repo, &[
        "-c",
        "core.quotepath=false",
        "ls-files",
        "--others",
        "--exclude-standard",
    ])
    .ok_or_else(|| format!("`git ls-files` failed in `{}`", repo.display()))?;
    for path in untracked.lines().filter(|l| !l.is_empty()) {
        touched.add_whole(path);
    }
    Ok(Change {
        touched,
        base,
    })
}

/// `rel` as it stood at `rev`, or `None` where it did not exist.
pub fn file_at(view: &View<'_>, repo: &Path, rev: &str, rel: &str) -> Option<String> {
    git(view, repo, &["show", &format!("{rev}:{rel}")])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canon_rows::testkit::{git as run, scratch, write};
    use mockspace::reading::Reading;

    /// The change at `d`, read through a reading of its own, since a reading asks git once.
    fn change(d: &Path) -> Result<Change, String> {
        read(&Reading::new(d).view(), d)
    }

    /// What git answers in `d`, through a reading of its own.
    fn ask(d: &Path, args: &[&str]) -> Option<String> {
        git(&Reading::new(d).view(), d, args)
    }

    /// A repository whose `dev` holds `file` as `text`, on a branch `work` cut from it.
    fn on_work(file: &str, text: &str) -> std::path::PathBuf {
        let d = scratch("touched");
        run(&d, &["init", "-q", "-b", "dev"]);
        write(&d, file, text);
        run(&d, &["add", file]);
        run(&d, &["commit", "-q", "-m", "base"]);
        run(&d, &["switch", "-q", "-c", "work"]);
        d
    }

    #[test]
    fn a_diff_gives_the_lines_it_adds_and_a_rewritten_line_counts_as_added() {
        let diff = "diff --git a/f.rs b/f.rs\n--- a/f.rs\n+++ b/f.rs\n@@ -2 +2 @@\n-old\n+new\n@@ -9,0 +10,2 @@\n\
                    +a\n+b\n@@ -20,2 +23,0 @@\n-x\n-y\n";
        let t = Touched::from_diff(diff);
        assert!(t.holds("f.rs", 2));
        assert!(t.holds("f.rs", 10) && t.holds("f.rs", 11));
        assert!(!t.holds("f.rs", 1) && !t.holds("f.rs", 12) && !t.holds("f.rs", 23));
        assert!(!t.holds("other.rs", 2));
    }

    #[test]
    fn an_added_line_that_reads_like_a_file_header_is_a_line_and_not_a_file() {
        let diff = "--- a/f.rs\n+++ b/f.rs\n@@ -1,0 +2,2 @@\n+++ b/evil.rs\n+@@ -1 +9 @@\n--- a/g.rs\n+++ b/g.rs\n@@ -3 +3 @@\n-p\n+q\n";
        let t = Touched::from_diff(diff);
        assert!(t.holds("f.rs", 2) && t.holds("f.rs", 3));
        assert!(!t.holds("evil.rs", 1) && !t.holds("f.rs", 9));
        assert!(
            t.holds("g.rs", 3),
            "the real header after them is still read"
        );
    }

    #[test]
    fn a_deleted_file_touches_nothing() {
        let diff = "--- a/gone.rs\n+++ /dev/null\n@@ -1,3 +0,0 @@\n-a\n-b\n-c\n";
        let t = Touched::from_diff(diff);
        assert!(!t.holds("gone.rs", 1) && !t.holds("/dev/null", 1));
    }

    #[test]
    fn a_line_with_no_newline_at_the_end_is_counted_as_the_line_it_is() {
        let diff = "--- a/f.rs\n+++ b/f.rs\n@@ -1 +1 @@\n-a\n\\ No newline at end of file\n+b\n\\ No newline at end of file\n\
                    @@ -5,0 +6 @@\n+c\n";
        let t = Touched::from_diff(diff);
        assert!(t.holds("f.rs", 1) && t.holds("f.rs", 6));
        assert!(!t.holds("f.rs", 5));
    }

    #[test]
    fn the_change_is_what_the_work_tree_holds_that_the_trunk_does_not() {
        let d = on_work("mock/a.rs", "one\ntwo\nthree\n");
        write(&d, "mock/a.rs", "one\nTWO\nthree\nfour\n");
        write(&d, "mock/new.rs", "x\ny\n");
        let c = change(&d).expect("a change");
        assert!(c.touched.holds("mock/a.rs", 2) && c.touched.holds("mock/a.rs", 4));
        assert!(!c.touched.holds("mock/a.rs", 1) && !c.touched.holds("mock/a.rs", 3));
        assert!(
            c.touched.holds("mock/new.rs", 1) && c.touched.holds("mock/new.rs", 99),
            "an untracked file is touched whole"
        );
        assert_eq!(
            c.base,
            ask(&d, &["rev-parse", "dev"]).unwrap().trim(),
            "the base is where the work was cut from"
        );
    }

    #[test]
    fn a_staged_edit_and_a_committed_one_are_both_in_the_change() {
        let d = on_work("mock/a.rs", "one\ntwo\nthree\n");
        write(&d, "mock/a.rs", "ONE\ntwo\nthree\n");
        run(&d, &["commit", "-q", "-am", "first"]);
        write(&d, "mock/a.rs", "ONE\ntwo\nTHREE\n");
        run(&d, &["add", "mock/a.rs"]);
        let c = change(&d).expect("a change");
        assert!(c.touched.holds("mock/a.rs", 1) && c.touched.holds("mock/a.rs", 3));
        assert!(!c.touched.holds("mock/a.rs", 2));
    }

    #[test]
    fn a_file_moved_without_a_changed_line_touches_none() {
        let d = on_work("mock/old.rs", "one\ntwo\nthree\nfour\nfive\nsix\n");
        run(&d, &["mv", "mock/old.rs", "mock/moved.rs"]);
        let c = change(&d).expect("a change");
        assert!(!c.touched.holds("mock/moved.rs", 2), "{c:?}");
    }

    #[test]
    fn work_the_trunk_gained_since_the_cut_is_not_the_changes() {
        let d = on_work("mock/a.rs", "one\ntwo\n");
        write(&d, "mock/b.rs", "mine\n");
        run(&d, &["add", "mock/b.rs"]);
        run(&d, &["commit", "-q", "-m", "mine"]);
        run(&d, &["switch", "-q", "dev"]);
        write(&d, "mock/c.rs", "theirs\n");
        run(&d, &["add", "mock/c.rs"]);
        run(&d, &["commit", "-q", "-m", "theirs"]);
        run(&d, &["switch", "-q", "work"]);
        let c = change(&d).expect("a change");
        assert!(c.touched.holds("mock/b.rs", 1));
        assert!(!c.touched.holds("mock/c.rs", 1), "{c:?}");
    }

    #[test]
    fn a_line_the_trunk_rewrote_since_the_cut_is_not_the_changes_though_the_work_still_holds_the_old_one()
     {
        // Mutation: the trunk's tip is the base in place of the merge base, which counts the work's old line.
        let d = on_work("mock/a.rs", "one\ntwo\nthree\n");
        write(&d, "mock/b.rs", "mine\n");
        run(&d, &["add", "mock/b.rs"]);
        run(&d, &["commit", "-q", "-m", "mine"]);
        run(&d, &["switch", "-q", "dev"]);
        write(&d, "mock/a.rs", "one\nTWO\nthree\n");
        run(&d, &["commit", "-q", "-am", "theirs"]);
        run(&d, &["switch", "-q", "work"]);
        let c = change(&d).expect("a change");
        assert!(c.touched.holds("mock/b.rs", 1));
        assert!(!c.touched.holds("mock/a.rs", 2), "{c:?}");
    }

    #[test]
    fn a_clone_too_shallow_to_hold_the_fork_point_is_compared_with_the_trunks_tip() {
        // Mutation: no merge base is read as an empty base, which no diff can start from.
        let origin = on_work("mock/a.rs", "one\ntwo\n");
        write(&origin, "mock/b.rs", "mine\n");
        run(&origin, &["add", "mock/b.rs"]);
        run(&origin, &["commit", "-q", "-m", "mine"]);
        run(&origin, &["switch", "-q", "dev"]);
        write(&origin, "mock/a.rs", "one\ntwo\nthree\n");
        run(&origin, &["commit", "-q", "-am", "theirs"]);
        let url = format!("file://{}", origin.display());
        let clone = scratch("touched-shallow");
        std::fs::remove_dir_all(&clone).expect("an empty place for the clone");
        let clone_arg = clone.to_string_lossy().into_owned();
        run(&origin, &[
            "clone", "-q", "--depth", "1", "--branch", "work", &url, &clone_arg,
        ]);
        run(&clone, &[
            "fetch",
            "-q",
            "--depth",
            "1",
            "origin",
            "dev:refs/remotes/origin/dev",
        ]);
        assert!(
            ask(&clone, &["merge-base", "HEAD", "origin/dev"]).is_none(),
            "the clone is meant to hold no fork point"
        );
        let c = change(&clone).expect("a change");
        assert_eq!(
            c.base,
            ask(&clone, &["rev-parse", "origin/dev"]).unwrap().trim()
        );
        assert!(c.touched.holds("mock/b.rs", 1), "{c:?}");
    }

    #[test]
    fn a_merge_of_the_trunk_into_the_work_leaves_the_trunks_lines_out_of_the_change() {
        let d = on_work("mock/a.rs", "one\ntwo\n");
        write(&d, "mock/b.rs", "mine\n");
        run(&d, &["add", "mock/b.rs"]);
        run(&d, &["commit", "-q", "-m", "mine"]);
        run(&d, &["switch", "-q", "dev"]);
        write(&d, "mock/c.rs", "theirs\n");
        run(&d, &["add", "mock/c.rs"]);
        run(&d, &["commit", "-q", "-m", "theirs"]);
        run(&d, &["switch", "-q", "work"]);
        run(&d, &["merge", "-q", "--no-commit", "--no-ff", "dev"]);
        let c = change(&d).expect("a change");
        assert!(c.touched.holds("mock/b.rs", 1));
        assert!(
            !c.touched.holds("mock/c.rs", 1),
            "the merge being committed brings the trunk's line in, and it is not the work's: {c:?}"
        );
    }

    #[test]
    fn a_clone_with_no_trunk_has_no_change_to_read() {
        let d = scratch("touched-none");
        run(&d, &["init", "-q", "-b", "work"]);
        write(&d, "f.rs", "x\n");
        run(&d, &["add", "f.rs"]);
        run(&d, &["commit", "-q", "-m", "base"]);
        let why = change(&d).expect_err("nothing to compare with");
        assert!(why.contains("origin/dev"), "{why}");
    }

    #[test]
    fn a_directory_that_is_no_repository_has_no_change_to_read() {
        assert!(change(&scratch("touched-no-git")).is_err());
    }

    #[test]
    fn file_at_reads_a_file_as_the_base_held_it_and_none_where_it_did_not() {
        let d = on_work("mock/a.rs", "one\n");
        write(&d, "mock/a.rs", "two\n");
        let base = change(&d).expect("a change").base;
        assert_eq!(file_at(&Reading::new(&d).view(), &d, &base, "mock/a.rs").as_deref(), Some("one\n"));
        assert_eq!(file_at(&Reading::new(&d).view(), &d, &base, "mock/none.rs"), None);
    }
}
