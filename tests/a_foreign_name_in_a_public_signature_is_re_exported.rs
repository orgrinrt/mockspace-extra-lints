//! Coverage test: `re-export-foreign-names`, the tier over a crate's own
//! signatures.
//!
//! The rule is a re-export discipline: owning a dependency means owning the
//! names it puts in your own API, so a foreign type in a public position is
//! reachable from here or the consumer has been made to depend on the crate
//! underneath. Every case below plants the position in source, so the lint is
//! measured against what a signature actually contains rather than against a
//! description of one.
//!
//! Every arm carries its control, the same source with the re-export added,
//! because a lint that reported nothing anywhere would pass half of these.

use std::collections::{BTreeMap, BTreeSet};

use mockspace_extra_lints::lints::re_export_foreign_names::ReExportForeignNames;
use mockspace_lint_rules::{CrateLint, CrateSourceFile, Lint, LintContext};

/// A crate of several files, with `deps` as the manifest names them.
/// `files[0]` is the root, as the engine documents.
fn ctx_over(
    files: &'static [(&'static str, &'static str)],
    deps: &'static [&'static str],
    siblings: &'static [&'static str],
) -> LintContext<'static> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(files[0].1, None).unwrap();
    let tree: &'static tree_sitter::Tree = Box::leak(Box::new(tree));
    let all_sources: &'static [CrateSourceFile] = Box::leak(Box::new(
        files
            .iter()
            .map(|(p, text)| {
                CrateSourceFile {
                    rel_path: std::path::PathBuf::from(*p),
                    text:     text.to_string(),
                }
            })
            .collect::<Vec<_>>(),
    ));
    let deps: &'static [String] = Box::leak(Box::new(
        deps.iter().map(|d| d.to_string()).collect::<Vec<_>>(),
    ));
    let all_crates: &'static BTreeSet<String> = Box::leak(Box::new(
        siblings
            .iter()
            .map(|s| s.to_string())
            .collect::<BTreeSet<_>>(),
    ));
    LintContext {
        crate_name: "kit-text",
        short_name: "text",
        source: files[0].1,
        tree,
        all_sources,
        deps,
        all_crates,
        design_doc: None,
        all_doc_content: "",
        shame_doc: None,
        // No `Cargo.toml` here, so the second tier has nothing to read; every
        // case in this file is about the first.
        workspace_root: std::path::Path::new("/nonexistent"),
        proc_macro_crates: &[],
        lint_proc_macro_source: false,
        crate_prefix: "kit",
        primitive_introductions: Box::leak(Box::new(BTreeMap::new())),
    }
}

fn findings(src: &'static str) -> Vec<String> {
    let files: &'static [(&'static str, &'static str)] = Box::leak(Box::new([("src/lib.rs", src)]));
    let ctx = ctx_over(files, &["riimu-face", "notko"], &[]);
    tier_one(&ctx)
}

/// The findings of the first tier alone.
///
/// A `pub use` of a foreign name makes the second tier run, and with no
/// workspace on disk it reports that the surface could not be read, which is
/// measured in `a_re_exported_type_carries_its_surface.rs` and is noise here.
fn tier_one(ctx: &LintContext<'static>) -> Vec<String> {
    ReExportForeignNames
        .check(ctx)
        .into_iter()
        .map(|e| e.message)
        .filter(|m| !m.contains("could not be read"))
        .collect()
}

fn names(src: &'static str) -> Vec<String> {
    let mut out = Vec::new();
    for message in findings(src) {
        let name = message
            .trim_start_matches('`')
            .split('`')
            .next()
            .unwrap_or("")
            .to_string();
        out.push(name);
    }
    out.sort();
    out
}

// --- the tier is per crate ---------------------------------------------------

#[test]
fn the_lint_walks_the_crate_once() {
    // It reads `all_sources` itself, so left at the engine's default it would
    // report every finding once per file.
    assert!(!ReExportForeignNames.per_file());
}

// --- parameters and returns --------------------------------------------------

#[test]
fn a_foreign_parameter_type_that_is_not_re_exported_is_reported() {
    let src = "use riimu_face::Face;\npub fn measure(face: &Face) {}\n";
    assert_eq!(names(src), ["Face"]);
}

#[test]
fn a_foreign_return_type_that_is_not_re_exported_is_reported() {
    let src = "use riimu_face::Subtable;\npub fn cmap() -> Subtable { todo!() }\n";
    assert_eq!(names(src), ["Subtable"]);
}

#[test]
fn a_re_exported_foreign_type_is_not_reported() {
    // The control for the two above, and the whole of the fix: one line.
    let src = "pub use riimu_face::Face;\npub fn measure(face: &Face) {}\n";
    assert!(findings(src).is_empty(), "{:?}", findings(src));
}

#[test]
fn a_re_export_under_another_name_counts() {
    // `Sample` is `kirjo_raster::Point` renamed on the way out, which the
    // kit's own design does; what is checked is the original, not the alias.
    let src =
        "pub use riimu_face::Point as Sample;\nuse riimu_face::Point;\npub fn at(p: Point) {}\n";
    assert!(findings(src).is_empty(), "{:?}", findings(src));
}

#[test]
fn a_re_export_in_another_file_counts() {
    // The `pub use` lives in the crate root and the signature in a module,
    // which is the ordinary shape and is why the index is crate-wide.
    let ctx = ctx_over(
        &[
            ("src/lib.rs", "pub mod glyph;\npub use riimu_face::Face;\n"),
            (
                "src/glyph.rs",
                "use riimu_face::Face;\npub fn read(face: &Face) {}\n",
            ),
        ],
        &["riimu-face"],
        &[],
    );
    let hits = tier_one(&ctx);
    assert!(hits.is_empty(), "{hits:?}");
}

#[test]
fn a_wildcard_re_export_covers_the_crate() {
    let src = "pub use riimu_face::*;\nuse riimu_face::Face;\npub fn measure(face: &Face) {}\n";
    assert!(findings(src).is_empty());
}

#[test]
fn a_qualified_path_in_a_signature_is_a_foreign_name_with_no_use_line() {
    // Nothing imports it, so the `use` map cannot say where it came from; the
    // path itself does.
    let src = "pub fn parse(bytes: &[u8]) -> riimu_face::Face<'_> { todo!() }\n";
    assert_eq!(names(src), ["Face"]);
}

#[test]
fn a_name_inside_a_generic_argument_is_reached() {
    // The wrapper and what it wraps are both names the caller writes to match
    // on the result.
    let src = "use riimu_face::{Fault, Read};\npub fn parse() -> Read<(), Fault> { todo!() }\n";
    assert_eq!(names(src), ["Fault", "Read"]);
}

#[test]
fn a_where_clause_bound_is_a_signature_position() {
    let src = "use riimu_face::Readable;\npub fn read<T>(t: T) where T: Readable {}\n";
    assert_eq!(names(src), ["Readable"]);
}

#[test]
fn an_impl_trait_bound_is_a_signature_position() {
    let src = "use riimu_face::Readable;\npub fn read(t: impl Readable) {}\n";
    assert_eq!(names(src), ["Readable"]);
}

// --- other public positions --------------------------------------------------

#[test]
fn a_public_field_of_a_foreign_type_is_reported_and_a_private_one_is_not() {
    let public = "use riimu_face::Face;\npub struct Held { pub face: Face }\n";
    assert_eq!(names(public), ["Face"]);
    let private = "use riimu_face::Face;\npub struct Held { face: Face }\n";
    assert!(findings(private).is_empty());
}

#[test]
fn a_public_tuple_field_of_a_foreign_type_is_reported() {
    let src = "use riimu_face::Face;\npub struct Held(pub Face);\n";
    assert_eq!(names(src), ["Face"]);
}

#[test]
fn an_enum_variant_field_of_a_foreign_type_is_reported() {
    // Every field of a public enum's variant is public, which is what makes a
    // variant field a position at all.
    let src = "use riimu_face::Fault;\npub enum Why { Face(Fault), Other }\n";
    assert_eq!(names(src), ["Fault"]);
}

#[test]
fn a_public_type_alias_over_a_foreign_type_is_reported() {
    let src = "use riimu_face::Face;\npub type Held<'a> = Face<'a>;\n";
    assert_eq!(names(src), ["Face"]);
}

#[test]
fn a_public_const_of_a_foreign_type_is_reported() {
    let src = "use riimu_face::Tag;\npub const CMAP: Tag = Tag::new();\n";
    assert_eq!(names(src), ["Tag"]);
}

#[test]
fn a_trait_method_signature_is_a_position() {
    let src = "use riimu_face::Face;\npub trait Reads { fn face(&self) -> &Face; }\n";
    assert_eq!(names(src), ["Face"]);
}

#[test]
fn a_supertrait_is_a_position() {
    let src = "use riimu_face::Readable;\npub trait Reads: Readable {}\n";
    assert_eq!(names(src), ["Readable"]);
}

#[test]
fn a_public_method_in_an_inherent_impl_is_a_position() {
    let src = "use riimu_face::Face;\npub struct Held;\nimpl Held { pub fn face(&self) -> Face { todo!() } }\n";
    assert_eq!(names(src), ["Face"]);
}

// --- what is not a position --------------------------------------------------

#[test]
fn a_private_function_is_not_a_position() {
    let src = "use riimu_face::Face;\nfn measure(face: &Face) {}\n";
    assert!(findings(src).is_empty());
}

#[test]
fn a_crate_visible_function_is_not_a_position() {
    let src = "use riimu_face::Face;\npub(crate) fn measure(face: &Face) {}\n";
    assert!(findings(src).is_empty());
}

#[test]
fn a_private_method_in_an_inherent_impl_is_not_a_position() {
    let src = "use riimu_face::Face;\npub struct Held;\nimpl Held { fn face(&self) -> Face { todo!() } }\n";
    assert!(findings(src).is_empty());
}

#[test]
fn a_trait_impl_method_is_the_traits_signature_and_not_this_crates() {
    let src = "use riimu_face::Face;\npub struct Held;\nimpl core::fmt::Debug for Held { fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { let _: Face; Ok(()) } }\n";
    assert!(findings(src).is_empty());
}

#[test]
fn a_test_module_is_not_a_position() {
    let src = "#[cfg(test)]\nmod tests {\n    use riimu_face::Face;\n    pub fn measure(face: &Face) {}\n}\n";
    assert!(findings(src).is_empty());
}

#[test]
fn a_generic_parameter_is_not_foreign() {
    let src = "pub fn id<T>(t: T) -> T { t }\npub struct Held<K> { pub key: K }\n";
    assert!(findings(src).is_empty());
}

#[test]
fn an_impl_generic_is_not_foreign_in_its_methods() {
    let src = "pub struct Held<K>(K);\nimpl<K> Held<K> { pub fn key(&self) -> &K { &self.0 } }\n";
    assert!(findings(src).is_empty());
}

#[test]
fn the_standard_library_is_never_foreign() {
    let src = "use core::fmt::Formatter;\nuse std::path::Path;\npub fn f(p: &Path, w: &mut Formatter<'_>) -> alloc::string::String { todo!() }\n";
    assert!(findings(src).is_empty(), "{:?}", findings(src));
}

#[test]
fn the_stacks_foundation_is_the_standard_library_here() {
    // `USize` and `Maybe` are the stack's vocabulary, which every consumer
    // depends on the way it depends on `core`, and the family prefix carries
    // `arvo_bits` with `arvo`. Run over a real kit crate before this arm, most
    // of what the lint reported was `USize`, `Bool` and `Maybe`, and none of
    // it was a leak.
    let files: &'static [(&'static str, &'static str)] = Box::leak(Box::new([(
        "src/lib.rs",
        "use arvo::USize;\nuse arvo_bits::Byte;\nuse notko::Maybe;\nuse hilavitkutin_str::Str;\npub fn f(n: USize, b: Byte, s: Str) -> Maybe<()> { todo!() }\n",
    )]));
    let ctx = ctx_over(
        files,
        &["arvo", "arvo-bits", "notko", "hilavitkutin-str", "riimu-face"],
        &[],
    );
    let hits = tier_one(&ctx);
    assert!(hits.is_empty(), "{hits:?}");
    // And the control: a crate whose name merely begins with a foundation's
    // letters is not in the family.
    let files: &'static [(&'static str, &'static str)] = Box::leak(Box::new([(
        "src/lib.rs",
        "use arvosaurus::Thing;\npub fn f(t: Thing) {}\n",
    )]));
    let ctx = ctx_over(files, &["arvosaurus"], &[]);
    assert_eq!(tier_one(&ctx).len(), 1);
}

#[test]
fn a_crate_of_the_same_workspace_is_not_foreign() {
    // A consumer names the kit's crates and nothing under them, so a sibling
    // in a signature is the kit's own name.
    let ctx = ctx_over(
        &[(
            "src/lib.rs",
            "use kit_value::Handle;\npub fn face(h: Handle) {}\n",
        )],
        &["kit-value", "riimu-face"],
        &["kit-value", "kit-text"],
    );
    let hits = ReExportForeignNames.check(&ctx);
    assert!(
        hits.is_empty(),
        "{:?}",
        hits.iter().map(|h| &h.message).collect::<Vec<_>>()
    );
}

#[test]
fn a_name_from_the_crates_own_module_is_not_foreign() {
    let src =
        "mod glyph { pub struct Glyph; }\nuse glyph::Glyph;\npub fn read() -> Glyph { Glyph }\n";
    assert!(findings(src).is_empty(), "{:?}", findings(src));
}

#[test]
fn a_local_type_with_no_use_line_is_not_foreign() {
    let src = "pub struct Glyph;\npub fn read() -> Glyph { Glyph }\n";
    assert!(findings(src).is_empty());
}

#[test]
fn a_root_that_is_not_a_declared_dependency_is_not_foreign() {
    // With the manifest's dependencies in hand, a root the manifest does not
    // name is a module this crate spelled some other way, not a crate.
    let src = "use something_else::Thing;\npub fn f(t: Thing) {}\n";
    assert!(findings(src).is_empty());
}

#[test]
fn with_no_manifest_in_hand_every_unreserved_root_is_foreign() {
    let ctx = ctx_over(
        &[(
            "src/lib.rs",
            "use something_else::Thing;\npub fn f(t: Thing) {}\n",
        )],
        &[],
        &[],
    );
    assert_eq!(ReExportForeignNames.check(&ctx).len(), 1);
}

#[test]
fn a_path_whose_root_is_an_alias_of_a_foreign_module_is_reached() {
    // `tbl` is `riimu_face::table` under another name, so `tbl::Cmap` is a
    // foreign name spelled through the alias. The first shape of the lint
    // read a qualified path's root as a crate and never asked the `use`
    // lines, so this whole spelling passed silently, which a reviewer found.
    let src = "use riimu_face::table as tbl;\npub fn cmap() -> tbl::Cmap { todo!() }\n";
    assert_eq!(names(src), ["Cmap"]);
    // And the control: the same spelling with the name re-exported.
    let fixed = "use riimu_face::table as tbl;\npub use riimu_face::table::Cmap;\npub fn cmap() -> tbl::Cmap { todo!() }\n";
    assert!(findings(fixed).is_empty(), "{:?}", findings(fixed));
}

#[test]
fn a_local_binding_of_the_same_name_in_another_module_does_not_hide_a_foreign_one() {
    // The file binds `Face` twice: to the foreign one at the top, where the
    // public signature is, and to a local one in a submodule below it. A map
    // keeping the last binding resolved the signature's `Face` as local and
    // reported nothing, which a reviewer found; every binding is kept now
    // and the foreign one is reported.
    let src = "use riimu_face::Face;\npub fn measure(face: &Face) {}\n\
               mod local {\n    pub struct Face;\n}\nmod other {\n    use super::local::Face;\n    fn f(_: Face) {}\n}\n";
    assert_eq!(names(src), ["Face"]);
}

// --- the escape hatch and the reporting ---------------------------------------

#[test]
fn an_allow_on_the_signature_line_silences_it() {
    let src = "use riimu_face::Face;\npub fn measure(face: &Face) {} // lint:allow(re-export-foreign-names) reason: boundary\n";
    assert!(findings(src).is_empty());
}

#[test]
fn one_name_is_reported_once_per_signature() {
    let src = "use riimu_face::Face;\npub fn pair(a: &Face, b: &Face) -> Face { todo!() }\n";
    assert_eq!(names(src), ["Face"]);
}

#[test]
fn the_finding_names_the_crate_the_name_came_from_and_where_it_sits() {
    let ctx = ctx_over(
        &[
            ("src/lib.rs", "pub mod glyph;\n"),
            (
                "src/glyph.rs",
                "use riimu_face::Face;\npub fn read(face: &Face) {}\n",
            ),
        ],
        &["riimu-face"],
        &[],
    );
    let hits = ReExportForeignNames.check(&ctx);
    assert_eq!(hits.len(), 1);
    let hit = &hits[0];
    assert_eq!(hit.path.as_deref(), Some("src/glyph.rs"));
    assert_eq!(hit.line, 2);
    assert!(hit.message.contains("`riimu_face`"), "{}", hit.message);
    assert!(hit.message.contains("`read`"), "{}", hit.message);
    assert_eq!(hit.lint_name, "re-export-foreign-names");
}
