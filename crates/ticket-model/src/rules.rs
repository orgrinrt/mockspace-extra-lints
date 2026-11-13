//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The rules a schema cannot say, held here and nowhere else.
//!
//! The makers refuse by them before they write, and the lint finds the rows that break them after, so
//! a hand edit or a merge is held to what the tool is. A rule is a function of the rows read; none of
//! them touches a file.

use std::collections::{BTreeMap, BTreeSet};

use mockspace_registry::is_valid_slug;

use crate::gives::gives;
use crate::model::{Fields, Kind, Move, Store, Ticket, parse_rank};

/// A row that breaks a rule, addressed by the row and said in a sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub row:     String,
    pub message: String,
}

fn open_dependents<'a>(store: &'a Store, ticket: &'a str) -> Vec<&'a Ticket> {
    store
        .dependents(ticket)
        .filter(|d| store.state_of(&d.id).is_some_and(|s| !s.closed))
        .collect()
}

/// The tickets in `needs` that do not yet meet the edge, each with the state it is in.
fn unmet<'a>(store: &'a Store, needs: &'a [String]) -> Vec<String> {
    needs
        .iter()
        .filter(|n| !store.state_of(n).is_some_and(|s| s.satisfies))
        .map(|n| {
            match store.now_of(n) {
                Some(m) => format!("`{n}` (in `{}`)", m.to),
                None => format!("`{n}` (which does not exist)"),
            }
        })
        .collect()
}

/// The fields a kind requires before a ticket is held in `to`, each filled: the first that is not,
/// said in a sentence. `entry` asks it of a ticket being added or moved and `check_store` of one
/// committed, so the refusal and the finding are the one sentence.
fn required(kind: &Kind, fields: &Fields, to: &str) -> Result<(), String> {
    for need in &kind.requires {
        // `field` is filled when it holds a value; `field:ns` when it holds a link to a row of `ns`
        // (`related:subject` wants a `subject::<slug>` among the related rows).
        let (field, ns) = need
            .split_once(':')
            .map_or((need.as_str(), None), |(f, n)| (f, Some(n)));
        let filled = fields.get(field).is_some_and(|v| {
            match ns {
                None => v.iter().any(|s| !s.is_empty()),
                Some(ns) => {
                    v.iter().any(|s| {
                        s.strip_prefix(ns)
                            .is_some_and(|r| r.starts_with("::") && r.len() > 2)
                    })
                },
            }
        });
        if !filled {
            let what = ns.map_or_else(
                || format!("`{field}`"),
                |n| format!("a `{n}` link in `{field}`"),
            );
            return Err(format!(
                "a `{}` ticket needs its {what} filled before it is held in `{to}`",
                kind.id
            ));
        }
    }
    Ok(())
}

/// What entering `to` asks of a ticket of `kind` with these `fields` and `needs` and this `holder`:
/// the same for a ticket being added and one being moved.
///
/// # Errors
///
/// A sentence naming the rule that refuses it.
pub fn entry(
    store: &Store,
    kind: &Kind,
    fields: &Fields,
    needs: &[String],
    to: &str,
    holder: &str,
) -> Result<(), String> {
    let Some(state) = store.states.get(to) else {
        return Err(format!("`{to}` is no ticket state"));
    };
    if !kind.states.iter().any(|s| s == to) {
        return Err(format!(
            "a `{}` ticket may be in {}, and `{to}` is not among them",
            kind.id,
            kind.states
                .iter()
                .map(|s| format!("`{s}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if state.needs_holder {
        if holder.is_empty() {
            return Err(format!("`{to}` needs a holder, and none is given"));
        }
        required(kind, fields, to)?;
    }
    if state.needs_met {
        let open = unmet(store, needs);
        if !open.is_empty() {
            return Err(format!(
                "`{to}` waits on {}, which has not met the need",
                open.join(", ")
            ));
        }
    }
    Ok(())
}

/// Whether the move `m` may be made: its ticket to its state, held by its holder, with the answer and
/// the picks it gives.
///
/// # Errors
///
/// A sentence naming the rule that refuses it.
pub fn check_move(store: &Store, m: &Move) -> Result<(), String> {
    let (ticket, to, holder) = (m.ticket.as_str(), m.to.as_str(), m.holder.as_str());
    let Some(t) = store.tickets.get(ticket) else {
        return Err(format!("`{ticket}` is no ticket"));
    };
    let Some(kind) = store.kinds.get(t.one("kind")) else {
        return Err(format!(
            "`{ticket}` has the kind `{}`, which is no ticket kind",
            t.one("kind")
        ));
    };
    // FIXME(kaski_tickets_the_canon_holds_no_row_on_the_order): the canon holds no row on the order of a ticket's moves; that a move earlier
    // than the newest is refused and one of the same second is not rests on the round's topic alone.
    if let Some(newest) = store.now_of(ticket)
        && m.at < newest.at
    {
        return Err(format!(
            "a move of `{ticket}` at {} is earlier than its newest, at {}",
            m.at, newest.at
        ));
    }
    entry(store, kind, &t.fields, t.many("needs"), to, holder)?;
    gives(
        t,
        kind,
        store.states.get(to).is_some_and(|s| s.satisfies),
        m,
    )?;
    crate::gives::fit_the_choice(t, m)?;
    if store
        .states
        .get(to)
        .is_some_and(|s| s.closed && !s.satisfies)
    {
        let open = open_dependents(store, ticket);
        if !open.is_empty() {
            let names: Vec<String> = open.iter().map(|d| format!("`{}`", d.id)).collect();
            return Err(format!(
                "`{ticket}` cannot go to `{to}` while {} still need it",
                names.join(", ")
            ));
        }
    }
    Ok(())
}

/// A cycle in the edges `field` names, as the ids round it, or nothing.
fn cycle(store: &Store, field: &str) -> Option<Vec<String>> {
    let edges: BTreeMap<&str, Vec<&str>> = store
        .tickets
        .values()
        .map(|t| {
            (
                t.id.as_str(),
                t.many(field).iter().map(String::as_str).collect(),
            )
        })
        .collect();
    let mut done: BTreeSet<&str> = BTreeSet::new();
    for start in edges.keys() {
        let mut path: Vec<&str> = vec![start];
        let mut next: Vec<usize> = vec![0];
        let mut on: BTreeSet<&str> = BTreeSet::from([*start]);
        while let Some(&top) = path.last() {
            let i = next.last().copied().unwrap_or(0);
            let children = edges.get(top).map_or(&[][..], Vec::as_slice);
            if let Some(&child) = children.get(i) {
                if let Some(n) = next.last_mut() {
                    *n += 1;
                }
                if on.contains(child) {
                    let at = path.iter().position(|p| *p == child).unwrap_or(0);
                    let mut ring: Vec<String> =
                        path[at ..].iter().map(|s| (*s).to_string()).collect();
                    ring.push(child.to_string());
                    return Some(ring);
                }
                if edges.contains_key(child) && !done.contains(child) {
                    path.push(child);
                    next.push(0);
                    on.insert(child);
                }
            } else {
                done.insert(top);
                on.remove(top);
                path.pop();
                next.pop();
            }
        }
    }
    None
}

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

/// The sentence for a branch another ticket holds already.
pub fn branch_taken(branch: &str, holder: &str) -> String {
    format!("`branch` names `{branch}`, which ticket `{holder}` holds already")
}

/// Whether two tickets are of one set: both hold the same non-empty `set`, the name a helper's set of
/// jobs shares on its one branch.
pub fn same_set(a: &Ticket, b: &Ticket) -> bool {
    !a.one("set").is_empty() && a.one("set") == b.one("set")
}

/// The first ticket other than `except` that holds the branch `branch` and is not of the set `set`.
/// A branch names one job, or one set of jobs, so a second ticket linked to it outside the set would be
/// the same job twice.
pub fn branch_holder<'a>(
    store: &'a Store,
    branch: &'a str,
    except: &str,
    set: &str,
) -> Option<&'a Ticket> {
    store
        .with_branch(branch)
        .find(|t| t.id != except && (set.is_empty() || t.one("set") != set))
}

/// Whether the `branch` a ticket holds in `store` may stand: no other ticket outside its set holds it. A branch
/// names one job, so a second ticket linked to it would be the same job twice. The store is the one
/// with the edit already applied.
///
/// # Errors
///
/// A sentence naming the ticket that holds the branch.
pub fn check_branch(store: &Store, ticket: &str) -> Result<(), String> {
    let Some(t) = store.tickets.get(ticket) else {
        return Err(format!("`{ticket}` is no ticket"));
    };
    match branch_holder(store, t.one("branch"), ticket, t.one("set")) {
        Some(other) => Err(branch_taken(t.one("branch"), &other.id)),
        None => Ok(()),
    }
}

/// Every rule the committed rows break, in a stable order.
pub fn check_store(store: &Store) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut say = |row: String, message: String| {
        out.push(Finding {
            row,
            message,
        })
    };
    for t in store.tickets.values() {
        let row = format!("ticket::{}", t.id);
        if !store.kinds.contains_key(t.one("kind")) {
            say(
                row.clone(),
                format!("its kind `{}` is no ticket kind", t.one("kind")),
            );
        }
        // a branch two tickets hold is said once on each ticket after the first, naming the one before it
        if let Some(before) = store
            .with_branch(t.one("branch"))
            .take_while(|o| o.id != t.id)
            .filter(|o| !same_set(o, t))
            .last()
        {
            say(row.clone(), branch_taken(t.one("branch"), &before.id));
        }
        for field in ["needs", "part_of"] {
            for to in t.many(field) {
                if !store.tickets.contains_key(to) {
                    say(
                        row.clone(),
                        format!("`{field}` names `{to}`, which is no ticket"),
                    );
                } else if *to == t.id {
                    say(row.clone(), format!("`{field}` names the ticket itself"));
                }
            }
        }
        if let Err(bad) = crate::contracts::check_fields(&t.fields) {
            say(row.clone(), bad);
        }
        if let Err(bad) = crate::options::check(t.many(crate::options::FIELD)) {
            say(row.clone(), bad);
        }
        if let Err(bad) =
            crate::choice::check(t.many(crate::choice::FIELD), t.many(crate::options::FIELD))
        {
            say(row.clone(), bad);
        }
        match store.now_of(&t.id) {
            None => {
                say(
                    row.clone(),
                    "it has no move, so it has no state".to_string(),
                )
            },
            Some(m) => {
                let holder = m.holder.as_str();
                if let (Some(kind), Some(state)) =
                    (store.kinds.get(t.one("kind")), store.states.get(&m.to))
                {
                    if !kind.states.contains(&m.to) {
                        say(
                            row.clone(),
                            format!("it is in `{}`, which its kind does not list", m.to),
                        );
                    }
                    if state.needs_holder && holder.is_empty() {
                        say(
                            row.clone(),
                            format!("it is in `{}`, which needs a holder, and has none", m.to),
                        );
                    }
                    if state.needs_holder
                        && let Err(unfilled) = required(kind, &t.fields, &m.to)
                    {
                        say(row.clone(), unfilled);
                    }
                    if state.needs_met {
                        let open = unmet(store, t.many("needs"));
                        if !open.is_empty() {
                            say(
                                row.clone(),
                                format!(
                                    "it is in `{}` while it waits on {}",
                                    m.to,
                                    open.join(", ")
                                ),
                            );
                        }
                    }
                    if state.closed && !state.satisfies {
                        for d in open_dependents(store, &t.id) {
                            say(
                                row.clone(),
                                format!("it is in `{}` while `{}` still needs it", m.to, d.id),
                            );
                        }
                    }
                }
            },
        }
    }
    for m in &store.moves {
        let row = format!("ticket_move::{}", m.id);
        if !store.tickets.contains_key(&m.ticket) {
            say(
                row.clone(),
                format!("it moves `{}`, which is no ticket", m.ticket),
            );
        }
        match store.states.get(&m.to) {
            None => {
                say(
                    row,
                    format!("it moves to `{}`, which is no ticket state", m.to),
                )
            },
            Some(state) => {
                let judged = store.tickets.get(&m.ticket).and_then(|t| {
                    store
                        .kinds
                        .get(t.one("kind"))
                        .map(|k| gives(t, k, state.satisfies, m))
                });
                if let Some(Err(e)) = judged {
                    say(row, e);
                }
            },
        }
    }
    for field in ["needs", "part_of"] {
        if let Some(ring) = cycle(store, field) {
            say(
                format!("ticket::{}", ring[0]),
                format!("`{field}` goes round: {}", ring.join(" -> ")),
            );
        }
    }
    out
}

/// Whether the `needs` and the `part_of` a ticket holds in `store` may stand: each one a ticket,
/// none the ticket itself, and no cycle among those of one field. The store is the one with the edit
/// already applied.
///
/// # Errors
///
/// A sentence naming the field and the rule that refuses it.
pub fn check_links(store: &Store, ticket: &str) -> Result<(), String> {
    let Some(t) = store.tickets.get(ticket) else {
        return Err(format!("`{ticket}` is no ticket"));
    };
    for field in ["needs", "part_of"] {
        for to in t.many(field) {
            if to == ticket {
                return Err(format!("`{field}` names the ticket itself"));
            }
            if !store.tickets.contains_key(to) {
                return Err(format!("`{field}` names `{to}`, which is no ticket"));
            }
        }
        if let Some(ring) = cycle(store, field) {
            return Err(format!("`{field}` would go round: {}", ring.join(" -> ")));
        }
    }
    crate::choice::check(t.many(crate::choice::FIELD), t.many(crate::options::FIELD))?;
    Ok(())
}
