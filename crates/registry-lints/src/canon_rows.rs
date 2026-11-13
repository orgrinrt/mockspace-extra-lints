//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Reading the canon's tenets, from the flattened view a lint is handed and from the registry files.
//!
//! This file declares no lint. The engine registers only the files defining `lint()`, `cross_lint()`,
//! `repo_lint()` or `message_lint()` and includes every file as a module either way, so the canon lints
//! share these readers, after kenno's `canon_rows.rs`.
//!
//! The view does not say which file a row came from, and a lint cannot add a TOML reader to the pack,
//! so where a lint needs the file it reads the registry text itself: a `[[tenet]]` table opens a row,
//! and its `id = "..."` and `subject = "..."` lines are the two it asks about. Every registry file in
//! this repository writes a row that way.

use std::path::{Path, PathBuf};

use mockspace::{LintError, RegistryView, View};

use crate::tree_walk::with_extension;

/// The separator the engine joins a list field with.
pub const JOIN: &str = ", ";

/// One finding against the registry, at the severity the lint declares.
///
/// The location is the registry rather than a line: a line number into a registry file names a
/// different row as soon as one is added above it, so the row's address goes in the message.
pub fn finding(lint: &'static str, message: String) -> LintError {
    LintError::error("registry".to_string(), 0, lint, message)
}

/// The entries of a list field, or nothing where it is absent or blank.
pub fn list<'a>(reg: &'a RegistryView, q: &str, field: &str) -> Vec<&'a str> {
    reg.field(q, field)
        .map(|v| {
            v.split(JOIN)
                .map(str::trim)
                .filter(|e| !e.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// The slugs a relation field holds that point into one namespace: an item written
/// `<namespace>::<slug>` is a link to that row, and an item of another namespace or of more segments
/// (a citation) is not one.
pub fn linked<'a>(reg: &'a RegistryView, q: &str, field: &str, namespace: &str) -> Vec<&'a str> {
    list(reg, q, field)
        .into_iter()
        .filter_map(|item| {
            let (ns, row) = item.split_once("::")?;
            (ns == namespace && !row.contains("::")).then_some(row)
        })
        .collect()
}

/// A scalar field, trimmed, or `None` where it is absent or blank.
pub fn text<'a>(reg: &'a RegistryView, q: &str, field: &str) -> Option<&'a str> {
    reg.field(q, field).map(str::trim).filter(|v| !v.is_empty())
}

/// The slug half of `namespace::slug`.
pub fn slug(qualified: &str) -> &str {
    qualified.rsplit("::").next().unwrap_or(qualified)
}

/// The fingerprint of a row's content, the one definition of "unchanged" a `lineage` entry is held to:
/// FNV-1a over the `what`, a newline and the `because`, in sixteen hex digits.
pub fn fingerprint(what: &str, because: &str) -> String {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for b in what
        .bytes()
        .chain(std::iter::once(b'\n'))
        .chain(because.bytes())
    {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01B3);
    }
    format!("{h:016x}")
}

/// Whether a character can sit inside an id.
fn id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The words of `text` that are whole ids `is_id` accepts, in the order first named, each once.
pub fn named<'a>(text: &'a str, is_id: impl Fn(&str) -> bool) -> Vec<&'a str> {
    let mut out: Vec<&'a str> = Vec::new();
    for word in text.split(|c: char| !id_char(c)) {
        if !word.is_empty() && is_id(word) && !out.contains(&word) {
            out.push(word);
        }
    }
    out
}

/// A tenet as its registry file writes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// The file it sits in.
    pub file:    PathBuf,
    /// Its id.
    pub id:      String,
    /// The subject it names, where it names one.
    pub subject: Option<String>,
}

/// The quoted value of a `key = "value"` line, where the line is one.
fn quoted<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line
        .trim()
        .strip_prefix(key)?
        .trim_start()
        .strip_prefix('=')?
        .trim();
    rest.strip_prefix('"')?.strip_suffix('"')
}

/// Every tenet written in any registry file under `registry`.
pub fn written_tenets(view: &View<'_>, registry: &Path) -> Vec<Written> {
    let mut out = Vec::new();
    for file in with_extension(view, registry, "toml", &[]) {
        let Some(body) = view.text(&file) else {
            continue;
        };
        let mut open: Option<Written> = None;
        for line in body.lines() {
            let t = line.trim();
            if t.starts_with('[') {
                out.extend(open.take().filter(|w| !w.id.is_empty()));
                if t == "[[tenet]]" {
                    open = Some(Written {
                        file:    file.clone(),
                        id:      String::new(),
                        subject: None,
                    });
                }
                continue;
            }
            let Some(w) = open.as_mut() else {
                continue;
            };
            if let Some(v) = quoted(t, "id") {
                w.id = v.to_string();
            } else if let Some(v) = quoted(t, "subject") {
                w.subject = Some(v.to_string());
            }
        }
        out.extend(open.take().filter(|w| !w.id.is_empty()));
    }
    out
}

#[cfg(test)]
pub mod testkit {
    //! Building a registry view in a test, and running a repository lint over a directory the way the
    //! runner hands one over.

    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    use mockspace::testkit::check_repo;
    use mockspace::{LintError, RegistryView, RepoInput, RepoLint};

    /// A view over the given rows, each `(namespace::slug, [(field, value)])`.
    pub fn view(rows: &[(&str, &[(&str, &str)])]) -> RegistryView {
        let rows = rows
            .iter()
            .map(|(q, f)| {
                let f = f
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                    .collect();
                ((*q).to_string(), f)
            })
            .collect::<BTreeMap<String, BTreeMap<String, String>>>();
        RegistryView::new(rows, BTreeMap::new())
    }

    /// What `lint` finds over the repository at `dir`, which is its own mock directory, with `registry`,
    /// no crates, no canon paths and no open panels.
    pub fn check(lint: &dyn RepoLint, dir: &Path, registry: &RegistryView) -> Vec<LintError> {
        check_repo(lint, RepoInput {
            mock_dir: dir,
            repo_root: dir,
            all_crates: &BTreeSet::new(),
            src_dirs: &[],
            invocation: None,
            canon_paths: &[],
            open_panels: &[],
            ref_files: &mockspace::src_layout::NO_REF_FILES,
            registry,
        })
    }

    /// An empty mock directory nothing else is using.
    pub fn scratch(tag: &str) -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let d = std::env::temp_dir().join(format!(
            "canon-lint-{tag}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("registry")).expect("a scratch registry");
        d
    }

    /// Runs git in `d`, with no hooks, no signing and a fixed identity, and panics on a failure.
    pub fn git(d: &Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
            ])
            .arg("-C")
            .arg(d)
            .args(args)
            .output()
            .expect("git runs");
        assert!(
            out.status.success(),
            "git {args:?}: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// Writes `body` at `rel` under `dir`.
    pub fn write(dir: &Path, rel: &str, body: &str) {
        let p = dir.join(rel);
        std::fs::create_dir_all(p.parent().expect("a parent")).expect("its directory");
        std::fs::write(p, body).expect("the file");
    }
}

#[cfg(test)]
mod tests {
    use super::testkit::{scratch, write};
    use super::*;

    #[test]
    fn a_fingerprint_is_fixed_and_moves_with_either_field() {
        // FNV-1a of "a\nb" fixed at the reference value, so the stored entries never silently lapse
        assert_eq!(fingerprint("a", "b"), "e5beb1190415e670");
        assert_ne!(fingerprint("a", "b"), fingerprint("a", "c"));
        assert_ne!(fingerprint("a", "b"), fingerprint("x", "b"));
        // the newline separates, so moving a character across the join changes it
        assert_ne!(fingerprint("ab", ""), fingerprint("a", "b"));
    }

    #[test]
    fn a_tenet_is_read_with_its_file_and_subject_and_other_tables_are_not() {
        let d = scratch("rows");
        write(
            &d,
            "registry/tenet/sky.toml",
            "# sky\n\n[[tenet]]\nid = \"a\"\nwhat = \"subject = \\\"no\\\"\"\nsubject = \"sky\"\n\n[[tenet]]\nid = \"b\"\n",
        );
        write(&d, "registry/subject.toml", "[[subject]]\nid = \"sky\"\n");
        let w = written_tenets(&mockspace::reading::Reading::new(&d).view(), &d.join("registry"));
        let file = d.join("registry/tenet/sky.toml");
        assert_eq!(w, [
            Written {
                file:    file.clone(),
                id:      "a".into(),
                subject: Some("sky".into()),
            },
            Written {
                file,
                id: "b".into(),
                subject: None
            },
        ]);
    }

    #[test]
    fn a_name_is_a_whole_id() {
        let ids = ["the_sun", "the_sun_rises"];
        let is = |w: &str| ids.contains(&w);
        assert_eq!(named("`the_sun_rises`, then the_sun and the_sun", is), [
            "the_sun_rises",
            "the_sun"
        ]);
        assert!(named("the_sun_rises_late", is).is_empty());
    }
}
