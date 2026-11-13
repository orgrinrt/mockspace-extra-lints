//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The items a branch is asked to answer, and the answers a report gives.

use std::collections::BTreeSet;

/// One thing to answer: a key and the line that shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub key:  String,
    pub line: String,
}

/// An open ticket as the board shows it, with the text that may name a crate.
#[derive(Debug, Clone)]
pub struct Open {
    pub id:    String,
    pub state: String,
    pub title: String,
    pub what:  String,
}

/// Whether `c` could be part of a longer crate name.
fn in_a_name(c: char) -> bool {
    c.is_alphanumeric() || c == '-' || c == '_'
}

/// Whether `text` names the crate `krate` as a whole word: the characters beside it are none of those
/// that make a longer crate name.
pub fn names_crate(text: &str, krate: &str) -> bool {
    text.match_indices(krate).any(|(at, hit)| {
        let before = text[.. at].chars().next_back();
        let after = text[at + hit.len() ..].chars().next();
        !before.is_some_and(in_a_name) && !after.is_some_and(in_a_name)
    })
}

/// The open tickets other than `own` that name a touched crate, one item each.
pub fn ticket_items(open: &[Open], touched: &BTreeSet<String>, own: &str) -> Vec<Item> {
    open.iter()
        .filter(|t| t.id != own)
        .filter_map(|t| {
            let named: Vec<&str> = touched
                .iter()
                .map(String::as_str)
                .filter(|k| names_crate(&t.title, k) || names_crate(&t.what, k))
                .collect();
            (!named.is_empty()).then(|| {
                Item {
                    key:  format!("ticket:{}", t.id),
                    line: format!("{}  {}  [{}]", t.state, t.title, named.join(", ")),
                }
            })
        })
        .collect()
}

/// The rows joined to a row of the brief that the branch does not cite and the brief does not hold, one item each.
pub fn row_items(
    brief: &[String],
    joined: &dyn Fn(&str) -> Vec<String>,
    cited: &dyn Fn(&str) -> bool,
) -> Vec<Item> {
    let mut seen: BTreeSet<String> = brief.iter().cloned().collect();
    let mut out = Vec::new();
    for from in brief {
        for row in joined(from) {
            if cited(&row) || !seen.insert(row.clone()) {
                continue;
            }
            out.push(Item {
                key:  format!("row:{row}"),
                line: format!("joined to {from}"),
            });
        }
    }
    out
}

/// The keys a report answers: a line `- <key> done <sha>`, `- <key> parted <ticket id>` or `- <key> left: <reason>`.
pub fn answered(report: &str) -> BTreeSet<String> {
    report
        .lines()
        .filter_map(|line| {
            let rest = line.trim().strip_prefix("- ")?;
            let (key, answer) = rest.split_once(' ')?;
            let known = key.starts_with("ticket:") || key.starts_with("row:");
            let given = if let Some(sha) = answer.strip_prefix("done ") {
                sha.len() >= 7 && sha.chars().all(|c| c.is_ascii_hexdigit())
            } else if let Some(id) = answer.strip_prefix("parted ") {
                !id.trim().is_empty() && !id.contains(char::is_whitespace)
            } else {
                answer
                    .strip_prefix("left:")
                    .is_some_and(|why| !why.trim().is_empty())
            };
            (known && given).then(|| key.to_owned())
        })
        .collect()
}
