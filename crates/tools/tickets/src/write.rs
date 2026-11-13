//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Adding a ticket and moving one: the requests, the rules applied, the text written.
//!
//! A ticket is one file, `ticket/items/<id>.toml` under the registry, holding its row and its moves,
//! so a ticket's history is its file's and two writers meet only when they touch the same ticket. A
//! move is appended to that file and a ticket is created with `create_new`, so nothing here replaces
//! text it did not write.

use std::io::Write as _;
use std::path::{Path, PathBuf};

use ticket_model::model::{Fields, Move, Store, parse_rank};
use ticket_model::rules;

use crate::ids::move_id;
use crate::row;
use crate::time::is_stamp;

/// Where tickets are kept, under the registry.
pub const ITEMS: &str = "ticket/items";

/// What `ticket-add` is asked for. Lists are already split.
#[derive(Debug, Clone, Default)]
pub struct AddRequest {
    pub title:  String,
    pub id:     Option<String>,
    pub kind:   String,
    pub state:  Option<String>,
    pub holder: String,
    pub by:     String,
    pub at:     String,
    pub why:    String,
    pub fields: Fields,
}

/// A member row to append: the span the ticket is put in (none keeps the current one, empty lets it
/// leave) and whether it is core there (none keeps the current standing).
#[derive(Debug, Clone, Default)]
pub struct MemberWrite {
    pub span: Option<String>,
    pub core: Option<bool>,
}

/// What `ticket-move` is asked for.
#[derive(Debug, Clone, Default)]
pub struct MoveRequest {
    pub ticket:    String,
    pub to:        String,
    /// Links of the ticket's row to record: a field and its new values, where none clears the field.
    pub links:     Fields,
    /// A holder named, or none to carry the last one forward.
    pub holder:    Option<String>,
    /// Let go of the holder.
    pub release:   bool,
    pub by:        String,
    pub at:        String,
    pub why:       String,
    /// The words a question is answered with, for a move to a state that satisfies it.
    pub answer:    String,
    /// The options chosen, in the order chosen.
    pub picks:     Vec<String>,
    /// A member row to append, for `--span` and `--core`.
    pub member:    Option<MemberWrite>,
    /// The ticket that takes over the placeholder markers citing this one, for a move that leaves some
    /// behind. The move only checks that it is one a marker can cite; the markers are the tool's to rewrite.
    pub successor: Option<String>,
    /// Tags to add to and take off what the ticket holds, for `--add-tags` and `--remove-tags`, where `--tags`
    /// replaces the whole list; the two are refused together.
    pub tags:      TagEdit,
}

/// An edit of a ticket's tags against the ones it holds: the held ones less `remove`, then `add` not held.
#[derive(Debug, Clone, Default)]
pub struct TagEdit {
    pub add:    Vec<String>,
    pub remove: Vec<String>,
}

impl TagEdit {
    /// Whether it changes nothing.
    pub fn is_empty(&self) -> bool {
        self.add.is_empty() && self.remove.is_empty()
    }

    /// The list that results from `held`, in the order held and then added.
    pub fn applied(&self, held: &[String]) -> Vec<String> {
        let mut out: Vec<String> = held
            .iter()
            .filter(|t| !self.remove.contains(t))
            .cloned()
            .collect();
        for t in &self.add {
            if !out.contains(t) {
                out.push(t.clone());
            }
        }
        out
    }
}

/// A TOML basic string.
pub(crate) fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn line(out: &mut String, key: &str, values: &[String], integer: bool, list: bool) {
    let values: Vec<&String> = values.iter().filter(|v| !v.is_empty()).collect();
    if values.is_empty() {
        return;
    }
    let one = |v: &String| if integer { v.clone() } else { quote(v) };
    if list {
        let items: Vec<String> = values.iter().map(|v| quote(v)).collect();
        out.push_str(&format!("{key} = [{}]\n", items.join(", ")));
    } else {
        out.push_str(&format!("{key} = {}\n", one(values[0])));
    }
}

/// Fields in the order a ticket is written, with whether each is a list or an integer.
const ORDER: [(&str, bool, bool); 20] = [
    ("title", false, false),
    ("kind", false, false),
    ("what", false, false),
    ("options", false, true),
    ("choice", false, false),
    ("needs", false, true),
    ("part_of", false, false),
    ("superseded_by", false, true),
    ("related", false, true),
    ("contracts", false, true),
    ("shows", false, false),
    ("repos", false, true),
    ("branch", false, false),
    ("set", false, false),
    ("brief", false, false),
    ("session", false, false),
    ("preview", false, true),
    ("tags", false, true),
    ("rank", true, false),
    ("priority", true, false),
];

/// The names of the fields a ticket row is written with, which the schema has to declare.
#[cfg(test)]
pub(crate) fn written_fields() -> Vec<&'static str> {
    ORDER.iter().map(|(key, ..)| *key).collect()
}

fn render_ticket(id: &str, fields: &Fields) -> String {
    let mut out = format!("[[ticket]]\nid = {}\n", quote(id));
    for (key, integer, list) in ORDER {
        if let Some(v) = fields.get(key) {
            line(&mut out, key, v, integer, list);
        }
    }
    out
}

fn render_move(m: &Move) -> String {
    let mut out = format!("[[ticket_move]]\nid = {}\n", quote(&m.id));
    for (k, v) in [
        ("ticket", &m.ticket),
        ("to", &m.to),
        ("by", &m.by),
        ("at", &m.at),
        ("holder", &m.holder),
        ("why", &m.why),
        ("answer", &m.answer),
    ] {
        line(&mut out, k, std::slice::from_ref(v), false, false);
    }
    line(&mut out, "picks", &m.picks, false, true);
    out
}

fn item_path(registry: &Path, id: &str) -> PathBuf {
    registry.join(ITEMS).join(format!("{id}.toml"))
}

fn check_who_when(by: &str, at: &str) -> Result<(), String> {
    if by.is_empty() {
        return Err("name who is making the change with `--by`".to_string());
    }
    if !is_stamp(at) {
        return Err(format!(
            "`{at}` is not a time written as YYYY-MM-DDTHH:MM:SSZ"
        ));
    }
    Ok(())
}

/// The tag the console's stamp chip filters on, written for a question that is a stamp: one whose id starts
/// `stamp_`, or that a move calls a provisional stamp. Four stamps were missing from the chip on 7 November for
/// want of it (proposal A7 of the ticket sweep). `held` is what the ticket carries where `fields` has no `tags`.
fn stamp_tag(fields: &mut Fields, held: &[String], kind: &str, ticket: &str, why: &str) {
    const TAG: &str = "provisional-stamp";
    if kind != "question"
        || !(ticket.starts_with("stamp_") || why.to_lowercase().contains("provisional stamp"))
    {
        return;
    }
    let tags = fields
        .entry("tags".to_string())
        .or_insert_with(|| held.to_vec());
    if !tags.iter().any(|t| t == TAG) {
        tags.push(TAG.to_string());
    }
}

/// Writes a new ticket and its first move, and returns the ticket's id.
///
/// # Errors
///
/// When a rule refuses it, the id is taken, or the file cannot be written.
pub fn add(registry: &Path, r: &AddRequest) -> Result<String, String> {
    check_who_when(&r.by, &r.at)?;
    let store = crate::load(registry)?;
    add_with(&store, registry, r)
}

/// `add`, against a store the caller has read, for a caller that adds many tickets in one run and
/// would otherwise read the whole registry for each. The store is not updated: a caller chooses ids
/// that differ from each other and from the store's, and names no ticket it has not read.
///
/// # Errors
///
/// When a rule refuses it, the id is taken, or the file cannot be written.
pub fn add_with(store: &Store, registry: &Path, r: &AddRequest) -> Result<String, String> {
    check_who_when(&r.by, &r.at)?;
    // the checks are the ticket model crate's, so anything built on it refuses the same request
    let made = rules::new_ticket(store, &rules::NewRequest {
        title:   r.title.clone(),
        id:      r.id.clone(),
        kind:    r.kind.clone(),
        state:   r.state.clone(),
        holder:  r.holder.clone(),
        by:      r.by.clone(),
        at:      r.at.clone(),
        move_id: move_id(&r.at, &r.by, 0),
        why:     r.why.clone(),
        fields:  r.fields.clone(),
    })?;
    let (id, mut fields, first) = (made.id, made.fields, made.first);
    stamp_tag(&mut fields, &[], &r.kind, &id, &r.why);
    let path = item_path(registry, &id);
    std::fs::create_dir_all(path.parent().unwrap_or(registry)).map_err(|e| e.to_string())?;
    let text = format!("{}\n{}", render_ticket(&id, &fields), render_move(&first));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| format!("cannot create {}: {e}", path.display()))?;
    file.write_all(text.as_bytes()).map_err(|e| e.to_string())?;
    Ok(id)
}

/// One field as the lines it is written in, in the form a ticket row gives it, or nothing when it
/// holds no value.
pub(crate) fn render_field(key: &str, values: &[String]) -> String {
    let mut out = String::new();
    if let Some((_, integer, list)) = ORDER.iter().find(|(k, ..)| *k == key) {
        line(&mut out, key, values, *integer, *list);
    }
    out
}

/// Whether `successor` is a ticket the markers citing `ticket` can be handed to: one that exists, that
/// is not `ticket`, and that is not closed, since a marker citing a closed ticket is refused.
fn check_successor(store: &Store, ticket: &str, successor: &str) -> Result<(), String> {
    if successor.is_empty() {
        return Err("`--for` names a ticket, and is given none".to_string());
    }
    if successor == ticket {
        return Err(format!("`--for` names `{ticket}`, the ticket itself"));
    }
    if !store.tickets.contains_key(successor) {
        return Err(format!("`--for` names `{successor}`, which is no ticket"));
    }
    if store.state_of(successor).is_some_and(|s| s.closed) {
        return Err(format!(
            "`--for` names `{successor}`, which is closed, so a marker handed to it would be refused \
             too: name a ticket that is open"
        ));
    }
    Ok(())
}

/// The links a move records, with a tag edit turned into the `tags` list it leaves: `--tags` replaced
/// the list whatever it held, and four tags were lost that way in one evening
/// (ticket_move_tags_replaces_a_ticket_s_tags_instead_of_adding).
fn tag_links(r: &MoveRequest, held: &[String]) -> Result<Fields, String> {
    let mut links = r.links.clone();
    if r.tags.is_empty() {
        return Ok(links);
    }
    if links.contains_key("tags") {
        return Err(
            "`--tags` replaces the list and `--add-tags` and `--remove-tags` edit it: give one or the other"
                .to_string(),
        );
    }
    links.insert("tags".to_string(), r.tags.applied(held));
    Ok(links)
}

/// Whether moving `ticket` to `to` hands back work it was holding: it stands in a state that needs a holder and
/// goes to an open one that needs none. The branch it held then belongs to a stint that ended, and a landing
/// of that branch later (a set's other jobs, 8 November) must not read as this ticket's
/// (a_released_ticket_still_reads_as_landed_by_the_branch_it_no).
fn releases_work(store: &Store, ticket: &str, to: &str) -> bool {
    let held = store.state_of(ticket).is_some_and(|s| s.needs_holder);
    held && store
        .states
        .get(to)
        .is_some_and(|s| !s.needs_holder && !s.closed)
}

/// Records links on a ticket's row, appends a move, or does both in one write; returns the move's id,
/// or nothing when no move was made.
///
/// The ticket is judged as it will be: the links are applied to the rows read, the new `needs` have to
/// stand, and the state it enters, or the one it is in when none is named, has to accept it, so a job
/// is started with its branch in the same call that records the branch.
///
/// # Errors
///
/// When a rule refuses it or the file cannot be written; nothing is written then.
pub fn move_ticket(registry: &Path, r: &MoveRequest) -> Result<String, String> {
    check_who_when(&r.by, &r.at)?;
    let mut store = crate::load(registry)?;
    let held = store
        .tickets
        .get(&r.ticket)
        .ok_or_else(|| format!("`{}` is no ticket", r.ticket))?;
    let file = PathBuf::from(&held.file);
    let mut links = tag_links(r, held.many("tags"))?;
    ticket_model::contracts::check_fields(&links)?;
    stamp_tag(
        &mut links,
        held.many("tags"),
        held.one("kind"),
        &r.ticket,
        &r.why,
    );
    if r.to.is_empty() {
        if links.is_empty() && r.member.is_none() {
            return Err("name a state to move it to, or a link or a member to record".to_string());
        }
        if r.holder.is_some()
            || r.release
            || !r.why.is_empty()
            || !r.answer.is_empty()
            || !r.picks.is_empty()
            || r.successor.is_some()
        {
            return Err(
                "`--holder`, `--release`, `--why`, `--answer`, `--picks` and `--for` belong to a \
                        move: name a state"
                    .to_string(),
            );
        }
    }
    if let Some(successor) = &r.successor {
        check_successor(&store, &r.ticket, successor)?;
    }
    if !links.contains_key("branch")
        && !store.tickets[&r.ticket].one("branch").is_empty()
        && releases_work(&store, &r.ticket, &r.to)
    {
        links.insert("branch".to_string(), Vec::new());
    }
    if let Some(t) = store.tickets.get_mut(&r.ticket) {
        for (key, values) in &links {
            let values: Vec<String> = values.iter().filter(|v| !v.is_empty()).cloned().collect();
            t.fields.insert(key.clone(), values);
        }
    }
    if let Some(p) = links.get("priority").and_then(|v| v.first()) {
        parse_rank(p)
            .ok_or_else(|| format!("`priority` is a whole number, and `{p}` is not one"))?;
    }
    rules::check_links(&store, &r.ticket)?;
    rules::check_branch(&store, &r.ticket)?;
    let now = store.now_of(&r.ticket).cloned();
    let to = if r.to.is_empty() {
        match &now {
            Some(m) => m.to.clone(),
            None => {
                let kind = store.tickets.get(&r.ticket).map_or("", |t| t.one("kind"));
                store
                    .kinds
                    .get(kind)
                    .map(|k| k.initial.clone())
                    .unwrap_or_default()
            },
        }
    } else {
        r.to.clone()
    };
    let holder = if r.release {
        String::new()
    } else {
        r.holder
            .clone()
            .or_else(|| now.as_ref().map(|m| m.holder.clone()))
            .unwrap_or_default()
    };
    // The move judged: the one made, or where only links are recorded, the move the ticket stands on,
    // so a ticket closed with an answer keeps it.
    let mut attempt = 0;
    let id = loop {
        let id = move_id(&r.at, &r.by, attempt);
        if !store.moves.iter().any(|m| m.id == id) {
            break id;
        }
        attempt += 1;
    };
    let m = match (&now, r.to.is_empty()) {
        (Some(now), true) => {
            Move {
                holder,
                ..now.clone()
            }
        },
        _ => {
            Move {
                id: id.clone(),
                ticket: r.ticket.clone(),
                to,
                by: r.by.clone(),
                at: r.at.clone(),
                holder,
                why: r.why.clone(),
                answer: r.answer.clone(),
                picks: r.picks.clone(),
            }
        },
    };
    rules::check_move(&store, &m)?;
    let mut text = std::fs::read_to_string(&file)
        .map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    if !links.is_empty() {
        text = row::set_links(&text, &r.ticket, &links)?;
    }
    let mut moved = String::new();
    if !r.to.is_empty() {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&format!("\n{}", render_move(&m)));
        moved = id;
    }
    if let Some(w) = &r.member {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        let plan = crate::load_plan(registry)?;
        text.push_str(&format!("\n{}", crate::member::render(&plan, r, w)?));
    }
    text.parse::<toml::Table>()
        .map_err(|e| format!("the edited file does not parse, so nothing is written: {e}"))?;
    let part = file.with_extension("toml.part");
    std::fs::write(&part, text.as_bytes())
        .map_err(|e| format!("cannot write {}: {e}", part.display()))?;
    std::fs::rename(&part, &file).map_err(|e| format!("cannot replace {}: {e}", file.display()))?;
    Ok(moved)
}
