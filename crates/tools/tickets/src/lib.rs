//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `mock tickets [check]`: the tickets of the registry, one line each, or the rules they break.
//!
//! The crate also holds what `mock ticket-add` and `mock ticket-move` write by: the reading of the
//! registry's files and the file writing. The model of the four ticket namespaces and the rules a
//! schema cannot say are `kaski-tickets`', which the console builds too, so the makers refuse by the
//! rules the lint finds breaches of and the page refuses by them as well. The design is
//! `mock/WORKFLOW.md.tmpl`, "Work items".

use mockspace::LintError;
use mockspace::tool::{ArgSpec, NotALint, Outcome, Purpose, Tool, ToolContext, ToolReport};

pub mod answerable;
pub mod args;
pub mod close_gone;
pub mod ids;
pub mod landing;
mod lint;
pub mod member;
pub mod refs;
pub mod repoint;
mod row;
#[cfg(any(test, feature = "testkit"))]
pub mod testkit;
pub mod time;
mod tree;
use registry_lints::tree_walk;
pub mod trunk;
pub mod view;
pub mod write;

use args::Verb;
use planning::{NAMESPACES, Plan};
use ticket_model::{Heirs, Store, Tree, rules};

/// Reads every ticket row under `registry`, wherever it sits, into the shared crate's `Store`.
///
/// # Errors
///
/// When a file cannot be read, or the crate refuses what it holds.
pub fn load(registry: &std::path::Path) -> Result<Store, String> {
    let texts = read_texts(registry)?;
    Store::from_texts(texts.iter().map(|(p, s)| (p.as_str(), s.as_str())))
}

/// Reads every planning row under `registry`, wherever it sits, into the planning crate's `Plan`.
///
/// # Errors
///
/// When a file cannot be read, or the crate refuses what it holds.
pub fn load_plan(registry: &std::path::Path) -> Result<Plan, String> {
    let texts = read_texts(registry)?;
    plan_of(&texts)
}

/// The planning rows of file texts already read.
///
/// # Errors
///
/// When the crate refuses what the texts hold.
pub fn plan_of(texts: &[(String, String)]) -> Result<Plan, String> {
    Plan::from_texts(
        &NAMESPACES,
        texts.iter().map(|(p, s)| (p.as_str(), s.as_str())),
    )
}

fn read_texts(registry: &std::path::Path) -> Result<Vec<(String, String)>, String> {
    let mut texts = Vec::new();
    for file in tree_walk::with_extension(registry, "toml", &[]) {
        let src = std::fs::read_to_string(&file)
            .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
        texts.push((file.display().to_string(), src));
    }
    Ok(texts)
}

/// The registry directory under the mock directory.
pub fn registry_dir(mock_dir: &std::path::Path) -> std::path::PathBuf {
    mock_dir.join("registry")
}

/// The check.
pub struct Tickets;

impl Tool for Tickets {
    fn name(&self) -> &'static str {
        "tickets"
    }

    fn description(&self) -> &'static str {
        "lists the tickets with their state and holder, ranks what to take next, checks their graph against the rules and git, shows each track with its steps and the waits between tracks, finds a branch's ticket, and finds the open tickets that change a contract"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Check(NotALint::TakesAQuestion)
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[ArgSpec {
            name:        "verb",
            required:    true,
            description: "`list` the tickets, `next` for what to take, `check` for the rules the committed rows and git break, `tracks` for each track with its steps and waits, `branch <name>` for the ticket holding a branch, `landed <id>` for the sha its branch landed at, or `contract <path>...` for the open tickets that change a contract",
        }]
    }

    fn value_flags(&self) -> &[&'static str] {
        args::LIST_FLAGS
    }

    fn help(&self) -> &'static str {
        "One line per ticket: id, state, holder (`-` for none) and title, in board order. `--state`, `--holder`, `--kind`, `--epoch` and `--tag` narrow `list` and `next` and take comma separated values, `--epoch none` meaning the tickets that are members of no epoch and `--tag` the tickets holding any of the tags named, each matched whole. A ticket whose newest member row names an epoch shows ` [epoch <id>]` after its title. `next` lists only what is free to take (open, unheld, nothing it needs unmet, its branch not on the trunk), the one that unblocks the most open tickets first. `check` also names a branch two tickets hold outside one set, an open ticket whose branch has landed and a planning row the planning rules refuse. It also lists, and does not fail on, the tickets that may have stopped being true: an open ticket whose title or `what` names a kaski crate or a path the trunk held once and holds no more, with the commit that deleted it; an open parent all of whose children and needs are closed, and a closed ticket with an open child; and a ticket still naming a superseded tenet, with the one that binds in its place. The count of the first two ends the output as `N tickets may be stale: tickets check`. `tracks` gives one block a track, a ticket holding `shows`: its id, state, holder and `shows`, how many of its first steps wait on another track, then its steps in the order of their `needs` and the board with state and holder, and under a step each `needs` edge that leaves the track for a ticket not yet done, with that ticket's track, state and holder; the filters narrow the tracks by the track ticket as they narrow `list`. `contract <path>...` gives, for each contract path (a crate and a type joined by `::`, or a bare type name matching the type in any crate), the open tickets that change it, a line of `list` and then the path, and refuses a path whose crate the tree does not hold. `branch <name>` prints the id of the ticket holding that branch and `landed <id>` the sha its branch landed at, each silent when there is none. Every verb answers from `origin/dev` after fetching it, whatever the working tree holds. `--tree` reads the working tree instead, which is how a branch is checked with the tickets it adds, and `--no-fetch` reads `origin/dev` as it was last fetched. When the trunk cannot be read as asked, or `--tree` was given, the answer opens with a notice naming what was read and how many commits behind `origin/dev` it is; `branch` and `landed`, which scripts read, refuse instead of printing one. Nothing is printed when no ticket matches."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let (notice, mut report) = self.answer(ctx);
        if !notice.is_empty() {
            report.output.insert_str(0, &notice);
        }
        report
    }
}

impl Tickets {
    /// The answer, and the notice that goes before it where the tickets were not the trunk's as just fetched.
    pub(crate) fn answer(&self, ctx: &ToolContext<'_>) -> (String, ToolReport) {
        let req = match args::tickets(ctx.args) {
            Ok(r) => r,
            Err(e) => return (String::new(), ToolReport::inconclusive(e)),
        };
        let reading = match trunk::read(ctx.repo_root, ctx.mock_dir, req.opts) {
            Ok(r) => r,
            Err(e) => return (String::new(), ToolReport::inconclusive(e)),
        };
        // `branch` and `landed` are read by scripts, so a trunk that could not be read as asked is
        // a refusal for them and a notice before the answer for the rest
        let machine = matches!(req.verb, Verb::Branch(_) | Verb::Landed(_));
        if machine && reading.degraded {
            return (
                String::new(),
                ToolReport::inconclusive(reading.notice.trim_end()),
            );
        }
        let notice = if machine { String::new() } else { reading.notice };
        let store = reading.store;
        let plan = reading.plan;
        let git = landing::Git::of(ctx);
        let report = match &req.verb {
            Verb::Check => {
                let tenets = canon::tenets(ctx.registry);
                check(
                    &store,
                    &plan,
                    &git,
                    &canon::heirs(&tenets),
                    &refs::Known::of(ctx.registry),
                )
            },
            Verb::Branch(name) => {
                let ids: String = store
                    .with_branch(name)
                    .map(|t| format!("{}\n", t.id))
                    .collect();
                ToolReport::reported(ids, store.tickets.len())
            },
            Verb::Landed(id) => {
                if !store.tickets.contains_key(id) {
                    return (
                        String::new(),
                        ToolReport::inconclusive(format!("`{id}` is no ticket")),
                    );
                }
                let sha = landing::landed(&store, id, &git)
                    .map_or_else(String::new, |s| format!("{s}\n"));
                ToolReport::reported(sha, 1)
            },
            Verb::Tracks => {
                let (out, n) = view::tracks(&store, &plan, &req.filter);
                ToolReport::reported(out, n)
            },
            Verb::Contract(paths) => {
                match contract_lines(&store, &plan, ctx.mock_dir, paths) {
                    Ok((out, n)) => ToolReport::reported(out, n),
                    Err(e) => return (String::new(), ToolReport::inconclusive(e)),
                }
            },
            Verb::Answerable => {
                let rows = view::list(&store, &plan, &answerable::open_questions(&req.filter));
                // The index is made only where there is a question to ask about.
                let ready = (!rows.is_empty()).then(|| {
                    let op = corpus::corpus::op_words_dir(
                        ctx.repo_root,
                        std::env::var(corpus::corpus::OP_WORDS_VAR).ok().as_deref(),
                    );
                    corpus::repo::ready(ctx.mock_dir, ctx.repo_root, op.as_deref())
                });
                let (mut out, n) = answerable::report(&store, &rows, &|text| {
                    ready.as_ref().map_or_else(Vec::new, |r| {
                        corpus::repo::find(&r.index, text, answerable::PER_QUESTION)
                    })
                });
                if let Some(r) = &ready {
                    for note in &r.notes {
                        out.insert_str(0, &format!("{note}\n"));
                    }
                }
                ToolReport::reported(out, n)
            },
            Verb::List | Verb::Next => {
                let next = req.verb == Verb::Next;
                let rows = if next {
                    view::next(&store, &plan, &req.filter, &git)
                } else {
                    view::list(&store, &plan, &req.filter)
                };
                let out: String = rows
                    .iter()
                    .map(|r| format!("{}\n", view::line(r, next)))
                    .collect();
                ToolReport::reported(out, rows.len())
            },
        };
        (notice, report)
    }
}

/// Whether a crate of the dashed spelling has a directory among the workspace's crates.
fn crate_held(mock_dir: &std::path::Path, krate: &str) -> bool {
    mock_dir.join("crates").join(krate).is_dir()
}

/// The lines of `tickets contract`: for each path the open tickets that change it, each line of `list`
/// followed by the path, and the count of lines. A path whose crate the tree does not hold is refused,
/// so a mistyped crate is not read as nothing changing it.
fn contract_lines(
    store: &Store,
    plan: &Plan,
    mock_dir: &std::path::Path,
    paths: &[String],
) -> Result<(String, usize), String> {
    let mut out = String::new();
    let mut count = 0;
    for path in paths {
        if let Some((krate, _)) = path.split_once("::") {
            ticket_model::contracts::shape(path)?;
            if !crate_held(mock_dir, krate) {
                return Err(format!(
                    "`{path}` names the crate `{krate}`, which the tree does not hold"
                ));
            }
        } else if path.is_empty() || path.contains(char::is_whitespace) {
            return Err(format!(
                "`{path}` is neither a crate and a type joined by `::` nor a type name"
            ));
        }
        for row in view::changing(store, plan, path) {
            out.push_str(&format!("{}  {path}\n", view::line(&row, false)));
            count += 1;
        }
    }
    Ok((out, count))
}

fn check(
    store: &Store,
    plan: &Plan,
    git: &(impl landing::Trunk + Tree),
    heirs: &Heirs,
    known: &refs::Known,
) -> ToolReport {
    let trunk = git;
    let findings: Vec<LintError> = rules::check_store(store)
        .into_iter()
        .chain(landing::findings(store, trunk))
        .chain(known.findings(store))
        .chain(planning::rules::check_plan(plan))
        .map(|f| {
            LintError::error(
                "registry".to_string(),
                0,
                "tickets-check",
                format!("`{}`: {}", f.row, f.message),
            )
        })
        .collect();
    let output = stale_lines(store, git, heirs);
    if findings.is_empty() {
        ToolReport::reported(output, store.tickets.len())
    } else {
        ToolReport {
            outcome: Outcome::Findings(findings),
            output,
        }
    }
}

mockspace::lint_pack! {
    repo_lints: [lint::ATicketGraphHolds],
    tools: [Tickets],
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_answerable;
#[cfg(test)]
mod tests_branch;
#[cfg(test)]
mod tests_choice;
#[cfg(test)]
mod tests_contracts;
#[cfg(test)]
mod tests_links;
#[cfg(test)]
mod tests_members;
#[cfg(test)]
mod tests_order;
#[cfg(test)]
mod tests_question;
#[cfg(test)]
mod tests_refs;
#[cfg(test)]
mod tests_shipped;
#[cfg(test)]
mod tests_successor;
#[cfg(test)]
mod tests_supersedes;
#[cfg(test)]
mod tests_tags;
#[cfg(test)]
mod tests_tracks;
#[cfg(test)]
mod tests_trunk;

/// The tickets that may have stopped being true, one line each and then the count, or nothing where there
/// are none. A list a person reads: it fails nothing.
fn stale_lines(store: &Store, tree: &impl Tree, heirs: &Heirs) -> String {
    let stale = ticket_model::stale::gone_names(store, tree)
        .into_iter()
        .chain(ticket_model::stale::left_open(store));
    let repoint = ticket_model::heirs::left_naming(store, heirs);
    let mut out = String::new();
    for f in stale {
        out.push_str(&format!("{}: {}\n", f.row, f.message));
    }
    for f in repoint {
        out.push_str(&format!("{}: {}\n", f.row, f.message));
    }
    let closable = ticket_model::stale::closable(store, tree).len();
    if closable > 0 {
        out.push_str(&format!(
            "{closable} of them name only what the tree lost: ticket-move --close-gone\n"
        ));
    }
    let count = ticket_model::stale::stale_count(store, tree);
    if count > 0 {
        out.push_str(&format!("{count} tickets may be stale: tickets check\n"));
    }
    out
}

#[cfg(test)]
mod tests_tree;

#[cfg(test)]
mod tests_repoint;

#[cfg(test)]
mod tests_stale;

#[cfg(test)]
mod tests_close_gone;
