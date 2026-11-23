//! Which files the index reads, each with its token and its class.
//!
//! Every file of the checkout git knows, tracked or untracked and not ignored, and every file of the
//! data refs the project declares, which sit under the mock directory and are ignored on the branch.
//! A file's token is its length and modification time, which changes whenever its content does, so
//! a refresh reads only what moved and a question says how far behind the index is without opening
//! a file.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;
use std::time::UNIX_EPOCH;

use muisti::repo::{DocClass, Listed, Root};

/// The registry directory under the mock dir, read as canon: its files are documents and its rows
/// and edges are the index's.
pub const REGISTRY_ROOT: &str = "registry";

/// The rounds under the mock dir.
pub const ROUNDS_ROOT: &str = "design_rounds";

/// The research under the mock dir.
pub const RESEARCH_ROOT: &str = "research";

/// The sketches under the research.
pub const SKETCHES_ROOT: &str = "research/sketches";

/// What a listing found: the roots by class, and what it could not read.
pub struct Listing {
    pub roots: Vec<Root>,
    /// One sentence each: a git that would not answer, a data ref that is not there.
    pub notes: Vec<String>,
}

/// The files of the checkout at `repo_root`, by class, as roots the index reads by their path from
/// the repository's root. `words` is the maintainer's captured words, a directory inside the
/// repository, read as theirs.
#[must_use]
pub fn list(mock_dir: &Path, repo_root: &Path, words: Option<&Path>) -> Listing {
    let mut notes = Vec::new();
    let mut paths: BTreeSet<String> = BTreeSet::new();
    match git(repo_root, &[
        "ls-files",
        "-z",
        "--cached",
        "--others",
        "--exclude-standard",
    ]) {
        Ok(out) => paths.extend(split_z(&out)),
        Err(e) => notes.push(format!("the checkout's files could not be listed: {e}")),
    }
    let mock = rel_of(mock_dir, repo_root);
    for (namespace, under) in data_refs(mock_dir, repo_root) {
        let reference = format!("refs/mock/{namespace}");
        match git(repo_root, &[
            "ls-tree",
            "-r",
            "-z",
            "--name-only",
            &reference,
        ]) {
            Ok(out) => {
                let base = join_rel(&mock, &under);
                paths.extend(split_z(&out).map(|p| join_rel(&base, &p)));
            },
            Err(e) => notes.push(format!("the data ref `{reference}` was not read: {e}")),
        }
    }
    let words = words.map(|w| rel_of(w, repo_root));
    let mut by_class: BTreeMap<DocClass, Vec<Listed>> = BTreeMap::new();
    for path in paths {
        let Some(token) = token_of(&repo_root.join(&path)) else {
            continue;
        };
        let class = class_of(&path, &mock, words.as_deref());
        by_class.entry(class).or_default().push(Listed {
            rel: path,
            token,
        });
    }
    let roots = by_class
        .into_iter()
        .map(|(class, files)| Root::listed(repo_root, "", class, files))
        .collect();
    Listing {
        roots,
        notes,
    }
}

/// The class a file is read as, by where it sits: the maintainer's words, the registry, the rounds,
/// the sketches, the research, and everything else as source.
#[must_use]
pub fn class_of(path: &str, mock: &str, words: Option<&str>) -> DocClass {
    let under = |dir: &str| {
        let dir = join_rel(mock, dir);
        path.strip_prefix(&dir)
            .is_some_and(|rest| rest.starts_with('/'))
    };
    let in_words = words.is_some_and(|w| {
        !w.is_empty()
            && path
                .strip_prefix(w)
                .is_some_and(|rest| rest.starts_with('/'))
    });
    if in_words {
        DocClass::Human
    } else if under(REGISTRY_ROOT) {
        DocClass::Canon
    } else if under(ROUNDS_ROOT) {
        DocClass::Design
    } else if under(SKETCHES_ROOT) {
        DocClass::Sketch
    } else if under(RESEARCH_ROOT) {
        DocClass::Research
    } else {
        DocClass::Source
    }
}

/// A file's token: its length and its modification time in nanoseconds, or nothing for a file that
/// is not there, such as one deleted and not yet committed.
fn token_of(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path)
        .ok()
        .filter(std::fs::Metadata::is_file)?;
    let nanos = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    Some(format!("{}:{nanos}", meta.len()))
}

/// The data refs the project declares, each its namespace and the directory under the mock dir it
/// holds, read from `[[data_ref]]` in `mockspace.toml`. None where the file declares none.
fn data_refs(mock_dir: &Path, repo_root: &Path) -> Vec<(String, String)> {
    let text = [repo_root.join("mockspace.toml"), mock_dir.join("mockspace.toml")]
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok());
    let Some(table) = text.and_then(|t| t.parse::<toml::Table>().ok()) else {
        return Vec::new();
    };
    let Some(refs) = table.get("data_ref").and_then(toml::Value::as_array) else {
        return Vec::new();
    };
    refs.iter()
        .filter_map(|r| {
            let namespace = r.get("namespace")?.as_str()?.to_string();
            let path = r.get("path").and_then(toml::Value::as_str).unwrap_or(".");
            let path = path.trim_start_matches("./").trim_matches('/');
            Some((
                namespace,
                if path == "." { String::new() } else { path.to_string() },
            ))
        })
        .collect()
}

/// `path` relative to `root`, with `/` between its parts; empty where they are one.
fn rel_of(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
        .trim_matches('/')
        .to_string()
}

/// Two relative paths joined, either of them empty.
fn join_rel(a: &str, b: &str) -> String {
    match (a.is_empty(), b.is_empty()) {
        (true, _) => b.to_string(),
        (_, true) => a.to_string(),
        _ => format!("{a}/{b}"),
    }
}

fn split_z(out: &str) -> impl Iterator<Item = String> + '_ {
    out.split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
}

/// A git command's output in `dir`, or its own words on failure.
pub(crate) fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| format!("git could not be run: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}
