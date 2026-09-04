//! What a dependency's item puts in front of a consumer: the names in the
//! public signatures of a type, trait or alias that some crate re-exports.
//!
//! A crate that re-exports `Face` from a dependency has handed its consumers
//! every method on `Face`, and the return type of `Face::cmap` is now a name
//! the consumer has to write. That name lives in the dependency's source, so
//! finding it means finding the dependency, which is what `cargo metadata`
//! answers and nothing else does reliably: a `[patch]`, a git revision and a
//! path dependency all end at a manifest path there.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use tree_sitter::Node;

use crate::paths::{
    RESERVED_ROOTS,
    as_root,
    for_each_kind,
    generic_names,
    is_plain_pub,
    parse,
    type_names,
    under_cfg_test,
};
use crate::util::txt;

/// Where each dependency of a package keeps its source, keyed by the name a
/// path root spells it with.
pub struct Dependencies {
    pub roots: BTreeMap<String, PathBuf>,
}

/// The dependencies of the package at `crate_dir`, resolved through
/// `cargo metadata` over the workspace at `workspace_root`.
///
/// Offline first, because the gate runs after a build and everything the
/// build fetched is on disk; online only where offline refuses, which is a
/// fresh clone whose first command is a commit. What comes back on failure is
/// the tool's own stderr rather than nothing, so a lint reporting it says why.
pub fn dependencies(workspace_root: &Path, crate_name: &str) -> Result<Dependencies, String> {
    let manifest = workspace_root.join("Cargo.toml");
    let run = |offline: bool| {
        let mut cmd = Command::new("cargo");
        cmd.arg("metadata")
            .arg("--format-version")
            .arg("1")
            .arg("--manifest-path")
            .arg(&manifest);
        if offline {
            cmd.arg("--offline");
        }
        cmd.output()
    };
    let output = match run(true) {
        Ok(o) if o.status.success() => o,
        _ => run(false).map_err(|e| format!("cargo metadata could not run: {e}"))?,
    };
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed for {}: {}",
            manifest.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("cargo metadata produced something that is not json: {e}"))?;
    resolve(&json, crate_name)
}

fn resolve(json: &serde_json::Value, crate_name: &str) -> Result<Dependencies, String> {
    let packages = json["packages"]
        .as_array()
        .ok_or("cargo metadata carries no packages")?;
    let by_id: BTreeMap<&str, &serde_json::Value> = packages
        .iter()
        .filter_map(|p| p["id"].as_str().map(|id| (id, p)))
        .collect();
    // The crate is named by its directory, which is what the lint context
    // carries, and a package may call itself something else. Either matches.
    let me = packages
        .iter()
        .find(|p| {
            p["name"].as_str() == Some(crate_name)
                || p["manifest_path"]
                    .as_str()
                    .map(Path::new)
                    .and_then(|m| m.parent())
                    .and_then(|d| d.file_name())
                    .and_then(|f| f.to_str())
                    == Some(crate_name)
        })
        .ok_or_else(|| format!("no package named or housed as `{crate_name}` in the workspace"))?;
    let my_id = me["id"].as_str().unwrap_or_default();
    let nodes = json["resolve"]["nodes"]
        .as_array()
        .ok_or("cargo metadata carries no resolve graph")?;
    let node = nodes
        .iter()
        .find(|n| n["id"].as_str() == Some(my_id))
        .ok_or_else(|| format!("`{crate_name}` is not in the resolve graph"))?;
    let mut roots = BTreeMap::new();
    for dep in node["deps"].as_array().into_iter().flatten() {
        let Some(pkg) = dep["pkg"].as_str().and_then(|id| by_id.get(id)) else {
            continue;
        };
        let name = pkg["name"].as_str().unwrap_or_default();
        let Some(dir) = pkg["manifest_path"]
            .as_str()
            .map(Path::new)
            .and_then(|m| m.parent())
        else {
            continue;
        };
        roots.insert(as_root(name), dir.to_path_buf());
    }
    Ok(Dependencies {
        roots,
    })
}

/// A dependency's sources, read once.
pub struct Sources {
    pub files:    Vec<(PathBuf, String)>,
    /// Every `pub` type, trait, enum and alias the crate declares, by name.
    pub declared: BTreeSet<String>,
}

/// Every Rust file under `dir/src`, with the crate's own public declarations
/// indexed.
pub fn sources(dir: &Path) -> Sources {
    let mut files = Vec::new();
    collect(&dir.join("src"), &mut files);
    let mut declared = BTreeSet::new();
    for (_, text) in &files {
        let Some(tree) = parse(text) else { continue };
        for kind in ["struct_item", "enum_item", "trait_item", "type_item", "union_item"] {
            for_each_kind(tree.root_node(), kind, &mut |node| {
                if is_plain_pub(node, text) {
                    if let Some(name) = node.child_by_field_name("name") {
                        declared.insert(txt(name, text).to_string());
                    }
                }
            });
        }
    }
    Sources {
        files,
        declared,
    }
}

fn collect(dir: &Path, into: &mut Vec<(PathBuf, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            collect(&path, into);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            if let Ok(text) = std::fs::read_to_string(&path) {
                into.push((path, text));
            }
        }
    }
}

/// One name a re-exported item's surface makes a consumer write.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Exposure {
    /// The name, declared by the dependency itself.
    pub name:    String,
    /// Which part of the item names it: a method, a field, a bound.
    pub through: String,
}

/// The dependency's own names in the public surface of its item `item`.
///
/// A struct's surface is its public fields and the public methods of its
/// inherent impls, a trait's is its supertraits, associated types and
/// methods, an enum's is its variants' fields, and an alias's is what it
/// stands for. Names the dependency did not declare itself are left out: they
/// belong to its own dependencies, which is one crate further than this
/// reads.
pub fn exposures(sources: &Sources, item: &str) -> Vec<Exposure> {
    let mut out = BTreeSet::new();
    for (_, text) in &sources.files {
        let Some(tree) = parse(text) else { continue };
        let root = tree.root_node();
        for kind in ["struct_item", "enum_item", "trait_item", "type_item"] {
            for_each_kind(root, kind, &mut |node| {
                if node.child_by_field_name("name").map(|n| txt(n, text)) != Some(item)
                    || under_cfg_test(node, text)
                {
                    return;
                }
                let mut generics = BTreeSet::new();
                generic_names(node, text, &mut generics);
                item_surface(node, text, &generics, sources, &mut out);
            });
        }
        for_each_kind(root, "impl_item", &mut |node| {
            if node.child_by_field_name("trait").is_some() || under_cfg_test(node, text) {
                return;
            }
            let Some(ty) = node.child_by_field_name("type") else { return };
            let mut names = Vec::new();
            type_names(ty, text, &mut names);
            if names.first().map(|n| n.name.as_str()) != Some(item) {
                return;
            }
            let mut generics = BTreeSet::new();
            generic_names(node, text, &mut generics);
            if let Some(body) = node.child_by_field_name("body") {
                methods(body, text, &generics, sources, &mut out);
            }
        });
    }
    out.into_iter().collect()
}

fn item_surface(
    node: Node,
    text: &str,
    generics: &BTreeSet<String>,
    sources: &Sources,
    out: &mut BTreeSet<Exposure>,
) {
    match node.kind() {
        "struct_item" => {
            if let Some(body) = node.child_by_field_name("body") {
                fields(body, text, generics, sources, "field", out);
            }
        },
        "enum_item" => {
            if let Some(body) = node.child_by_field_name("body") {
                for_each_kind(body, "enum_variant", &mut |variant| {
                    let label = variant
                        .child_by_field_name("name")
                        .map(|n| format!("variant `{}`", txt(n, text)))
                        .unwrap_or_else(|| "a variant".to_string());
                    if let Some(vb) = variant.child_by_field_name("body") {
                        fields(vb, text, generics, sources, &label, out);
                    }
                });
            }
        },
        "trait_item" => {
            if let Some(bounds) = node.child_by_field_name("bounds") {
                take(bounds, text, generics, sources, "a supertrait", out);
            }
            if let Some(body) = node.child_by_field_name("body") {
                methods(body, text, generics, sources, out);
                for_each_kind(body, "associated_type", &mut |assoc| {
                    if let Some(bounds) = assoc.child_by_field_name("bounds") {
                        take(bounds, text, generics, sources, "an associated type", out);
                    }
                });
            }
        },
        "type_item" => {
            if let Some(ty) = node.child_by_field_name("type") {
                take(ty, text, generics, sources, "the alias", out);
            }
        },
        _ => {},
    }
}

fn fields(
    body: Node,
    text: &str,
    generics: &BTreeSet<String>,
    sources: &Sources,
    label: &str,
    out: &mut BTreeSet<Exposure>,
) {
    let mut cursor = body.walk();
    for field in body.named_children(&mut cursor) {
        let public = match field.kind() {
            "field_declaration" => is_plain_pub(field, text),
            // A tuple field is `pub` where its own modifier says so, and the
            // node has no visibility field, so the modifier is read as a
            // leading child.
            _ if body.kind() == "ordered_field_declaration_list" => {
                field.kind() == "visibility_modifier" && txt(field, text).trim() == "pub"
            },
            _ => false,
        };
        if !public {
            continue;
        }
        let ty = match field.kind() {
            "field_declaration" => field.child_by_field_name("type"),
            _ => field.next_named_sibling(),
        };
        if let Some(ty) = ty {
            take(ty, text, generics, sources, label, out);
        }
    }
}

fn methods(
    body: Node,
    text: &str,
    generics: &BTreeSet<String>,
    sources: &Sources,
    out: &mut BTreeSet<Exposure>,
) {
    let mut cursor = body.walk();
    for item in body.named_children(&mut cursor) {
        if item.kind() != "function_item" && item.kind() != "function_signature_item" {
            continue;
        }
        // A trait's methods are public with the trait; an impl's are public
        // only where they say so.
        if body.parent().map(|p| p.kind()) == Some("impl_item") && !is_plain_pub(item, text) {
            continue;
        }
        let label = item
            .child_by_field_name("name")
            .map(|n| format!("`{}`", txt(n, text)))
            .unwrap_or_else(|| "a method".to_string());
        let mut own = generics.clone();
        generic_names(item, text, &mut own);
        for field in ["parameters", "return_type"] {
            if let Some(part) = item.child_by_field_name(field) {
                take(part, text, &own, sources, &label, out);
            }
        }
        let mut c = item.walk();
        for child in item.children(&mut c) {
            if child.kind() == "where_clause" {
                take(child, text, &own, sources, &label, out);
            }
        }
    }
}

fn take(
    node: Node,
    text: &str,
    generics: &BTreeSet<String>,
    sources: &Sources,
    through: &str,
    out: &mut BTreeSet<Exposure>,
) {
    let mut names = Vec::new();
    type_names(node, text, &mut names);
    for named in names {
        if named.name == "Self" || generics.contains(&named.name) {
            continue;
        }
        // Qualified with a root that is not the crate's own is a name from one
        // crate further down, which this does not read.
        if let Some(root) = &named.root {
            if !matches!(root.as_str(), "crate" | "self" | "super") {
                continue;
            }
        }
        if !sources.declared.contains(&named.name) {
            continue;
        }
        out.insert(Exposure {
            name:    named.name,
            through: through.to_string(),
        });
    }
}

/// Whether a path root names something that is never a dependency.
pub fn is_reserved(root: &str) -> bool {
    RESERVED_ROOTS.contains(&root)
}
