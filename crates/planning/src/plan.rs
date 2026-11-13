//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The planning rows, read from the texts of the registry's files.

use std::collections::{BTreeMap, BTreeSet};

use ticket_model::Fields;

/// One planning row: its namespace, its id and its fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub ns:     String,
    pub id:     String,
    pub fields: Fields,
}

impl Row {
    /// The first value of a field, or empty.
    pub fn one(&self, field: &str) -> &str {
        self.fields
            .get(field)
            .and_then(|v| v.first())
            .map_or("", String::as_str)
    }

    /// The row's address, `namespace::id`.
    pub fn address(&self) -> String {
        format!("{}::{}", self.ns, self.id)
    }
}

/// A scalar or an array as a list of text; a table is not a field value here.
fn texts(v: &toml::Value) -> Vec<String> {
    match v {
        toml::Value::String(s) => vec![s.clone()],
        toml::Value::Integer(i) => vec![i.to_string()],
        toml::Value::Boolean(b) => vec![b.to_string()],
        toml::Value::Datetime(d) => vec![d.to_string()],
        toml::Value::Array(a) => a.iter().flat_map(texts).collect(),
        toml::Value::Float(_) | toml::Value::Table(_) => Vec::new(),
    }
}

/// Every planning row a registry holds, in the order read.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Plan {
    pub rows: Vec<Row>,
}

impl Plan {
    /// Reads every row of the namespaces `namespaces` from `files`, each a file's path and its text,
    /// wherever in the registry it sits. A file holding none of them adds nothing.
    ///
    /// # Errors
    ///
    /// When a text is not TOML, a row has no id, or an id is held twice in a namespace.
    pub fn from_texts<'a>(
        namespaces: &[&str],
        files: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<Self, String> {
        let mut plan = Plan::default();
        let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
        for (path, src) in files {
            let doc: toml::Table = src
                .parse()
                .map_err(|e| format!("cannot parse {path}: {e}"))?;
            for ns in namespaces {
                let Some(rows) = doc.get(*ns).and_then(toml::Value::as_array) else {
                    continue;
                };
                for row in rows.iter().filter_map(toml::Value::as_table) {
                    let id = row
                        .get("id")
                        .and_then(toml::Value::as_str)
                        .ok_or_else(|| format!("a `{ns}` row in {path} has no id"))?
                        .to_string();
                    if !seen.insert(((*ns).to_string(), id.clone())) {
                        return Err(format!("`{ns}::{id}` is declared twice"));
                    }
                    let fields: Fields = row
                        .iter()
                        .filter(|(k, _)| k.as_str() != "id")
                        .map(|(k, v)| (k.clone(), texts(v)))
                        .collect();
                    plan.rows.push(Row {
                        ns: (*ns).to_string(),
                        id,
                        fields,
                    });
                }
            }
        }
        Ok(plan)
    }

    /// The rows of a namespace, in the order read.
    pub fn rows_of<'a>(&'a self, ns: &'a str) -> impl Iterator<Item = &'a Row> + 'a {
        self.rows.iter().filter(move |r| r.ns == ns)
    }

    /// The row `ns::id`, where it exists.
    pub fn row(&self, ns: &str, id: &str) -> Option<&Row> {
        self.rows.iter().find(|r| r.ns == ns && r.id == id)
    }

    /// The rows of `ns` whose `subject_field` is `subject` that no other such row names in `over`:
    /// one is the current fact, more than one is a fork.
    pub fn heads<'a>(&'a self, ns: &'a str, subject_field: &str, subject: &str) -> Vec<&'a Row> {
        let chain: Vec<&Row> = self
            .rows_of(ns)
            .filter(|r| r.one(subject_field) == subject)
            .collect();
        let replaced: BTreeSet<&str> = chain
            .iter()
            .map(|r| r.one("over"))
            .filter(|o| !o.is_empty())
            .collect();
        chain
            .into_iter()
            .filter(|r| !replaced.contains(r.id.as_str()))
            .collect()
    }

    /// Where each ticket is now, by its newest member row: the span, empty where it left, and whether
    /// it is core there. Of a fork the head with the newest `at` is read, then the greatest id.
    pub fn members_now(&self) -> BTreeMap<String, (String, bool)> {
        let tickets: BTreeSet<&str> = self.rows_of("member").map(|r| r.one("ticket")).collect();
        let mut now = BTreeMap::new();
        for t in tickets {
            let mut heads = self.heads("member", "ticket", t);
            heads.sort_by(|a, b| (a.one("at"), &a.id).cmp(&(b.one("at"), &b.id)));
            if let Some(h) = heads.last() {
                now.insert(
                    t.to_string(),
                    (h.one("span").to_string(), h.one("core") != "false"),
                );
            }
        }
        now
    }

    /// Whether a `milestone_merge` row names the milestone.
    pub fn reached(&self, milestone: &str) -> bool {
        self.rows_of("milestone_merge")
            .any(|r| r.one("milestone") == milestone)
    }
}
