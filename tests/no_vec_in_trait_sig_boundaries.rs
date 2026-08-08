//! Coverage test: `no-vec-in-trait-sig` names types, it does not match letters.
//!
//! Every entry in the forbidden list but one ends in `<`, which anchors it.
//! `String` does not, and unanchored it matches any type whose name merely
//! begins with those letters. A real consumer trait taking a
//! `&StringInterner<impl ArenaInterner>` was reported as containing `String`,
//! and the file already carried a `lint:allow` where an earlier author had hit
//! the same confusion with a different lint.
//!
//! That is worse than a missed finding: a lint that is right about everything
//! else teaches readers to reach for an allow when it is wrong, and the allows
//! stay after the lint is fixed.

use std::collections::{BTreeMap, BTreeSet};

use mockspace_extra_lints::lints::no_vec_in_trait_sig::NoVecInTraitSig;
use mockspace_lint_rules::{CrateLint, CrateSourceFile, LintContext};

fn ctx_with(source: &'static str) -> LintContext<'static> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let tree: &'static tree_sitter::Tree = Box::leak(Box::new(tree));

    let all_sources: &'static [CrateSourceFile] =
        Box::leak(Box::new(vec![CrateSourceFile {
            rel_path: std::path::PathBuf::from("src/lib.rs"),
            text:     source.to_string(),
        }]));

    LintContext {
        crate_name: "test-crate",
        short_name: "test-crate",
        source,
        tree,
        all_sources,
        deps: &[],
        all_crates: Box::leak(Box::new(BTreeSet::new())),
        design_doc: None,
        all_doc_content: "",
        shame_doc: None,
        workspace_root: std::path::Path::new("/tmp"),
        proc_macro_crates: &[],
        lint_proc_macro_source: false,
        crate_prefix: "test",
        primitive_introductions: Box::leak(Box::new(BTreeMap::new())),
    }
}

/// The real case. A wrapper around an interner is not a heap string.
#[test]
fn a_type_merely_beginning_with_string_is_not_a_string() {
    let src = "pub trait IntoStr {\n    fn into_str(self, interner: &StringInterner<impl \
               ArenaInterner>) -> Str;\n}\n";
    let hits = NoVecInTraitSig.check(&ctx_with(src));
    assert!(
        hits.is_empty(),
        "StringInterner is not String; got: {:?}",
        hits.iter().map(|h| &h.message).collect::<Vec<_>>()
    );
}

/// The same trap in the return position, which is a different code path in the
/// signature the lint assembles.
#[test]
fn a_returned_type_merely_beginning_with_string_is_not_a_string() {
    let src = "pub trait Build {\n    fn builder(&self) -> StringBuilder;\n}\n";
    let hits = NoVecInTraitSig.check(&ctx_with(src));
    assert!(hits.is_empty(), "StringBuilder is not String");
}

/// And the lint must still do its job, or the fix above is just a hole.
#[test]
fn a_real_string_parameter_is_still_caught() {
    let src = "pub trait Name {\n    fn set(&mut self, name: String);\n}\n";
    let hits = NoVecInTraitSig.check(&ctx_with(src));
    assert_eq!(hits.len(), 1, "a bare String parameter must be caught");
}

#[test]
fn a_real_string_return_is_still_caught() {
    let src = "pub trait Name {\n    fn get(&self) -> String;\n}\n";
    let hits = NoVecInTraitSig.check(&ctx_with(src));
    assert_eq!(hits.len(), 1, "a bare String return must be caught");
}

/// A `String` inside a generic argument is still a `String`, and the closing
/// angle bracket is not an identifier character, so the boundary rule keeps it.
#[test]
fn a_string_inside_a_generic_argument_is_still_caught() {
    let src = "pub trait Names {\n    fn all(&self) -> Maybe<String>;\n}\n";
    let hits = NoVecInTraitSig.check(&ctx_with(src));
    assert_eq!(hits.len(), 1, "Maybe<String> still contains a String");
}

/// The anchored entries were never affected and must not regress.
#[test]
fn the_anchored_entries_still_fire() {
    let src = "pub trait Bag {\n    fn items(&self) -> Vec<u8>;\n}\n";
    let hits = NoVecInTraitSig.check(&ctx_with(src));
    assert_eq!(hits.len(), 1, "Vec< is anchored and must still fire");
}
