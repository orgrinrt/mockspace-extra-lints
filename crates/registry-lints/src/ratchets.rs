//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The one reading of `mock/ratchets.toml`, which the four ratchet lints share. This file declares no lint.
//!
//! A ratchet is a check that reads every unit and a closed list of the units that do not yet pass it, each with
//! the ticket for getting it off the list. The list is `[[row]]` tables of `lint`, `subject`, `ticket` and, for
//! an exemption, `reason`, one `key = "value"` to a line. What a subject is, a crate or a file and the ticket it
//! cites, is the lint's to say. The verdict on a unit is the same for all of them, so it is written once here.

use std::collections::BTreeSet;
use std::path::Path;

use mockspace::RegistryView;

/// Where the rows stand, under the mock directory.
pub const FILE: &str = "ratchets.toml";

/// One row: the lint it belongs to, the unit it allows, the open ticket for removing it and why it is exempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The line the row's table opens on, from 1.
    pub line:     usize,
    pub lint:     String,
    pub subject:  String,
    pub ticket:   String,
    pub reason:   String,
    /// For a row that holds a size: the paths under the crate it measures, comma separated, and the whole crate
    /// when empty.
    pub paths:    String,
    /// For a row that holds a size: the files and the counted lines it allowed when it was last written down.
    pub files:    usize,
    pub lines:    usize,
    /// How many findings the row allows its unit, or none for a row that allows every finding the unit has. A
    /// branch that ports part of a unit lowers the count to what is left, and one that ports all of it removes
    /// the row.
    pub findings: usize,
}

/// The rows of a `ratchets.toml` text. Only the four keys are read, and a line that is none of a table header,
/// a key or a comment is left alone.
pub fn parse(text: &str) -> Vec<Row> {
    let mut rows: Vec<Row> = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line == "[[row]]" {
            rows.push(Row {
                line:     i + 1,
                lint:     String::new(),
                subject:  String::new(),
                ticket:   String::new(),
                reason:   String::new(),
                paths:    String::new(),
                files:    0,
                lines:    0,
                findings: 0,
            });
            continue;
        }
        let Some(row) = rows.last_mut() else {
            continue;
        };
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_start_matches('"').trim_end_matches('"');
        match key.trim() {
            "lint" => row.lint = value.to_owned(),
            "subject" => row.subject = value.to_owned(),
            "ticket" => row.ticket = value.to_owned(),
            "reason" => row.reason = value.to_owned(),
            "paths" => row.paths = value.to_owned(),
            "files" => row.files = value.parse().unwrap_or_default(),
            "lines" => row.lines = value.parse().unwrap_or_default(),
            "findings" => row.findings = value.parse().unwrap_or_default(),
            _ => {},
        }
    }
    rows
}

/// The rows in the mock directory's file, none when the file is missing.
pub fn read(mock: &Path) -> Vec<Row> {
    std::fs::read_to_string(mock.join(FILE))
        .map(|t| parse(&t))
        .unwrap_or_default()
}

/// The rows that belong to `lint`.
pub fn of<'a>(rows: &'a [Row], lint: &str) -> Vec<&'a Row> {
    rows.iter().filter(|r| r.lint == lint).collect()
}

/// What a ratchet says of one unit.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The unit passes with no row, or is allowed its findings by a row.
    Holds,
    /// The unit has findings and no row: a new offender.
    New,
    /// The unit has a row and no finding left: the row is to go.
    Stale,
}

/// The verdict on a unit that has findings or not, and a row or not.
pub const fn verdict(found: bool, waived: bool) -> Verdict {
    match (found, waived) {
        (true, false) => Verdict::New,
        (false, true) => Verdict::Stale,
        _ => Verdict::Holds,
    }
}

/// One finding of a ratchet lint: the file it is in, the line and the words.
pub type Finding = (String, usize, String);

/// The verdict of lint `lint` over every unit its lint read, `found` being the findings of each unit by name.
/// A unit with findings and no row is refused on each finding, and a row for a unit with none is refused on
/// the row. Every repo lint that reads units by name holds its ratchet here, so the reading is written once.
pub fn judge(
    mock: &Path,
    lint: &'static str,
    found: &std::collections::BTreeMap<String, Vec<Finding>>,
) -> Vec<mockspace::LintError> {
    let rows = read(mock);
    let rows = of(&rows, lint);
    let units: BTreeSet<&str> = found
        .keys()
        .map(String::as_str)
        .chain(rows.iter().map(|r| r.subject.as_str()))
        .collect();
    let mut out = Vec::new();
    for unit in units {
        let row = rows.iter().find(|r| r.subject == unit).copied();
        out.extend(judge_unit(lint, unit, row, found.get(unit)));
    }
    out
}

/// The verdict of lint `lint` over one unit, given its row if it has one and its findings if it has any. A lint that
/// reads one unit at a time calls this for its own unit, so the verdict is the same as the whole-register one.
pub fn judge_unit(
    lint: &'static str,
    unit: &str,
    row: Option<&Row>,
    findings: Option<&Vec<Finding>>,
) -> Vec<mockspace::LintError> {
    let mut out = Vec::new();
    match verdict(findings.is_some(), row.is_some()) {
        Verdict::Holds => {
            let (Some(row), Some(findings)) = (row, findings) else {
                return out;
            };
            let (cap, has) = (row.findings, findings.len());
            if cap == 0 || cap == has {
                return out;
            }
            if has > cap {
                for (path, line, msg) in findings {
                    let mut e = mockspace::LintError::error(
                        unit.to_owned(),
                        *line,
                        lint,
                        format!(
                            "{msg}; `mock/{FILE}` allows `{unit}` {cap} findings of `{lint}` for the ticket `{}` \
                                 and it has {has}: a unit is never allowed more than its row counts",
                            row.ticket
                        ),
                    );
                    e.path = Some(path.clone());
                    out.push(e);
                }
            } else {
                let mut e = mockspace::LintError::error(
                    unit.to_owned(),
                    row.line,
                    lint,
                    format!(
                        "`mock/{FILE}` allows `{unit}` {cap} findings of `{lint}` for the ticket `{}` and it has \
                             {has} left: lower `findings` to {has}",
                        row.ticket
                    ),
                );
                e.path = Some(format!("mock/{FILE}"));
                out.push(e);
            }
        },
        Verdict::New => {
            for (path, line, msg) in findings.into_iter().flatten() {
                let mut e = mockspace::LintError::error(
                    unit.to_owned(),
                    *line,
                    lint,
                    format!(
                        "{msg}; `{unit}` has no row in `mock/{FILE}`, which names an open ticket for a unit that \
                             holds this today, and a new offender is not allowed one"
                    ),
                );
                e.path = Some(path.clone());
                out.push(e);
            }
        },
        Verdict::Stale => {
            let row = row.expect("a stale verdict has a row");
            let mut e = mockspace::LintError::error(
                unit.to_owned(),
                row.line,
                lint,
                format!(
                    "`mock/{FILE}` allows `{unit}` findings of `{lint}` for the ticket `{}` and it has none left: \
                         remove the row",
                    row.ticket
                ),
            );
            e.path = Some(format!("mock/{FILE}"));
            out.push(e);
        },
    }
    out
}

/// The tickets of the registry in a closed state: the newest `ticket_move` of each, by time and then by id, is
/// to a state whose row says `closed`. A ticket with no move is not closed.
pub fn closed_tickets(reg: &RegistryView) -> BTreeSet<String> {
    closing_moves(reg).into_keys().collect()
}

/// The closed tickets whose newest move carries an answer or picks, each with the time of that move: a
/// question that someone answered. The answer is the work to do and not a fault in a marker that waited for it, so a marker
/// citing one is owed work and the lint that refuses a marker citing a closed ticket reports it as a warning.
/// A ticket closed with no answer is not here.
pub fn answered_tickets(reg: &RegistryView) -> std::collections::BTreeMap<String, String> {
    closing_moves(reg)
        .into_iter()
        .filter(|(_, (_, q))| {
            // the kind is satisfied by an answer or by picks, so a move carrying either is an answer
            ["answer", "picks"].iter().any(|key| {
                reg.field(q, key)
                    .is_some_and(|a| !matches!(a.trim(), "" | "[]"))
            })
        })
        .map(|(ticket, (at, _))| (ticket, at))
        .collect()
}

/// For each ticket whose newest `ticket_move` is to a closed state, the time and the row of that move.
fn closing_moves(reg: &RegistryView) -> std::collections::BTreeMap<String, (String, String)> {
    let mut newest: std::collections::BTreeMap<String, (String, String, String)> =
        std::collections::BTreeMap::new();
    for q in reg.rows_in("ticket_move") {
        let (Some(ticket), Some(to)) = (reg.field(q, "ticket"), reg.field(q, "to")) else {
            continue;
        };
        let at = reg.field(q, "at").unwrap_or_default().to_owned();
        let entry = newest.entry(ticket.to_owned()).or_default();
        if (at.as_str(), q.as_str()) >= (entry.0.as_str(), entry.1.as_str()) {
            *entry = (at, q.clone(), to.to_owned());
        }
    }
    newest
        .into_iter()
        .filter(|(_, (_, _, to))| {
            reg.field(&format!("ticket_state::{to}"), "closed") == Some("true")
        })
        .map(|(ticket, (at, q, _))| (ticket, (at, q)))
        .collect()
}

/// Whether the registry has the ticket.
pub fn has_ticket(reg: &RegistryView, id: &str) -> bool {
    reg.row(&format!("ticket::{id}")).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canon_rows::testkit::view;

    const TEXT: &str = "# a comment\n[[row]]\nlint = \"a\"\nsubject = \"kaski-x\"\nticket = \"t_one\"\n\n[[row]]\nlint = \"b\"\nsubject = \"f.rs::t_two\"\nticket = \"t_three\"\nreason = \"over\"\n";

    #[test]
    fn rows_are_read_with_their_keys_and_lines() {
        let rows = parse(TEXT);
        assert_eq!(rows.len(), 2);
        assert_eq!(
            (
                rows[0].lint.as_str(),
                rows[0].subject.as_str(),
                rows[0].line
            ),
            ("a", "kaski-x", 2)
        );
        assert_eq!(rows[1].reason, "over");
        assert_eq!(of(&rows, "b").len(), 1);
    }

    #[test]
    fn a_size_row_keeps_its_paths_files_and_lines() {
        let rows = parse(
            "[[row]]\nlint = \"d\"\nsubject = \"kaski-x\"\nticket = \"t\"\npaths = \"src/a.rs,src/b\"\nfiles = 3\nlines = 120\n",
        );
        assert_eq!(rows[0].paths, "src/a.rs,src/b");
        assert_eq!((rows[0].files, rows[0].lines), (3, 120));
        // the control: a row naming none has no size, so nothing reads one into it
        let bare = parse("[[row]]\nlint = \"d\"\n");
        assert_eq!(
            (bare[0].files, bare[0].lines, bare[0].paths.as_str()),
            (0, 0, "")
        );
    }

    #[test]
    fn a_key_before_any_table_and_an_unknown_key_are_left_alone() {
        assert!(parse("lint = \"a\"\n").is_empty());
        assert_eq!(parse("[[row]]\nwhat = \"a\"\n")[0].lint, "");
    }

    #[test]
    fn the_verdict_refuses_a_new_offender_and_a_row_with_nothing_left() {
        assert_eq!(verdict(true, false), Verdict::New);
        assert_eq!(verdict(false, true), Verdict::Stale);
        assert_eq!(verdict(true, true), Verdict::Holds);
        assert_eq!(verdict(false, false), Verdict::Holds);
    }

    fn reg() -> RegistryView {
        view(&[
            ("ticket::open_one", &[("title", "x")]),
            ("ticket::shut_one", &[("title", "y")]),
            ("ticket_state::doing", &[("closed", "false")]),
            ("ticket_state::done", &[("closed", "true")]),
            ("ticket_move::m1", &[
                ("ticket", "open_one"),
                ("to", "doing"),
                ("at", "2026-11-01T00:00:00Z"),
            ]),
            ("ticket_move::m2", &[
                ("ticket", "shut_one"),
                ("to", "doing"),
                ("at", "2026-11-01T00:00:00Z"),
            ]),
            ("ticket_move::m3", &[
                ("ticket", "shut_one"),
                ("to", "done"),
                ("at", "2026-11-02T00:00:00Z"),
            ]),
        ])
    }

    #[test]
    fn a_ticket_whose_newest_move_is_to_a_closed_state_is_closed() {
        let closed = closed_tickets(&reg());
        assert!(closed.contains("shut_one"));
        assert!(!closed.contains("open_one"));
    }

    #[test]
    fn a_ticket_closed_with_an_answer_or_picks_is_answered_and_one_closed_with_neither_is_not() {
        let r = view(&[
            ("ticket_state::doing", &[("closed", "false")]),
            ("ticket_state::done", &[("closed", "true")]),
            ("ticket_move::w", &[
                ("ticket", "worded"),
                ("to", "done"),
                ("at", "2026-11-02T00:00:00Z"),
                ("answer", "Any roof."),
            ]),
            ("ticket_move::p", &[
                ("ticket", "picked"),
                ("to", "done"),
                ("at", "2026-11-02T01:00:00Z"),
                ("picks", "[\"Tool required\"]"),
            ]),
            ("ticket_move::e", &[
                ("ticket", "empty"),
                ("to", "done"),
                ("at", "2026-11-02T02:00:00Z"),
                ("answer", ""),
                ("picks", "[]"),
            ]),
            ("ticket_move::n", &[
                ("ticket", "bare"),
                ("to", "done"),
                ("at", "2026-11-02T03:00:00Z"),
            ]),
            ("ticket_move::o", &[
                ("ticket", "reopened"),
                ("to", "done"),
                ("at", "2026-11-01T00:00:00Z"),
                ("answer", "yes"),
            ]),
            ("ticket_move::o2", &[
                ("ticket", "reopened"),
                ("to", "doing"),
                ("at", "2026-11-02T00:00:00Z"),
            ]),
        ]);
        let answered = answered_tickets(&r);
        assert_eq!(answered.keys().map(String::as_str).collect::<Vec<_>>(), [
            "picked", "worded"
        ]);
        assert_eq!(answered["worded"], "2026-11-02T00:00:00Z");
        // the control: every one of them is closed but the reopened one, answered or not
        let closed = closed_tickets(&r);
        for t in ["worded", "picked", "empty", "bare"] {
            assert!(closed.contains(t), "{t}");
        }
        assert!(!closed.contains("reopened"));
    }

    #[test]
    fn an_older_move_to_a_closed_state_does_not_close_a_reopened_ticket() {
        let r = view(&[
            ("ticket::t", &[("title", "x")]),
            ("ticket_state::doing", &[("closed", "false")]),
            ("ticket_state::done", &[("closed", "true")]),
            ("ticket_move::a", &[
                ("ticket", "t"),
                ("to", "done"),
                ("at", "2026-11-01T00:00:00Z"),
            ]),
            ("ticket_move::b", &[
                ("ticket", "t"),
                ("to", "doing"),
                ("at", "2026-11-02T00:00:00Z"),
            ]),
        ]);
        assert!(closed_tickets(&r).is_empty());
    }

    #[test]
    fn the_judge_refuses_a_new_unit_and_a_stale_row_and_lets_a_listed_unit_hold() {
        let d = crate::canon_rows::testkit::scratch("judge");
        let found = |units: &[&str]| {
            units
                .iter()
                .map(|u| ((*u).to_owned(), vec![("p".to_owned(), 3, "bad".to_owned())]))
                .collect::<std::collections::BTreeMap<_, _>>()
        };
        crate::canon_rows::testkit::write(
            &d,
            FILE,
            "[[row]]\nlint = \"l\"\nsubject = \"listed\"\nticket = \"t\"\n\n[[row]]\nlint = \"l\"\nsubject = \"gone\"\nticket = \"t\"\n",
        );
        let said: Vec<String> = judge(&d, "l", &found(&["listed", "new"]))
            .into_iter()
            .map(|e| e.message)
            .collect();
        assert_eq!(said.len(), 2);
        assert!(said.iter().any(|m| m.contains("`new` has no row")));
        assert!(
            said.iter()
                .any(|m| m.contains("`gone`") && m.contains("remove the row"))
        );
    }

    #[test]
    fn a_row_keeps_the_count_it_allows_and_a_row_naming_none_allows_any() {
        let rows = parse("[[row]]\nlint = \"l\"\nsubject = \"u\"\nticket = \"t\"\nfindings = 7\n");
        assert_eq!(rows[0].findings, 7);
        // the control: a row with no count reads as none, which allows every finding it ever did
        assert_eq!(parse("[[row]]\nlint = \"l\"\n")[0].findings, 0);
    }

    /// The messages the judge gives a unit with `n` findings under a row allowing `cap`.
    fn counted(cap: usize, n: usize) -> Vec<String> {
        let d = crate::canon_rows::testkit::scratch("counted");
        crate::canon_rows::testkit::write(
            &d,
            FILE,
            &format!("[[row]]\nlint = \"l\"\nsubject = \"u\"\nticket = \"t\"\nfindings = {cap}\n"),
        );
        let found = std::iter::once((
            "u".to_owned(),
            (0 .. n)
                .map(|i| ("p".to_owned(), i + 1, "bad".to_owned()))
                .collect::<Vec<_>>(),
        ))
        .collect::<std::collections::BTreeMap<_, _>>();
        judge(&d, "l", &found)
            .into_iter()
            .map(|e| e.message)
            .collect()
    }

    #[test]
    fn a_counted_row_holds_a_unit_at_its_count() {
        assert!(counted(4, 4).is_empty());
    }

    #[test]
    fn a_counted_row_refuses_a_unit_over_its_count() {
        let said = counted(4, 5);
        assert!(!said.is_empty());
        assert!(
            said.iter()
                .all(|m| m.contains("4 findings") && m.contains("it has 5")),
            "{said:?}"
        );
    }

    #[test]
    fn a_counted_row_above_the_findings_left_says_to_lower_it() {
        let said = counted(7, 4);
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(
            said[0].contains("lower") && said[0].contains('4') && said[0].contains('7'),
            "{said:?}"
        );
    }

    #[test]
    fn a_ticket_is_in_the_registry_or_not() {
        assert!(has_ticket(&reg(), "open_one"));
        assert!(!has_ticket(&reg(), "nope"));
    }
}
