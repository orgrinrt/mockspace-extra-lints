//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Adding a ticket: the id made from its title, and the rules a new ticket is refused by before
//! anything is written.

use mockspace_registry::is_valid_slug;

use super::{branch_holder, branch_taken, entry};
use crate::model::{Fields, Move, Store, parse_rank};

/// The longest id made from a title; a longer title is cut at a word.
const MAX_ID: usize = 60;

/// A slug from words: lowercase, anything else an underscore, runs of them one, cut at a word.
pub fn slugify(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let mut out = out.trim_matches('_').to_string();
    if out.len() > MAX_ID {
        out.truncate(MAX_ID);
        if let Some(cut) = out.rfind('_') {
            out.truncate(cut);
        }
    }
    if !out.starts_with(|c: char| c.is_ascii_lowercase()) {
        out.insert_str(0, "t_");
    }
    out
}

/// What adding a ticket is asked for. The caller makes the first move's id, since the clock and the
/// nonce it is made of are not this crate's.
#[derive(Debug, Clone, Default)]
pub struct NewRequest {
    pub title:   String,
    pub id:      Option<String>,
    pub kind:    String,
    pub state:   Option<String>,
    pub holder:  String,
    pub by:      String,
    pub at:      String,
    pub move_id: String,
    pub why:     String,
    pub fields:  Fields,
}

/// A ticket a request made: its id, its fields with the title and the kind among them, and its first
/// move.
#[derive(Debug, Clone, PartialEq)]
pub struct NewTicket {
    pub id:     String,
    pub fields: Fields,
    pub first:  Move,
}

fn missing(store: &Store, ids: &[String], what: &str) -> Result<(), String> {
    match ids.iter().find(|i| !store.tickets.contains_key(*i)) {
        Some(i) => Err(format!("`{what}` names `{i}`, which is no ticket")),
        None => Ok(()),
    }
}

/// The ticket a request makes, or the sentence that refuses it: the id is a slug no ticket holds, the kind
/// exists, a rank or a priority is a whole number, `needs` and `part_of` name tickets that exist, the
/// branch is no other ticket's, and `entry` accepts the first state. Nothing is written.
///
/// # Errors
///
/// A sentence naming the rule that refuses it.
pub fn new_ticket(store: &Store, r: &NewRequest) -> Result<NewTicket, String> {
    if r.title.trim().is_empty() {
        return Err("a ticket needs a title".to_string());
    }
    let id = r.id.clone().unwrap_or_else(|| slugify(&r.title));
    if !is_valid_slug(&id) {
        return Err(format!(
            "`{id}` is not a slug: lowercase letters, digits and underscores, starting with a letter"
        ));
    }
    if store.tickets.contains_key(&id) {
        return Err(format!(
            "`{id}` is a ticket already; name another with `--id`"
        ));
    }
    let kind = store
        .kinds
        .get(&r.kind)
        .ok_or_else(|| format!("`{}` is no ticket kind", r.kind))?;
    let to = r.state.clone().unwrap_or_else(|| kind.initial.clone());
    let mut fields = r.fields.clone();
    fields.insert("title".to_string(), vec![r.title.clone()]);
    fields.insert("kind".to_string(), vec![r.kind.clone()]);
    // a rank and a priority are whole numbers, read the way the board reads them, and written plain
    for key in ["rank", "priority"] {
        if let Some(values) = fields.get(key) {
            let text = values.first().map_or("", String::as_str);
            let n = parse_rank(text)
                .ok_or_else(|| format!("`{key}` is a whole number, and `{text}` is not one"))?;
            fields.insert(key.to_string(), vec![n.to_string()]);
        }
    }
    let needs = fields.get("needs").cloned().unwrap_or_default();
    let part_of = fields.get("part_of").cloned().unwrap_or_default();
    missing(store, &needs, "needs")?;
    missing(store, &part_of, "part_of")?;
    crate::contracts::check_fields(&fields)?;
    crate::choice::check(
        fields.get(crate::choice::FIELD).map_or(&[], Vec::as_slice),
        fields.get(crate::options::FIELD).map_or(&[], Vec::as_slice),
    )?;
    let set = fields
        .get("set")
        .and_then(|s| s.first())
        .map_or("", String::as_str);
    if let Some(other) = fields
        .get("branch")
        .and_then(|b| b.first())
        .and_then(|b| branch_holder(store, b, &id, set))
    {
        return Err(branch_taken(&fields["branch"][0], &other.id));
    }
    entry(store, kind, &fields, &needs, &to, &r.holder)?;
    let first = Move {
        id: r.move_id.clone(),
        ticket: id.clone(),
        to,
        by: r.by.clone(),
        at: r.at.clone(),
        holder: r.holder.clone(),
        why: r.why.clone(),
        answer: String::new(),
        picks: Vec::new(),
    };
    Ok(NewTicket {
        id,
        fields,
        first,
    })
}
