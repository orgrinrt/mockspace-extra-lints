//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The probe set: questions put the way a reader would type them, and the tenets that ought to come
//! back, run against `canon find` and `canon related` as they answer now.
//!
//! The expected tenets are chosen by reading the rows, not by running the tool, so a probe fails where
//! the metadata does not carry what a reader would arrive with, and passes once it does.

use canon::{Tenet, carries, joined_to};

/// How a probe asks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Via {
    /// `canon find <word>`.
    Find(String),
    /// `canon related <id>`: the tenets joined to it through `cites` or a tenet link in `related`, either direction.
    Related(String),
}

/// One question and the tenets that ought to answer it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Probe {
    /// The question in a reader's words.
    pub ask:    String,
    pub via:    Via,
    pub expect: Vec<String>,
}

/// What a probe returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub probe:    Probe,
    /// How many tenets the answer lists.
    pub returned: usize,
    pub found:    Vec<String>,
    pub missing:  Vec<String>,
    /// Expected ids that are no tenet, or a `related` id that is none: the probe is wrong.
    pub unknown:  Vec<String>,
}

/// The probes in `text`.
///
/// # Errors
///
/// When the text is not TOML, a probe lacks `ask`, asks both ways or neither, or expects nothing.
pub fn parse(text: &str) -> Result<Vec<Probe>, String> {
    let doc: toml::Table = text
        .parse()
        .map_err(|e| format!("the probe set is not TOML: {e}"))?;
    let Some(rows) = doc.get("probe").and_then(toml::Value::as_array) else {
        return Err("the probe set has no `[[probe]]` table".to_string());
    };
    let mut out = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let t = row
            .as_table()
            .ok_or_else(|| format!("probe {} is not a table", i + 1))?;
        let get = |k: &str| t.get(k).and_then(toml::Value::as_str).map(str::to_string);
        let ask = get("ask").ok_or_else(|| format!("probe {} has no `ask`", i + 1))?;
        let via = match (get("find"), get("related")) {
            (Some(w), None) => Via::Find(w),
            (None, Some(id)) => Via::Related(id),
            _ => {
                return Err(format!(
                    "probe `{ask}` asks `find` or `related`, one of them"
                ));
            },
        };
        let expect: Vec<String> = t
            .get("expect")
            .and_then(toml::Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        if expect.is_empty() {
            return Err(format!("probe `{ask}` expects nothing"));
        }
        out.push(Probe {
            ask,
            via,
            expect,
        });
    }
    Ok(out)
}

/// Every probe, put to `all`.
pub fn run(all: &[Tenet<'_>], probes: &[Probe]) -> Vec<Outcome> {
    probes.iter().map(|p| one(all, p)).collect()
}

fn one(all: &[Tenet<'_>], probe: &Probe) -> Outcome {
    let mut unknown: Vec<String> = probe
        .expect
        .iter()
        .filter(|e| !all.iter().any(|t| t.id == e.as_str()))
        .cloned()
        .collect();
    let returned: Vec<&str> = match &probe.via {
        Via::Find(word) => {
            let needle = word.to_lowercase();
            all.iter()
                .filter(|t| carries(t, &needle))
                .map(|t| t.id)
                .collect()
        },
        Via::Related(id) => {
            if !all.iter().any(|t| t.id == id.as_str()) {
                unknown.push(id.clone());
            }
            joined_to(all, id)
        },
    };
    let (found, missing) = probe
        .expect
        .iter()
        .filter(|e| !unknown.contains(e))
        .cloned()
        .partition(|e| returned.contains(&e.as_str()));
    Outcome {
        probe: probe.clone(),
        returned: returned.len(),
        found,
        missing,
        unknown,
    }
}
