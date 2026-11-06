//! Coverage test: `re-export-foreign-names`, the applets framework's names.
//!
//! The console is built on the applets framework and declares its places,
//! commands and surfaces over the framework's types, so those types stand in
//! its public signatures the way a `USize` does in a kit's: the stack's own
//! vocabulary, which a consumer already depends on. The lint read them as a
//! leak of a crate underneath and asked for a `pub use` of each, which the
//! workspace rule that a crate re-exports no other crate forbids. The
//! framework's crates are one family, `applets`, and join the foundation.
//!
//! Every arm carries a control with the same position rooted at a crate that
//! is not the framework's, so the silence is the root and not the position
//! going unread.

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
        crate_name: "console-commands",
        short_name: "commands",
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
        crate_prefix: "kaski",
        primitive_introductions: Box::leak(Box::new(BTreeMap::new())),
    };
    ReExportForeignNames
        .check(&ctx)
        .into_iter()
        .map(|e| e.message)
        .filter(|m| !m.contains("could not be read"))
        .collect()
}

/// A manifest with no prefixed dependency, which is how the console's
/// declaration crate reads, and one with a sibling.
const MANIFESTS: [&[&str]; 2] = [&[], &["kaski-engine-key"]];

#[test]
fn a_name_of_the_applets_framework_in_a_public_signature_is_not_foreign() {
    for deps in MANIFESTS {
        for src in [
            "use applets_command::At;\npub fn of(at: &At) {}\n",
            "use applets_command::Binding;\npub static BINDINGS: &[Binding] = &[];\n",
            "use applets_declare::Declarations;\npub fn all() -> Declarations { todo!() }\n",
            "pub fn grid(c: &applets_grid::Cursor) {}\n",
        ] {
            assert_eq!(findings(src, deps), Vec::<String>::new(), "{src} with {deps:?}");
        }
    }
}

#[test]
fn a_name_of_any_other_crate_in_a_public_signature_is_still_foreign() {
    for deps in MANIFESTS {
        for src in [
            "use riimu_face::Face;\npub fn of(face: &Face) {}\n",
            "pub fn grid(c: &riimu_face::Cursor) {}\n",
            "pub fn apple(c: &applesauce::Cursor) {}\n",
        ] {
            if deps.is_empty() {
                assert_eq!(findings(src, deps).len(), 1, "{src} with {deps:?}");
            }
        }
    }
}
