//! The files of a repository, for the lints that read tests wherever they sit.
//!
//! A crate lint is handed `src/**` of one crate, which leaves out an integration test under
//! `tests/` and any file that sits beside the crates, so a lint about tests walks the repository
//! itself. This is the one walk: it skips what is never authored (build output, vendored sources,
//! hidden directories) and names each file's crate by the first path component that is one.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Directory names never entered, wherever they sit.
const SKIPPED: &[&str] = &["target", "vendor", "node_modules"];

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

/// Every readable `.rs` file under `root`, in path order.
pub(crate) fn rust_files(root: &Path, all_crates: &BTreeSet<String>) -> Vec<RustFile> {
    let mut out = Vec::new();
    for rel in paths(root, |name, _| name.ends_with(".rs")) {
        let Ok(text) = fs::read_to_string(root.join(&rel)) else {
            continue;
        };
        out.push(RustFile {
            crate_name: crate_of(&rel, all_crates),
            rel,
            text,
        });
    }
    out
}

/// Every file under `root` whose path from the root matches one of `globs`, in path order.
pub(crate) fn matching(root: &Path, globs: &[String]) -> Vec<String> {
    if globs.is_empty() {
        return Vec::new();
    }
    paths(root, |_, rel| globs.iter().any(|g| glob(g, rel)))
}

/// The crate a repository-relative path sits in.
fn crate_of(rel: &str, all_crates: &BTreeSet<String>) -> String {
    rel.split('/')
        .find(|c| all_crates.contains(*c))
        .unwrap_or(WORKSPACE)
        .to_string()
}

/// The paths under `root` that `accept` takes, given the file name and the path from the root.
fn paths(root: &Path, accept: impl Fn(&str, &str) -> bool) -> Vec<String> {
    let mut out = Vec::new();
    walk(root, root, &accept, &mut out);
    out.sort();
    out
}

fn walk(root: &Path, dir: &Path, accept: &dyn Fn(&str, &str) -> bool, out: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let rel = slashed(rel);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if name.starts_with('.') || SKIPPED.contains(&name) {
                continue;
            }
            walk(root, &path, accept, out);
        } else if accept(name, &rel) {
            out.push(rel);
        }
    }
}

/// A path with forward slashes, whatever the platform.
fn slashed(p: &Path) -> String {
    p.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Whether `text` matches `pattern`, where `*` stands for any run of characters but a slash and
/// `**` for any run at all.
pub(crate) fn glob(pattern: &str, text: &str) -> bool {
    fn go(p: &[u8], t: &[u8]) -> bool {
        match p {
            [] => t.is_empty(),
            [b'*', b'*', rest @ ..] => (0..=t.len()).any(|i| go(rest, &t[i..])),
            [b'*', rest @ ..] => {
                let run = t.iter().take_while(|&&c| c != b'/').count();
                (0..=run).any(|i| go(rest, &t[i..]))
            }
            [c, rest @ ..] => t.first() == Some(c) && go(rest, &t[1..]),
        }
    }
    go(pattern.as_bytes(), text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, text: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn crates(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|n| (*n).to_string()).collect()
    }

    #[test]
    fn the_walk_reads_source_and_integration_tests_and_names_their_crate() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "mock/crates/alpha/src/lib.rs", "");
        write(d.path(), "mock/crates/alpha/tests/it.rs", "");
        write(d.path(), "mock/tools/x.rs", "");
        write(d.path(), "mock/crates/alpha/README.md", "");
        let found = rust_files(d.path(), &crates(&["alpha"]));
        let got: Vec<_> = found.iter().map(|f| (f.rel.as_str(), f.crate_name.as_str())).collect();
        assert_eq!(
            got,
            [
                ("mock/crates/alpha/src/lib.rs", "alpha"),
                ("mock/crates/alpha/tests/it.rs", "alpha"),
                ("mock/tools/x.rs", WORKSPACE),
            ]
        );
    }

    #[test]
    fn build_output_vendored_sources_and_hidden_directories_are_not_entered() {
        let d = tempfile::tempdir().unwrap();
        for dir in ["target", "mock/vendor", "node_modules", ".git", "mock/.cache"] {
            write(d.path(), &format!("{dir}/x.rs"), "");
        }
        assert!(rust_files(d.path(), &crates(&[])).is_empty());
    }

    #[test]
    fn matching_takes_the_globs_and_nothing_without_any() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "mock/research/a/LEDGER.md", "");
        write(d.path(), "mock/research/a/NOTES.md", "");
        write(d.path(), "LEDGER.md", "");
        assert!(matching(d.path(), &[]).is_empty());
        let one = matching(d.path(), &["mock/research/*/LEDGER.md".to_string()]);
        assert_eq!(one, ["mock/research/a/LEDGER.md"]);
        let deep = matching(d.path(), &["**/LEDGER.md".to_string()]);
        assert_eq!(deep, ["mock/research/a/LEDGER.md"]);
        let either = matching(d.path(), &["LEDGER.md".to_string(), "mock/**/NOTES.md".to_string()]);
        assert_eq!(either, ["LEDGER.md", "mock/research/a/NOTES.md"]);
    }

    #[test]
    fn a_star_stops_at_a_slash_and_two_do_not() {
        assert!(glob("a/*.md", "a/b.md"));
        assert!(!glob("a/*.md", "a/b/c.md"));
        assert!(glob("a/**/c.md", "a/b/d/c.md"));
        assert!(!glob("a/b", "a/bc"));
    }
}
