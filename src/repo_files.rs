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
/// `mock_dir` and the paths matching one of the `exclude` globs.
pub(crate) fn rust_files(
    root: &Path,
    mock_dir: &Path,
    all_crates: &BTreeSet<String>,
    exclude: &[String],
) -> Vec<RustFile> {
    let record: Vec<String> = RECORD
        .iter()
        .filter_map(|d| mock_dir.join(d).strip_prefix(root).ok().map(slashed))
        .collect();
    let mut out = Vec::new();
    for rel in paths(root, &record, |name, rel| {
        name.ends_with(".rs") && !exclude.iter().any(|g| glob(g, rel))
    }) {
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
    paths(root, &[], |_, rel| globs.iter().any(|g| glob(g, rel)))
}

/// The crate a repository-relative path sits in.
fn crate_of(rel: &str, all_crates: &BTreeSet<String>) -> String {
    rel.split('/')
        .find(|c| all_crates.contains(*c))
        .unwrap_or(WORKSPACE)
        .to_string()
}

/// The paths under `root` that `accept` takes, given the file name and the path from the root.
fn paths(root: &Path, skip: &[String], accept: impl Fn(&str, &str) -> bool) -> Vec<String> {
    let mut out = Vec::new();
    walk(root, root, skip, &accept, &mut out);
    out.sort();
    out
}

fn walk(
    root: &Path,
    dir: &Path,
    skip: &[String],
    accept: &dyn Fn(&str, &str) -> bool,
    out: &mut Vec<String>,
) {
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
            if name.starts_with('.') || SKIPPED.contains(&name) || skip.contains(&rel) {
                continue;
            }
            walk(root, &path, skip, accept, out);
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

/// Whether `text` matches `pattern`, where `*` stands for any run of characters but a slash, `**`
/// for any run at all, and `**/` for zero directories or more, so `a/**/*.toml` matches `a/x.toml`.
pub(crate) fn glob(pattern: &str, text: &str) -> bool {
    fn go(p: &[u8], t: &[u8]) -> bool {
        match p {
            [] => t.is_empty(),
            [b'*', b'*', b'/', rest @ ..] => {
                go(rest, t) || (0..t.len()).any(|i| t[i] == b'/' && go(rest, &t[i + 1..]))
            }
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

    fn scan(root: &Path, all: &BTreeSet<String>, exclude: &[String]) -> Vec<RustFile> {
        rust_files(root, &root.join("mock"), all, exclude)
    }

    #[test]
    fn the_walk_reads_source_and_integration_tests_and_names_their_crate() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "mock/crates/alpha/src/lib.rs", "");
        write(d.path(), "mock/crates/alpha/tests/it.rs", "");
        write(d.path(), "mock/tools/x.rs", "");
        write(d.path(), "mock/crates/alpha/README.md", "");
        let found = scan(d.path(), &crates(&["alpha"]), &[]);
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
        assert!(scan(d.path(), &crates(&[]), &[]).is_empty());
    }

    #[test]
    fn the_record_directories_of_the_mock_directory_are_not_entered_but_a_namesake_elsewhere_is() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "mock/research/sketches/x/probe.rs", "");
        write(d.path(), "mock/design_rounds/1/probe.rs", "");
        write(d.path(), "mock/crates/alpha/research/real.rs", "");
        write(d.path(), "other/research/real.rs", "");
        let found = scan(d.path(), &crates(&["alpha"]), &[]);
        let got: Vec<_> = found.iter().map(|f| f.rel.as_str()).collect();
        assert_eq!(got, ["mock/crates/alpha/research/real.rs", "other/research/real.rs"]);
    }

    #[test]
    fn a_path_matching_an_exclude_glob_is_not_read() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "mock/crates/alpha/tests/fixtures/a.rs", "");
        write(d.path(), "mock/crates/alpha/tests/it.rs", "");
        let exclude = ["**/fixtures/*.rs".to_string()];
        let found = scan(d.path(), &crates(&["alpha"]), &exclude);
        let got: Vec<_> = found.iter().map(|f| f.rel.as_str()).collect();
        assert_eq!(got, ["mock/crates/alpha/tests/it.rs"]);
    }

    #[test]
    fn matching_still_reads_the_record_directories_since_ledgers_live_there() {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "mock/research/a/LEDGER.md", "");
        let found = matching(d.path(), &["mock/research/*/LEDGER.md".to_string()]);
        assert_eq!(found, ["mock/research/a/LEDGER.md"]);
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
        assert_eq!(deep, ["LEDGER.md", "mock/research/a/LEDGER.md"]);
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

    #[test]
    fn a_double_star_slash_matches_zero_directories_one_and_several() {
        assert!(glob("a/**/*.toml", "a/x.toml"));
        assert!(glob("a/**/*.toml", "a/b/x.toml"));
        assert!(glob("a/**/*.toml", "a/b/c/x.toml"));
        let tenet = "mock/registry/tenet/**/*.toml";
        assert!(glob(tenet, "mock/registry/tenet/x.toml"));
        assert!(glob(tenet, "mock/registry/tenet/k/x.toml"));
    }

    #[test]
    fn a_double_star_slash_still_refuses_what_its_suffix_refuses() {
        assert!(!glob("a/**/*.toml", "b/x.toml"));
        assert!(!glob("a/**/*.toml", "a/x.rs"));
        assert!(!glob("a/**/*.toml", "ax.toml"));
        assert!(!glob("a/**/*.toml", "a/x.toml/y"));
    }
}
