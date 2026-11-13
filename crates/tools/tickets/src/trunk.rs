//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The tickets as the trunk holds them, and what is said when they are read from anywhere else.
//!
//! A ticket counts once it is on `origin/dev`, so the commands that answer from the tickets answer
//! from there: they fetch it and read the registry at that commit, whatever the working tree holds.
//! A tree that fell behind would answer with a question still open that was answered on `dev`.
//! When the trunk cannot be read, or the tree was asked for with `--tree`, the answer opens with a
//! notice that says what it read instead and how far behind the trunk that is.

use std::io::{Read as _, Write as _};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use planning::Plan;
use ticket_model::Store;

/// The trunk branch the tickets are real on.
pub const TRUNK: &str = "dev";

/// The longest a fetch is waited for.
const FETCH_LIMIT: Duration = Duration::from_secs(60);

/// What the reading was asked to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Opts {
    /// Read the working tree, which is how a branch is checked with the tickets it adds.
    pub tree:  bool,
    /// Fetch the trunk first. Off where the answer must not wait on the network.
    pub fetch: bool,
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            tree:  false,
            fetch: true,
        }
    }
}

/// The tickets read, and what to say about where they were read from.
#[derive(Debug)]
pub struct Reading {
    pub store:    Store,
    /// The planning rows, read from the same files.
    pub plan:     Plan,
    /// Empty when the tickets are the trunk's as just fetched. Otherwise the lines to print before
    /// the answer, each ending in a newline.
    pub notice:   String,
    /// The trunk could not be read as asked, so the answer is of something else than it was meant
    /// to be. A deliberate `--tree` is a notice and not this.
    pub degraded: bool,
}

fn git(root: &Path, args: &[&str]) -> Option<std::process::Output> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .ok()
}

fn ok(root: &Path, args: &[&str]) -> Option<String> {
    let out = git(root, args)?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn trunk_ref() -> String {
    format!("refs/remotes/origin/{TRUNK}")
}

/// Whether the trunk is in the repository as a remote branch.
pub fn has_trunk(root: &Path) -> bool {
    ok(root, &[
        "rev-parse",
        "--verify",
        "-q",
        &format!("{}^{{commit}}", trunk_ref()),
    ])
    .is_some()
}

/// Fetches the trunk into `refs/remotes/origin/dev`, giving up after a minute.
///
/// # Errors
///
/// What git said, in a line, when the fetch failed or timed out.
pub fn fetch(root: &Path) -> Result<(), String> {
    let spec = format!("+refs/heads/{TRUNK}:{}", trunk_ref());
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["fetch", "--quiet", "origin", &spec])
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot run git: {e}"))?;
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if status.success() {
                    return Ok(());
                }
                let mut said = String::new();
                if let Some(mut e) = child.stderr.take() {
                    let _ = e.read_to_string(&mut said);
                }
                let line = said.lines().rev().find(|l| !l.trim().is_empty());
                return Err(line.unwrap_or("git fetch failed").trim().to_string());
            },
            Ok(None) if start.elapsed() > FETCH_LIMIT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "git fetch took longer than {} seconds",
                    FETCH_LIMIT.as_secs()
                ));
            },
            Ok(None) => std::thread::sleep(Duration::from_millis(25)),
            Err(e) => return Err(format!("git fetch: {e}")),
        }
    }
}

/// How many commits of the trunk the checked out commit does not have, where both are there.
pub fn behind(root: &Path) -> Option<usize> {
    ok(root, &[
        "rev-list",
        "--count",
        &format!("HEAD..{}", trunk_ref()),
    ])?
    .parse()
    .ok()
}

/// How long ago the trunk was last fetched, as git words the age of its commit.
fn age(root: &Path) -> String {
    ok(root, &["log", "-1", "--format=%cr", &trunk_ref()])
        .unwrap_or_else(|| "at an unknown time".to_string())
}

/// The text of every `.toml` file under `dir` at the trunk, with its path, in path order.
fn trunk_files(root: &Path, dir: &str) -> Result<Vec<(String, String)>, String> {
    let listing = git(root, &[
        "ls-tree",
        "-r",
        "-z",
        "--name-only",
        &trunk_ref(),
        "--",
        dir,
    ])
    .filter(|o| o.status.success())
    .ok_or_else(|| format!("cannot list `{dir}` at origin/{TRUNK}"))?;
    let paths: Vec<String> = String::from_utf8_lossy(&listing.stdout)
        .split('\0')
        .filter(|p| p.ends_with(".toml"))
        .map(String::from)
        .collect();
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("cannot run git: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("git cat-file has no input")?;
    let wanted: Vec<String> = paths
        .iter()
        .map(|p| format!("{}:{p}\n", trunk_ref()))
        .collect();
    let writer = std::thread::spawn(move || {
        for w in wanted {
            if stdin.write_all(w.as_bytes()).is_err() {
                break;
            }
        }
    });
    let mut raw = Vec::new();
    child
        .stdout
        .take()
        .ok_or("git cat-file has no output")?
        .read_to_end(&mut raw)
        .map_err(|e| format!("cannot read from git: {e}"))?;
    let _ = writer.join();
    let _ = child.wait();
    let mut out = Vec::new();
    let mut at = 0;
    for path in paths {
        let end = raw[at ..]
            .iter()
            .position(|b| *b == b'\n')
            .ok_or_else(|| format!("git cat-file ended before `{path}`"))?
            + at;
        let head = String::from_utf8_lossy(&raw[at .. end]).into_owned();
        let mut words = head.split_whitespace();
        let (Some(_sha), Some("blob"), Some(size)) = (words.next(), words.next(), words.next())
        else {
            return Err(format!("`{path}` is not a file at origin/{TRUNK}: {head}"));
        };
        let size: usize = size.parse().map_err(|_| format!("bad size for `{path}`"))?;
        let body = raw
            .get(end + 1 .. end + 1 + size)
            .ok_or_else(|| format!("git cat-file cut `{path}` short"))?;
        out.push((
            format!("origin/{TRUNK}:{path}"),
            String::from_utf8_lossy(body).into_owned(),
        ));
        at = end + 1 + size + 1;
    }
    Ok(out)
}

/// The ticket and planning rows of the working tree, read the way `load` reads them.
fn tree_store(registry: &Path) -> Result<(Store, Plan), String> {
    Ok((crate::load(registry)?, crate::load_plan(registry)?))
}

/// The ticket and planning rows at the trunk, as a store and a plan.
fn trunk_store(root: &Path, registry_rel: &str) -> Result<(Store, Plan), String> {
    let mut files = trunk_files(root, &format!("{registry_rel}/ticket"))?;
    files.extend(trunk_files(root, &format!("{registry_rel}/plan"))?);
    let store = Store::from_texts(files.iter().map(|(p, s)| (p.as_str(), s.as_str())))?;
    Ok((store, crate::plan_of(&files)?))
}

/// The registry's path from the repository root, with `/`.
fn registry_rel(root: &Path, mock_dir: &Path) -> String {
    let rel = mock_dir.strip_prefix(root).unwrap_or(Path::new("mock"));
    let rel = rel.to_string_lossy().replace('\\', "/");
    format!("{}/registry", rel.trim_matches('/'))
}

/// The tickets, from the trunk unless `--tree` says otherwise.
///
/// The trunk is fetched first. Where the fetch fails but the trunk was fetched before, that is read
/// and the notice says how old it is; where the repository has no trunk at all, the working tree is
/// read and the notice says so.
///
/// # Errors
///
/// When the files read are not tickets, or the working tree cannot be read.
pub fn read(root: &Path, mock_dir: &Path, opts: Opts) -> Result<Reading, String> {
    let rel = registry_rel(root, mock_dir);
    let registry = mock_dir.join("registry");
    let fetched = if opts.fetch { Some(fetch(root)) } else { None };
    if opts.tree {
        let (store, plan) = tree_store(&registry)?;
        let mut notice = format!(
            "reading the working tree of {} (--tree), not origin/{TRUNK}.\n",
            root.display()
        );
        match (&fetched, has_trunk(root)) {
            (Some(Err(why)), true) => {
                notice.push_str(&format!(
                    "origin/{TRUNK} was not fetched ({why}), so it is as it was {}.\n",
                    age(root)
                ));
            },
            (_, false) => {
                notice.push_str(&format!(
                    "there is no origin/{TRUNK} here to compare with.\n"
                ))
            },
            _ => {},
        }
        if let Some(n) = behind(root).filter(|n| *n > 0) {
            notice.push_str(&format!(
                "the checked out commit is {n} commit{} behind origin/{TRUNK}.\n",
                if n == 1 { "" } else { "s" }
            ));
        }
        return Ok(Reading {
            store,
            plan,
            notice,
            degraded: false,
        });
    }
    if !has_trunk(root) {
        let why = match &fetched {
            Some(Err(why)) => format!(" ({why})"),
            _ => String::new(),
        };
        let (store, plan) = tree_store(&registry)?;
        return Ok(Reading {
            store,
            plan,
            notice: format!(
                "origin/{TRUNK} is not in this repository{why}, so this is the working tree of {}, which may be behind what has been answered there.\n",
                root.display()
            ),
            degraded: true,
        });
    }
    let (store, plan) = trunk_store(root, &rel)?;
    let notice = match fetched {
        Some(Err(why)) => {
            format!(
                "origin/{TRUNK} was not fetched ({why}); this is origin/{TRUNK} as it was {}, and questions may have been answered since.\n",
                age(root)
            )
        },
        None => {
            format!(
                "origin/{TRUNK} was not fetched (--no-fetch); this is origin/{TRUNK} as it was {}.\n",
                age(root)
            )
        },
        Some(Ok(())) => String::new(),
    };
    Ok(Reading {
        store,
        plan,
        degraded: !notice.is_empty(),
        notice,
    })
}

/// What a tool that answers from the working tree says first, where the tree is behind the trunk or
/// the trunk could not be fetched. Empty where the tree is current.
pub fn tree_notice(root: &Path, fetch_first: bool) -> String {
    let fetched = if fetch_first { Some(fetch(root)) } else { None };
    let mut notice = String::new();
    if !has_trunk(root) {
        return format!(
            "origin/{TRUNK} is not in this repository, so nothing says how far behind this tree is.\n"
        );
    }
    if let Some(Err(why)) = &fetched {
        notice.push_str(&format!(
            "origin/{TRUNK} was not fetched ({why}); it is as it was {}.\n",
            age(root)
        ));
    }
    if let Some(n) = behind(root).filter(|n| *n > 0) {
        notice.push_str(&format!(
            "this answer reads the working tree of {}, which is {n} commit{} behind origin/{TRUNK}; \
             rows answered or added since are not in it. `git merge origin/{TRUNK}` brings them.\n",
            root.display(),
            if n == 1 { "" } else { "s" }
        ));
    }
    notice
}
