//! Where `no-std`, `no-alloc` and `no-dyn-dispatch` stop.
//!
//! A consumer that carries its own enforcer for one of these concerns has to
//! decide whether the pack's lint subsumes it before deleting anything, and
//! that decision wants the boundary written down rather than re-derived by
//! reading three lint bodies. arvo is the live case: it runs `no-std`,
//! `no-alloc` and `no-dyn-dispatch` from this pack at `error` and its own
//! `no-std-enforcer`, `no-alloc-enforcer` and `no-dynamic-dispatch` at
//! `error` beside them, six lints over three concerns.
//!
//! So each concern gets both directions here: a shape the pack catches that a
//! narrower local scanner is documented to miss, and a shape the pack does not
//! catch at all. The second kind is what a local enforcer is still being kept
//! for, and it is the half that would go silently if one were deleted on the
//! strength of the names matching.
//!
//! The fixtures are planted rather than read off arvo's tree, so the arms say
//! what the shapes are and do not move when arvo does. arvo's tree is green
//! under all six today, which is exactly why running them over it distinguishes
//! nothing and the shapes have to be planted.

use std::collections::{BTreeMap, BTreeSet};

use mockspace_extra_lints::lints::no_alloc::NoAlloc;
use mockspace_extra_lints::lints::no_dyn_dispatch::NoDynDispatch;
use mockspace_extra_lints::lints::no_std::NoStd;
use mockspace_lint_rules::{CrateLint, CrateSourceFile, LintContext};

fn ctx(source: &'static str) -> LintContext<'static> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let tree: &'static tree_sitter::Tree = Box::leak(Box::new(tree));

    // `no-std` decides whether it is looking at the crate root by comparing
    // this text against the first source file, so the fixture has to be that
    // file or the root half of the lint never runs on it.
    let all_sources: &'static [CrateSourceFile] = Box::leak(Box::new(vec![CrateSourceFile {
        rel_path: std::path::PathBuf::from("src/lib.rs"),
        text: source.to_string(),
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
        // These fixtures are ordinary crates. Turning the proc-macro opt-in on
        // would test a different thing, since all three lints skip that source.
        lint_proc_macro_source: false,
        crate_prefix: "test",
        primitive_introductions: Box::leak(Box::new(BTreeMap::new())),
    }
}

// ---- no-std ---------------------------------------------------------------

/// The shape a `use`-anchored scanner misses: a fully qualified call that
/// never imports the path. arvo's own enforcer catalogues this as a red test
/// against itself, and the pack's per-line scan closes it.
#[test]
fn a_fully_qualified_std_call_with_no_import_fires() {
    let src = "#![no_std]\n\nfn f() {\n    std::process::exit(0);\n}\n";
    let hits = NoStd.check(&ctx(src));
    assert_eq!(
        hits.len(),
        1,
        "a fully qualified `std::` call must fire even with no `use`: {hits:?}"
    );
    assert_eq!(hits[0].line, 4, "the finding names the calling line");
}

/// The root half, which a per-line scanner has no notion of at all: the
/// attribute's absence is a fact about the crate rather than about any line.
#[test]
fn a_root_missing_the_attribute_fires_with_no_offending_line() {
    let src = "use core::fmt;\n";
    let hits = NoStd.check(&ctx(src));
    assert!(
        hits.iter().any(|h| h.line == 1),
        "a root with no `#![no_std]` must fire: {hits:?}"
    );
}

#[test]
fn core_paths_under_the_attribute_are_silent() {
    let src = "#![no_std]\nuse core::fmt;\n";
    let hits = NoStd.check(&ctx(src));
    assert!(hits.is_empty(), "`core::` is the replacement, not a hit: {hits:?}");
}

// ---- no-alloc -------------------------------------------------------------

/// Containers past `Vec`, `String` and `Box`. A local enforcer written against
/// those three names alone is silent here, and this is the widest of the three
/// concerns in the pack's favour.
#[test]
fn the_map_and_handle_containers_fire() {
    for src in [
        "let m: HashMap<K, V> = todo!();\n",
        "let m: BTreeMap<K, V> = todo!();\n",
        "let s: VecDeque<T> = todo!();\n",
        "let a: Arc<T> = todo!();\n",
        "let r: Rc<T> = todo!();\n",
    ] {
        let hits = NoAlloc.check(&ctx(Box::leak(src.to_string().into_boxed_str())));
        assert!(!hits.is_empty(), "must fire on {src:?}");
    }
}

/// The boundary discipline, which is what keeps the wider name list from
/// costing more than it buys.
#[test]
fn a_name_merely_containing_a_container_is_silent() {
    let src = "let v: SmallVec<T, N> = todo!();\n";
    let hits = NoAlloc.check(&ctx(src));
    assert!(hits.is_empty(), "`SmallVec` is not `Vec`: {hits:?}");
}

#[test]
fn a_fixed_size_array_is_silent() {
    let src = "let a: [u8; 4] = [0; 4];\n";
    let hits = NoAlloc.check(&ctx(src));
    assert!(hits.is_empty(), "the array is the replacement: {hits:?}");
}

// ---- no-dyn-dispatch ------------------------------------------------------

#[test]
fn the_dyn_shapes_fire() {
    for src in [
        "fn f(t: &dyn Trait) {}\n",
        "fn f(t: Box<dyn Trait>) {}\n",
        "fn f(t: &mut dyn Trait) {}\n",
    ] {
        let hits = NoDynDispatch.check(&ctx(Box::leak(src.to_string().into_boxed_str())));
        assert!(!hits.is_empty(), "must fire on {src:?}");
    }
}

/// The load-bearing half of this file. Runtime type erasure reaches the same
/// end as `dyn` by a different spelling, and this lint has no notion of it: it
/// scans for `dyn` in a type position and nothing else. A consumer that forbids
/// dynamic dispatch because monomorphisation is the dispatch is not served by
/// this lint alone, and deleting a local enforcer that does catch these leaves
/// no gate on them and no report that a gate went.
#[test]
fn type_erasure_without_the_dyn_keyword_is_not_caught() {
    for src in [
        "let id = TypeId::of::<T>();\n",
        "use core::any::Any;\n",
        "use std::any::TypeId;\n",
        "fn f(v: &dyn_free_alias) {}\n",
    ] {
        let hits = NoDynDispatch.check(&ctx(Box::leak(src.to_string().into_boxed_str())));
        assert!(
            hits.is_empty(),
            "this lint does not read {src:?} as dispatch; if it now does, the \
             boundary this file pins has moved and a consumer's local enforcer \
             may have become redundant: {hits:?}"
        );
    }
}

/// The mirror of the case above, so the boundary is stated from both sides:
/// the one `std::any` spelling that does fire, and it fires for the wrong
/// reason. `no-dyn-dispatch` is silent on it; `no-std` catches it as a `std`
/// path, which means the coverage a consumer gets depends on which lints it
/// enabled rather than on what the shape is.
#[test]
fn a_std_any_import_is_caught_only_by_the_std_lint() {
    let src = "#![no_std]\nuse std::any::TypeId;\n";
    let dispatch = NoDynDispatch.check(&ctx(src));
    assert!(
        dispatch.is_empty(),
        "the dispatch lint has no notion of `any`: {dispatch:?}"
    );
    let std_hits = NoStd.check(&ctx(src));
    assert!(
        !std_hits.is_empty(),
        "the std lint catches it as a std path, not as erasure: {std_hits:?}"
    );
}
