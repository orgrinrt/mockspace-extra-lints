//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `mock owed [--dry] [--only <patterns>] [--no-fetch]`: a task held by the coordinator for every
//! placeholder marker that cites none, and for every candidate file waiting to be ratified. It files
//! no question: an agent evaluates the task against the code and the canon, and puts a question to the
//! maintainer only with a subject and real options.
//!
//! What owes work is written down where it is, as a marker in the code or a candidate file in the
//! research, and a person comes to it by finding it. This files each as a ticket on the registry,
//! groups the markers that say one thing under one ticket, rewrites each marker to name its ticket,
//! and closes the tickets it filed whose markers are gone. A second run with nothing changed writes
//! nothing. The reading of a marker is `mock/lints/fixme_markers.rs`, which the lint that refuses a
//! marker with no ticket reads by too. The design is `mock/WORKFLOW.md.tmpl`, "Work items".

use std::collections::BTreeSet;

use mockspace::tool::{ArgSpec, Purpose, Tool, ToolContext, ToolReport};

mod apply;
use registry_lints::fixme_markers;
mod group;
mod plan;

use fixme_markers::Scope;
use plan::{Candidate, File, Inputs};

/// The maker.
pub struct Owed;

/// What the command line said.
#[derive(Debug, Default, PartialEq, Eq)]
struct Args {
    dry:      bool,
    no_fetch: bool,
    only:     Vec<String>,
}

fn parse(args: &[&str]) -> Result<Args, String> {
    let mut out = Args::default();
    let mut it = args.iter().copied();
    while let Some(a) = it.next() {
        match a {
            "--dry" => out.dry = true,
            "--no-fetch" => out.no_fetch = true,
            "--only" => {
                let v = it.next().ok_or("`--only` wants a pattern")?;
                out.only.extend(
                    v.split(',')
                        .map(str::trim)
                        .filter(|p| !p.is_empty())
                        .map(String::from),
                );
            },
            other => return Err(format!("`{other}` is no flag of owed")),
        }
    }
    Ok(out)
}

/// Whether `text` matches `pattern`, where `*` stands for any run of characters.
fn glob(pattern: &str, text: &str) -> bool {
    fn go(p: &[char], t: &[char]) -> bool {
        match p.split_first() {
            None => t.is_empty(),
            Some(('*', rest)) => (0 ..= t.len()).any(|i| go(rest, &t[i ..])),
            Some((c, rest)) => t.first() == Some(c) && go(rest, &t[1 ..]),
        }
    }
    go(
        &pattern.chars().collect::<Vec<_>>(),
        &text.chars().collect::<Vec<_>>(),
    )
}

impl Tool for Owed {
    fn name(&self) -> &'static str {
        "owed"
    }

    fn description(&self) -> &'static str {
        "files a task held by the coordinator for every placeholder marker that cites none and every candidate file, rewrites each marker to cite its task, and closes the tickets whose markers are gone"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Make {
            writes: fixme_markers::WRITES,
            roots:  &[],
        }
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[]
    }

    fn value_flags(&self) -> &[&'static str] {
        &["--only"]
    }

    fn help(&self) -> &'static str {
        "Reads every authored file for placeholder markers, and the research for candidate files. A marker that cites no ticket is grouped with the markers that say the same, a task is filed for the group on the registry, in the backlog and held by the coordinator, and the marker is rewritten to cite it, keeping its reason. A canon silence, a `maintainer` tag and a candidate file are tasks too, never questions: the task says an agent is to evaluate each marker against the code and the canon, and the list gives each marker with its file and line as it stood then. A marker an earlier run listed on a ticket cites that ticket again and files nothing, whatever line it has moved to. A ticket this filed whose markers are all gone is closed, a task as done and a question filed before this as dropped. Silent when there is nothing to do. `--dry` says what it would do and writes nothing, `--only` takes comma separated patterns, `*` standing for any run of characters, and limits the run to the files they match, so nothing is closed, and `--no-fetch` does not fetch the trunk to say whether the tree is behind it."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let args = match parse(ctx.args) {
            Ok(a) => a,
            Err(e) => return ToolReport::inconclusive(e),
        };
        let mut report = self.work(ctx, &args);
        let notice = tickets::trunk::tree_notice(ctx.repo_root, !args.no_fetch);
        if !notice.is_empty() {
            report.output.insert_str(0, &format!("{notice}\n"));
        }
        report
    }
}

impl Owed {
    /// The run, without the notice of a tree behind the trunk.
    fn work(&self, ctx: &ToolContext<'_>, args: &Args) -> ToolReport {
        let registry = tickets::registry_dir(ctx.mock_dir);
        let store = match tickets::load(&registry) {
            Ok(s) => s,
            Err(e) => return ToolReport::inconclusive(e),
        };
        let wanted = |rel: &str| args.only.is_empty() || args.only.iter().any(|p| glob(p, rel));
        let mut files = Vec::new();
        let mut candidates = Vec::new();
        let mut all_candidates = BTreeSet::new();
        for s in fixme_markers::sources(ctx.repo_root) {
            let name = s.rel.rsplit('/').next().unwrap_or(&s.rel);
            if s.scope == Scope::Record {
                // FIXME(pack_lints_and_tools_read_the_mock_directory_as_mock): the mock directory is read as `mock`, not as the one `mockspace.toml` declares
                if s.rel.starts_with("mock/research/")
                    && name.starts_with("CANDIDATE")
                    && name.ends_with(".md")
                {
                    all_candidates.insert(s.rel.clone());
                    if wanted(&s.rel) {
                        candidates.push(Candidate {
                            rel:  s.rel,
                            text: s.text,
                        });
                    }
                }
                continue;
            }
            if !wanted(&s.rel) {
                continue;
            }
            let markers = fixme_markers::scan(&s.text, s.kind);
            if !markers.is_empty() {
                files.push(File {
                    rel: s.rel,
                    text: s.text,
                    markers,
                });
            }
        }
        let subjects: BTreeSet<String> = ctx
            .registry
            .rows_in("subject")
            .iter()
            .map(|q| {
                q.split_once("::")
                    .map_or(q.as_str(), |(_, s)| s)
                    .to_string()
            })
            .collect();
        let examined: usize = files.iter().map(|f| f.markers.len()).sum();
        let plan = plan::plan(&Inputs {
            files:          &files,
            candidates:     &candidates,
            all_candidates: &all_candidates,
            store:          &store,
            subjects:       &subjects,
            prefix:         &registry_lints::project::Project::of(ctx.mock_dir).prefix,
            full:           args.only.is_empty(),
        });
        if plan.is_empty() {
            return ToolReport::reported("", examined);
        }
        if args.dry {
            return ToolReport::reported(describe(&plan, &files), examined);
        }
        let at = tickets::time::format_utc(tickets::time::now_secs());
        match apply::apply(ctx.repo_root, &registry, &store, &files, &plan, &at) {
            Ok(d) => ToolReport::reported(summary(&d), examined),
            Err(e) => ToolReport::inconclusive(e),
        }
    }
}

fn summary(d: &apply::Done) -> String {
    format!(
        "filed {} tasks covering {} markers, cited {} markers, closed {} tickets\n",
        d.tasks, d.covered, d.cited, d.closed
    )
}

fn describe(plan: &plan::Plan, files: &[File]) -> String {
    let mut out = String::new();
    for t in &plan.new {
        out.push_str(&format!(
            "would file task {}: {} ({} markers)\n",
            t.id,
            t.title,
            t.covers.len()
        ));
        for &(f, m) in &t.covers {
            out.push_str(&format!(
                "  {}:{}\n",
                files[f].rel, files[f].markers[m].line
            ));
        }
    }
    for c in &plan.cites {
        out.push_str(&format!(
            "would cite {} at {}:{}\n",
            c.id, files[c.file].rel, files[c.file].markers[c.marker].line
        ));
    }
    for c in &plan.closes {
        out.push_str(&format!("would close {} as {}: {}\n", c.id, c.to, c.why));
    }
    out
}

mockspace::lint_pack! {
    tools: [Owed],
}

#[cfg(test)]
mod tests;
