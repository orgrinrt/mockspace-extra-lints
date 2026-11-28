//! The signatures a crate writes only because a trait of another crate fixed them.
//!
//! A crate implementing a trait it does not define copies that trait's method
//! signatures and associated constant types; it cannot change them. A parser's
//! `Visitor` returning `core::result::Result<(), Self::Error>`, or a store
//! contract taking `&'t str`, is judged where the trait is declared, in the
//! crate that owns it. The bare-type lints read the signature of a method in an
//! `impl <Trait> for <Type>` block as exempt when the trait is not the crate's
//! own, and everything else as before.
//!
//! What is exempt, and only that:
//!
//! - a method's signature, from the start of the item (its `fn` keyword, or the
//!   `const`, `unsafe`, `async` or `extern` in front of it) to its opening `{`,
//!   or to its `;` where it has no body, across lines, but for a return type
//!   the method marks as its own refinement of the trait's, by an attribute
//!   naming `refine` or the compiler's `refining_impl_trait` lints, which is
//!   read like any other;
//! - an associated constant's declaration, from `const` to its `=`, or to its
//!   `;` where it has no value.
//!
//! A method body, the value of a constant, an associated type, a field, an
//! inherent impl and an impl of a trait the crate defines are all still read.
//!
//! Which trait is the crate's own is read off the path the impl names it by:
//!
//! - a path rooted at `crate`, `self` or `super`, or at a module the crate
//!   declares, is the crate's own;
//! - a path rooted at anything else, another crate's name or the prelude's, is
//!   foreign;
//! - a bare name the file brings in with `use`, under its own name or another
//!   with `as`, is what the path it is brought in by is;
//! - a bare name the file brings in from nowhere is the crate's own where a
//!   `trait` item of that name is anywhere in the crate's parsed sources, and
//!   foreign otherwise, as a prelude trait is.
//!
//! What it misjudges:
//!
//! - A `use` is read for the whole file, not for the module it stands in, so
//!   two modules of one file bringing one name in from two places are read as
//!   one of them.
//! - A trait the crate re-exports from another crate and names by a path of the
//!   crate is read as its own, so its signatures stay checked. This errs
//!   toward reporting.
//! - A trait the crate defines only through a macro expansion has no `trait`
//!   item in the parse, so an impl of it by a bare name is read as foreign; the
//!   trait's own declaration in the macro's body is still read by the line
//!   scans, so its signatures are judged there, where it is declared.
//! - An impl written inside a macro body is a token tree, not an impl, so
//!   nothing in it is exempt.
//! - A return type an impl refines without saying so is read as the trait's;
//!   the compiler warns on such an impl.
//!
//! Blanking keeps every byte offset and newline, so a scan over the result
//! reports the file's own lines, and `lint:allow` is still read off the raw line.

use core::ops::Range;
use std::collections::{BTreeMap, BTreeSet};

use mockspace_lint_rules::LintContext;
use tree_sitter::{Node, Tree};

use crate::util::{blank_spans, txt};

/// What the checked crate declares that tells its own traits from others':
/// the names of its traits and of its modules.
pub struct OwnTraits {
    traits:  BTreeSet<String>,
    modules: BTreeSet<String>,
}

impl OwnTraits {
    /// Whether the crate declares a trait of this name.
    #[must_use]
    pub fn defines(&self, name: &str) -> bool {
        self.traits.contains(name)
    }

    /// Whether a path rooted at `root` is a path of the crate.
    fn roots(&self, root: &str) -> bool {
        matches!(root, "crate" | "self" | "super") || self.modules.contains(root)
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
/// The engine hands a crate lint every source of the crate in `all_sources`.
/// A context carrying none, as one a test builds from a single text does, is a
/// crate of that one file, read as `src/lib.rs`, so its own traits are the
/// ones that file declares.
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

/// The traits and modules declared across the crate's sources.
#[must_use]
pub fn crate_traits<'a>(sources: impl IntoIterator<Item = (&'a str, &'a Tree)>) -> OwnTraits {
    let mut own = OwnTraits {
        traits:  BTreeSet::new(),
        modules: BTreeSet::new(),
    };
    for (text, tree) in sources {
        collect_declared(tree.root_node(), text, &mut own);
    }
    own
}

fn collect_declared(node: Node, src: &str, own: &mut OwnTraits) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        let names = match child.kind() {
            "trait_item" => Some(&mut own.traits),
            "mod_item" => Some(&mut own.modules),
            _ => None,
        };
        if let Some(names) = names
            && let Some(name) = child.child_by_field_name("name")
        {
            names.insert(txt(name, src).to_string());
        }
        collect_declared(child, src, own);
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
    let mut imported = BTreeMap::new();
    collect_uses(tree.root_node(), source, own, &mut imported);
    let mut spans = Vec::new();
    collect_spans(tree.root_node(), source, own, &imported, &mut spans);
    spans
}

/// Whether `offset` falls inside one of `spans`.
#[must_use]
pub fn within(spans: &[Range<usize>], offset: usize) -> bool {
    spans.iter().any(|s| s.contains(&offset))
}

/// Every name the file's `use` declarations bring in, each with whether the
/// path it comes by is the crate's own.
fn collect_uses(node: Node, src: &str, own: &OwnTraits, imported: &mut BTreeMap<String, bool>) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "use_declaration"
            && let Some(tree) = child.child_by_field_name("argument")
        {
            bindings(tree, src, None, own, imported);
        } else {
            collect_uses(child, src, own, imported);
        }
    }
}

/// The names a use tree binds, `root` the first segment of the path above it.
fn bindings(
    node: Node,
    src: &str,
    root: Option<&str>,
    own: &OwnTraits,
    imported: &mut BTreeMap<String, bool>,
) {
    let mut bind = |name: &str, first: &str| {
        imported.insert(name.to_string(), own.roots(root.unwrap_or(first)));
    };
    match node.kind() {
        "identifier" => bind(txt(node, src), txt(node, src)),
        "scoped_identifier" => {
            if let Some(name) = node.child_by_field_name("name") {
                bind(txt(name, src), leftmost(node, src));
            }
        },
        "use_as_clause" => {
            if let (Some(path), Some(alias)) = (
                node.child_by_field_name("path"),
                node.child_by_field_name("alias"),
            ) {
                bind(txt(alias, src), leftmost(path, src));
            }
        },
        "scoped_use_list" => {
            let first = node
                .child_by_field_name("path")
                .map_or("", |p| leftmost(p, src));
            let root = Some(root.unwrap_or(first));
            if let Some(list) = node.child_by_field_name("list") {
                let mut cursor = list.walk();
                for item in list.named_children(&mut cursor) {
                    bindings(item, src, root, own, imported);
                }
            }
        },
        "use_list" => {
            let mut cursor = node.walk();
            for item in node.named_children(&mut cursor) {
                bindings(item, src, root, own, imported);
            }
        },
        _ => {},
    }
}

/// The first segment of a path: `crate`, `self`, `super` or a name, or the
/// empty text for a path rooted at `::`.
fn leftmost<'a>(mut node: Node<'a>, src: &'a str) -> &'a str {
    loop {
        match node.kind() {
            "scoped_identifier" | "scoped_type_identifier" => {
                match node.child_by_field_name("path") {
                    Some(path) => node = path,
                    None => return "",
                }
            },
            "generic_type" => {
                match node.child_by_field_name("type") {
                    Some(t) => node = t,
                    None => return "",
                }
            },
            _ => return txt(node, src),
        }
    }
}

fn collect_spans(
    node: Node,
    src: &str,
    own: &OwnTraits,
    imported: &BTreeMap<String, bool>,
    spans: &mut Vec<Range<usize>>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "impl_item"
            && let Some(foreign) = child
                .child_by_field_name("trait")
                .and_then(|t| is_foreign(t, src, own, imported))
            && foreign
            && let Some(body) = child.child_by_field_name("body")
        {
            item_signatures(body, src, spans);
        }
        collect_spans(child, src, own, imported, spans);
    }
}

/// Whether an impl's trait is another crate's, by the path it is named by;
/// `None` for a trait written in no shape this reads.
fn is_foreign(node: Node, src: &str, own: &OwnTraits, imported: &BTreeMap<String, bool>) -> Option<bool> {
    match node.kind() {
        "type_identifier" => {
            let name = txt(node, src);
            Some(match imported.get(name) {
                Some(of_the_crate) => !of_the_crate,
                None => !own.defines(name),
            })
        },
        "scoped_type_identifier" => {
            let root = leftmost(node, src);
            let of_the_crate = own.roots(root) || imported.get(root).copied().unwrap_or(false);
            Some(!of_the_crate)
        },
        "generic_type" => node.child_by_field_name("type").and_then(|t| is_foreign(t, src, own, imported)),
        _ => None,
    }
}

/// Whether the attributes just above `item` mark its return type as the
/// impl's own refinement of the trait's.
fn refines(item: Node, src: &str) -> bool {
    let mut at = item.prev_named_sibling();
    while let Some(node) = at {
        if node.kind() != "attribute_item" {
            return false;
        }
        let text = txt(node, src);
        if text.contains("refining_impl_trait") || text.split(|c: char| !c.is_alphanumeric() && c != '_').any(|w| w == "refine") {
            return true;
        }
        at = node.prev_named_sibling();
    }
    false
}

fn item_signatures(body: Node, src: &str, spans: &mut Vec<Range<usize>>) {
    let mut cursor = body.walk();
    for item in body.children(&mut cursor) {
        let end = match item.kind() {
            "function_item" => {
                let own_return = refines(item, src)
                    .then(|| item.child_by_field_name("return_type"))
                    .flatten();
                match own_return {
                    // up to the `->`, which stands right before the type
                    Some(ret) => Some(ret.start_byte()),
                    None => item.child_by_field_name("body").map(|b| b.start_byte() + 1),
                }
            },
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
