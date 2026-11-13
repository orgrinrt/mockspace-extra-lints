//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Rewriting fields of a ticket's row inside its file, and nothing else in it.
//!
//! The file is edited as text and not re-rendered, so a comment, an order or a spelling somebody gave
//! the row survives, and the moves under it are not touched. The result is parsed again and has to say
//! what was asked before anyone writes it.

use kaski_tickets::model::Fields;

use crate::write::render_field;

/// The lines `start..end` of the `[[ticket]]` block holding `id`, header included.
fn block_of(lines: &[&str], id: &str) -> Option<(usize, usize)> {
    let mut at = 0;
    while at < lines.len() {
        if lines[at].trim() == "[[ticket]]" {
            let end = (at + 1 .. lines.len())
                .find(|i| lines[*i].trim_start().starts_with("[["))
                .unwrap_or(lines.len());
            let own = lines[at + 1 .. end].iter().any(|l| {
                let l = l.trim();
                l.strip_prefix("id")
                    .map(str::trim_start)
                    .and_then(|r| r.strip_prefix('='))
                    .is_some_and(|v| v.trim() == format!("\"{id}\""))
            });
            if own {
                return Some((at, end));
            }
            at = end;
        } else {
            at += 1;
        }
    }
    None
}

/// Where the field `key` starts and ends (exclusive) among `lines`, or none when the row lacks it.
/// A value that opens a bracket runs to the line where the brackets close, strings aside.
fn field_span(lines: &[&str], from: usize, to: usize, key: &str) -> Option<(usize, usize)> {
    for i in from .. to {
        let Some(rest) = lines[i]
            .trim_start()
            .strip_prefix(key)
            .map(str::trim_start)
            .and_then(|r| r.strip_prefix('='))
        else {
            continue;
        };
        let mut depth = 0_i32;
        let mut in_string = false;
        let mut escaped = false;
        let mut end = i + 1;
        let mut text = rest.to_string();
        loop {
            for c in text.chars() {
                if in_string {
                    match (escaped, c) {
                        (true, _) => escaped = false,
                        (false, '\\') => escaped = true,
                        (false, '"') => in_string = false,
                        _ => {},
                    }
                } else {
                    match c {
                        '"' => in_string = true,
                        '[' => depth += 1,
                        ']' => depth -= 1,
                        '#' => break,
                        _ => {},
                    }
                }
            }
            if depth <= 0 || end >= to {
                break;
            }
            text = lines[end].to_string();
            end += 1;
        }
        return Some((i, end));
    }
    None
}

/// `text` with the fields in `links` set in the row of ticket `id`: a value replaces the field's
/// lines, an empty list removes them, and a field the row lacks is added after its last line.
///
/// # Errors
///
/// When the file holds no row for `id`, or the edited text does not say what was asked.
pub fn set_links(text: &str, id: &str, links: &Fields) -> Result<String, String> {
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let (start, end) =
        block_of(&lines, id).ok_or_else(|| format!("the file holds no row for `{id}`"))?;
    let mut out: Vec<String> = lines.iter().map(|l| (*l).to_string()).collect();
    // from the bottom up, so the spans found stay where they were found
    let mut spans: Vec<(usize, usize, &str)> = Vec::new();
    let mut added = String::new();
    for (key, values) in links {
        let new = render_field(key, values);
        match field_span(&lines, start + 1, end, key) {
            Some((a, b)) => spans.push((a, b, key.as_str())),
            None => added.push_str(&new),
        }
    }
    spans.sort_by(|x, y| y.0.cmp(&x.0));
    for (a, b, key) in spans {
        out.splice(a .. b, [render_field(key, &links[key])]);
    }
    if !added.is_empty() {
        let shifted_end = out.len() - (lines.len() - end);
        let last = (start .. shifted_end)
            .rev()
            .find(|i| !out[*i].trim().is_empty())
            .unwrap_or(start);
        if !out[last].ends_with('\n') {
            out[last].push('\n');
        }
        out.insert(last + 1, added);
    }
    let result: String = out.concat();
    verify(&result, id, links)?;
    Ok(result)
}

/// Whether `text` parses and its row for `id` holds exactly `links`.
fn verify(text: &str, id: &str, links: &Fields) -> Result<(), String> {
    let doc: toml::Table = text
        .parse()
        .map_err(|e| format!("the edited row does not parse, so nothing is written: {e}"))?;
    let row = doc
        .get("ticket")
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_table)
        .find(|t| t.get("id").and_then(toml::Value::as_str) == Some(id))
        .ok_or_else(|| format!("the edited file holds no row for `{id}`"))?;
    for (key, want) in links {
        let got: Vec<String> = match row.get(key) {
            None => Vec::new(),
            Some(toml::Value::String(s)) => vec![s.clone()],
            Some(toml::Value::Array(a)) => {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            },
            Some(toml::Value::Integer(n)) => vec![n.to_string()],
            Some(_) => vec![String::from("?")],
        };
        if got != *want {
            return Err(format!(
                "the row's `{key}` would read {got:?} and not {want:?}, so nothing is written"
            ));
        }
    }
    Ok(())
}
