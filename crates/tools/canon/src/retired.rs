//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The verbs about what a tenet rules out: `forbids` and `contradicts`.
//!
//! The matching, the files read and the lines let through are `mock/lints/retired_words.rs`, which the
//! lint `a-retired-word-stays-out` reads as well. These verbs only report: a gate is a lint, and a
//! count of what contradicts the canon is a question put to the tree.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::dropped::{SHOWN_WORDS, show, still_named, word_counts};
use crate::retired_words::{EXCUSE_REACH, Hit, Retiring, split_under, unit_of};
use crate::rows::Tenet;
use crate::tree::{Tier, mentions};

/// How many characters of a hit's line a listing shows.
const SHOWN: usize = 110;

/// The tenets that still bind and rule out at least one word, as the scan reads them.
pub fn retiring(all: &[Tenet<'_>]) -> Vec<Retiring> {
    all.iter()
        .filter(|t| t.binds() && !t.rules_out.is_empty())
        .map(|t| {
            Retiring {
                id:    t.id.to_owned(),
                terms: t.rules_out.iter().map(|w| (*w).to_owned()).collect(),
            }
        })
        .collect()
}

/// `forbids <id>`: whether the tenet binds, the words it rules out, and the tenets it replaces with
/// theirs, since a replaced tenet's words are the first place a retired concept hides.
pub fn forbids_answer(repo: &Path, all: &[Tenet<'_>], id: &str) -> Result<(String, usize), String> {
    let Some(t) = all.iter().find(|t| t.id == id) else {
        return Err(format!(
            "there is no tenet `{id}`; `canon find <word>` looks for one"
        ));
    };
    let words = |t: &Tenet<'_>| {
        if t.rules_out.is_empty() {
            "nothing".to_owned()
        } else {
            t.rules_out.join(", ")
        }
    };
    let mut out = format!("{id}\n");
    let binds = if t.binds() {
        "yes".to_owned()
    } else {
        format!("no, superseded by {}", t.superseded_by.join(", "))
    };
    let _ = writeln!(out, "  binds:      {binds}");
    let _ = writeln!(out, "  rules out:  {}", words(t));
    for old in all.iter().filter(|o| o.superseded_by.contains(&id)) {
        let _ = writeln!(out, "  supersedes: {} (rules out: {})", old.id, words(old));
    }
    if t.binds() && t.rules_out.is_empty() {
        out.push_str(
            "\nit names no word it rules out, so nothing can find the designs and code it \
             falsifies: a tenet that retires or narrows a concept lists the words in `rules_out`\n",
        );
    }
    if !t.binds() && t.rules_out.is_empty() {
        let named = still_named(all, t, &word_counts(repo));
        if named.is_empty() {
            out.push_str("  dropped:    no word it used and its successors do not is named by a design, code or content line\n");
        } else {
            let shown: Vec<String> = named
                .iter()
                .take(SHOWN_WORDS)
                .map(|(w, c)| show(w, *c))
                .collect();
            let _ = writeln!(
                out,
                "  dropped:    words only it used that lines still name, as design/code/content: {}",
                shown.join(", ")
            );
        }
    }
    Ok((out, 1))
}

/// `forbids` with no tenet: the work list a supersession leaves. Every replaced tenet is a concept the
/// canon retired or narrowed, and a design or code that still describes it passes every check on ids
/// until the words are written down. This lists the replaced tenets for which neither the tenet nor
/// any tenet replacing it names a word it rules out, by subject, so a person with the canon in hand
/// can say for each whether it retires a word and put it in `rules_out`.
pub fn unretired_answer(repo: &Path, all: &[Tenet<'_>]) -> (String, usize) {
    let counts = word_counts(repo);
    let replaced: Vec<&Tenet<'_>> = all.iter().filter(|t| !t.binds()).collect();
    let named = |t: &Tenet<'_>| {
        !t.rules_out.is_empty()
            || t.superseded_by
                .iter()
                .any(|s| all.iter().any(|n| n.id == *s && !n.rules_out.is_empty()))
    };
    let mut by_subject: BTreeMap<&str, Vec<&Tenet<'_>>> = BTreeMap::new();
    for t in replaced.iter().filter(|t| !named(t)) {
        by_subject
            .entry(t.subject.unwrap_or("(no subject)"))
            .or_default()
            .push(t);
    }
    let silent: usize = by_subject.values().map(Vec::len).sum();
    let mut out = format!(
        "{} of {} replaced tenets name no word they rule out, nor does a tenet replacing them\n",
        silent,
        replaced.len()
    );
    for (subject, rows) in &by_subject {
        let _ = writeln!(out, "\n{subject} ({})", rows.len());
        for t in rows {
            let _ = writeln!(
                out,
                "  {} (replaced by {})",
                t.id,
                t.superseded_by.join(", ")
            );
            let named = still_named(all, t, &counts);
            if !named.is_empty() {
                let shown: Vec<String> = named
                    .iter()
                    .take(SHOWN_WORDS)
                    .map(|(w, c)| show(w, *c))
                    .collect();
                let _ = writeln!(
                    out,
                    "      still named, design/code/content: {}",
                    shown.join(", ")
                );
            }
        }
    }
    (out, silent)
}

/// `contradicts [path]...`: every design, code and content line naming a word a binding tenet rules out, in
/// the whole repository or in the paths named. Several paths are read in one pass, which is what a landing
/// asks of the files a branch touches.
pub fn contradicts_answer(repo: &Path, all: &[Tenet<'_>], paths: &[&str]) -> (String, usize) {
    let rules = retiring(all);
    let under: Vec<PathBuf> = paths.iter().map(|p| relative(repo, p)).collect();
    if rules.is_empty() {
        return (
            "no binding tenet rules out a word, so there is nothing to find; a tenet lists them in `rules_out`\n"
                .to_owned(),
            0,
        );
    }
    if let Some(u) = under.iter().find(|u| !repo.join(u).exists()) {
        return (
            format!("there is no path `{}` in the repository\n", u.display()),
            0,
        );
    }
    let named: Vec<&Path> = under.iter().map(PathBuf::as_path).collect();
    let (hits, excused) = split_under(repo, &rules, &named);
    let mut by_rule: BTreeMap<&str, Vec<&Hit>> = BTreeMap::new();
    for h in &hits {
        by_rule.entry(h.rule.as_str()).or_default().push(h);
    }
    let scope = match under.as_slice() {
        [] => "the repository".to_owned(),
        [one] => one.display().to_string(),
        many => format!("{} paths", many.len()),
    };
    let mut out = format!(
        "{} lines in {scope} name a word a binding tenet rules out, under {} of {} tenets that rule \
         words out\n",
        hits.len(),
        by_rule.len(),
        rules.len()
    );
    for r in &rules {
        let found = by_rule
            .get(r.id.as_str())
            .map(Vec::as_slice)
            .unwrap_or_default();
        let _ = writeln!(out, "\n{} (rules out: {})", r.id, r.terms.join(", "));
        if found.is_empty() {
            out.push_str("  nothing names them\n");
        } else if !under.is_empty() {
            listing(&mut out, found);
        } else {
            summary(&mut out, found);
        }
    }
    if !excused.is_empty() {
        let _ = writeln!(
            out,
            "\nlet through, within {EXCUSE_REACH} lines of the ruling tenet's id: {}",
            excused.len()
        );
        if !under.is_empty() {
            let shown: Vec<&Hit> = excused.iter().collect();
            listing(&mut out, &shown);
        } else {
            out.push_str("  (name a path to list them: a nearness is not a reading)\n");
        }
    }
    if let [one] = under.as_slice() {
        named_by(&mut out, repo, all, one);
    }
    (out, hits.len())
}

/// A path as the repository root sees it: relative, without a leading `./`.
fn relative(repo: &Path, given: &str) -> PathBuf {
    let p = Path::new(given.trim_start_matches("./"));
    p.strip_prefix(repo).unwrap_or(p).to_path_buf()
}

/// Every hit on its own line, as `path:line: text`.
fn listing(out: &mut String, found: &[&Hit]) {
    for h in found {
        let text: String = h.text.chars().take(SHOWN).collect();
        let _ = writeln!(out, "  {}:{}: {text}", h.path.display(), h.line);
    }
}

/// The count by tier, every design line by place, and the count by unit for the rest.
fn summary(out: &mut String, found: &[&Hit]) {
    for tier in [Tier::Design, Tier::Code, Tier::Content, Tier::Other] {
        let of: Vec<&&Hit> = found.iter().filter(|h| h.tier == tier).collect();
        if of.is_empty() {
            continue;
        }
        let files: BTreeSet<&Path> = of.iter().map(|h| h.path.as_path()).collect();
        let _ = writeln!(
            out,
            "  {:<8}{} lines in {} files",
            tier.label(),
            of.len(),
            files.len()
        );
    }
    for h in found.iter().filter(|h| h.tier == Tier::Design) {
        let _ = writeln!(out, "  design  {}:{}", h.path.display(), h.line);
    }
    let mut units: BTreeMap<String, usize> = BTreeMap::new();
    for h in found {
        *units.entry(unit_of(&h.path)).or_default() += 1;
    }
    let mut ranked: Vec<(String, usize)> = units.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let line: Vec<String> = ranked.iter().map(|(u, n)| format!("{u} {n}")).collect();
    let _ = writeln!(out, "  by unit ({}): {}", ranked.len(), line.join(", "));
}

/// The tenets the path names by id, each with what it rules out and how many lines of the path it
/// stands behind: the first thing to read in a design before building on it.
fn named_by(out: &mut String, repo: &Path, all: &[Tenet<'_>], under: &Path) {
    let known: BTreeSet<&str> = all.iter().map(|t| t.id).collect();
    let found = mentions(repo, &known);
    let _ = writeln!(out, "\ntenets {} names:", under.display());
    let mut any = false;
    for t in all {
        let Some(lines) = found.get(t.id) else { continue };
        let here = lines.iter().filter(|l| l.path.starts_with(under)).count();
        if here == 0 {
            continue;
        }
        any = true;
        let state = if t.binds() { "binds" } else { "superseded" };
        let words = if t.rules_out.is_empty() {
            "nothing".to_owned()
        } else {
            t.rules_out.join(", ")
        };
        let _ = writeln!(
            out,
            "  {} ({state}, {here} lines, rules out: {words})",
            t.id
        );
    }
    if !any {
        out.push_str("  none\n");
    }
}
