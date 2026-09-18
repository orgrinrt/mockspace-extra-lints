//! Coverage test: `re-export-foreign-names`, a path whose root is one of the
//! item's own type parameters.
//!
//! `S::Items` where `S: Store` names an associated type of whatever `S` turns
//! out to be. The first segment is a type parameter, not a crate, so there is
//! nothing to re-export and no crate a consumer would have to depend on. The
//! lint read the root as a crate called `S` and asked for `pub use S::Items`,
//! which is not Rust, and it did so against renki's own `renki-config`.
//!
//! Every arm carries a control with the same position rooted at a crate, so
//! the silence is the root and not the position going unread.

use std::collections::{BTreeMap, BTreeSet};

use mockspace_extra_lints::lints::re_export_foreign_names::ReExportForeignNames;
use mockspace_lint_rules::{CrateLint, CrateSourceFile, LintContext};

fn findings(src: &'static str, deps: &'static [&'static str]) -> Vec<String> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree: &'static tree_sitter::Tree = Box::leak(Box::new(parser.parse(src, None).unwrap()));
    let all_sources: &'static [CrateSourceFile] = Box::leak(Box::new(vec![CrateSourceFile {
        rel_path: std::path::PathBuf::from("src/lib.rs"),
        text:     src.to_string(),
    }]));
    let deps: &'static [String] =
        Box::leak(Box::new(deps.iter().map(|d| d.to_string()).collect::<Vec<_>>()));
    let ctx = LintContext {
        crate_name: "kit-text",
        short_name: "text",
        source: src,
        tree,
        all_sources,
        deps,
        all_crates: Box::leak(Box::new(BTreeSet::new())),
        design_doc: None,
        all_doc_content: "",
        shame_doc: None,
        workspace_root: std::path::Path::new("/nonexistent"),
        proc_macro_crates: &[],
        lint_proc_macro_source: false,
        crate_prefix: "kit",
        primitive_introductions: Box::leak(Box::new(BTreeMap::new())),
    };
    ReExportForeignNames
        .check(&ctx)
        .into_iter()
        .map(|e| e.message)
        .filter(|m| !m.contains("could not be read"))
        .collect()
}

/// Both manifests: one declaring dependencies, where an unknown root is not
/// foreign unless it is one of them, and one declaring none, where every
/// unreserved root is.
const MANIFESTS: [&[&str]; 2] = [&["riimu-face", "notko"], &[]];

#[test]
fn a_path_rooted_at_a_functions_own_parameter_is_not_foreign() {
    for deps in MANIFESTS {
        for src in [
            "pub trait Store { type Document; }\npub fn read<St: Store>(doc: &St::Document) {}\n",
            "pub trait Store { type Document; }\npub fn make<St: Store>() -> St::Document { todo!() }\n",
            "pub trait Store { type Document; }\npub fn both<St: Store, L>(a: St::Document, b: L) {}\n",
            "pub trait Store { type Document; }\npub fn nested<St: Store>(d: Option<&[St::Document]>) {}\n",
            "pub trait Store { type Document; }\npub fn later<St>(d: St::Document) where St: Store {}\n",
        ] {
            assert_eq!(findings(src, deps), Vec::<String>::new(), "{src} with {deps:?}");
        }
    }
    let control = "pub fn read(doc: &riimu_face::Document) {}\n";
    for deps in MANIFESTS {
        assert_eq!(findings(control, deps).len(), 1, "{control} with {deps:?}");
    }
}

#[test]
fn a_path_rooted_at_a_types_own_parameter_is_not_foreign() {
    for deps in MANIFESTS {
        for src in [
            "pub trait Store { type Items; }\npub enum ListValue<S: Store> { FromStore(S::Items) }\n",
            "pub trait Store { type Items; }\npub struct Held<S: Store> { pub items: S::Items }\n",
            "pub trait Store { type Items; }\npub struct Pair<S: Store>(pub S::Items, pub u8);\n",
        ] {
            assert_eq!(findings(src, deps), Vec::<String>::new(), "{src} with {deps:?}");
        }
    }
    let control = "pub struct Held { pub items: riimu_face::Items }\n";
    for deps in MANIFESTS {
        assert_eq!(findings(control, deps).len(), 1, "{control} with {deps:?}");
    }
}

#[test]
fn a_path_rooted_at_an_impls_parameter_is_not_foreign() {
    for deps in MANIFESTS {
        let src = "pub trait Store { type Items; }\npub struct Held<S>(S);\n\
                   impl<S: Store> Held<S> {\n    pub fn items(&self) -> S::Items { todo!() }\n}\n";
        assert_eq!(findings(src, deps), Vec::<String>::new(), "with {deps:?}");
    }
}

#[test]
fn a_parameter_of_one_item_does_not_excuse_the_same_name_in_another() {
    // `S` is a parameter of `first` only. In `second` it is a root nothing in
    // scope binds, and with no dependencies declared that reads as a crate.
    let src = "pub trait Store { type Items; }\n\
               pub fn first<S: Store>(i: S::Items) {}\n\
               pub fn second(i: S::Items) {}\n";
    assert_eq!(findings(src, &[]).len(), 1, "{:?}", findings(src, &[]));
}

#[test]
fn a_path_rooted_at_a_traits_own_parameter_is_not_foreign() {
    for deps in MANIFESTS {
        let src = "pub trait Store { type Items; }\npub trait Shelf<S: Store> { fn items(&self) -> S::Items; }\n";
        assert_eq!(findings(src, deps), Vec::<String>::new(), "with {deps:?}");
    }
}

#[test]
fn a_path_rooted_at_a_trait_methods_parameter_is_not_foreign() {
    for deps in MANIFESTS {
        let src = "pub trait Store { type Items; }\npub trait Shelf { fn put<S: Store>(&self, items: S::Items); }\n";
        assert_eq!(findings(src, deps), Vec::<String>::new(), "with {deps:?}");
    }
}

#[test]
fn a_parameter_shadowing_an_imported_crate_alias_is_the_parameter() {
    // `S` is both a crate alias in scope and the function's parameter, and the
    // parameter is the nearer binding.
    let src = "use riimu_face as S;\npub trait Store { type Items; }\npub fn f<S: Store>(x: S::Items) {}\n";
    assert_eq!(findings(src, &["riimu-face"]), Vec::<String>::new());
}

#[test]
fn the_crate_alias_is_still_foreign_where_no_parameter_shadows_it() {
    let src = "use riimu_face as S;\npub fn f(x: S::Items) {}\n";
    assert_eq!(findings(src, &["riimu-face"]).len(), 1, "{:?}", findings(src, &["riimu-face"]));
}

#[test]
#[ignore = "catalogue: an impl's associated type is not read as a position, so a foreign path there goes unreported; tracked agenda an-impl-associated-type-is-a-position"]
fn a_foreign_path_in_an_impls_associated_type_is_reported() {
    for deps in MANIFESTS {
        let src = "pub trait Has { type X; }\npub struct Held;\nimpl Has for Held { type X = riimu_face::Items; }\n";
        assert_eq!(findings(src, deps).len(), 1, "with {deps:?}: {:?}", findings(src, deps));
    }
}
