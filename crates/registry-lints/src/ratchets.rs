//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The one reading of `ratchets.toml` in the mock directory, which `every-fixme-names-a-ticket` and a
//! consumer's own ratchet lints share. This file declares no lint.
//!
//! A ratchet is a check that reads every unit and a closed list of the units that do not yet pass it, each with
//! the ticket for getting it off the list. The list is `[[row]]` tables of `lint`, `subject`, `ticket` and, for
//! an exemption, `reason`, one `key = "value"` to a line. What a subject is, a crate or a file and the ticket it
//! cites, is the lint's to say. The verdict on a unit is the same for all of them, so it is written once here.

use std::collections::BTreeSet;
use std::path::Path;

use mockspace::{RegistryView, View};

use crate::layout::{ContentDirs, Layout, mock_rel};

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

/// The rows of a `ratchets.toml` text. Only the keys a row has are read, and a line that is none of a table header,
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
pub fn read(view: &View<'_>, mock: &Path) -> Vec<Row> {
    view.text(mock.join(FILE))
        .map(parse)
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
    view: &View<'_>,
    mock: &Path,
    lint: &'static str,
    found: &std::collections::BTreeMap<String, Vec<Finding>>,
) -> Vec<mockspace::LintError> {
    let file = Layout::new(&mock_rel(mock, view.root()), ContentDirs::default()).in_mock(FILE);
    let rows = read(view, mock);
    let rows = of(&rows, lint);
    let units: BTreeSet<&str> = found
        .keys()
        .map(String::as_str)
        .chain(rows.iter().map(|r| r.subject.as_str()))
        .collect();
    let mut out = Vec::new();
    for unit in units {
        let row = rows.iter().find(|r| r.subject == unit).copied();
        out.extend(judge_unit(lint, &file, unit, row, found.get(unit)));
    }
    out
}

/// The verdict of lint `lint` over one unit, given the rows' file as the repository names it, its row if it has
/// one and its findings if it has any. A lint that reads one unit at a time calls this for its own unit, so the
/// verdict is the same as the whole-register one.
pub fn judge_unit(
    lint: &'static str,
    file: &str,
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
                            "{msg}; `{file}` allows `{unit}` {cap} findings of `{lint}` for the ticket `{}` \
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
                        "`{file}` allows `{unit}` {cap} findings of `{lint}` for the ticket `{}` and it has \
                             {has} left: lower `findings` to {has}",
                        row.ticket
                    ),
                );
                e.path = Some(file.to_owned());
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
                        "{msg}; `{unit}` has no row in `{file}`, which names an open ticket for a unit that \
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
                    "`{file}` allows `{unit}` findings of `{lint}` for the ticket `{}` and it has none left: \
                         remove the row",
                    row.ticket
                ),
            );
            e.path = Some(file.to_owned());
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
#[path = "ratchets_tests.rs"]
mod tests;
