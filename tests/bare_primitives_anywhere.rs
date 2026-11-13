//! Coverage test: the tightened `no-bare-*` and `arvo-types-only`
//! lints fire on drift ANYWHERE in source, not just public fn
//! signatures.
//!
//! Every case here captures a real leak in the pre-tightening lints:
//! private fields, tuple-struct fields, let bindings, cast
//! expressions, literal suffixes, trait method signatures, const
//! declarations. All must produce at least one violation per the
//! rule "primitives don't exist in this stack".

use mockspace_extra_lints::lints::arvo_types_only::ArvoTypesOnly;
use mockspace_extra_lints::lints::no_bare_numeric::NoBareNumeric;
use mockspace_extra_lints::lints::no_bare_option::NoBareOption;
use mockspace_extra_lints::lints::no_bare_result::NoBareResult;
use mockspace_extra_lints::lints::no_bare_string::NoBareString;
use mockspace_extra_lints::lints::no_public_raw_field::NoPublicRawField;
use mockspace_lint_rules::CrateLint;
use mockspace_lint_rules::testkit::LintFixture;

/// An ordinary crate of one file. Source lints skip proc-macro crates by default, because a proc
/// macro's own heap-using parser never ships in a consumer binary, and these fixtures are not
/// proc-macro crates, so the default is the honest setting.
fn ctx_with(source: &str) -> LintFixture {
    LintFixture::new(source).with_workspace_root(std::path::Path::new("/tmp"))
}

// ---- arvo-types-only ------------------------------------------------------

#[test]
fn arvo_types_only_fires_on_private_tuple_struct_field() {
    let src = "pub struct Handle(u32);\n";
    let hits = ArvoTypesOnly.check(&ctx_with(src).ctx());
    assert!(!hits.is_empty(), "private tuple-struct u32 must be flagged");
}

#[test]
fn arvo_types_only_fires_on_private_named_field() {
    let src = "pub struct Cache { count: usize }\n";
    let hits = ArvoTypesOnly.check(&ctx_with(src).ctx());
    assert!(
        !hits.is_empty(),
        "private named usize field must be flagged"
    );
}

#[test]
fn arvo_types_only_fires_on_cast_expression() {
    let src = "fn inside() { let _ = 0_i64 as u32; }\n";
    let hits = ArvoTypesOnly.check(&ctx_with(src).ctx());
    assert!(!hits.is_empty(), "`as u32` cast must be flagged");
}

#[test]
fn arvo_types_only_fires_on_a_const_binding_beside_a_literal_suffix() {
    // Named for what it establishes. It used to be named for the literal suffix
    // and passed on the `usize` binding on the same line; the suffix itself goes
    // unreported, and the arm that says so is catalogued red in
    // `the_const_generic_parameter_is_excepted.rs`.
    let src = "const X: usize = 0u32 as usize;\n";
    let hits = ArvoTypesOnly.check(&ctx_with(src).ctx());
    assert!(!hits.is_empty(), "the bare `usize` binding must be flagged");

    let suffix_alone = "fn f() { let _ = 0u32; }\n";
    assert!(
        ArvoTypesOnly.check(&ctx_with(suffix_alone).ctx()).is_empty(),
        "pinning the gap rather than hiding it: a suffix on its own is missed",
    );
}

#[test]
fn arvo_types_only_fires_on_trait_method_param() {
    let src = "pub trait Foo { fn bar(&self, x: bool); }\n";
    let hits = ArvoTypesOnly.check(&ctx_with(src).ctx());
    assert!(
        !hits.is_empty(),
        "trait method bare `bool` param must be flagged"
    );
}

#[test]
fn arvo_types_only_fires_on_array_element_type() {
    let src = "const BUF: [u8; 16] = [0; 16];\n";
    let hits = ArvoTypesOnly.check(&ctx_with(src).ctx());
    assert!(!hits.is_empty(), "array element type `u8` must be flagged");
}

#[test]
fn arvo_types_only_accepts_allow_comment() {
    let src = "pub struct Handle(u32); // lint:allow(arvo-types-only) reason: rkyv Archived; tracked: #72\n";
    let hits = ArvoTypesOnly.check(&ctx_with(src).ctx());
    assert!(hits.is_empty(), "lint:allow(arvo-types-only) must silence");
}

#[test]
fn arvo_types_only_ignores_inside_string_literal() {
    let src = "pub const MSG: &str = \"u32 is forbidden\";\n";
    let hits = ArvoTypesOnly.check(&ctx_with(src).ctx());
    // MSG: &str still violates no-bare-string, but arvo-types-only
    // must NOT false-positive on the "u32" token inside the literal.
    assert!(
        hits.iter().all(|e| !e.message.contains("u32")),
        "u32 inside string literal must not be reported",
    );
}

// ---- no-bare-numeric ------------------------------------------------------

#[test]
fn no_bare_numeric_fires_on_private_fn_param() {
    let src = "fn helper(n: usize) -> usize { n }\n";
    let hits = NoBareNumeric.check(&ctx_with(src).ctx());
    assert!(
        !hits.is_empty(),
        "private fn signature usize must be flagged"
    );
}

#[test]
fn no_bare_numeric_fires_on_let_binding() {
    let src = "pub fn f() { let _x: u64 = 0u64; }\n";
    let hits = NoBareNumeric.check(&ctx_with(src).ctx());
    assert!(!hits.is_empty(), "local let binding u64 must be flagged");
}

// ---- no-bare-option -------------------------------------------------------

#[test]
fn no_bare_option_fires_on_private_fn_return() {
    let src = "fn lookup() -> Option<u8> { None }\n";
    let hits = NoBareOption.check(&ctx_with(src).ctx());
    assert!(!hits.is_empty(), "private fn Option return must be flagged");
}

#[test]
fn no_bare_option_fires_on_struct_field() {
    let src = "pub struct Cached { handle: Option<u32> }\n";
    let hits = NoBareOption.check(&ctx_with(src).ctx());
    assert!(!hits.is_empty(), "struct field Option must be flagged");
}

#[test]
fn no_bare_option_accepts_allow_comment() {
    let src = "fn iter() -> Option<u8> { None } // lint:allow(no-bare-option) reason: core::Iterator parity; tracked: #115\n";
    let hits = NoBareOption.check(&ctx_with(src).ctx());
    assert!(hits.is_empty(), "lint:allow must silence no-bare-option");
}

// ---- no-bare-result -------------------------------------------------------

#[test]
fn no_bare_result_fires_on_private_fn_return() {
    let src = "fn load() -> Result<u8, ()> { Ok(0) }\n";
    let hits = NoBareResult.check(&ctx_with(src).ctx());
    assert!(!hits.is_empty(), "private fn Result return must be flagged");
}

#[test]
fn no_bare_result_accepts_fmt_result() {
    let src = "pub fn fmt(f: &mut core::fmt::Formatter) -> fmt::Result { Ok(()) }\n";
    let hits = NoBareResult.check(&ctx_with(src).ctx());
    assert!(hits.is_empty(), "fmt::Result (std trait parity) must pass");
}

#[test]
fn no_bare_result_accepts_io_result() {
    let src = "pub fn open() -> io::Result<()> { Ok(()) }\n";
    let hits = NoBareResult.check(&ctx_with(src).ctx());
    assert!(hits.is_empty(), "io::Result (std trait parity) must pass");
}

// ---- no-bare-string -------------------------------------------------------

#[test]
fn no_bare_string_fires_on_private_field() {
    let src = "struct Msg { body: String }\n";
    let hits = NoBareString.check(&ctx_with(src).ctx());
    assert!(!hits.is_empty(), "private String field must be flagged");
}

#[test]
fn no_bare_string_fires_on_private_fn_param() {
    let src = "fn shout(s: &str) {}\n";
    let hits = NoBareString.check(&ctx_with(src).ctx());
    assert!(
        !hits.is_empty(),
        "private fn non-static &str param must be flagged"
    );
}

#[test]
fn no_bare_string_accepts_static_str() {
    let src = "pub fn label() -> &'static str { \"ok\" }\n";
    let hits = NoBareString.check(&ctx_with(src).ctx());
    assert!(hits.is_empty(), "&'static str must pass");
}

// ---- no-public-raw-field (now universal) ----------------------------------

#[test]
fn no_public_raw_field_fires_on_private_field() {
    let src = "pub struct Foo { inner: u64 }\n";
    let hits = NoPublicRawField.check(&ctx_with(src).ctx());
    assert!(
        !hits.is_empty(),
        "private u64 field of pub struct must be flagged"
    );
}

#[test]
fn no_public_raw_field_fires_on_private_struct() {
    let src = "struct Internal { count: usize }\n";
    let hits = NoPublicRawField.check(&ctx_with(src).ctx());
    assert!(
        !hits.is_empty(),
        "private struct with raw field must be flagged"
    );
}

#[test]
fn no_public_raw_field_fires_on_tuple_struct() {
    let src = "pub struct Handle(u32);\n";
    let hits = NoPublicRawField.check(&ctx_with(src).ctx());
    assert!(
        !hits.is_empty(),
        "tuple-struct raw u32 field must be flagged"
    );
}

#[test]
fn no_public_raw_field_accepts_allow_comment() {
    let src = "pub struct H { v: u32 } // lint:allow(no-public-raw-field) reason: rkyv Archived; tracked: #72\n";
    let hits = NoPublicRawField.check(&ctx_with(src).ctx());
    assert!(
        hits.is_empty(),
        "lint:allow must silence no-public-raw-field"
    );
}

// ---- multi-file coverage (LintContext.all_sources) ----------------------
//
// Previously every per-crate lint scanned only src/lib.rs. These tests
// exercise drift living in a module file (src/bits.rs), which old lint
// scopes missed silently.

fn ctx_with_files(files: Vec<(&str, &str)>) -> LintFixture {
    let lib_source = files
        .iter()
        .find(|(p, _)| *p == "src/lib.rs")
        .map(|(_, t)| *t)
        .unwrap_or("");
    LintFixture::new(lib_source)
        .with_sources(&files)
        .with_workspace_root(std::path::Path::new("/tmp"))
}

#[test]
fn arvo_types_only_fires_in_module_file_not_lib_rs() {
    let ctx = ctx_with_files(vec![
        ("src/lib.rs", "pub mod bits;\n"),
        ("src/bits.rs", "pub struct Bits(u64);\n"),
    ]);
    let hits = ArvoTypesOnly.check(&ctx.ctx());
    assert!(!hits.is_empty(), "drift in src/bits.rs must be flagged");
    assert!(
        hits.iter().any(|e| e.path.as_deref() == Some("src/bits.rs")),
        "the finding should carry the offending file as its path"
    );
}

#[test]
fn no_bare_option_fires_in_module_file_not_lib_rs() {
    let ctx = ctx_with_files(vec![
        ("src/lib.rs", "pub mod resolve;\n"),
        ("src/resolve.rs", "fn lookup() -> Option<u8> { None }\n"),
    ]);
    let hits = NoBareOption.check(&ctx.ctx());
    assert!(!hits.is_empty(), "Option in src/resolve.rs must be flagged");
    assert!(
        hits.iter().any(|e| e.path.as_deref() == Some("src/resolve.rs")),
        "the finding should carry the offending file as its path"
    );
}

// ---- primitive-introductions (per-crate per-primitive skip) -------------

fn ctx_with_crate_and_introductions(
    crate_name: &str,
    introductions: Vec<(&str, Vec<&str>)>,
    source: &str,
) -> LintFixture {
    let introductions_map: std::collections::BTreeMap<String, Vec<String>> = introductions
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.into_iter().map(|s| s.to_string()).collect()))
        .collect();
    LintFixture::new(source)
        .with_crate_name(crate_name, crate_name)
        .with_workspace_root(std::path::Path::new("/tmp"))
        .with_primitive_introductions(introductions_map)
}

#[test]
fn arvo_types_only_skips_numeric_introducer() {
    // arvo is tagged as introducing the numeric category; the lint
    // self-exempts for this crate regardless of internal impl.
    let ctx = ctx_with_crate_and_introductions(
        "arvo",
        vec![("arvo", vec!["numeric"])],
        "pub struct USize(pub usize);\n",
    );
    let hits = ArvoTypesOnly.check(&ctx.ctx());
    assert!(
        hits.is_empty(),
        "arvo introduces numeric; lint must skip entirely on arvo"
    );
}

#[test]
fn arvo_types_only_fires_on_non_introducer_crate() {
    // hilavitkutin is NOT an introducer of the numeric category.
    // Even though arvo is configured, hilavitkutin's bare usize is drift.
    let ctx = ctx_with_crate_and_introductions(
        "hilavitkutin",
        vec![("arvo", vec!["numeric"])],
        "pub struct Counter(pub usize);\n",
    );
    let hits = ArvoTypesOnly.check(&ctx.ctx());
    assert!(
        !hits.is_empty(),
        "hilavitkutin doesn't introduce numeric; lint must fire"
    );
}

#[test]
fn arvo_types_only_ignores_unknown_category() {
    // Adding a string that isn't a known category (e.g. a raw std
    // primitive name) has no effect; the lint only matches known
    // categories. This is the anti-bypass property: you can't just
    // add "u32" to your list to silence the lint.
    let ctx = ctx_with_crate_and_introductions(
        "arvo",
        vec![("arvo", vec!["u32", "Option", "String"])],
        "pub fn width() -> u32 { 0 }\n",
    );
    let hits = ArvoTypesOnly.check(&ctx.ctx());
    assert!(
        !hits.is_empty(),
        "unknown tokens in introductions list must not bypass the lint"
    );
}

#[test]
fn no_bare_option_skips_fallibility_introducer() {
    // notko introduces the fallibility category; bare Option in its source
    // is acknowledged as part of defining Maybe.
    let ctx = ctx_with_crate_and_introductions(
        "notko",
        vec![("notko", vec!["fallibility"])],
        "impl<T> From<Option<T>> for Maybe<T> { fn from(_: Option<T>) -> Self { Self::Isnt } }\n",
    );
    let hits = NoBareOption.check(&ctx.ctx());
    assert!(
        hits.is_empty(),
        "notko introduces fallibility; no-bare-option must skip"
    );
}

#[test]
fn no_bare_option_fires_on_non_fallibility_crate() {
    // hilavitkutin consumes fallibility but doesn't introduce it.
    let ctx = ctx_with_crate_and_introductions(
        "hilavitkutin",
        vec![("notko", vec!["fallibility"])],
        "pub fn lookup() -> Option<u8> { None }\n",
    );
    let hits = NoBareOption.check(&ctx.ctx());
    assert!(
        !hits.is_empty(),
        "hilavitkutin doesn't introduce fallibility; no-bare-option must fire"
    );
}

#[test]
fn no_public_raw_field_numeric_introducer_still_flags_string_fields() {
    // Numeric introducer gets numeric field types exempted, but a
    // bare `String` field still fires; the crate doesn't introduce
    // the string category.
    let ctx = ctx_with_crate_and_introductions(
        "arvo",
        vec![("arvo", vec!["numeric"])],
        "pub struct Msg { body: String }\n",
    );
    let hits = NoPublicRawField.check(&ctx.ctx());
    assert!(
        !hits.is_empty(),
        "`String` field in numeric-only crate must still fire"
    );
    assert!(
        hits.iter().any(|e| e.message.contains("`String`")),
        "error should name the String type"
    );
}

#[test]
fn no_public_raw_field_skips_numeric_introducer() {
    // arvo-bits introduces the numeric category at L1 (opaque-bit
    // containers). Its u64 field in Bits<N>(u64) is legitimate.
    let ctx = ctx_with_crate_and_introductions(
        "arvo-bits",
        vec![("arvo-bits", vec!["numeric"])],
        "pub struct Bits<const N: u8>(u64);\n",
    );
    let hits = NoPublicRawField.check(&ctx.ctx());
    assert!(
        hits.is_empty(),
        "arvo-bits introduces numeric; no-public-raw-field must skip"
    );
}

#[test]
fn no_public_raw_field_fires_in_module_file_not_lib_rs() {
    let ctx = ctx_with_files(vec![
        ("src/lib.rs", "pub mod handle;\n"),
        ("src/handle.rs", "pub struct Str(pub u32);\n"),
    ]);
    let hits = NoPublicRawField.check(&ctx.ctx());
    assert!(
        !hits.is_empty(),
        "tuple-struct raw u32 in src/handle.rs must be flagged"
    );
    assert!(
        hits.iter().any(|e| e.path.as_deref() == Some("src/handle.rs")),
        "the finding should carry the offending file as its path"
    );
}

/// The finding carries its file in `path` and its line in `line`, so the
/// message repeats neither.
#[test]
fn a_bare_primitive_message_does_not_repeat_its_path() {
    let files = |module: &str, text: &str| {
        ctx_with_files(vec![("src/lib.rs", "pub mod m;\n"), (module, text)])
    };
    let cases: Vec<(&str, Vec<mockspace_lint_rules::LintError>)> = vec![
        ("arvo-types-only", ArvoTypesOnly.check(&files("src/m.rs", "pub struct B(u64);\n").ctx())),
        ("no-bare-option", NoBareOption.check(&files("src/m.rs", "fn f() -> Option<u8> { None }\n").ctx())),
        ("no-bare-result", NoBareResult.check(&files("src/m.rs", "fn f() -> Result<u8, u8> { Ok(1) }\n").ctx())),
        ("no-bare-string", NoBareString.check(&files("src/m.rs", "fn f(s: String) {}\n").ctx())),
        ("no-public-raw-field", NoPublicRawField.check(&files("src/m.rs", "pub struct S(pub u32);\n").ctx())),
    ];
    for (lint, hits) in cases {
        assert!(!hits.is_empty(), "{lint} must flag src/m.rs");
        for e in &hits {
            assert_eq!(e.path.as_deref(), Some("src/m.rs"), "{lint}");
            assert!(!e.message.contains("src/m.rs"), "{lint}: {}", e.message);
            assert!(!e.message.contains(" line "), "{lint}: {}", e.message);
        }
    }
}
