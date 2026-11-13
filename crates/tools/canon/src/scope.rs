//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Which namespaces of the registry a verb reads.
//!
//! By default all of them: a verb that reads only the tenets cannot tell somebody that a reference
//! namespace holds what they are looking for. `--in` narrows, to namespaces by name, and to the ones
//! that are canon by the word `canon`, which is the namespaces whose rows sit in the files
//! `canon_paths` in `mockspace.toml` names. That declaration is the only answer to what is canon:
//! a registry holds rows nobody has called canon, so no `canon_paths` means no canon is declared.

use std::collections::BTreeSet;
use std::path::Path;

use mockspace::{RegistryView, glob_match};

use crate::canon_tree::files;

/// The word in `--in` that stands for the namespaces `canon_paths` reaches.
pub const CANON: &str = "canon";

/// The flag that narrows a verb.
pub const FLAG: &str = "--in";

/// The namespaces a verb reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Every namespace of the registry.
    All,
    /// Only these.
    Only(BTreeSet<String>),
}

impl Scope {
    /// The scope `--in` asks for, from its value, or every namespace where there is none.
    ///
    /// # Errors
    ///
    /// When the value names no namespace, names one the registry does not hold, or says `canon` in a
    /// repository that declares no `canon_paths`.
    pub fn parse(spec: Option<&str>, reg: &RegistryView, repo: &Path) -> Result<Self, String> {
        let Some(spec) = spec else {
            return Ok(Self::All);
        };
        let words: Vec<&str> = spec
            .split(',')
            .map(str::trim)
            .filter(|w| !w.is_empty())
            .collect();
        if words.is_empty() {
            return Err(format!(
                "`{FLAG}` wants the namespaces to read, comma separated, or `{CANON}`"
            ));
        }
        let held: BTreeSet<&str> = reg.namespaces().collect();
        let mut chosen = BTreeSet::new();
        for word in words {
            if word == CANON {
                chosen.extend(canon_namespaces(repo)?);
            } else if held.contains(word) {
                chosen.insert(word.to_owned());
            } else {
                let names: Vec<&str> = held.iter().copied().collect();
                return Err(format!(
                    "there is no namespace `{word}`; the namespaces are {}, and `{CANON}` for the ones \
                     `canon_paths` names",
                    names.join(", ")
                ));
            }
        }
        Ok(Self::Only(chosen))
    }

    /// Whether a row of `namespace` is read.
    pub fn admits(&self, namespace: &str) -> bool {
        match self {
            Self::All => true,
            Self::Only(chosen) => chosen.contains(namespace),
        }
    }

    /// The namespaces of `reg` that are read, in name order.
    pub fn namespaces<'r>(&self, reg: &'r RegistryView) -> Vec<&'r str> {
        reg.namespaces().filter(|ns| self.admits(ns)).collect()
    }

    /// Where the scope reaches, as an error names it.
    pub fn label(&self) -> String {
        match self {
            Self::All => "any namespace".to_owned(),
            Self::Only(chosen) => chosen.iter().cloned().collect::<Vec<_>>().join(", "),
        }
    }

    /// Whether the scope is the tenets alone.
    pub fn is_tenets_alone(&self) -> bool {
        matches!(self, Self::Only(chosen) if chosen.len() == 1 && chosen.contains("tenet"))
    }
}

/// The namespaces whose rows sit in the files `canon_paths` names.
///
/// # Errors
///
/// When `mockspace.toml` is not there, does not parse, or declares no `canon_paths`.
pub fn canon_namespaces(repo: &Path) -> Result<BTreeSet<String>, String> {
    let none = || {
        format!(
            "this repository declares no `canon_paths` in mockspace.toml, so no namespace is canon; \
             name the namespaces to read after `{FLAG}` instead"
        )
    };
    let config = std::fs::read_to_string(repo.join("mockspace.toml")).map_err(|_| none())?;
    let config: toml::Table = config
        .parse()
        .map_err(|e| format!("mockspace.toml does not parse: {e}"))?;
    let globs: Vec<&str> = config
        .get("canon_paths")
        .and_then(toml::Value::as_array)
        .map(|a| a.iter().filter_map(toml::Value::as_str).collect())
        .unwrap_or_default();
    if globs.is_empty() {
        return Err(none());
    }
    let mut out = BTreeSet::new();
    let reading = mockspace::reading::Reading::new(repo);
    let view = reading.view();
    for rel in files(&view, repo) {
        let path = rel.to_string_lossy().replace('\\', "/");
        if !path.ends_with(".toml") || !globs.iter().any(|g| glob_match(g, &path)) {
            continue;
        }
        let Some(body) = view.text(repo.join(&rel)) else {
            continue;
        };
        let rows: toml::Table = body
            .parse()
            .map_err(|e| format!("{path} does not parse: {e}"))?;
        out.extend(
            rows.into_iter()
                .filter(|(_, v)| {
                    v.as_array()
                        .is_some_and(|a| a.iter().any(toml::Value::is_table))
                })
                .map(|(k, _)| k),
        );
    }
    Ok(out)
}

/// The verbs `--in` narrows.
pub const SCOPED: [&str; 4] = ["show", "find", "related", "citers"];

/// The same, as an error lists them.
pub const SCOPED_NAMES: &str = "find, related, citers and show";

/// The value of `--in` among a verb's arguments, as `--in <value>` or `--in=<value>`.
///
/// # Errors
///
/// When `--in` is the last word and has no value.
pub fn flag_value<'a>(args: &[&'a str]) -> Result<Option<&'a str>, String> {
    let mut words = args.iter();
    while let Some(word) = words.next() {
        if *word == FLAG {
            return words.next().copied().map(Some).ok_or_else(|| {
                format!("`{FLAG}` wants the namespaces to read, comma separated, or `{CANON}`")
            });
        }
        if let Some(value) = word.strip_prefix("--in=") {
            return Ok(Some(value));
        }
    }
    Ok(None)
}
