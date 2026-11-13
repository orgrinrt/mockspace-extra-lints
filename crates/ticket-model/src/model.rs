//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The rows of the four ticket namespaces, read from the texts of the registry's files.
//!
//! Read from the files' texts and not from a flattened view, because a maker has to see what it wrote
//! a moment ago, the lint has to see the same rows the maker refused by, and the console has the texts
//! it fetched and no disk. One reader, so none of them can disagree about what a ticket is.

use std::collections::BTreeMap;

/// A row's fields, every value a list: a scalar is a list of one, an absent field is an empty list.
pub type Fields = BTreeMap<String, Vec<String>>;

fn first<'a>(f: &'a Fields, field: &str) -> &'a str {
    f.get(field)
        .and_then(|v| v.first())
        .map_or("", String::as_str)
}

fn every<'a>(f: &'a Fields, field: &str) -> &'a [String] {
    f.get(field).map_or(&[], Vec::as_slice)
}

/// A column of the board.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub id:           String,
    pub column:       i64,
    pub needs_holder: bool,
    pub needs_met:    bool,
    pub closed:       bool,
    pub satisfies:    bool,
    pub fields:       Fields,
}

impl State {
    /// The first value of a field of the row, or empty.
    pub fn one(&self, field: &str) -> &str {
        first(&self.fields, field)
    }
}

/// A kind of ticket and what it is held to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kind {
    pub id:           String,
    pub states:       Vec<String>,
    pub initial:      String,
    pub requires:     Vec<String>,
    /// The move fields of which a move to a state that satisfies carries at least one; none asks
    /// nothing of such a move.
    pub satisfied_by: Vec<String>,
    pub fields:       Fields,
}

impl Kind {
    /// The first value of a field of the row, or empty.
    pub fn one(&self, field: &str) -> &str {
        first(&self.fields, field)
    }
}

/// A ticket, and the file its row sits in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ticket {
    pub id:     String,
    pub fields: Fields,
    pub file:   String,
}

impl Ticket {
    /// The first value of a field, or empty.
    pub fn one(&self, field: &str) -> &str {
        first(&self.fields, field)
    }

    /// Whether the question it puts may be answered with several options: its `choice` is `many`.
    pub fn takes_many(&self) -> bool {
        self.one(crate::choice::FIELD) == crate::choice::MANY
    }

    /// Every value of a field.
    pub fn many(&self, field: &str) -> &[String] {
        every(&self.fields, field)
    }

    /// The ticket's rank as a whole number, or nothing where it holds none or holds something else.
    pub fn rank(&self) -> Option<i64> {
        parse_rank(self.one("rank"))
    }
}

/// A rank read as the whole number it is written as, the one reading the board and the makers share.
pub fn parse_rank(text: &str) -> Option<i64> {
    text.parse().ok()
}

/// One change of state and holder.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Move {
    pub id:     String,
    pub ticket: String,
    pub to:     String,
    pub by:     String,
    pub at:     String,
    /// Empty when nobody holds the ticket from this move.
    pub holder: String,
    pub why:    String,
    /// The words a question was answered with, empty where none were given.
    pub answer: String,
    /// The options chosen, in the order they were chosen.
    pub picks:  Vec<String>,
}

impl Move {
    /// Whether the move gives the move field `field`: `answer` its words, `picks` at least one.
    pub fn gives(&self, field: &str) -> bool {
        match field {
            "answer" => !self.answer.is_empty(),
            "picks" => self.picks.iter().any(|p| !p.is_empty()),
            _ => false,
        }
    }
}

/// Every ticket row the registry holds.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Store {
    pub states:  BTreeMap<String, State>,
    pub kinds:   BTreeMap<String, Kind>,
    pub tickets: BTreeMap<String, Ticket>,
    /// Oldest first, by `at`, and in the order read where two share a second.
    pub moves:   Vec<Move>,
}

/// A scalar or an array as a list of text, a float included so that a number a rule reads as a whole number
/// is refused by what it says and not read as missing; a table or a date is not a field value here.
fn texts(v: &toml::Value) -> Vec<String> {
    match v {
        toml::Value::String(s) => vec![s.clone()],
        toml::Value::Integer(i) => vec![i.to_string()],
        toml::Value::Float(x) => vec![x.to_string()],
        toml::Value::Boolean(b) => vec![b.to_string()],
        toml::Value::Array(a) => a.iter().flat_map(texts).collect(),
        _ => Vec::new(),
    }
}

fn fields_of(row: &toml::Table) -> Fields {
    row.iter()
        .filter(|(k, _)| k.as_str() != "id")
        .map(|(k, v)| (k.clone(), texts(v)))
        .collect()
}

fn flag(f: &Fields, name: &str) -> bool {
    first(f, name) == "true"
}

fn text(f: &Fields, name: &str) -> String {
    first(f, name).to_string()
}

/// The namespaces read, in the order a file's tables are taken.
const NAMESPACES: [&str; 4] = ["ticket_state", "ticket_kind", "ticket", "ticket_move"];

impl Store {
    /// Reads every ticket row of `files`, each a file's path and its text, wherever in the registry it
    /// sits. A file holding none of the four namespaces adds nothing.
    ///
    /// # Errors
    ///
    /// When a text is not TOML, a row has no id, or an id is held twice in a namespace.
    pub fn from_texts<'a>(
        files: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<Self, String> {
        let mut store = Store::default();
        for (path, src) in files {
            let doc: toml::Table = src
                .parse()
                .map_err(|e| format!("cannot parse {path}: {e}"))?;
            for ns in NAMESPACES {
                let Some(rows) = doc.get(ns).and_then(toml::Value::as_array) else {
                    continue;
                };
                for row in rows.iter().filter_map(toml::Value::as_table) {
                    let id = row
                        .get("id")
                        .and_then(toml::Value::as_str)
                        .ok_or_else(|| format!("a `{ns}` row in {path} has no id"))?
                        .to_string();
                    store.add_row(ns, id, fields_of(row), path)?;
                }
            }
        }
        // by `at` alone: the sort keeps the order of equal keys, so moves of one second stay in the order
        // they were read, which is the order they were written, and an id's nonce orders nothing
        store.moves.sort_by(|a, b| a.at.cmp(&b.at));
        Ok(store)
    }

    fn add_row(&mut self, ns: &str, id: String, f: Fields, file: &str) -> Result<(), String> {
        let taken = |held: bool| {
            if held { Err(format!("`{ns}::{id}` is declared twice")) } else { Ok(()) }
        };
        match ns {
            "ticket_state" => {
                taken(self.states.contains_key(&id))?;
                let column = match text(&f, "column").as_str() {
                    "" => {
                        return Err(format!(
                            "`ticket_state::{id}` in {file} has no column, and the board orders its states by it"
                        ));
                    },
                    read => {
                        read.parse().map_err(|_| {
                            format!(
                                "`ticket_state::{id}` in {file} has the column `{read}`, which is no whole number"
                            )
                        })?
                    },
                };
                self.states.insert(id.clone(), State {
                    id,
                    column,
                    needs_holder: flag(&f, "needs_holder"),
                    needs_met: flag(&f, "needs_met"),
                    closed: flag(&f, "closed"),
                    satisfies: flag(&f, "satisfies"),
                    fields: f,
                });
            },
            "ticket_kind" => {
                taken(self.kinds.contains_key(&id))?;
                self.kinds.insert(id.clone(), Kind {
                    id,
                    states: every(&f, "states").to_vec(),
                    initial: text(&f, "initial"),
                    requires: every(&f, "requires").to_vec(),
                    satisfied_by: every(&f, "satisfied_by").to_vec(),
                    fields: f,
                });
            },
            "ticket" => {
                taken(self.tickets.contains_key(&id))?;
                self.tickets.insert(id.clone(), Ticket {
                    id,
                    fields: f,
                    file: file.to_string(),
                });
            },
            _ => {
                taken(self.moves.iter().any(|m| m.id == id))?;
                self.moves.push(Move {
                    ticket: text(&f, "ticket"),
                    to: text(&f, "to"),
                    by: text(&f, "by"),
                    at: text(&f, "at"),
                    holder: text(&f, "holder"),
                    why: text(&f, "why"),
                    answer: text(&f, "answer"),
                    picks: every(&f, "picks").to_vec(),
                    id,
                });
            },
        }
        Ok(())
    }

    /// A ticket's moves, oldest first.
    pub fn moves_of<'a>(&'a self, ticket: &'a str) -> impl Iterator<Item = &'a Move> + 'a {
        self.moves.iter().filter(move |m| m.ticket == ticket)
    }

    /// A ticket's newest move: its state and its holder.
    pub fn now_of(&self, ticket: &str) -> Option<&Move> {
        self.moves.iter().rev().find(|m| m.ticket == ticket)
    }

    /// The state row a ticket is in now.
    pub fn state_of(&self, ticket: &str) -> Option<&State> {
        self.now_of(ticket).and_then(|m| self.states.get(&m.to))
    }

    /// The tickets whose `branch` is `branch`, in id order. An empty name is no branch, so no ticket
    /// holds it.
    pub fn with_branch<'a>(&'a self, branch: &'a str) -> impl Iterator<Item = &'a Ticket> + 'a {
        self.tickets
            .values()
            .filter(move |t| !branch.is_empty() && t.one("branch") == branch)
    }

    /// The tickets that name `ticket` in `needs`: what it blocks, asked and not written.
    pub fn dependents<'a>(&'a self, ticket: &'a str) -> impl Iterator<Item = &'a Ticket> + 'a {
        self.tickets
            .values()
            .filter(move |t| t.many("needs").iter().any(|n| n == ticket))
    }
}
