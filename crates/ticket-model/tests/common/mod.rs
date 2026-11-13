//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! A scratch registry for the lists over the tickets: two states, one open and one closed, and a task
//! kind that is either.

use kaski_tickets::Store;

const VOCABULARY: &str = r#"
[[ticket_state]]
id = "backlog"
column = 1

[[ticket_state]]
id = "done"
column = 2
closed = true
satisfies = true

[[ticket_kind]]
id = "task"
states = ["backlog", "done"]
initial = "backlog"
"#;

/// One ticket: its id, the lines of its row, and whether it is closed.
pub struct Row<'a> {
    pub id:     &'a str,
    pub lines:  &'a str,
    pub closed: bool,
}

pub fn open<'a>(id: &'a str, lines: &'a str) -> Row<'a> {
    Row {
        id,
        lines,
        closed: false,
    }
}

pub fn closed<'a>(id: &'a str, lines: &'a str) -> Row<'a> {
    Row {
        id,
        lines,
        closed: true,
    }
}

fn text(r: &Row<'_>) -> String {
    let to = if r.closed { "done" } else { "backlog" };
    format!(
        "[[ticket]]\nid = \"{id}\"\nkind = \"task\"\n{lines}\n\n[[ticket_move]]\nid = \"m_{id}\"\nticket = \"{id}\"\nto = \"{to}\"\nby = \"coord\"\nat = \"2026-11-02T10:00:00Z\"\n",
        id = r.id,
        lines = r.lines,
    )
}

pub fn store(rows: &[Row<'_>]) -> Store {
    let texts: Vec<(String, String)> = rows
        .iter()
        .map(|r| (format!("items/{}.toml", r.id), text(r)))
        .chain([("vocabulary.toml".to_string(), VOCABULARY.to_string())])
        .collect();
    Store::from_texts(texts.iter().map(|(p, s)| (p.as_str(), s.as_str()))).unwrap()
}
