//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `show` and `citers` over the rows of every namespace.
//!
//! Every row is a row of the registry whose fields are whatever its namespace declares, so it is read by
//! what every row has: an id and fields. Finding rows by word or by relation is `mock search`.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

use mockspace::RegistryView;

use crate::layout::Layout;
use crate::scope::Scope;
use crate::tree::mentions;

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
            "there is no row `{term}` in {}; `search <word> --rows` looks for one",
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

/// `citers <row>`: every line of the tree that names it, by tier.
///
/// # Errors
///
/// When the term names no row in scope or several.
pub fn citers(
    reg: &RegistryView,
    repo: &Path,
    layout: &Layout,
    term: &str,
    scope: &Scope,
) -> Result<(String, usize), String> {
    let q = resolve(reg, scope, term)?;
    let (ns, slug) = parts(&q);
    let name = if ns == "tenet" { slug } else { q.as_str() };
    let known: BTreeSet<&str> = [slug].into_iter().collect();
    let found = mentions(repo, layout, &known);
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
