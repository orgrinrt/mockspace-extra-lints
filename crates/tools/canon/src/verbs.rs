//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The verbs, each a question answered as text.
//!
//! A listed tenet is an indented line starting with its id, so an answer reads in a terminal and its
//! ids can still be cut out of it.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use mockspace::RegistryView;
use registry_lints::standing::Standing;

use crate::layout::Layout;
use crate::rows::{Tenet, carries, subjects, tenets};
use crate::scope::Scope;
use crate::tree::mentions;
use crate::{VERBS, reference};

/// How much of a tenet's `what` a listing shows.
const GIST: usize = 96;

/// The answer to `verb` about `term`, with how many rows it examined. `more` is what follows the term, which
/// only `contradicts` reads, as further paths.
///
/// # Errors
///
/// When the verb is not one, wants a term it was not given, or names a subject or tenet that is not one.
pub fn answer(
    reg: &RegistryView,
    repo: &Path,
    layout: &Layout,
    verb: &str,
    term: Option<&str>,
    more: &[&str],
    scope: &Scope,
) -> Result<(String, usize), String> {
    // `show` and `citers` ask for one row and need no tenet list
    let all = if matches!(verb, "show" | "citers") { Vec::new() } else { tenets(reg) };
    let mut examined = all.len();
    let want = |what: &str| term.ok_or_else(|| format!("`canon {verb}` wants {what}"));
    let (mut text, shown) = match verb {
        "subjects" => subjects_answer(reg, &all),
        "subject" => subject_answer(reg, &all, want("a subject")?)?,
        "show" => reference::show(reg, want("a row")?, scope)?,
        "find" => reference::find(reg, &all, want("a word")?, scope),
        "related" => reference::related(reg, &all, want("a row")?, scope)?,
        "citers" => reference::citers(reg, repo, layout, want("a row")?, scope)?,
        "uncited" => uncited_answer(repo, layout, &all),
        "forbids" => {
            match term {
                Some(id) => crate::retired::forbids_answer(repo, layout, &all, id)?,
                None => crate::retired::unretired_answer(repo, layout, &all),
            }
        },
        "contradicts" => {
            let paths: Vec<&str> = term.into_iter().chain(more.iter().copied()).collect();
            crate::retired::contradicts_answer(repo, layout, &all, &paths)
        },
        other => {
            return Err(format!(
                "there is no verb `{other}`; the verbs are {}",
                VERBS.join(", ")
            ));
        },
    };
    let mut read = format!("{} tenets", all.len());
    if matches!(verb, "show" | "find" | "related" | "citers") {
        let (label, rows) = reference::read(reg, scope);
        read = label;
        examined = rows;
        if verb == "citers" {
            read.push_str(" and the tree");
        }
    } else if matches!(verb, "subjects" | "subject") {
        let n = subjects(reg).len();
        examined += n;
        let _ = write!(read, " and {n} subjects");
    } else if matches!(verb, "uncited" | "forbids" | "contradicts") {
        read.push_str(" and the tree");
    }
    let _ = writeln!(text, "\nread {read}, shown {shown}");
    Ok((text, examined))
}

/// The opening of a tenet's `what`, cut at a word.
pub(crate) fn gist(what: &str) -> String {
    if what.chars().count() <= GIST {
        return what.to_string();
    }
    let cut: String = what.chars().take(GIST).collect();
    let at = cut.rfind(' ').unwrap_or(cut.len());
    format!("{}...", &cut[.. at])
}

/// One tenet as a listing line, marked with its standing where the maintainer has not ratified it, with its stamp
/// where it carries one and with what replaced it where it is superseded.
pub(crate) fn line(out: &mut String, t: &Tenet<'_>, width: usize) {
    let mut note = gist(t.what);
    if t.stamped() {
        note = format!("({}, stamped) {note}", t.standing.word());
    } else if t.standing != Standing::Ratified {
        note = format!("({}) {note}", t.standing.word());
    }
    if t.replaced() {
        note = format!("(superseded by {}) {note}", t.superseded_by.join(", "));
    }
    let _ = writeln!(out, "  {:<width$}  {note}", t.id);
}

/// The width the ids of a listing are padded to.
pub(crate) fn width<'a>(ts: impl Iterator<Item = &'a Tenet<'a>>) -> usize {
    ts.map(|t| t.id.len()).max().unwrap_or(0)
}

fn subjects_answer(reg: &RegistryView, all: &[Tenet<'_>]) -> (String, usize) {
    let subs = subjects(reg);
    let binding = all.iter().filter(|t| t.binds()).count();
    let ratified = all
        .iter()
        .filter(|t| t.standing == Standing::Ratified)
        .count();
    let mut out = format!(
        "{} subjects, {} tenets, {binding} binding, {ratified} ratified\n\n",
        subs.len(),
        all.len()
    );
    let w = subs.iter().map(|(s, _)| s.len()).max().unwrap_or(0);
    for (s, what) in &subs {
        let held: Vec<&Tenet<'_>> = all.iter().filter(|t| t.subject == Some(*s)).collect();
        let binds = held.iter().filter(|t| t.binds()).count();
        let _ = writeln!(
            out,
            "  {s:<w$}  {:>3} tenets, {binds:>3} binding  {what}",
            held.len()
        );
    }
    let homeless: Vec<&str> = all
        .iter()
        .filter(|t| t.subject.is_none_or(|s| !subs.iter().any(|(k, _)| *k == s)))
        .map(|t| t.id)
        .collect();
    if !homeless.is_empty() {
        let _ = writeln!(out, "\nin no subject that exists: {}", homeless.join(", "));
    }
    (out, subs.len())
}

fn subject_answer(
    reg: &RegistryView,
    all: &[Tenet<'_>],
    slug: &str,
) -> Result<(String, usize), String> {
    let subs = subjects(reg);
    let Some((_, what)) = subs.iter().find(|(s, _)| *s == slug) else {
        let names: Vec<&str> = subs.iter().map(|(s, _)| *s).collect();
        return Err(format!(
            "there is no subject `{slug}`; the subjects are {}",
            names.join(", ")
        ));
    };
    let held: Vec<&Tenet<'_>> = all.iter().filter(|t| t.subject == Some(slug)).collect();
    let mut out = format!("{slug}: {what} ({} tenets)\n\n", held.len());
    let w = width(held.iter().copied());
    let shown = held.len();
    for t in held {
        line(&mut out, t, w);
    }
    Ok((out, shown))
}

pub(crate) fn tenet_hits(all: &[Tenet<'_>], word: &str) -> (String, usize) {
    let needle = word.to_lowercase();
    let hits: Vec<&Tenet<'_>> = all.iter().filter(|t| carries(t, &needle)).collect();
    let mut out = format!("{} tenets carry `{word}`\n\n", hits.len());
    let w = width(hits.iter().copied());
    let shown = hits.len();
    for t in hits {
        line(&mut out, t, w);
    }
    (out, shown)
}

fn find_tenet<'a, 'b>(all: &'b [Tenet<'a>], id: &str) -> Result<&'b Tenet<'a>, String> {
    all.iter()
        .find(|t| t.id == id)
        .ok_or_else(|| format!("there is no tenet `{id}`; `canon find <word>` looks for one"))
}

fn ids_or_nothing(ids: &[&str]) -> String {
    if ids.is_empty() { "nothing".to_string() } else { ids.join(", ") }
}

pub(crate) fn related_answer(all: &[Tenet<'_>], id: &str) -> Result<(String, usize), String> {
    let t = find_tenet(all, id)?;
    let cited_by: Vec<&str> = all
        .iter()
        .filter(|o| o.cites.contains(&id))
        .map(|o| o.id)
        .collect();
    let related_by: Vec<&str> = all
        .iter()
        .filter(|o| o.links.contains(&id))
        .map(|o| o.id)
        .collect();
    let supersedes: Vec<&str> = all
        .iter()
        .filter(|o| o.superseded_by.contains(&id))
        .map(|o| o.id)
        .collect();
    let mut out = format!("{id}\n");
    let _ = writeln!(out, "  standing:      {}", t.standing.word());
    let _ = writeln!(out, "  subject:       {}", t.subject.unwrap_or("none"));
    let _ = writeln!(out, "  cites:         {}", ids_or_nothing(&t.cites));
    let _ = writeln!(out, "  cited by:      {}", ids_or_nothing(&cited_by));
    let _ = writeln!(out, "  links to:      {}", ids_or_nothing(&t.links));
    let _ = writeln!(out, "  related by:    {}", ids_or_nothing(&related_by));
    let _ = writeln!(out, "  supersedes:    {}", ids_or_nothing(&supersedes));
    let _ = writeln!(out, "  superseded by: {}", ids_or_nothing(&t.superseded_by));
    let shown: BTreeSet<&str> =
        [&t.cites, &cited_by, &t.links, &related_by, &supersedes, &t.superseded_by]
            .into_iter()
            .flatten()
            .copied()
            .collect();
    Ok((out, shown.len()))
}

fn uncited_answer(repo: &Path, layout: &Layout, all: &[Tenet<'_>]) -> (String, usize) {
    let known: BTreeSet<&str> = all.iter().map(|t| t.id).collect();
    let found = mentions(repo, layout, &known);
    let lonely: Vec<&Tenet<'_>> = all
        .iter()
        .filter(|t| t.binds() && !found.contains_key(t.id))
        .collect();
    let mut out = format!(
        "{} tenets still binding that nothing names\n\n",
        lonely.len()
    );
    let w = width(lonely.iter().copied());
    let shown = lonely.len();
    for t in lonely {
        line(&mut out, t, w);
    }
    (out, shown)
}

/// The tenets `ids` name that still bind and the maintainer has not ratified, one line each naming its
/// standing. Not a finding: a tenet that matters now is provisional until the maintainer goes through
/// it, so a check that blocked on one would pass nothing.
pub fn unratified(all: &[Tenet<'_>], ids: &[&str]) -> Vec<String> {
    ids.iter()
        .filter_map(|raw| {
            let id = raw.strip_prefix("tenet::").unwrap_or(raw);
            all.iter()
                .find(|t| t.id == id && t.binds() && t.standing != Standing::Ratified)
                .map(|t| format!("`{id}` is {}, not ratified", t.standing.word()))
        })
        .collect()
}

/// What is wrong with the tenets `ids` name, one line each: an id that is no tenet, an id whose tenet
/// something supersedes, and an id whose tenet is stamped or demoted and binds nothing. A `tenet::` prefix is the
/// namespace and is read off, as a citation spells it. Empty when every id names a tenet that still
/// binds, provisional or ratified.
pub fn check(all: &[Tenet<'_>], ids: &[&str]) -> Vec<String> {
    ids.iter()
        .filter_map(|raw| {
            let id = raw.strip_prefix("tenet::").unwrap_or(raw);
            match all.iter().find(|t| t.id == id) {
                None => {
                    Some(format!(
                        "`{id}` is no tenet; `canon find <word>` looks for one"
                    ))
                },
                Some(t) if t.replaced() => {
                    Some(format!(
                        "`{id}` is superseded by {}",
                        t.superseded_by.join(", ")
                    ))
                },
                Some(t) if t.stamped() => {
                    let by = if t.stamped_by.is_empty() { "nobody named" } else { t.stamped_by };
                    Some(format!(
                        "`{id}` is {}, stamped by {by}: {}",
                        t.standing.word(),
                        t.unbound()
                    ))
                },
                Some(t) if !t.binds() => {
                    Some(format!("`{id}` is {}: {}", t.standing.word(), t.unbound()))
                },
                Some(_) => None,
            }
        })
        .collect()
}
