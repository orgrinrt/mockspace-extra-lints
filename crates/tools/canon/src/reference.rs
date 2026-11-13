//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `find`, `show`, `related` and `citers` over the rows of every namespace.
//!
//! A tenet has relations of its own (`cites`, a tenet link in `related`, `superseded_by`) and is listed, related and
//! cited as it always was, so its answers read the same. Every other row is a row of the registry whose
//! fields are whatever its namespace declares, so it is read by what every row has: an id, fields, and the
//! rows that name it through a typed field, which the engine computes and the view hands over.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use mockspace::RegistryView;

use crate::rows::Tenet;
use crate::scope::Scope;
use crate::tree::mentions;
use crate::verbs::{gist, related_answer, tenet_hits};

/// The fields that say what a row is, in the order a listing prefers them.
const SAYS: [&str; 4] = ["what", "name", "title", "summary"];

/// How wide the column of ids in a listing of another namespace grows before a longer id stands out of it.
const ID_WIDTH: usize = 64;

/// Fields that hold words to be found by and not references to another row.
const NOT_EDGES: [&str; 2] = ["keywords", "answers"];

/// A row's namespace and slug.
fn parts(qualified: &str) -> (&str, &str) {
    qualified.split_once("::").unwrap_or(("", qualified))
}

/// How much a verb read, as the closing line says it, and how many rows that was.
pub fn read(reg: &RegistryView, scope: &Scope) -> (String, usize) {
    let namespaces = scope.namespaces(reg);
    let rows: usize = namespaces.iter().map(|ns| reg.rows_in(ns).len()).sum();
    if scope.is_tenets_alone() {
        return (format!("{rows} tenets"), rows);
    }
    let k = namespaces.len();
    let rows_label = if rows == 1 { "row" } else { "rows" };
    let ns_label = if k == 1 { "namespace" } else { "namespaces" };
    (format!("{rows} {rows_label} of {k} {ns_label}"), rows)
}

/// The row a term names, as `namespace::slug`.
///
/// A bare id is looked for in every namespace the scope reads, and it is an error for it to be a row of
/// more than one, since choosing between them is the person's.
///
/// # Errors
///
/// When no row in scope is named so, or the bare id is a row of several namespaces.
pub fn resolve(reg: &RegistryView, scope: &Scope, term: &str) -> Result<String, String> {
    let gone = || {
        format!(
            "there is no row `{term}` in {}; `canon find <word>` looks for one",
            scope.label()
        )
    };
    if let Some((ns, _)) = term.split_once("::") {
        return (reg.row(term).is_some() && scope.admits(ns))
            .then(|| term.to_owned())
            .ok_or_else(gone);
    }
    let held: Vec<String> = scope
        .namespaces(reg)
        .into_iter()
        .map(|ns| format!("{ns}::{term}"))
        .filter(|q| reg.row(q).is_some())
        .collect();
    match held.as_slice() {
        [] => Err(gone()),
        [one] => Ok(one.clone()),
        many => {
            Err(format!(
                "`{term}` is a row of more than one namespace: {}; name the namespace before it, or \
                 narrow with `--in`",
                many.join(", ")
            ))
        },
    }
}

/// What a row says it is, shortened: the first of the fields that say so.
fn gist_of(reg: &RegistryView, qualified: &str) -> String {
    SAYS.iter()
        .filter_map(|f| reg.field(qualified, f))
        .map(str::trim)
        .find(|v| !v.is_empty())
        .map(|v| gist(v.lines().next().unwrap_or_default()))
        .unwrap_or_default()
}

/// Whether the lowercased `needle` is in a row's id or in any of its fields.
fn row_carries(reg: &RegistryView, qualified: &str, needle: &str) -> bool {
    let (_, slug) = parts(qualified);
    slug.to_lowercase().contains(needle)
        || reg
            .row(qualified)
            .is_some_and(|f| f.values().any(|v| v.to_lowercase().contains(needle)))
}

/// `find <word>`: the tenets that carry it as they always did, then for each other namespace in scope the
/// rows that carry it anywhere in their fields, a quote and a name included.
pub fn find(reg: &RegistryView, all: &[Tenet<'_>], word: &str, scope: &Scope) -> (String, usize) {
    let needle = word.to_lowercase();
    let mut sections: Vec<String> = Vec::new();
    let mut shown = 0;
    if scope.admits("tenet") {
        let (text, n) = tenet_hits(all, word);
        sections.push(text);
        shown += n;
    }
    for ns in scope
        .namespaces(reg)
        .into_iter()
        .filter(|ns| *ns != "tenet")
    {
        let hits: Vec<&str> = reg
            .rows_in(ns)
            .iter()
            .filter(|q| row_carries(reg, q, &needle))
            .map(|q| parts(q).1)
            .collect();
        if hits.is_empty() {
            continue;
        }
        let w = hits
            .iter()
            .map(|s| s.len())
            .max()
            .unwrap_or(0)
            .min(ID_WIDTH);
        let mut text = format!("{} {ns} rows carry `{word}`\n\n", hits.len());
        for slug in &hits {
            let note = gist_of(reg, &format!("{ns}::{slug}"));
            let _ = writeln!(text, "  {slug:<w$}  {note}");
        }
        shown += hits.len();
        sections.push(text);
    }
    if sections.is_empty() {
        sections.push(format!("0 rows carry `{word}` in {}\n", scope.label()));
    }
    (sections.join("\n"), shown)
}

/// `show <row>`: every field of a row, its `doc` and its `quote` last, the quote whole.
///
/// # Errors
///
/// When the term names no row in scope or several.
pub fn show(reg: &RegistryView, term: &str, scope: &Scope) -> Result<(String, usize), String> {
    let q = resolve(reg, scope, term)?;
    let fields = reg
        .row(&q)
        .ok_or_else(|| format!("there is no row `{q}`"))?;
    let mut keys: Vec<&str> = fields
        .keys()
        .map(String::as_str)
        .filter(|k| !matches!(*k, "id" | "doc" | "quote"))
        .collect();
    keys.extend(
        ["doc", "quote"]
            .into_iter()
            .filter(|k| fields.contains_key(*k)),
    );
    let w = keys
        .iter()
        .filter(|k| !fields[**k].contains('\n') && **k != "quote")
        .map(|k| k.len() + 1)
        .max()
        .unwrap_or(0);
    let mut out = format!("{q}\n");
    for k in keys {
        let v = fields[k].as_str();
        if v.contains('\n') || k == "quote" {
            let _ = writeln!(out, "  {k}:");
            for l in v.lines() {
                if l.is_empty() {
                    out.push('\n');
                } else {
                    let _ = writeln!(out, "    {l}");
                }
            }
        } else if v.trim().is_empty() {
            let _ = writeln!(out, "  {k}:");
        } else {
            let _ = writeln!(out, "  {:<w$}  {v}", format!("{k}:"));
        }
    }
    Ok((out, 1))
}

/// The fields of `from` that name `to`, by its id or as `namespace::id`.
fn via<'r>(reg: &'r RegistryView, from: &str, to: &str) -> Vec<&'r str> {
    let slug = parts(to).1;
    reg.row(from)
        .map(|f| {
            f.iter()
                .filter(|(k, _)| k.as_str() != "id" && !NOT_EDGES.contains(&k.as_str()))
                .filter(|(_, v)| v.split(',').map(str::trim).any(|t| t == slug || t == to))
                .map(|(k, _)| k.as_str())
                .collect()
        })
        .unwrap_or_default()
}

/// A row as an edge's end: its qualified id, and the fields that make the edge in brackets.
fn end(reg: &RegistryView, holder: &str, other: &str, held: &str) -> String {
    let fields = via(reg, holder, held);
    if fields.is_empty() {
        other.to_owned()
    } else {
        format!("{other} [{}]", fields.join(", "))
    }
}

/// The rows `q` names through a typed field, and the rows that name it, each end read by the scope.
/// A tenet's relations to other tenets are `related_answer`'s and are not repeated here.
fn edges(reg: &RegistryView, q: &str, scope: &Scope) -> (Vec<String>, Vec<String>) {
    let tenet = parts(q).0 == "tenet";
    let kept = |other: &str| {
        let ns = parts(other).0;
        other != q && scope.admits(ns) && !(tenet && ns == "tenet")
    };
    let into: Vec<String> = reg
        .referrers(q)
        .iter()
        .filter(|r| kept(r))
        .map(|r| end(reg, r, r, q))
        .collect();
    let mut out: Vec<String> = Vec::new();
    for ns in reg.namespaces() {
        for row in reg.rows_in(ns) {
            if kept(row) && reg.referrers(row).iter().any(|r| r == q) {
                out.push(end(reg, q, row, row));
            }
        }
    }
    (out, into)
}

/// `related <row>`: a tenet's own block as it always was, then the rows of other namespaces it reaches and
/// is reached by; any other row, those two lists.
///
/// # Errors
///
/// When the term names no row in scope or several.
pub fn related(
    reg: &RegistryView,
    all: &[Tenet<'_>],
    term: &str,
    scope: &Scope,
) -> Result<(String, usize), String> {
    let q = resolve(reg, scope, term)?;
    let (ns, slug) = parts(&q);
    let (mut out, mut shown) =
        if ns == "tenet" { related_answer(all, slug)? } else { (format!("{q}\n"), 0) };
    let beyond = scope.namespaces(reg).iter().any(|n| *n != "tenet");
    if ns != "tenet" || beyond {
        let (to, from) = edges(reg, &q, scope);
        let list = |v: &[String]| if v.is_empty() { "nothing".to_owned() } else { v.join(", ") };
        let _ = writeln!(out, "  refers to:    {}", list(&to));
        let _ = writeln!(out, "  referred by:  {}", list(&from));
        shown += to.len() + from.len();
    }
    Ok((out, shown))
}

/// `citers <row>`: every line of the tree that names it, by tier.
///
/// # Errors
///
/// When the term names no row in scope or several.
pub fn citers(
    reg: &RegistryView,
    repo: &Path,
    term: &str,
    scope: &Scope,
) -> Result<(String, usize), String> {
    let q = resolve(reg, scope, term)?;
    let (ns, slug) = parts(&q);
    let name = if ns == "tenet" { slug } else { q.as_str() };
    let known: BTreeSet<&str> = [slug].into_iter().collect();
    let found = mentions(repo, &known);
    let lines = found.get(slug).map(Vec::as_slice).unwrap_or_default();
    let mut out = format!("{name} is named {} times\n", lines.len());
    let tiers: BTreeSet<_> = lines.iter().map(|l| l.tier).collect();
    for tier in tiers {
        let _ = writeln!(out, "\n{}", tier.label());
        for l in lines.iter().filter(|l| l.tier == tier) {
            let _ = writeln!(out, "  {}:{}", l.path.display(), l.line);
        }
    }
    let also: Vec<&str> = reg
        .namespaces()
        .filter(|n| *n != ns && reg.row(&format!("{n}::{slug}")).is_some())
        .collect();
    if !also.is_empty() {
        let _ = writeln!(
            out,
            "\nnote: `{slug}` is also a row of {}, and a line is counted by the name it carries",
            also.join(", ")
        );
    }
    Ok((out, lines.len()))
}
