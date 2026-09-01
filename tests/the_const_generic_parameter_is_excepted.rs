//! The one excepted position, and the whole boundary around it.
//!
//! The bare-primitive rule is that the machine's own types do not exist at an
//! API position. A const generic parameter is excepted, and inside the exception
//! the bare form is used only where the alternative is genuinely painful. The
//! second half is a judgement and stays with a reviewer; the first half is a
//! position and belongs here.
//!
//! Every silent arm below is paired with a firing one, because an arm that only
//! checks the permitted case is satisfied by a lint that fires on nothing. The
//! mixed cases are the load-bearing ones: a parameter and an ordinary type on
//! the same line, where exactly one of the two has to report.

use std::collections::{BTreeMap, BTreeSet};

use mockspace_extra_lints::lints::{
    arvo_types_only::ArvoTypesOnly, no_bare_numeric::NoBareNumeric,
    no_public_raw_field::NoPublicRawField,
};
use mockspace_lint_rules::{CrateLint, CrateSourceFile, LintContext, LintError};

fn ctx(source: &'static str) -> LintContext<'static> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .unwrap();
    let tree = parser.parse(source, None).unwrap();
    let tree: &'static tree_sitter::Tree = Box::leak(Box::new(tree));

    let all_sources: &'static [CrateSourceFile] = Box::leak(Box::new(vec![CrateSourceFile {
        rel_path: std::path::PathBuf::from("src/lib.rs"),
        text: source.to_string(),
    }]));

    LintContext {
        crate_name: "a-consumer",
        short_name: "a-consumer",
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
        crate_prefix: "a",
        primitive_introductions: Box::leak(Box::new(BTreeMap::new())),
    }
}

/// Both lints, because the pack's own documentation says they are semantically
/// equivalent today and a change that moves one and not the other breaks that.
fn hits(source: &'static str) -> Vec<LintError> {
    let context = ctx(source);
    let numeric = NoBareNumeric.check(&context);
    let types_only = ArvoTypesOnly.check(&context);
    assert_eq!(
        numeric.iter().map(|e| e.line).collect::<Vec<_>>(),
        types_only.iter().map(|e| e.line).collect::<Vec<_>>(),
        "the two lints have to agree line for line on {source:?}",
    );
    types_only
}

fn lines(source: &'static str) -> Vec<usize> {
    hits(source).into_iter().map(|e| e.line).collect()
}

// ---- the excepted position, silent ---------------------------------------

#[test]
fn a_const_generic_parameter_on_a_struct_passes() {
    assert!(lines("pub struct Signed<const BITS: u32>;\n").is_empty());
}

#[test]
fn a_const_generic_parameter_on_an_impl_header_passes() {
    assert!(lines(
        "impl<const BITS: u32, const FRAC: i32> Format for UFixed<BITS, FRAC> {}\n"
    )
    .is_empty());
}

#[test]
fn a_const_generic_parameter_on_a_function_passes() {
    assert!(lines("pub fn widen<const N: usize>() {}\n").is_empty());
}

#[test]
fn the_platform_sized_type_op_names_passes_in_a_parameter() {
    // `usize` is the only type his own words name, and the registry states the
    // exception over the position rather than over one type. Both spellings of
    // the claim have to hold here.
    assert!(lines("pub struct Buf<const CAP: usize>;\n").is_empty());
}

#[test]
fn a_boolean_const_generic_parameter_passes() {
    assert!(lines("pub struct Guarded<const CHECKED: bool>;\n").is_empty());
}

#[test]
fn a_parameter_with_a_default_passes() {
    assert!(lines("pub struct Buf<const CAP: usize = 8>;\n").is_empty());
}

// ---- everything else, still refused --------------------------------------
//
// Each of these is what makes a silent arm above mean something. Delete the
// carve-out and they all still pass; break the carve-out into blanking whole
// lines and every one of them goes green wrongly.

#[test]
fn an_associated_constant_still_reports() {
    assert_eq!(lines("pub trait Format { const PHASE_NUM: i64; }\n"), vec![1]);
}

#[test]
fn an_item_constant_still_reports() {
    assert_eq!(lines("const WIDTH: u32 = 32;\n"), vec![1]);
}

#[test]
fn a_tuple_struct_field_still_reports() {
    assert_eq!(lines("pub struct Handle(u32);\n"), vec![1]);
}

#[test]
fn a_named_field_still_reports() {
    assert_eq!(lines("pub struct Cache { count: usize }\n"), vec![1]);
}

#[test]
fn a_cast_still_reports() {
    assert_eq!(lines("fn f() { let _ = 0i64 as u32; }\n"), vec![1]);
}

#[test]
#[ignore = "catalogue: a literal suffix is not detected at all. `contains_bare_word` \
            wants a non-identifier byte on each side and a suffix always has a digit or \
            an underscore before it, so `0u32`, `1_usize` and `0.0_f32` all pass. Both \
            lints' module docs claimed the case was covered and the test that named it \
            passed on a different token on the same line. Closing it wants the parse \
            rather than the line, which is a separate change from this one"]
fn a_literal_suffix_still_reports() {
    assert_eq!(lines("fn f() { let _ = 0u32; }\n"), vec![1]);
}

#[test]
fn a_literal_suffix_is_not_what_the_exception_let_through() {
    // The control on the catalogued arm above, and the reason it is catalogued
    // rather than blamed on this change: the suffix went unreported before the
    // exception existed too, because the exception blanks nothing on this line.
    let source = "fn f() { let _ = 0u32; }\n";
    assert!(
        mockspace_extra_lints_probe::has_no_const_parameter(source),
        "nothing here is an excepted position, so the miss is the scanner's",
    );
}

mod mockspace_extra_lints_probe {
    /// Whether the source declares no const generic parameter at all.
    ///
    /// The pack keeps its span helper private, so this reproduces the one
    /// question this file needs of it: was there anything here to except.
    pub fn has_no_const_parameter(source: &str) -> bool {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .unwrap();
        let tree = parser.parse(source, None).unwrap();
        !names_a_const_parameter(tree.root_node())
    }

    fn names_a_const_parameter(node: tree_sitter::Node) -> bool {
        let mut cursor = node.walk();
        let children: Vec<tree_sitter::Node> = node.children(&mut cursor).collect();
        children
            .into_iter()
            .any(|child| child.kind() == "const_parameter" || names_a_const_parameter(child))
    }
}

#[test]
fn an_ordinary_generic_bound_still_reports() {
    assert_eq!(lines("fn f<T: Into<u32>>(_: T) {}\n"), vec![1]);
}

#[test]
fn a_const_generic_argument_at_a_use_site_carries_nothing_to_except() {
    // Nothing to report and nothing excepted: the argument is a value, so the
    // arm is here to pin that the exception is not doing the work.
    assert!(lines("pub type Byte = Signed<8>;\n").is_empty());
    assert_eq!(lines("pub type Byte = Signed<8>;\npub fn n() -> u32 { 0 }\n"), vec![2]);
}

// ---- the mixed line, which is where a blunt carve-out would fail ----------

#[test]
fn a_parameter_and_an_ordinary_parameter_on_one_line_reports_once() {
    assert_eq!(lines("pub fn go<const N: usize>(x: u32) {}\n"), vec![1]);
}

#[test]
fn a_parameter_and_a_return_type_on_one_line_reports_once() {
    assert_eq!(lines("pub fn go<const N: usize>() -> i64 { 0 }\n"), vec![1]);
}

#[test]
fn a_where_clause_naming_the_same_type_still_reports() {
    assert_eq!(
        lines("impl<const B: u32> T for S<B> where u32: Copy {}\n"),
        vec![1],
        "only the parameter's own type is excepted, not every mention of it"
    );
}

#[test]
fn an_impl_body_under_an_excepted_header_still_reports() {
    let source = "impl<const N: usize> Thing<N> {\n    fn raw(&self) -> u64 { 0 }\n}\n";
    assert_eq!(lines(source), vec![2]);
}

// ---- the line the report names is the line in the file --------------------

#[test]
fn blanking_does_not_shift_the_reported_line() {
    let source = concat!(
        "pub struct A<const N: u32>;\n",  // 1, excepted
        "\n",                             // 2
        "pub struct B<const M: usize>;\n", // 3, excepted
        "pub struct C(u64);\n",           // 4, reported
    );
    assert_eq!(lines(source), vec![4]);
}

#[test]
fn every_reported_line_is_reported_once() {
    let source = concat!(
        "pub struct A<const N: u32>(u64);\n", // 1: parameter excepted, field reported
        "const B: usize = 0;\n",              // 2
    );
    assert_eq!(lines(source), vec![1, 2]);
}

// ---- the allow marker is unchanged ---------------------------------------

#[test]
fn the_allow_marker_still_silences_a_refused_position() {
    assert!(lines(
        "pub struct Handle(u32); // lint:allow(arvo-types-only, no-bare-numeric) reason: probe; tracked: #1\n"
    )
    .is_empty());
}

// ---- the neighbouring lint is not loosened -------------------------------

#[test]
fn the_raw_field_lint_never_read_a_parameter_as_a_field() {
    // A guard rather than a change: `no-public-raw-field` walks fields, so an
    // excepted parameter was never its business and must stay that way.
    let quiet = NoPublicRawField.check(&ctx("pub struct A<const N: u32>(Width);\n"));
    assert!(quiet.is_empty(), "a parameter is not a field: {quiet:?}");

    let loud = NoPublicRawField.check(&ctx("pub struct A<const N: u32>(u64);\n"));
    assert!(!loud.is_empty(), "the field beside it is still a field");
}

// ---- the surface a consumer of arvo-format actually writes ----------------
//
// The shape is arvo's own, from `mock/crates/arvo-format/src/{format,ambient,
// quantum,slots,adapt}.rs` and its `points` module, reduced to the declarations
// an outside crate has to reproduce to implement the traits. arvo-format itself
// is exempt from these lints by `[primitive-introductions]`; nothing outside it
// is, so this is what the rule costs a consumer.
//
// Six of the sixteen positions are const generic parameters, spread over five
// declaration lines because `Indexed` carries two, and all six pass. The other
// ten are associated constants, which the exception does not reach, and every
// one of them reports. That split is the finding, and it is here rather than in
// prose so it cannot drift.
//
// Ten rather than nine, and sixteen rather than fifteen: `Operation::ARITY` was
// missed by the count this started from, and a fixture that models ten as nine
// is a law asserted over a sample.

const THE_FORMAT_SURFACE: &str = concat!(
    "pub struct Constant<const EXP: i32>;\n",                                     //  1 parameter
    "pub struct Indexed<const MIN_EXP: i32, const COUNT: u32>;\n",                 //  2 parameters
    "pub struct Signed<const BITS: u32>;\n",                                       //  1 parameter
    "pub struct Unsigned<const BITS: u32>;\n",                                     //  1 parameter
    "pub struct Integer<const BITS: u32>;\n",                                      //  1 parameter
    "pub trait Ambient {\n",
    "    const RADIX: u32;\n",                                                     //  reported
    "    const SIGNED: bool;\n",                                                   //  reported
    "}\n",
    "pub trait Quantum {\n",
    "    const BASE: i32;\n",                                                      //  reported
    "    const SLOPE: i32;\n",                                                     //  reported
    "    const MAGNITUDES: u32;\n",                                                //  reported
    "}\n",
    "pub trait Slots {\n",
    "    const MIN: i64;\n",                                                       //  reported
    "    const MAX: i64;\n",                                                       //  reported
    "    const WIDTH: Width;\n",                                                   //  arvo's own
    "}\n",
    "pub trait Format {\n",
    "    const PHASE_NUM: i64;\n",                                                 //  reported
    "    const PHASE_DEN: i64;\n",                                                 //  reported
    "}\n",
    "pub trait Operation {\n",
    "    const ARITY: u32;\n",                                                     //  reported
    "}\n",
);

#[test]
fn the_six_const_generic_parameter_positions_pass() {
    let reported = lines(THE_FORMAT_SURFACE);
    for line in 1 ..= 5 {
        assert!(
            !reported.contains(&line),
            "line {line} declares const generic parameters and is excepted: {reported:?}",
        );
    }
    assert_eq!(
        THE_FORMAT_SURFACE.matches("const ").count() - 11,
        6,
        "five declaration lines carrying six parameters, beside eleven associated \
         constants of which ten are typed with a bare primitive",
    );
}

#[test]
fn the_ten_associated_constant_positions_are_refused() {
    assert_eq!(
        lines(THE_FORMAT_SURFACE),
        vec![7, 8, 11, 12, 13, 16, 17, 21, 22, 25],
        "every associated constant typed with a bare primitive still reports, \
         and the one typed `Width` does not",
    );
}

#[test]
fn the_whole_surface_was_refused_before_the_exception_reached_it() {
    // The control for the two arms above. Every line of the surface that names a
    // bare primitive anywhere is one the lint would have reported without the
    // carve-out, so the five that now pass are the five it changed and nothing
    // else moved.
    let would_have_reported: Vec<usize> = THE_FORMAT_SURFACE
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            ["u32", "i32", "i64", "bool"]
                .iter()
                .any(|p| line.split(|c: char| !c.is_ascii_alphanumeric() && c != '_').any(|w| w == *p))
        })
        .map(|(idx, _)| idx + 1)
        .collect();
    assert_eq!(
        would_have_reported,
        vec![1, 2, 3, 4, 5, 7, 8, 11, 12, 13, 16, 17, 21, 22, 25]
    );

    let now = lines(THE_FORMAT_SURFACE);
    let changed: Vec<usize> = would_have_reported
        .iter()
        .copied()
        .filter(|l| !now.contains(l))
        .collect();
    assert_eq!(changed, vec![1, 2, 3, 4, 5], "only the parameter declarations moved");
}

// ---- a lifetime tick opens what looks like a char literal -----------------

#[test]
#[ignore = "catalogue: two lifetime ticks on one line pair as a char literal and \
            everything between them is stripped before the scan, so a field, a \
            parameter or a cast sitting there is never seen. Raised by a reviewer \
            against this change and confirmed: it predates the carve-out and \
            nothing on the line below is an excepted position. The stripper wants \
            the parse, same as the literal suffix, and that is a separate change"]
fn a_bare_primitive_between_two_lifetime_ticks_still_reports() {
    let source = "pub struct S<'a> { pub n: u32, pub r: &'a str }\n";
    assert_eq!(lines(source), vec![1], "the `u32` field sits between two ticks");
}

#[test]
fn one_lifetime_tick_alone_does_not_hide_anything() {
    // The control on the catalogued arm above, and what bounds it: a single tick
    // finds no partner, so the line is scanned whole and the field reports. Two
    // is what it takes.
    assert_eq!(lines("pub struct S<'a>(&'a u32);\n"), vec![1]);
}
