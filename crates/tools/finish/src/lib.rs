//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `mock finish`: what a branch leaves beside its own ticket.
//!
//! A helper finishes its ticket and leaves undone the canon rows beside it and the open tickets in the
//! crates it touched. This lists them for a branch, and reports; it never gates. The tickets are read
//! through `tickets` and the tenets through `canon`, so the registry has no second reader here. A
//! project that gates its landings on it refuses a branch while `--unanswered` prints a key.

pub mod items;
pub mod touched;

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use mockspace::tool::{ArgSpec, NotALint, Purpose, Tool, ToolContext, ToolReport, positional_args};
use registry_lints::layout::{ContentDirs, Layout};
use tickets::trunk::{self, Opts};
use tickets::view::{self, Filter};

use crate::items::{Item, Open, answered, row_items, ticket_items};

/// The trunk a branch is compared with when `--base` names none.
const BASE: &str = "origin/dev";

const VALUE_FLAGS: &[&str] = &["--base", "--report", "--rows"];

pub struct Finish;

/// What the command line asked.
struct Request<'a> {
    branch:     Option<&'a str>,
    base:       &'a str,
    report:     Option<&'a str>,
    rows:       Vec<String>,
    unanswered: bool,
}

fn flag<'a>(args: &'a [&'a str], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| *a == name)
        .and_then(|i| args.get(i + 1))
        .copied()
}

fn request<'a>(tool: &Finish, args: &'a [&'a str]) -> Request<'a> {
    Request {
        branch:     positional_args(tool, args).first().copied(),
        base:       flag(args, "--base").unwrap_or(BASE),
        report:     flag(args, "--report"),
        rows:       flag(args, "--rows")
            .map(|r| {
                r.split(',')
                    .map(|s| s.trim().to_owned())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default(),
        unanswered: args.contains(&"--unanswered"),
    }
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("git {}: {e}", args.join(" ")))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    }
}

impl Tool for Finish {
    fn name(&self) -> &'static str {
        "finish"
    }

    fn description(&self) -> &'static str {
        "for a branch, the open tickets in the crates it touched and the canon rows beside its brief that it does not cite, and which of them its report has answered"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Check(NotALint::TakesAQuestion)
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[ArgSpec {
            name:        "branch",
            required:    true,
            description: "the branch to report on, `HEAD` for the checked out one",
        }]
    }

    fn value_flags(&self) -> &[&'static str] {
        VALUE_FLAGS
    }

    fn help(&self) -> &'static str {
        "Lists three things for a branch against `--base` (default `origin/dev`): the crates its diff touches, the open tickets other than its own whose title or text names one of them, and the tenets joined to the rows of its own ticket (the `tenet::` entries of its `related`, or `--rows a,b`) that its diff and commit messages do not name. Each ticket and row is an item with a key, `ticket:<id>` or `row:<id>`. A report answers an item on a line of its own: `- <key> done <sha>`, `- <key> parted <ticket id>` or `- <key> left: <reason>`. `--report <file>` marks each item answered or not, and `--unanswered` prints only the keys with no answer, one a line, which is what a landing gate refuses on. It reports and never fails on what it finds."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let req = request(self, ctx.args);
        match build(ctx, &req) {
            Ok((out, n)) => ToolReport::reported(out, n),
            Err(e) => ToolReport::inconclusive(e),
        }
    }
}

fn build(ctx: &ToolContext<'_>, req: &Request<'_>) -> Result<(String, usize), String> {
    let root = ctx.repo_root;
    let branch = match req.branch {
        Some(b) if b != "HEAD" => b.to_owned(),
        _ => {
            git(root, &["rev-parse", "--abbrev-ref", "HEAD"])?
                .trim()
                .to_owned()
        },
    };
    // a branch only the remote holds is read at its remote ref
    let tip = if git(root, &["rev-parse", "--verify", "-q", &branch]).is_ok() {
        branch.clone()
    } else {
        format!("origin/{branch}")
    };
    let range = format!("{}...{tip}", req.base);
    let paths: Vec<String> = git(root, &["diff", "--name-only", &range])?
        .lines()
        .map(str::to_owned)
        .collect();
    let layout = Layout::of(ctx.mock_dir, root, &ContentDirs::default());
    let touched = touched::touched(&paths, &layout);

    let reading = trunk::read(root, ctx.mock_dir, Opts {
        tree:  false,
        fetch: false,
    })?;
    let (store, plan) = (&reading.store, &reading.plan);
    let own = store.with_branch(&branch).next();
    let own_id = own.map_or("", |t| t.id.as_str()).to_owned();

    let open: Vec<Open> = view::list(store, plan, &Filter::default())
        .into_iter()
        .filter(|r| store.states.get(&r.state).is_some_and(|s| !s.closed))
        .map(|r| {
            let what = store
                .tickets
                .get(&r.id)
                .map_or("", |t| t.one("what"))
                .to_owned();
            Open {
                id: r.id,
                state: r.state,
                title: r.title,
                what,
            }
        })
        .collect();
    let mut items: Vec<Item> = ticket_items(&open, &touched, &own_id);

    let mut brief: Vec<String> = own
        .map(|t| {
            t.many("related")
                .iter()
                .filter_map(|r| r.strip_prefix("tenet::").map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    brief.extend(req.rows.iter().cloned());
    let all = canon::tenets(ctx.registry);
    let diff_text = git(root, &["diff", "-U0", &range])?;
    let log_text = git(root, &[
        "log",
        "--format=%B",
        &format!("{}..{tip}", req.base),
    ])?;
    let joined = |id: &str| -> Vec<String> {
        canon::joined_to(&all, id)
            .into_iter()
            .filter(|j| all.iter().any(|t| t.id == *j && t.binds()))
            .map(str::to_owned)
            .collect()
    };
    let cited = |id: &str| diff_text.contains(id) || log_text.contains(id);
    items.extend(row_items(&brief, &joined, &cited));

    let answers: Option<BTreeSet<String>> = match req.report {
        Some(file) => {
            Some(answered(&std::fs::read_to_string(file).map_err(|e| {
                format!("cannot read the report {file}: {e}")
            })?))
        },
        None => None,
    };
    let is_answered = |key: &str| answers.as_ref().is_some_and(|a| a.contains(key));

    if req.unanswered {
        let out: String = items
            .iter()
            .filter(|i| !is_answered(&i.key))
            .map(|i| format!("{}\n", i.key))
            .collect();
        let n = out.lines().count();
        return Ok((out, n));
    }

    let mut out = format!("{branch} against {}\n", req.base);
    out.push_str(&format!(
        "crates touched ({}): {}\n",
        touched.len(),
        touched.iter().cloned().collect::<Vec<_>>().join(", ")
    ));
    out.push_str(&format!(
        "brief rows ({}): {}\n",
        brief.len(),
        brief.join(", ")
    ));
    for item in &items {
        let mark = if answers.is_none() {
            ""
        } else if is_answered(&item.key) {
            "[answered] "
        } else {
            "[open] "
        };
        out.push_str(&format!("{mark}{}  {}\n", item.key, item.line));
    }
    if answers.is_some() {
        let open = items.iter().filter(|i| !is_answered(&i.key)).count();
        out.push_str(&format!(
            "{} of {} answered\n",
            items.len() - open,
            items.len()
        ));
    }
    Ok((out, items.len()))
}

mockspace::lint_pack! {
    tools: [Finish],
}

#[cfg(test)]
mod tests;
