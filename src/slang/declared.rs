//! The types a crate's shaders declare, in the form the Rust tools compare types by.
//!
//! A shader struct, interface or enum is the same kind of thing as a Rust struct, trait or enum for
//! the tools that look for near copies and for one name over two shapes, so it is turned into the
//! `DeclaredType` they already read, and they need no second comparison for it.

use mockspace_lint_rules::{DeclaredMember, DeclaredParts, DeclaredType};

use super::{Item, ItemKind, SlangSource};

/// The declared types of the shipped sources, one per struct, interface and enum.
pub fn declared_types(sources: &[SlangSource]) -> Vec<DeclaredType> {
    sources
        .iter()
        .filter(|s| s.shipped())
        .flat_map(|s| {
            s.file
                .items
                .iter()
                .filter_map(|item| declared(s, item))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The keyword the Rust side spells a Slang kind by.
fn keyword(kind: ItemKind) -> Option<&'static str> {
    match kind {
        ItemKind::Struct => Some("struct"),
        ItemKind::Interface => Some("trait"),
        ItemKind::Enum => Some("enum"),
        _ => None,
    }
}

fn declared(source: &SlangSource, item: &Item) -> Option<DeclaredType> {
    let kind = keyword(item.kind)?;
    let members: Vec<DeclaredMember> = item
        .members
        .iter()
        .map(|m| DeclaredMember { name: m.name.clone(), body: m.body.clone() })
        .collect();
    let list: Vec<String> = members.iter().map(DeclaredMember::whole).collect();
    let shape = format!("{kind} {}<{}> {{ {} }}", item.name, item.generics, list.join("; "));
    Some(DeclaredType {
        crate_name: source.crate_name.clone(),
        path:       Some(within_crate(&source.rel, &source.crate_name)),
        line:       item.line,
        name:       item.name.clone(),
        kind,
        vis:        if item.exported() { "pub".to_string() } else { String::new() },
        shape,
        parts:      DeclaredParts { generics: item.generics.clone(), members, frame: kind.to_string() },
    })
}

/// The path of a file from its crate's directory, or the whole path when the crate is not in it.
fn within_crate(rel: &str, crate_name: &str) -> String {
    let marker = format!("/{crate_name}/");
    rel.find(&marker).map_or_else(|| rel.to_string(), |at| rel[at + marker.len() ..].to_string())
}
