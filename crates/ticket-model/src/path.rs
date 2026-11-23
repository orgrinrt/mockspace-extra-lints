//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! A ticket's id as a path, where the consumer's `ticket` namespace declares `path`.
//!
//! The consumer writes `path = ["kind"]` on its `ticket` namespace in `mockspace.toml`, and then a
//! ticket's id is `<kind>::<free names>::<leaf>`: its `kind` is read from the id and not written in
//! the row. The registry crate owns the rules of such an id (`fold_row_id`,
//! `folded_fields_written`, `row_id_names`, `id_prefix_pairs`); this module only reads the
//! declaration, hands the crate the ids it asks for, and names the ids a request makes. A namespace
//! with no `path` folds nothing, and every function here then leaves the id alone.

use std::collections::{BTreeMap, BTreeSet};

use mockspace_registry::{
    RegistryNamespace, fold_row_id, folded_fields_written, id_is_a_branch_message, id_prefix_pairs,
    is_valid_row_id, row_id_names,
};

/// The key of the ticket namespace.
pub const NAMESPACE: &str = "ticket";

/// What the consumer declares of its `ticket` namespace: whether its ids fold fields, and which.
#[derive(Debug, Clone, Default)]
pub struct Declaration {
    ns:       Option<RegistryNamespace>,
    declared: BTreeSet<String>,
}

impl PartialEq for Declaration {
    fn eq(&self, other: &Self) -> bool {
        self.path() == other.path() && self.declared == other.declared
    }
}

impl Eq for Declaration {}

impl Declaration {
    /// The declaration the text of a `mockspace.toml` makes. A text declaring no `ticket` namespace,
    /// or one with no `path`, gives the declaration that folds nothing.
    ///
    /// # Errors
    ///
    /// When the text is not TOML or a `[[registry.namespace]]` table is not a namespace.
    pub fn from_config(text: &str) -> Result<Self, String> {
        let doc: toml::Table = text
            .parse()
            .map_err(|e| format!("cannot parse mockspace.toml: {e}"))?;
        let Some(rows) = doc
            .get("registry")
            .and_then(|r| r.get("namespace"))
            .and_then(toml::Value::as_array)
        else {
            return Ok(Self::default());
        };
        let mut out = Self::default();
        for row in rows {
            let ns: RegistryNamespace = row
                .clone()
                .try_into()
                .map_err(|e| format!("a `registry.namespace` in mockspace.toml is not a namespace: {e}"))?;
            out.declared.insert(ns.key.clone());
            if ns.key == NAMESPACE && !ns.path.is_empty() {
                out.ns = Some(ns);
            }
        }
        Ok(out)
    }

    /// Whether ticket ids fold fields.
    pub fn folds(&self) -> bool {
        self.ns.is_some()
    }

    /// The folded fields, in the order they stand in the id; none where nothing folds.
    pub fn path(&self) -> &[String] {
        self.ns.as_ref().map_or(&[], |n| n.path.as_slice())
    }

    /// Whether the id folds `field`, so the id is the only place it is written.
    pub fn is_folded(&self, field: &str) -> bool {
        self.path().iter().any(|p| p == field)
    }

    /// The fields among `written` that the id folds, each a row that may not write it.
    pub fn written<'a>(&self, written: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
        match &self.ns {
            Some(ns) => folded_fields_written(ns, written),
            None => Vec::new(),
        }
    }

    /// The value each folded field takes from `id`, in `path` order, read against the ids the
    /// namespaces a folded field references hold. Nothing is returned where nothing folds.
    ///
    /// # Errors
    ///
    /// The registry crate's finding about the id, worded for `ticket::<id>`.
    pub fn fold(
        &self,
        referenced: &BTreeMap<String, BTreeSet<String>>,
        id: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let Some(ns) = &self.ns else { return Ok(Vec::new()) };
        let declared: BTreeSet<&str> = self.declared.iter().map(String::as_str).collect();
        fold_row_id(ns, &declared, referenced, id).map_err(|fault| fault.message(ns, &format!("{NAMESPACE}::{id}")))
    }
}

/// Every pair of ids where the first is a proper prefix of the second, each as the registry's
/// `id-is-a-branch` sentence says it, those naming `involving` where it is given, `where_of` naming the place each id is written.
pub fn branch_messages(ids: &[&str], involving: Option<&str>, where_of: impl Fn(&str) -> String) -> Vec<String> {
    id_prefix_pairs(ids)
        .into_iter()
        .filter(|(short, long)| involving.is_none_or(|id| *short == id || *long == id))
        .map(|(short, long)| id_is_a_branch_message(short, &where_of(short), long, &where_of(long)))
        .collect()
}

/// Whether `text` is an id of a ticket: a path of one or more plain names.
pub fn is_ticket_id(text: &str) -> bool {
    is_valid_row_id(text)
}

/// The names of an id, or the sentence saying why it is none.
///
/// # Errors
///
/// The registry crate's sentence.
pub fn names(id: &str) -> Result<Vec<&str>, String> {
    row_id_names(id)
}

/// The leaf of an id: its last name, which is the whole id where it has no path.
pub fn leaf(id: &str) -> &str {
    id.rsplit("::").next().unwrap_or(id)
}

/// The path of an id's file under the items directory, relative and without the extension: the
/// names of the id joined by `/`.
pub fn file_stem(id: &str) -> String {
    id.split("::").collect::<Vec<_>>().join("/")
}

/// Whether `value` reads as a commit sha: twelve to forty hex digits and nothing else. A sha names a
/// commit, and a rehoused history leaves none of them standing.
pub fn looks_like_sha(value: &str) -> bool {
    (12 ..= 40).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The refusal of a ticket reference that is a commit sha, or nothing.
///
/// # Errors
///
/// `<value> looks like a commit sha; a ticket is named by its id, never a sha`.
pub fn refuse_sha(value: &str) -> Result<(), String> {
    if looks_like_sha(value) {
        return Err(format!("{value} looks like a commit sha; a ticket is named by its id, never a sha"));
    }
    Ok(())
}

/// Words that carry no name of a ticket, passed over where a leaf is made from a title.
const FUNCTION_WORDS: &[&str] = &[
    "a", "an", "the", "of", "to", "in", "on", "at", "by", "for", "from", "with", "without", "and", "or", "but", "is",
    "are", "was", "be", "been", "it", "its", "this", "that", "these", "those", "as", "into", "onto", "over", "under",
    "not", "no", "so", "if", "then", "than", "do", "does", "can", "should", "will", "we",
];

/// The leaf a title makes: its first four words that are not function words, lowercase, joined by
/// underscores, and a `t_` before one that would not start with a letter. Empty where the title has
/// no word left.
pub fn leaf_of_title(title: &str) -> String {
    let words: Vec<String> = title
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .filter(|w| !FUNCTION_WORDS.contains(&w.as_str()))
        .take(4)
        .collect();
    let mut leaf = words.join("_");
    if !leaf.starts_with(|c: char| c.is_ascii_lowercase()) && !leaf.is_empty() {
        leaf.insert_str(0, "t_");
    }
    leaf
}

/// The ids among `ids` that a query over `ticket::<id>` selects: `ticket::question::engine`,
/// `ticket::**::internals::**`, `ticket::{task,question}::engine::*`. A branch stands for the tickets
/// below it.
///
/// # Errors
///
/// The registry crate's refusal of the query, worded for the person.
pub fn select<'a>(query: &str, ids: impl IntoIterator<Item = &'a str>) -> Result<Vec<&'a str>, String> {
    let ids: Vec<&str> = ids.into_iter().collect();
    let qualified: Vec<String> = ids.iter().map(|id| format!("{NAMESPACE}::{id}")).collect();
    let refs: Vec<&str> = qualified.iter().map(String::as_str).collect();
    let chosen = mockspace_registry::select_ids(query, &refs).map_err(|r| r.say(query))?;
    let prefix = format!("{NAMESPACE}::");
    Ok(ids
        .into_iter()
        .filter(|id| chosen.iter().any(|c| c.strip_prefix(&prefix) == Some(*id)))
        .collect())
}
