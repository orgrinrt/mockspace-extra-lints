//! What a source file imports, what a crate re-exports, and which names a
//! signature is made of.
//!
//! Shared by the re-export lint and by the surface reader that looks inside a
//! dependency, because both ask the same three questions of a parse tree and
//! answering them twice is how the two drift.

use std::collections::{BTreeMap, BTreeSet};

use tree_sitter::{Node, Parser, Tree};

use crate::util::txt;

/// The path roots that never name another crate.
pub const RESERVED_ROOTS: &[&str] = &["crate", "self", "super", "core", "std", "alloc"];

/// A crate name as a path root spells it: hyphens become underscores.
pub fn as_root(name: &str) -> String {
    name.replace('-', "_")
}

/// Parse one file, or nothing where the grammar fails to load.
pub fn parse(text: &str) -> Option<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .ok()?;
    parser.parse(text, None)
}

/// One name a `use` brings into scope: the crate it came from and what it was
/// called there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Imported {
    pub root: String,
    pub name: String,
}

/// Every leaf of a `use` tree, as `(local name, root, original name)`.
///
/// `use a::b::{C, D as E, f::*}` yields `C` from `a` as `C`, `E` from `a` as
/// `D`, and a wildcard entry whose local name and original name are both `*`
/// under the root `a`. The root is the first segment, whatever depth the path
/// has, because a crate is what a root names and the modules between are not
/// what a consumer depends on.
pub fn use_leaves(node: Node, src: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    if let Some(arg) = node.child_by_field_name("argument") {
        walk_use(arg, src, &[], &mut out);
    }
    out
}

fn walk_use(node: Node, src: &str, prefix: &[String], out: &mut Vec<(String, String, String)>) {
    match node.kind() {
        "identifier" | "crate" | "self" | "super" | "metavariable" => {
            let name = txt(node, src).to_string();
            let mut full = prefix.to_vec();
            full.push(name.clone());
            push_leaf(full, name, out);
        },
        "scoped_identifier" => {
            let mut full = prefix.to_vec();
            if let Some(path) = node.child_by_field_name("path") {
                full.extend(segments(path, src));
            }
            let name = node
                .child_by_field_name("name")
                .map(|n| txt(n, src).to_string())
                .unwrap_or_default();
            full.push(name.clone());
            push_leaf(full, name, out);
        },
        "use_as_clause" => {
            let alias = node
                .child_by_field_name("alias")
                .map(|n| txt(n, src).to_string())
                .unwrap_or_default();
            if let Some(path) = node.child_by_field_name("path") {
                let mut inner = Vec::new();
                walk_use(path, src, prefix, &mut inner);
                for (_, root, original) in inner {
                    out.push((alias.clone(), root, original));
                }
            }
        },
        "scoped_use_list" => {
            let mut full = prefix.to_vec();
            if let Some(path) = node.child_by_field_name("path") {
                full.extend(segments(path, src));
            }
            if let Some(list) = node.child_by_field_name("list") {
                walk_use(list, src, &full, out);
            }
        },
        "use_list" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk_use(child, src, prefix, out);
            }
        },
        "use_wildcard" => {
            let mut full = prefix.to_vec();
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                full.extend(segments(child, src));
            }
            full.push("*".to_string());
            push_leaf(full, "*".to_string(), out);
        },
        _ => {},
    }
}

/// A path as its segments, leftmost first.
fn segments(node: Node, src: &str) -> Vec<String> {
    match node.kind() {
        "scoped_identifier" => {
            let mut v = Vec::new();
            if let Some(path) = node.child_by_field_name("path") {
                v.extend(segments(path, src));
            }
            if let Some(name) = node.child_by_field_name("name") {
                v.push(txt(name, src).to_string());
            }
            v
        },
        _ => vec![txt(node, src).to_string()],
    }
}

fn push_leaf(full: Vec<String>, local: String, out: &mut Vec<(String, String, String)>) {
    // `use a::b::{self}` names the module `b`, which is not a type and is
    // resolved by the root and original `b`.
    let root = full.first().cloned().unwrap_or_default();
    let original = full.last().cloned().unwrap_or_default();
    let local = if local == "self" { original.clone() } else { local };
    out.push((local, root, original));
}

/// Whether an item is `pub` with no restriction, which is the only visibility
/// a consumer of the crate can reach.
pub fn is_plain_pub(node: Node, src: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            return txt(child, src).trim() == "pub";
        }
    }
    false
}

/// Whether an item sits under `#[cfg(test)]`, either on itself or on a module
/// enclosing it.
pub fn under_cfg_test(node: Node, src: &str) -> bool {
    let mut current = Some(node);
    while let Some(n) = current {
        if has_cfg_test(n, src) {
            return true;
        }
        current = n.parent();
    }
    false
}

fn has_cfg_test(node: Node, src: &str) -> bool {
    let mut sibling = node.prev_sibling();
    while let Some(s) = sibling {
        if s.kind() == "attribute_item" {
            if txt(s, src).contains("cfg(test)") {
                return true;
            }
        } else if s.kind() != "line_comment" && s.kind() != "block_comment" {
            break;
        }
        sibling = s.prev_sibling();
    }
    false
}

/// A file's imports, keyed by the local name, plus every wildcard's root.
///
/// A local name maps to every import that binds it, because one file can
/// bind the same name twice in two modules and a map keeping the last one
/// would drop the first, which is the one a signature above the second
/// resolves to. A reader asks whether any binding is foreign.
#[derive(Default, Debug)]
pub struct Imports {
    pub named:     BTreeMap<String, Vec<Imported>>,
    pub wildcards: BTreeSet<String>,
}

/// The imports of one file, at every module depth, as one map.
///
/// A `use` inside a submodule scopes to that module and this flattens it to
/// the file, keeping every binding of a name rather than the last, so the map
/// is wider than the language and never narrower: a name it resolves as
/// foreign was bound to a foreign crate somewhere in the file. A local
/// binding shadowing a foreign one in another module is the price, reported
/// rather than hidden, since a false finding is visible and a missed one is
/// not.
pub fn imports_of(root: Node, src: &str) -> Imports {
    let mut imports = Imports::default();
    for_each_kind(root, "use_declaration", &mut |node| {
        for (local, root, original) in use_leaves(node, src) {
            if local == "*" {
                imports.wildcards.insert(root);
            } else {
                imports.named.entry(local).or_default().push(Imported {
                    root,
                    name: original,
                });
            }
        }
    });
    imports
}

/// What a crate re-exports, as `(root, original name)` pairs, with `*` for a
/// wildcard.
pub fn re_exports_of(root: Node, src: &str, into: &mut BTreeSet<(String, String)>) {
    for_each_kind(root, "use_declaration", &mut |node| {
        if !is_plain_pub(node, src) {
            return;
        }
        for (_, root, original) in use_leaves(node, src) {
            into.insert((root, original));
        }
    });
}

/// Every `mod` a crate declares, which is what tells a bare path root from a
/// dependency when the manifest is not in hand.
pub fn modules_of(root: Node, src: &str, into: &mut BTreeSet<String>) {
    for_each_kind(root, "mod_item", &mut |node| {
        if let Some(name) = node.child_by_field_name("name") {
            into.insert(txt(name, src).to_string());
        }
    });
}

/// A type name as it appears in a signature: bare, or with the path it was
/// written under.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Named {
    pub name: String,
    /// The path's first segment where the name was written qualified, and
    /// nothing where it was bare.
    pub root: Option<String>,
    pub line: usize,
}

/// Every type name under `node`, qualified or not.
///
/// Walks into generic arguments, references, tuples, arrays, function types,
/// `impl` and `dyn` bounds and where clauses alike, because a name a consumer
/// has to write is a name wherever it sits.
pub fn type_names(node: Node, src: &str, out: &mut Vec<Named>) {
    match node.kind() {
        "type_identifier" => {
            out.push(Named {
                name: txt(node, src).to_string(),
                root: None,
                line: node.start_position().row + 1,
            })
        },
        "scoped_type_identifier" => {
            let root = node
                .child_by_field_name("path")
                .map(|p| segments(p, src))
                .and_then(|s| s.first().cloned());
            let name = node
                .child_by_field_name("name")
                .map(|n| txt(n, src).to_string())
                .unwrap_or_default();
            out.push(Named {
                name,
                root,
                line: node.start_position().row + 1,
            });
        },
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                type_names(child, src, out);
            }
        },
    }
}

/// The names an item's own generics declare, which are never foreign.
pub fn generic_names(node: Node, src: &str, into: &mut BTreeSet<String>) {
    let Some(params) = node.child_by_field_name("type_parameters") else {
        return;
    };
    let mut cursor = params.walk();
    for child in params.named_children(&mut cursor) {
        match child.kind() {
            "type_identifier" => {
                into.insert(txt(child, src).to_string());
            },
            "constrained_type_parameter" | "optional_type_parameter" => {
                if let Some(left) = child.child_by_field_name("left") {
                    into.insert(txt(left, src).to_string());
                }
                if let Some(name) = child.child_by_field_name("name") {
                    into.insert(txt(name, src).to_string());
                }
            },
            _ => {},
        }
    }
}

/// Visit every node of `kind` under `root`, at any depth.
pub fn for_each_kind<'a, F: FnMut(Node<'a>)>(root: Node<'a>, kind: &str, visit: &mut F) {
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        if child.kind() == kind {
            visit(child);
        }
        if child.named_child_count() > 0 {
            for_each_kind(child, kind, visit);
        }
    }
}
