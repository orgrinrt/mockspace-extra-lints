//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Whether a ticket's branch has landed on the trunk, asked of git, and where that disagrees with the
//! ticket.
//!
//! `kaski-tickets` judges rows and touches no repository, so the question of git is this tool's. It is
//! asked through `Trunk`, which the repository answers and a test answers from a table.

use std::cell::{OnceCell, RefCell};
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use ticket_model::{Finding, Store};

/// Where a branch stands against the trunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Landing {
    /// The branch or the trunk is not in the repository, so nothing can be said.
    Unknown,
    /// The branch holds work the trunk does not.
    Open,
    /// The branch's tip is on the trunk, at this sha.
    Reachable(String),
    /// The branch's tip is the second parent of a merge on the trunk, which is this sha: a landing.
    Merged(String),
}

/// The branches of an `ls-remote --heads` listing, by name without `refs/heads/`, each with its sha.
fn parse_heads(listing: &str) -> BTreeMap<String, String> {
    listing
        .lines()
        .filter_map(|line| {
            let (sha, name) = line.split_once('\t')?;
            Some((
                name.strip_prefix("refs/heads/")?.to_string(),
                sha.to_string(),
            ))
        })
        .collect()
}

/// What answers where a branch stands.
pub trait Trunk {
    /// Where `branch` stands against the trunk.
    fn landing(&self, branch: &str) -> Landing;
}

/// The trunk of the repository at a path, `origin/dev` where it has one and `dev` otherwise.
pub struct Git<'a> {
    root:            &'a Path,
    /// The heads of `origin` by branch name, asked once and only when a branch is not among the local refs: a
    /// clone fetches `dev` alone, so every other branch read as unknown and a landed ticket went unnamed until a
    /// move made somebody fetch its branch (job_72_sky and job_75, 7 November).
    heads:           OnceCell<BTreeMap<String, String>>,
    /// What the tree said of a name already, so a name that many tickets repeat is asked of git once.
    pub(crate) gone: RefCell<BTreeMap<String, Option<String>>>,
    /// The repository's `crate_prefix`, which a crate a ticket names alone starts with.
    pub(crate) prefix: String,
    /// The directories crates live in, relative to the root: the source directories of the config.
    pub(crate) crate_dirs: Vec<String>,
}

impl<'a> Git<'a> {
    /// The repository whose tree is at `root`.
    pub const fn at(root: &'a Path) -> Self {
        Git {
            root,
            heads: OnceCell::new(),
            gone: RefCell::new(BTreeMap::new()),
            prefix: String::new(),
            crate_dirs: Vec::new(),
        }
    }

    /// The repository a tool is run in, its crates named and found as its config says: the
    /// `crate_prefix` and the source directories.
    pub fn of(ctx: &mockspace::tool::ToolContext<'a>) -> Self {
        let project = registry_lints::project::Project::of(ctx.mock_dir);
        let dirs = ctx
            .src_dirs
            .iter()
            .filter_map(|d| d.strip_prefix(ctx.repo_root).ok())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .collect();
        Git::at(ctx.repo_root).with_crates(project.prefix, dirs)
    }

    /// The same repository, its crates named with `prefix` and found under `crate_dirs`.
    #[must_use]
    pub fn with_crates(mut self, prefix: String, crate_dirs: Vec<String>) -> Self {
        self.prefix = prefix;
        self.crate_dirs = crate_dirs;
        self
    }

    pub(crate) fn git(&self, args: &[&str]) -> Option<std::process::Output> {
        Command::new("git")
            .arg("-C")
            .arg(self.root)
            .args(args)
            .output()
            .ok()
    }

    /// The tip `origin` holds for `branch`, where that commit is in this repository: a landed tip is, since the
    /// trunk holds it, and one the clone never fetched reads as unknown as before.
    fn remote_tip(&self, branch: &str) -> Option<String> {
        let heads = self.heads.get_or_init(|| {
            self.git(&["ls-remote", "--heads", "origin"])
                .filter(|o| o.status.success())
                .map(|o| parse_heads(&String::from_utf8_lossy(&o.stdout)))
                .unwrap_or_default()
        });
        let sha = heads.get(branch)?;
        self.commit(std::slice::from_ref(sha))
    }

    /// The commit the first of `refs` names.
    fn commit(&self, refs: &[String]) -> Option<String> {
        refs.iter().find_map(|r| {
            let out = self.git(&["rev-parse", "--verify", "-q", &format!("{r}^{{commit}}")])?;
            out.status
                .success()
                .then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
        })
    }
}

impl Trunk for Git<'_> {
    fn landing(&self, branch: &str) -> Landing {
        let Some(trunk) = self.commit(&["refs/remotes/origin/dev".into(), "refs/heads/dev".into()])
        else {
            return Landing::Unknown;
        };
        let Some(tip) = self
            .commit(&[format!("refs/remotes/origin/{branch}"), format!("refs/heads/{branch}")])
            .or_else(|| self.remote_tip(branch))
        else {
            return Landing::Unknown;
        };
        let on_trunk = self
            .git(&["merge-base", "--is-ancestor", &tip, &trunk])
            .is_some_and(|o| o.status.success());
        if !on_trunk {
            return Landing::Open;
        }
        let range = format!("{tip}..{trunk}");
        let merges = self
            .git(&["rev-list", "--merges", "--parents", "--ancestry-path", &range])
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default();
        for line in merges.lines() {
            let mut shas = line.split_whitespace();
            let (Some(merge), Some(_first)) = (shas.next(), shas.next()) else {
                continue;
            };
            if shas.any(|p| p == tip) {
                return Landing::Merged(merge.to_string());
            }
        }
        Landing::Reachable(tip)
    }
}

/// Whether somebody is working the ticket: it stands in a state that needs a holder, whatever its newest move
/// names, or its newest move names one. A job started by a move that named none still reads as held, so the
/// branch cut for it is not a landing before it has a commit (tickets_check_reads_a_branch_cut_from_dev_with_no_commit_of).
fn held(store: &Store, ticket: &str) -> bool {
    store.state_of(ticket).is_some_and(|s| s.needs_holder)
        || store.now_of(ticket).is_some_and(|m| !m.holder.is_empty())
}

/// The sha a ticket's branch landed at, or nothing where it has not.
///
/// A branch landed when its tip is the second parent of a merge on the trunk, which is what a local
/// landing makes. Where nobody holds the ticket, a tip the trunk reaches counts as well: no helper
/// is working on it, so the work the branch carried is on the trunk. A held ticket whose tip the
/// trunk reaches without a merge is a branch cut from the trunk and not yet committed to.
pub fn landed(store: &Store, ticket: &str, trunk: &impl Trunk) -> Option<String> {
    let t = store.tickets.get(ticket)?;
    let branch = t.one("branch");
    if branch.is_empty() {
        return None;
    }
    let held = held(store, ticket);
    match trunk.landing(branch) {
        Landing::Merged(sha) => Some(sha),
        Landing::Reachable(sha) if !held => Some(sha),
        _ => None,
    }
}

/// Every open ticket whose branch has landed, said by the ticket and the state it still stands in.
pub fn findings(store: &Store, trunk: &impl Trunk) -> Vec<Finding> {
    let mut out = Vec::new();
    for t in store.tickets.values() {
        let Some(state) = store.state_of(&t.id) else {
            continue;
        };
        if state.closed {
            continue;
        }
        if let Some(sha) = landed(store, &t.id, trunk) {
            out.push(Finding {
                row:     format!("ticket::{}", t.id),
                message: format!(
                    "its branch `{}` landed at {sha}, and the ticket is still in `{}`",
                    t.one("branch"),
                    state.id
                ),
            });
        }
    }
    out
}

impl Git<'_> {
    /// The commit the trunk is at, `origin/dev` where it has one and `dev` otherwise.
    pub(crate) fn trunk(&self) -> Option<String> {
        self.commit(&["refs/remotes/origin/dev".into(), "refs/heads/dev".into()])
    }
}
