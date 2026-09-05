//! Coverage test: `re-export-foreign-names`, the tier over what a re-export
//! carries with it.
//!
//! Re-exporting `Face` from a dependency hands a consumer every public method
//! on `Face`, so what `Face::cmap` returns is a name the consumer has to
//! write, and it lives in the dependency's source rather than in this crate's.
//! The case this was written from: a kit re-exported the face and not the two
//! tables the face's own accessors return, and its first real consumer could
//! not hold either without spelling the crate underneath.
//!
//! The dependency is a real crate on disk, in a temporary workspace, because
//! the tier is resolved through `cargo metadata` and a fixture that bypassed
//! it would test the walk and not the resolution.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use mockspace_extra_lints::lints::re_export_foreign_names::ReExportForeignNames;
use mockspace_lint_rules::{CrateLint, CrateSourceFile, LintContext};

/// A workspace of two crates: `kit`, which re-exports, and `face`, which
/// declares. `kit_root` is `kit/src/lib.rs`.
fn workspace(kit_root: &str, face_lib: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"kit\", \"face\"]\nresolver = \"2\"\n",
    )
    .unwrap();
    for (name, lib, deps) in
        [("face", face_lib, ""), ("kit", kit_root, "face = { path = \"../face\" }\n")]
    {
        let crate_dir = root.join(name);
        std::fs::create_dir_all(crate_dir.join("src")).unwrap();
        std::fs::write(
            crate_dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[dependencies]\n{deps}"
            ),
        )
        .unwrap();
        std::fs::write(crate_dir.join("src/lib.rs"), lib).unwrap();
    }
    dir
}

fn check(root: &Path, kit_root: &'static str) -> Vec<String> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(kit_root, None).unwrap();
    let tree: &'static tree_sitter::Tree = Box::leak(Box::new(tree));
    let all_sources: &'static [CrateSourceFile] = Box::leak(Box::new(vec![CrateSourceFile {
        rel_path: std::path::PathBuf::from("src/lib.rs"),
        text:     kit_root.to_string(),
    }]));
    let deps: &'static [String] = Box::leak(Box::new(vec!["face".to_string()]));
    let ctx = LintContext {
        crate_name: "kit",
        short_name: "kit",
        source: kit_root,
        tree,
        all_sources,
        deps,
        all_crates: Box::leak(Box::new(BTreeSet::new())),
        design_doc: None,
        all_doc_content: "",
        shame_doc: None,
        workspace_root: Box::leak(root.to_path_buf().into_boxed_path()),
        proc_macro_crates: &[],
        lint_proc_macro_source: false,
        crate_prefix: "kit",
        primitive_introductions: Box::leak(Box::new(BTreeMap::new())),
    };
    ReExportForeignNames
        .check(&ctx)
        .into_iter()
        .map(|e| e.message)
        .collect()
}

/// The dependency as the case was found: a face whose accessors return two
/// tables it declares.
const FACE: &str = "pub struct Subtable;\npub struct Hmtx;\npub struct Fault;\n\
pub struct Face<'a> { bytes: &'a [u8] }\n\
impl<'a> Face<'a> {\n    pub fn cmap(&self) -> Subtable { Subtable }\n    pub fn hmtx(&self) -> Hmtx { Hmtx }\n    fn private(&self) -> Fault { Fault }\n}\n";

#[test]
fn a_re_exported_type_whose_methods_name_two_unexported_types_reports_both() {
    let kit = "pub use face::Face;\n";
    let ws = workspace(kit, FACE);
    let mut hits = check(ws.path(), kit);
    hits.sort();
    assert_eq!(hits.len(), 2, "{hits:?}");
    assert!(
        hits[0].contains("`Hmtx`") && hits[0].contains("`hmtx`"),
        "{}",
        hits[0]
    );
    assert!(
        hits[1].contains("`Subtable`") && hits[1].contains("`cmap`"),
        "{}",
        hits[1]
    );
    for hit in &hits {
        assert!(hit.contains("`Face`") && hit.contains("`face`"), "{hit}");
    }
}

#[test]
fn re_exporting_the_tables_beside_the_face_is_the_fix() {
    // The control, and the two-name change the case was closed with.
    let kit = "pub use face::{Face, Hmtx, Subtable};\n";
    let ws = workspace(kit, FACE);
    let hits = check(ws.path(), kit);
    assert!(hits.is_empty(), "{hits:?}");
}

#[test]
fn a_private_method_carries_nothing() {
    // `Face::private` returns `Fault`, and nobody outside the dependency can
    // call it, so `Fault` is not a name a consumer has to write.
    let kit = "pub use face::{Face, Hmtx, Subtable};\n";
    let ws = workspace(kit, FACE);
    let hits = check(ws.path(), kit);
    assert!(!hits.iter().any(|h| h.contains("`Fault`")), "{hits:?}");
}

#[test]
fn a_public_field_of_a_re_exported_struct_is_part_of_its_surface() {
    // Two fields of two types, one public and one not, so the private one
    // has a name of its own to be absent. With both fields typed alike the
    // case passed with the visibility check deleted outright, which a
    // reviewer established by mutation; the dedupe on the name hid it.
    let face = "pub struct Units;\npub struct Hidden;\npub struct Metrics { pub ascent: Units, descent: Hidden }\n";
    let kit = "pub use face::Metrics;\n";
    let ws = workspace(kit, face);
    let hits = check(ws.path(), face_kit(kit));
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(
        hits[0].contains("`Units`") && hits[0].contains("field"),
        "{}",
        hits[0]
    );
    assert!(
        !hits.iter().any(|h| h.contains("`Hidden`")),
        "a private field is not part of the surface: {hits:?}"
    );
}

#[test]
fn a_re_exported_trait_carries_its_method_signatures() {
    let face = "pub struct Glyph;\npub trait Shapes { fn next(&mut self) -> Glyph; }\n";
    let kit = "pub use face::Shapes;\n";
    let ws = workspace(kit, face);
    let hits = check(ws.path(), face_kit(kit));
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(
        hits[0].contains("`Glyph`") && hits[0].contains("`next`"),
        "{}",
        hits[0]
    );
}

#[test]
fn a_re_exported_alias_carries_what_it_stands_for() {
    let face = "pub struct Wide;\npub type Key = Wide;\n";
    let kit = "pub use face::Key;\n";
    let ws = workspace(kit, face);
    let hits = check(ws.path(), face_kit(kit));
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(hits[0].contains("`Wide`"), "{}", hits[0]);
}

#[test]
fn a_name_the_dependency_took_from_its_own_dependency_is_not_reached() {
    // `Outcome` is one crate further down than this reads, and the walk says
    // so by leaving it out rather than by reporting it against the wrong crate.
    let face =
        "pub struct Face;\nimpl Face { pub fn parse() -> notko::Outcome<Face, ()> { todo!() } }\n";
    let kit = "pub use face::Face;\n";
    let ws = workspace(kit, face);
    let hits = check(ws.path(), face_kit(kit));
    assert!(hits.is_empty(), "{hits:?}");
}

#[test]
#[ignore = "catalogue: a name the dependency itself took from one further down is not resolved; \
            a consumer of `Face::parse` still has to write `Outcome`"]
fn a_name_two_crates_down_is_reported_against_the_crate_that_declares_it() {
    let face =
        "pub struct Face;\nimpl Face { pub fn parse() -> notko::Outcome<Face, ()> { todo!() } }\n";
    let kit = "pub use face::Face;\n";
    let ws = workspace(kit, face);
    let hits = check(ws.path(), face_kit(kit));
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(hits[0].contains("`Outcome`") && hits[0].contains("`notko`"));
}

#[test]
fn a_generic_of_the_impl_is_not_part_of_the_surface() {
    let face =
        "pub struct Sheet<K>(K);\nimpl<K> Sheet<K> { pub fn key(&self) -> &K { &self.0 } }\n";
    let kit = "pub use face::Sheet;\n";
    let ws = workspace(kit, face);
    let hits = check(ws.path(), face_kit(kit));
    assert!(hits.is_empty(), "{hits:?}");
}

#[test]
fn a_re_exported_function_carries_no_surface() {
    // A function's own signature is what a consumer writes, and a re-exported
    // one is checked by nothing here: the finding it would want is against
    // the dependency's signature, which is the dependency's own lint to run.
    let face = "pub struct Placed;\npub fn shape() -> Placed { Placed }\n";
    let kit = "pub use face::shape;\n";
    let ws = workspace(kit, face);
    let hits = check(ws.path(), face_kit(kit));
    assert!(hits.is_empty(), "{hits:?}");
}

#[test]
fn an_allow_on_the_re_export_line_silences_its_surface() {
    let kit = "pub use face::Face; // lint:allow(re-export-foreign-names) reason: measured leak\n";
    let ws = workspace(kit, FACE);
    let hits = check(ws.path(), face_kit(kit));
    assert!(hits.is_empty(), "{hits:?}");
}

#[test]
fn a_workspace_that_cannot_be_read_is_reported_rather_than_passed() {
    // The promise a re-export makes cannot be checked without the dependency's
    // source, and a check that cannot run says so on the `pub use` rather than
    // reading as verified.
    let kit = "pub use face::Face;\n";
    let dir = tempfile::tempdir().unwrap();
    let hits = check(dir.path(), face_kit(kit));
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(hits[0].contains("could not be read"), "{}", hits[0]);
}

#[test]
#[ignore = "catalogue: an associated type bound to a foreign type in a trait impl is not reached; \
            a consumer of the impl's method still writes the type"]
fn an_associated_type_in_a_trait_impl_is_part_of_the_surface() {
    // `Held::go` answers `<Held as Reads>::Out`, which is `Face`, and the
    // module doc's line that a trait impl is the trait's signature leaves the
    // binding unreported. The consumer writes `Face` all the same.
    let face = "pub struct Face;\npub trait Reads { type Out; fn go(&self) -> Self::Out; }\n\
                pub struct Held;\nimpl Reads for Held { type Out = Face; fn go(&self) -> Face { Face } }\n";
    let kit = "pub use face::{Held, Reads};\n";
    let ws = workspace(kit, face);
    let hits = check(ws.path(), face_kit(kit));
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(hits[0].contains("`Face`"), "{}", hits[0]);
}

/// The kit root as a `'static` string, which the context borrows for the life
/// of the test.
fn face_kit(kit: &str) -> &'static str {
    Box::leak(kit.to_string().into_boxed_str())
}
