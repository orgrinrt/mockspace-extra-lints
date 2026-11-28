//! The signatures a crate writes only because a trait of another crate fixed them.
//!
//! A crate implementing a trait it does not define copies that trait's method
//! signatures and associated constant types; it cannot change them. A parser's
//! `Visitor` returning `core::result::Result<(), Self::Error>`, or a store
//! contract taking `&'t str`, is judged where the trait is declared, in the
//! crate that owns it. The bare-type lints read the signature of a method in an
//! `impl <Trait> for <Type>` block as exempt when the crate does not define
//! `<Trait>`, and everything else as before.
//!
//! What is exempt, and only that:
//!
//! - a method's signature, from the start of the item (its `fn` keyword, or the
//!   `const`, `unsafe`, `async` or `extern` in front of it) to its opening `{`,
//!   or to its `;` where it has no body, across lines;
//! - an associated constant's declaration, from `const` to its `=`, or to its
//!   `;` where it has no value.
//!
//! A method body, the value of a constant, an associated type, a field, an
//! inherent impl and an impl of a trait the crate defines are all still read.
//!
//! The rule for a trait the crate defines: the last segment of the impl's trait
//! path, its generic arguments dropped, is the name of a `trait` item anywhere in
//! the crate's parsed sources, at any depth. `impl a::b::Store<'t> for X` asks
//! after `Store`. What it misjudges:
//!
//! - A foreign trait sharing its name with one the crate defines is read as the
//!   crate's own, so its signatures stay checked. This errs toward reporting.
//! - A trait the crate defines only through a macro expansion has no `trait`
//!   item in the parse, so an impl of it is read as foreign and its signatures
//!   go unchecked. This errs toward silence.
//! - A trait the crate re-exports from another crate is foreign, which is
//!   what it is.
//! - An impl written inside a macro body is a token tree, not an impl, so
//!   nothing in it is exempt.
//!
//! Blanking keeps every byte offset and newline, so a scan over the result
//! reports the file's own lines, and `lint:allow` is still read off the raw line.

use core::ops::Range;
use std::collections::BTreeSet;

use mockspace_lint_rules::LintContext;
use tree_sitter::{Node, Tree};

use crate::util::{blank_spans, txt};

/// The names of every trait the checked crate declares.
pub struct OwnTraits(BTreeSet<String>);

impl OwnTraits {
    /// Whether the crate declares a trait of this name.
    #[must_use]
    pub fn defines(&self, name: &str) -> bool {
        self.0.contains(name)
    }
}

/// One file of the checked crate, with the text a bare-type scan reads.
pub struct ScannedSource<'a> {
    /// The path the finding names.
    pub rel_path: String,
    /// The file as written, for `//` and `lint:allow` checks.
    pub text:     &'a str,
    /// The file with every foreign-trait signature blanked out.
    pub scanned:  String,
    /// The file's parse.
    pub tree:     &'a Tree,
}

/// Every source of the crate in `ctx`, each with its foreign-trait signatures blanked.
///
/// A context carrying no `all_sources` falls back to its one source as `src/lib.rs`.
#[must_use]
pub fn scanned_sources<'a>(ctx: &LintContext<'a>) -> Vec<ScannedSource<'a>> {
    let files: Vec<(String, &'a str, &'a Tree)> = if ctx.all_sources().is_empty() {
        vec![("src/lib.rs".to_string(), ctx.source(), ctx.tree())]
    } else {
        ctx.all_sources()
            .iter()
            .map(|f| (f.rel_path.display().to_string(), f.text, f.tree()))
            .collect()
    };
    let own = crate_traits(files.iter().map(|(_, text, tree)| (*text, *tree)));
    files
        .into_iter()
        .map(|(rel_path, text, tree)| {
            ScannedSource {
                scanned: without_foreign_trait_signatures(text, tree, &own),
                rel_path,
                text,
                tree,
            }
        })
        .collect()
}

/// The traits declared across the crate's sources.
#[must_use]
pub fn crate_traits<'a>(sources: impl IntoIterator<Item = (&'a str, &'a Tree)>) -> OwnTraits {
    let mut names = BTreeSet::new();
    for (text, tree) in sources {
        collect_trait_names(tree.root_node(), text, &mut names);
    }
    OwnTraits(names)
}

fn collect_trait_names(node: Node, src: &str, names: &mut BTreeSet<String>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "trait_item"
            && let Some(name) = child.child_by_field_name("name")
        {
            names.insert(txt(name, src).to_string());
        }
        collect_trait_names(child, src, names);
    }
}

/// The source with the signature of every item of a foreign-trait impl blanked.
#[must_use]
pub fn without_foreign_trait_signatures(source: &str, tree: &Tree, own: &OwnTraits) -> String {
    let spans = foreign_trait_signature_spans(source, tree, own);
    if spans.is_empty() {
        return source.to_string();
    }
    blank_spans(source, &spans)
}

/// The byte range of every signature a foreign trait fixed in this file.
#[must_use]
pub fn foreign_trait_signature_spans(
    source: &str,
    tree: &Tree,
    own: &OwnTraits,
) -> Vec<Range<usize>> {
    let mut spans = Vec::new();
    collect_spans(tree.root_node(), source, own, &mut spans);
    spans
}

/// Whether `offset` falls inside one of `spans`.
#[must_use]
pub fn within(spans: &[Range<usize>], offset: usize) -> bool {
    spans.iter().any(|s| s.contains(&offset))
}

fn collect_spans(node: Node, src: &str, own: &OwnTraits, spans: &mut Vec<Range<usize>>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "impl_item"
            && let Some(name) = child
                .child_by_field_name("trait")
                .and_then(|t| trait_name(t, src))
            && !own.defines(name)
            && let Some(body) = child.child_by_field_name("body")
        {
            item_signatures(body, spans);
        }
        collect_spans(child, src, own, spans);
    }
}

/// The last path segment of an impl's trait, its generic arguments dropped.
fn trait_name<'a>(node: Node<'a>, src: &'a str) -> Option<&'a str> {
    match node.kind() {
        "type_identifier" => Some(txt(node, src)),
        "scoped_type_identifier" => node.child_by_field_name("name").map(|n| txt(n, src)),
        "generic_type" => {
            node.child_by_field_name("type")
                .and_then(|t| trait_name(t, src))
        },
        _ => None,
    }
}

fn item_signatures(body: Node, spans: &mut Vec<Range<usize>>) {
    let mut cursor = body.walk();
    for item in body.children(&mut cursor) {
        let end = match item.kind() {
            "function_item" => item.child_by_field_name("body").map(|b| b.start_byte() + 1),
            "function_signature_item" => Some(item.end_byte()),
            "const_item" => {
                Some(
                    item.child_by_field_name("value")
                        .map_or(item.end_byte(), |v| v.start_byte()),
                )
            },
            _ => None,
        };
        if let Some(end) = end {
            spans.push(item.start_byte() .. end);
        }
    }
}

#[cfg(test)]
mod tests {
    use mockspace_lint_rules::Reading;

    use super::{crate_traits, without_foreign_trait_signatures};

    fn masked(source: &str) -> String {
        let tree = Reading::new(".").view().parse(source);
        let own = crate_traits([(source, &tree)]);
        without_foreign_trait_signatures(source, &tree, &own)
    }

    #[test]
    fn a_foreign_method_signature_is_blanked_across_lines() {
        let src = "impl de::Visitor for V {\n    fn visit(\n        &mut self,\n    ) -> Result<(), E> {\n        let x: Option<u8> = None;\n    }\n}\n";
        let out = masked(src);
        assert!(!out.contains("Result"), "{out:?}");
        assert!(out.contains("Option<u8>"), "the body stays: {out:?}");
        assert_eq!(out.len(), src.len());
        assert_eq!(out.lines().count(), src.lines().count());
    }

    #[test]
    fn a_one_line_method_keeps_its_body() {
        let out =
            masked("impl Store<'t> for S { fn put(&self, k: &'t str) { let n: u32 = 0; } }\n");
        assert!(!out.contains("&'t str"), "{out:?}");
        assert!(out.contains("u32"), "{out:?}");
    }

    #[test]
    fn an_own_trait_impl_is_untouched() {
        let src = "trait Store { fn put(&self, k: &str); }\nimpl Store for S { fn put(&self, k: &str) {} }\n";
        assert_eq!(masked(src), src);
    }

    #[test]
    fn an_own_trait_reached_by_a_path_with_generics_is_untouched() {
        let src = "mod m { pub trait Store<T> { fn put(&self) -> T; } }\nimpl crate::m::Store<u32> for S { fn put(&self) -> u32 { 0 } }\n";
        assert_eq!(masked(src), src);
    }

    #[test]
    fn an_inherent_impl_is_untouched() {
        let src = "impl S { fn put(&self, k: &str) -> Result<(), E> { Ok(()) } }\n";
        assert_eq!(masked(src), src);
    }

    #[test]
    fn an_associated_const_loses_its_type_and_keeps_its_value() {
        let out = masked("impl Format for S { const BITS: u32 = 7u32 as u32; type Out = u8; }\n");
        assert!(out.contains("7u32 as u32"), "the value stays: {out:?}");
        assert!(!out.contains("BITS"), "{out:?}");
        assert!(
            out.contains("type Out = u8"),
            "an associated type is chosen, not fixed: {out:?}"
        );
    }
}
