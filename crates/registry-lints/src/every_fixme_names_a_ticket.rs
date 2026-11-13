//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `every-fixme-names-a-ticket`: a placeholder marker cites the ticket opened for it.
//!
//! A marker is the word `FIXME`, the id of a ticket in parentheses, a colon and the reason. The
//! shape is fixed here and `cargo mock owed` files the tickets and rewrites the markers to
//! it; this refuses what is left: a marker with no parentheses, with a tag such as `canon silent` or
//! `maintainer` in them, with the id of a ticket the registry does not have, or with no colon after
//! the head.
//! A finding names the file and the line, and the fix is `cargo mock owed`, or the id of a ticket
//! opened by hand.
//!
//! What a marker is, and which files are read, is `fixme_markers.rs`, which `owed` reads by too. The
//! closed design rounds, the locked changelists and the research notes quote markers and cannot be
//! rewritten; vendored code and the generated documents are not this repository's to edit. None of
//! them is read, and `TODO` is not a marker. The ticket is looked up in the registry of the tree the
//! lint runs on, so a branch that adds a ticket and the marker citing it in one commit passes, and the
//! gate that runs on the merge is what makes the ticket one on the trunk. A ticket that is closed is refused: a
//! placeholder waiting on closed work was resolved or belongs to another ticket, so a closed ticket waives nothing.
//! The one exception is a question that was answered: the answer is the work to do and the marker waited for it, so
//! a marker citing it is owed work and is a warning, which never stops a landing. Whoever answers, from the console
//! or by `ticket-move --answer`, has no way to rewrite the markers in the same commit, and an answer arriving must
//! not turn the trunk red for every branch that has nothing to do with it.
//! The markers that cite one today are rows of `mock/ratchets.toml` (file and cited ticket, each with the open
//! ticket for removing it); a row whose file holds no such marker is refused, so the list only shrinks.
//!
//! In a merge that is not yet committed the lint judges what the merge adds and not what it carries in:
//! a marker with the same tag and words in the same file at either parent is that parent's, and is left
//! to the gate of the branch it came from (`merge_carried.rs`). Outside a merge every marker is judged.
//!
//! The home is this repository and not `mockspace-extra-lints`: no other repository has a `ticket`
//! namespace to look the ids up in, and the scopes above are this repository's layout. The reading is
//! a file of its own so that it can be lifted into the pack when a second repository has tickets.

use std::collections::BTreeSet;
use std::path::Path;

use mockspace::{Lint, LintError, RegistryView, RepoContext, RepoLint, Severity, View};

use crate::fixme_markers::{self, Scope};
use crate::retired_words::unit_of;
use crate::{merge_carried, ratchets};

pub fn repo_lint() -> Box<dyn RepoLint> {
    Box::new(EveryFixmeNamesATicket)
}

pub const NAME: &str = "every-fixme-names-a-ticket";

struct EveryFixmeNamesATicket;

impl Lint for EveryFixmeNamesATicket {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

impl RepoLint for EveryFixmeNamesATicket {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        let view = ctx.view();
        check(view, ctx.repo_root, ctx.registry(), &ratchets::read(view, ctx.mock_dir))
    }
}

/// Whether `s` is shaped like a ticket id: a slug.
fn is_slug(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_lowercase())
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn check(view: &View<'_>, repo: &Path, reg: &RegistryView, rows: &[ratchets::Row]) -> Vec<LintError> {
    let closed = ratchets::closed_tickets(reg);
    let answered = ratchets::answered_tickets(reg);
    let waivers = ratchets::of(rows, NAME);
    let mut used: BTreeSet<(String, String)> = BTreeSet::new();
    let tickets: BTreeSet<&str> = reg
        .rows_in("ticket")
        .iter()
        .map(|q| q.split_once("::").map_or(q.as_str(), |(_, id)| id))
        .collect();
    let fix = "run `cargo mock owed` to file the ticket and rewrite the marker, or write the id of the ticket you opened";
    let merging = merge_carried::in_progress(view, repo);
    let mut out = Vec::new();
    for source in fixme_markers::sources(view, repo) {
        if source.scope != Scope::Live {
            continue;
        }
        // Read from git only for a file with a marker to refuse, and once.
        let mut carried: Option<BTreeSet<merge_carried::Identity>> = None;
        for m in fixme_markers::scan(&source.text, source.kind) {
            let why = match m.tag.as_deref().map(str::trim) {
                None | Some("") => "cites no ticket".to_string(),
                Some(tag) if !is_slug(tag) => {
                    format!("cites `{tag}`, which is a tag and not a ticket id")
                },
                Some(tag) if !tickets.contains(tag) => {
                    format!("cites `{tag}`, which is no ticket in the registry")
                },
                Some(tag) if closed.contains(tag) => {
                    let waived = waivers
                        .iter()
                        .any(|r| r.subject.split_once("::") == Some((source.rel.as_str(), tag)));
                    if waived {
                        used.insert((source.rel.clone(), tag.to_owned()));
                        continue;
                    }
                    if let Some(at) = answered.get(tag) {
                        // A question someone answered is closed by the answer, and the work the answer sets
                        // has not been cited yet: owed work, which must not stop an unrelated landing.
                        let mut e = LintError::warning(
                            unit_of(Path::new(&source.rel)),
                            m.line,
                            NAME,
                            format!(
                                "`{}:{}` is a placeholder marker that cites `{tag}`, a question answered on {}: the work its \
                                 answer sets is owed. Cite the open task that holds it (`cargo mock owed` files one), or hold \
                                 the marker with a row of `mock/{}` where the file is in a gated crate",
                                source.rel,
                                m.line,
                                at.split('T').next().unwrap_or(at),
                                ratchets::FILE
                            ),
                        );
                        e.path = Some(source.rel.clone());
                        out.push(e);
                        continue;
                    }
                    format!(
                        "cites `{tag}`, a ticket that is closed, so the placeholder it waits on is resolved or \
                         belongs to an open ticket: resolve it, or cite the open ticket"
                    )
                },
                Some(_) if !m.colon => {
                    "has no colon after it, so its reason is not a reason".to_string()
                },
                Some(_) => continue,
            };
            if merging
                && carried
                    .get_or_insert_with(|| merge_carried::carried(view, repo, &source.rel, source.kind))
                    .contains(&merge_carried::identity(&m))
            {
                continue;
            }
            let mut e = LintError::error(
                unit_of(Path::new(&source.rel)),
                m.line,
                NAME,
                format!(
                    "`{}:{}` is a placeholder marker that {why}: {fix}",
                    source.rel, m.line
                ),
            );
            e.path = Some(source.rel.clone());
            out.push(e);
        }
    }
    for r in waivers {
        let held = r
            .subject
            .split_once("::")
            .is_some_and(|(file, tag)| used.contains(&(file.to_owned(), tag.to_owned())));
        if !held {
            let mut e = LintError::error(
                // FIXME(pack_lints_and_tools_read_the_mock_directory_as_mock): the mock directory is read as `mock`, not as the one `mockspace.toml` declares
                "mock".to_owned(),
                r.line,
                NAME,
                format!(
                    "`mock/{}` allows `{}` for the ticket `{}` and no marker in that file cites that closed ticket \
                     now: remove the row",
                    ratchets::FILE,
                    r.subject,
                    r.ticket
                ),
            );
            // FIXME(pack_lints_and_tools_read_the_mock_directory_as_mock): the mock directory is read as `mock`, not as the one `mockspace.toml` declares
            e.path = Some(format!("mock/{}", ratchets::FILE));
            out.push(e);
        }
    }
    out
}

#[cfg(test)]
#[path = "every_fixme_names_a_ticket_tests.rs"]
mod tests;
