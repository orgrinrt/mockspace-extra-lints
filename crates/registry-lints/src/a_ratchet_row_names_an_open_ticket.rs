//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `a-ratchet-row-names-an-open-ticket`: a row of `mock/ratchets.toml` is a waiver for a unit that does not yet
//! pass a lint, and a waiver is only as good as the work it waits on, after
//! `a_ticket_closes_only_when_fixed_unless_the_behaviour_is_intended_and_then_with_a_line_saying_why`.
//!
//! A row names a lint of those that read the file, a subject, and a ticket. It is refused when the lint is none
//! of them, when the subject is empty, when the ticket is no ticket of the registry or is in a closed state
//! (a closed ticket waives forever, which is the defect the file exists to end), when an exemption of the
//! lint that held a crate list carries no reason, and when the same lint and subject stand in two rows. What a
//! row's unit does is the owning lint's to judge; this lint reads only the rows, once, for all of them.

use std::collections::BTreeSet;

use mockspace::{Lint, LintError, RegistryView, RepoContext, RepoLint, Severity};

use crate::ratchets::{self, Row};
use crate::{
    a_bevy_is_taken_part_by_part as bevy,
    a_crate_due_for_deletion_does_not_grow as due,
    a_domain_exports_its_contract_only as exports,
    a_shader_reads_a_shared_result_not_a_stored_copy as stored,
    an_exempt_crate_has_a_ratchet_row as exempt,
    an_external_thing_has_an_intake_row as intake,
    every_fixme_names_a_ticket as fixmes,
    no_lock_in_a_resource as locks,
    no_per_thing_map_in_a_resource as maps,
    slang_concept_has_one_owner as owners,
    slang_constants_are_generated as generated,
    slang_imports_resolve as slang,
    the_simulation_links_no_renderer as sim,
};

pub fn repo_lint() -> Box<dyn RepoLint> {
    Box::new(ARatchetRowNamesAnOpenTicket)
}

const NAME: &str = "a-ratchet-row-names-an-open-ticket";

/// The lints that read `mock/ratchets.toml`.
const LINTS: [&str; 17] = [
    stored::NAME,
    slang::NAME,
    owners::NAME,
    generated::NAME,
    due::NAME,
    locks::NAME,
    maps::NAME,
    fixmes::NAME,
    bevy::NAME,
    exports::NAME,
    sim::NAME,
    intake::NAME,
    exempt::FORMER[0],
    exempt::FORMER[1],
    exempt::FORMER[2],
    exempt::FORMER[3],
    exempt::FORMER[4],
];

struct ARatchetRowNamesAnOpenTicket;

impl Lint for ARatchetRowNamesAnOpenTicket {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

impl RepoLint for ARatchetRowNamesAnOpenTicket {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        check(&ratchets::read(ctx.mock_dir), ctx.registry)
    }
}

fn check(rows: &[Row], reg: &RegistryView) -> Vec<LintError> {
    let closed = ratchets::closed_tickets(reg);
    let mut seen: BTreeSet<(&str, &str)> = BTreeSet::new();
    let mut out = Vec::new();
    for r in rows {
        let mut why = Vec::new();
        if !LINTS.contains(&r.lint.as_str()) {
            why.push(format!(
                "`{}` is none of the lints that read this file",
                r.lint
            ));
        }
        if r.subject.is_empty() {
            why.push("it names no subject".to_owned());
        }
        if !ratchets::has_ticket(reg, &r.ticket) {
            why.push(format!("`{}` is no ticket in the registry", r.ticket));
        } else if closed.contains(&r.ticket) {
            why.push(format!(
                "`{}` is closed, so it waives nothing: name the open ticket",
                r.ticket
            ));
        }
        if exempt::FORMER.contains(&r.lint.as_str()) && r.reason.is_empty() {
            why.push("an exemption from a lint carries a reason".to_owned());
        }
        if !seen.insert((r.lint.as_str(), r.subject.as_str())) {
            why.push("the same lint and subject stand in an earlier row".to_owned());
        }
        for w in why {
            let mut e = LintError::error(
                "mock".to_owned(),
                r.line,
                NAME,
                format!(
                    "the row of `{}` for `{}` is refused: {w}",
                    r.lint, r.subject
                ),
            );
            e.path = Some(format!("mock/{}", ratchets::FILE));
            out.push(e);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canon_rows::testkit::view;

    fn reg() -> RegistryView {
        view(&[
            ("ticket::open_one", &[("title", "x")]),
            ("ticket::shut_one", &[("title", "y")]),
            ("ticket_state::doing", &[("closed", "false")]),
            ("ticket_state::done", &[("closed", "true")]),
            ("ticket_move::a", &[
                ("ticket", "open_one"),
                ("to", "doing"),
                ("at", "2026-11-01T00:00:00Z"),
            ]),
            ("ticket_move::b", &[
                ("ticket", "shut_one"),
                ("to", "done"),
                ("at", "2026-11-01T00:00:00Z"),
            ]),
        ])
    }

    fn found(text: &str) -> Vec<LintError> {
        check(&ratchets::parse(text), &reg())
    }

    fn row(lint: &str, subject: &str, ticket: &str) -> String {
        format!("[[row]]\nlint = \"{lint}\"\nsubject = \"{subject}\"\nticket = \"{ticket}\"\n")
    }

    #[test]
    fn a_row_with_an_open_ticket_passes() {
        assert!(found(&row(locks::NAME, "kaski-a", "open_one")).is_empty());
    }

    #[test]
    fn a_row_with_a_closed_ticket_is_refused() {
        let f = found(&row(locks::NAME, "kaski-a", "shut_one"));
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("is closed"), "{}", f[0].message);
    }

    #[test]
    fn a_crate_due_for_deletion_whose_ticket_is_closed_is_refused_while_the_row_stands() {
        let f = found(&row(due::NAME, "kaski-render", "shut_one"));
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].message.contains("is closed"), "{}", f[0].message);
        assert!(found(&row(due::NAME, "kaski-render", "open_one")).is_empty());
    }

    #[test]
    fn a_row_with_no_such_ticket_is_refused() {
        let f = found(&row(maps::NAME, "kaski-a", "nowhere"));
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(
            f[0].message.contains("no ticket in the registry"),
            "{}",
            f[0].message
        );
    }

    #[test]
    fn a_row_for_a_lint_that_does_not_read_the_file_is_refused() {
        let f = found(&row("some-other-lint", "kaski-a", "open_one"));
        assert_eq!(f.len(), 1, "{f:?}");
    }

    #[test]
    fn an_exemption_from_a_lint_needs_a_reason() {
        let bare = found(&row(exempt::FORMER[0], "kaski-app", "open_one"));
        assert_eq!(bare.len(), 1, "{bare:?}");
        let with = format!(
            "{}reason = \"over\"\n",
            row(exempt::FORMER[0], "kaski-app", "open_one")
        );
        assert!(found(&with).is_empty());
    }

    #[test]
    fn the_same_lint_and_subject_twice_is_refused() {
        let twice = format!(
            "{}{}",
            row(locks::NAME, "kaski-a", "open_one"),
            row(locks::NAME, "kaski-a", "open_one")
        );
        let f = found(&twice);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].line, 5);
    }

    #[test]
    fn the_same_subject_under_two_lints_is_two_rows() {
        let two = format!(
            "{}{}",
            row(locks::NAME, "kaski-a", "open_one"),
            row(maps::NAME, "kaski-a", "open_one")
        );
        assert!(found(&two).is_empty());
    }
}
