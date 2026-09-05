//! Coverage test: the dispatch axis itself.
//!
//! The engine's `Lint::per_file` defaults to `true`, and the dispatcher hands a
//! per-file lint the same context with `source` swapped for each module file.
//! Two consequences follow, and neither shows up in a lint's own findings when
//! it is called once by hand, which is why the split-trait migration shipped
//! both defects with a green suite:
//!
//! - A lint that walks `all_sources` itself must declare `per_file(false)`, or
//!   the dispatcher asks it once per file and it reports its whole crate each
//!   time.
//! - A lint making a claim about the crate root must fire that claim only when
//!   `source` is the root, or every module file is reported as a root.
//!
//! These tests assert the declarations and simulate the dispatcher, because the
//! defect lives in the interaction rather than in either half alone.

use std::collections::{BTreeMap, BTreeSet};

use mockspace_extra_lints::lints::{
    arvo_types_only::ArvoTypesOnly, no_bare_numeric::NoBareNumeric,
    no_bare_option::NoBareOption, no_bare_result::NoBareResult,
    no_bare_static_str::NoBareStaticStr, no_bare_string::NoBareString,
    no_public_raw_field::NoPublicRawField, no_std::NoStd,
    re_export_foreign_names::ReExportForeignNames,
};
use mockspace_lint_rules::{CrateLint, CrateSourceFile, Lint, LintContext};

/// A crate of several files. `files[0]` is the root, as the engine documents.
fn ctx_over(files: &'static [(&'static str, &'static str)]) -> LintContext<'static> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(files[0].1, None).unwrap();
    let tree: &'static tree_sitter::Tree = Box::leak(Box::new(tree));

    let all_sources: &'static [CrateSourceFile] = Box::leak(Box::new(
        files
            .iter()
            .map(|(p, text)| CrateSourceFile {
                rel_path: std::path::PathBuf::from(*p),
                text:     text.to_string(),
            })
            .collect::<Vec<_>>(),
    ));

    LintContext {
        crate_name: "test-crate",
        short_name: "test-crate",
        source: files[0].1,
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

/// What the dispatcher does to a per-file lint: one call per file, with
/// `source` swapped. Mirrors `check_every_file` in the engine.
fn dispatch<L: CrateLint>(lint: &L, ctx: &LintContext<'static>) -> usize {
    if lint.per_file() {
        ctx.all_sources
            .iter()
            .map(|f| {
                let mut per = *ctx;
                // The engine swaps `tree` too; these lints read `source` and
                // `all_sources`, so swapping `source` is what they observe.
                per.source = Box::leak(f.text.clone().into_boxed_str());
                lint.check(&per).len()
            })
            .sum()
    } else {
        lint.check(ctx).len()
    }
}

const THREE_FILE_CRATE: &[(&str, &str)] = &[
    ("src/lib.rs", "#![no_std]\npub struct A(u32);\n"),
    ("src/b.rs", "pub struct B(u32);\n"),
    ("src/c.rs", "pub struct C(u32);\n"),
];

// ---- the walkers declare per-crate dispatch -------------------------------

#[test]
fn every_all_sources_walker_declares_per_crate_dispatch() {
    // Left at the engine's default, each of these reports its whole crate once
    // per file. The declaration is the fix, so assert the declaration.
    assert!(!ArvoTypesOnly.per_file(), "arvo-types-only walks all_sources");
    assert!(!NoBareNumeric.per_file(), "no-bare-numeric walks all_sources");
    assert!(!NoBareOption.per_file(), "no-bare-option walks all_sources");
    assert!(!NoBareResult.per_file(), "no-bare-result walks all_sources");
    assert!(!NoBareString.per_file(), "no-bare-string walks all_sources");
    assert!(
        !NoBareStaticStr.per_file(),
        "no-bare-static-str walks all_sources"
    );
    assert!(
        !NoPublicRawField.per_file(),
        "no-public-raw-field walks all_sources"
    );
    assert!(
        !ReExportForeignNames.per_file(),
        "re-export-foreign-names walks all_sources"
    );
}

#[test]
fn a_walker_reports_each_finding_once_across_the_crate() {
    let ctx = ctx_over(THREE_FILE_CRATE);
    // Three files, one bare u32 each. Dispatched correctly that is three
    // findings; dispatched per file it would be nine.
    let n = dispatch(&ArvoTypesOnly, &ctx);
    assert_eq!(
        n, 3,
        "expected one finding per offending file, got {n}; a count that is a \
         multiple of the file count means the walker is being dispatched per file"
    );
}

#[test]
fn a_walker_names_the_file_it_found_the_drift_in() {
    // Per-crate dispatch means the engine cannot supply the path, so the lint
    // must. Folding it into the message leaves `path` empty and the rendered
    // location points at nothing.
    let ctx = ctx_over(THREE_FILE_CRATE);
    let hits = ArvoTypesOnly.check(&ctx);
    assert!(!hits.is_empty(), "fixture must produce findings");
    for h in &hits {
        assert!(
            h.path.is_some(),
            "a per-crate lint must set LintError::path; message was: {}",
            h.message
        );
    }
}

// ---- no-std's two halves --------------------------------------------------

#[test]
fn no_std_reports_a_missing_root_attribute_once() {
    const NO_ATTR: &[(&str, &str)] = &[
        ("src/lib.rs", "pub struct A;\n"),
        ("src/b.rs", "pub struct B;\n"),
        ("src/c.rs", "pub struct C;\n"),
    ];
    let ctx = ctx_over(NO_ATTR);
    let n = dispatch(&NoStd, &ctx);
    assert_eq!(
        n, 1,
        "the root is missing `#![no_std]` once, not once per module file"
    );
}

#[test]
fn no_std_does_not_report_module_files_as_roots() {
    let ctx = ctx_over(THREE_FILE_CRATE);
    let n = dispatch(&NoStd, &ctx);
    assert_eq!(
        n, 0,
        "the root carries `#![no_std]`, so no file should be reported"
    );
}

#[test]
fn no_std_still_sees_use_std_in_a_module_file() {
    // The per-line half is why this lint stays per-file. Before the trait
    // split it saw only the root, and a module file could import std unseen.
    const STD_IN_MODULE: &[(&str, &str)] = &[
        ("src/lib.rs", "#![no_std]\npub mod b;\n"),
        ("src/b.rs", "use std::vec::Vec;\npub struct B;\n"),
    ];
    let ctx = ctx_over(STD_IN_MODULE);
    let n = dispatch(&NoStd, &ctx);
    assert!(
        n >= 1,
        "a module file importing std must be caught, which is what per-file \
         dispatch is for"
    );
}

#[test]
fn no_std_stays_per_file_because_its_second_half_needs_every_file() {
    assert!(
        NoStd.per_file(),
        "no-std scans lines for `use std::` and must see every file; only its \
         crate-root half is guarded"
    );
}
