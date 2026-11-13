//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Finding the words a binding tenet rules out in the designs, code and content that still use them.
//!
//! This file declares no lint. It is the one owner of how a word is matched, which files are read and what
//! lets a line through, so the lint `a-retired-word-stays-out` and the verbs `forbids` and `contradicts`
//! of the `canon` tool, which includes this file by path, answer from one definition. It uses `std` and
//! `canon_tree` only, for that reason.
//!
//! A tenet lists in `rules_out` the words and names it rules out. Every check on ids passes a text that
//! cites the tenet while describing the thing it rules out, because the citation is an id and the
//! description is words; this reads the words.

use std::path::{Path, PathBuf};

use crate::canon_tree::{Tier, files, tier_of};

/// How many lines either side of a line may carry the ruling tenet's id and let the line through: a
/// sentence about the retirement says which tenet retired the word, and a design wraps its lines.
pub const EXCUSE_REACH: usize = 3;

/// What a tenet rules out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Retiring {
    /// The tenet's id.
    pub id:    String,
    /// The words it rules out, as the row writes them.
    pub terms: Vec<String>,
}

/// One line naming a word a tenet rules out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hit {
    /// The tenet that rules the word out.
    pub rule: String,
    /// The term of that tenet the line names.
    pub term: String,
    /// The file, relative to the repository root.
    pub path: PathBuf,
    /// The line, from one.
    pub line: usize,
    /// The tier the file sits in.
    pub tier: Tier,
    /// The line itself, trimmed.
    pub text: String,
}

/// The lower-case words of a text, an identifier read as the words it is made of: `WidgetVolume` is
/// `widget` and `volume`, `widget_cells` is `widget` and `cells`, `HTTPServer` is `http` and `server`, and
/// `contract` is the one word it looks like.
pub fn words(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<String> = Vec::new();
    let mut word = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            flush(&mut word, &mut out);
            continue;
        }
        let prev = i.checked_sub(1).map(|p| chars[p]);
        let next = chars.get(i + 1).copied();
        let boundary = prev.is_some_and(|p| {
            (p.is_lowercase() && c.is_uppercase())
                || (p.is_alphabetic() != c.is_alphabetic())
                || (p.is_uppercase() && c.is_uppercase() && next.is_some_and(char::is_lowercase))
        });
        if boundary {
            flush(&mut word, &mut out);
        }
        word.extend(c.to_lowercase());
    }
    flush(&mut word, &mut out);
    out
}

fn flush(word: &mut String, out: &mut Vec<String>) {
    if !word.is_empty() {
        out.push(std::mem::take(word));
    }
}

/// Whether `needle`, a term's words, stands in `hay`, a line's words, as consecutive words.
pub fn names(hay: &[String], needle: &[String]) -> bool {
    !needle.is_empty() && hay.windows(needle.len()).any(|w| w == needle)
}

/// Whether `id` stands in `text` as a whole id, bounded by anything that cannot be part of one.
fn names_id(text: &str, id: &str) -> bool {
    let part = |c: char| c.is_ascii_alphanumeric() || c == '_';
    text.match_indices(id).any(|(at, _)| {
        !text[.. at].chars().next_back().is_some_and(part)
            && !text[at + id.len() ..].chars().next().is_some_and(part)
    })
}

/// `text` with every whole occurrence of `id` blanked: a line that cites the ruling tenet is not read for
/// the words of the id it cites, which would name the retired word in the act of retiring it.
fn without_id(text: &str, id: &str) -> String {
    let part = |c: char| c.is_ascii_alphanumeric() || c == '_';
    let mut out = String::with_capacity(text.len());
    let mut from = 0;
    for (at, _) in text.match_indices(id) {
        let end = at + id.len();
        let bounded = !text[.. at].chars().next_back().is_some_and(part)
            && !text[end ..].chars().next().is_some_and(part);
        if at >= from && bounded {
            out.push_str(&text[from .. at]);
            out.push(' ');
            from = end;
        }
    }
    out.push_str(&text[from ..]);
    out
}

/// Whether a file is read for retired words: a design, code or content, and the shaders under `mock/`
/// that the tiers leave in `Other`, and the root `README.md`, which is written by hand and has no template to
/// be read through; the generated `docs/` are read through their templates. Research, rounds and the canon
/// record what was once said.
pub fn scanned(rel: &Path) -> bool {
    match tier_of(rel) {
        Tier::Design | Tier::Code | Tier::Content => true,
        Tier::Other => {
            let s = rel.to_string_lossy().replace('\\', "/");
            if s == "README.md" {
                return true;
            }
            s.starts_with("mock/") && (s.ends_with(".slang") || s.ends_with(".wgsl"))
        },
        _ => false,
    }
}

/// The unit of the repository a file belongs to, which the lint's debt list names: a crate by its
/// directory name, `lints`, `tools`, `benches` or whichever directory of `mock/` it sits in, `content`,
/// and `workspace` for what sits at the top of `mock/` or of the repository.
pub fn unit_of(rel: &Path) -> String {
    let s = rel.to_string_lossy().replace('\\', "/");
    let parts: Vec<&str> = s.split('/').collect();
    match parts.as_slice() {
        ["mock", "crates", name, _, ..] => (*name).to_owned(),
        ["mock", dir, _, ..] => (*dir).to_owned(),
        ["content", _, ..] => "content".to_owned(),
        _ => "workspace".to_owned(),
    }
}

/// The lines of one file's text that name a word a tenet rules out, as `(held, excused)`: the first the
/// lines that stand, a line being one hit for a tenet however many of its terms it names, the second the
/// lines within `EXCUSE_REACH` lines of the ruling tenet's id.
/// An excuse is a nearness and not a reading, so a verb that reports shows what it let through.
pub fn split_text(rel: &Path, body: &str, rules: &[Retiring]) -> (Vec<Hit>, Vec<Hit>) {
    let lines: Vec<&str> = body.lines().collect();
    let tokens: Vec<Vec<String>> = lines.iter().map(|l| words(l)).collect();
    let tier = tier_of(rel);
    let mut out = Vec::new();
    let mut excused = Vec::new();
    for rule in rules {
        let terms: Vec<(&String, Vec<String>)> = rule.terms.iter().map(|t| (t, words(t))).collect();
        for (n, shared) in tokens.iter().enumerate() {
            let own;
            let toks = if lines[n].contains(rule.id.as_str()) {
                own = words(&without_id(lines[n], &rule.id));
                &own
            } else {
                shared
            };
            let Some((term, _)) = terms.iter().find(|(_, w)| names(toks, w)) else {
                continue;
            };
            let from = n.saturating_sub(EXCUSE_REACH);
            let to = (n + EXCUSE_REACH + 1).min(lines.len());
            let hit = Hit {
                rule: rule.id.clone(),
                term: (*term).clone(),
                path: rel.to_path_buf(),
                line: n + 1,
                tier,
                text: lines[n].trim().to_owned(),
            };
            if lines[from .. to].iter().any(|l| names_id(l, &rule.id)) {
                excused.push(hit);
            } else {
                out.push(hit);
            }
        }
    }
    (out, excused)
}

/// Every hit in the repository, or only under any of `under` (paths relative to the repository root) where
/// any is given, in path and line order, as `(held, excused)`: the excused are the lines the ruling tenet's
/// id stands beside, which it lets through.
pub fn split_under(repo: &Path, rules: &[Retiring], under: &[&Path]) -> (Vec<Hit>, Vec<Hit>) {
    let mut out = Vec::new();
    let mut excused = Vec::new();
    for rel in files(repo) {
        if !scanned(&rel) || (!under.is_empty() && !under.iter().any(|u| rel.starts_with(u))) {
            continue;
        }
        if let Ok(body) = std::fs::read_to_string(repo.join(&rel)) {
            let (held, let_through) = split_text(&rel, &body, rules);
            out.extend(held);
            excused.extend(let_through);
        }
    }
    (out, excused)
}
