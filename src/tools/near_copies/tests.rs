//! The two lists and what is left out of them. Each negative sits beside the positive it is the
//! other half of, so a tool that listed every pair and a tool that listed none would each fail one.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use mockspace_lint_rules::tool::{Tool, ToolContext};
use mockspace_lint_rules::{CrateSourceFile, DeclaredType, Outcome, RegistryView, declared_types};

use super::compare::{Found, Options, close, compare};
use super::{NearCopies, parse, render};

fn crate_of(name: &str, source: &str) -> Vec<DeclaredType> {
    let file = CrateSourceFile {
        rel_path: PathBuf::from("src/lib.rs"),
        text:     source.to_string(),
    };
    declared_types(name, &[file])
}

pub(super) fn both(a: &str, b: &str) -> Vec<DeclaredType> {
    let mut types = crate_of("crate-a", a);
    types.extend(crate_of("crate-b", b));
    types
}

const OPTIONS: Options = Options {
    min_percent: 60,
    pub_only:    false,
};

pub(super) fn found(types: &[DeclaredType]) -> Found<'_> {
    compare(types, &|_| true, &OPTIONS)
}

fn near(a: &str, b: &str) -> usize {
    found(&both(a, b)).near_copies.len()
}

#[test]
fn a_copy_with_one_member_more_is_a_near_copy_with_what_differs() {
    let types = both(
        "pub struct Frame { pub index: u64, pub at: u32, pub extra: u8 }",
        "pub struct Tick { pub index: u64, pub at: u32 }",
    );
    let found = found(&types);
    assert_eq!(found.near_copies.len(), 1);
    let pair = &found.near_copies[0];
    assert_eq!(pair.close.shared, ["at", "index"]);
    assert_eq!(pair.close.only_first.len(), 1);
    assert!(pair.close.only_first[0].starts_with("extra"));
    assert_eq!(pair.close.percent(), 66);
}

#[test]
fn a_renamed_field_counts_half_and_is_named() {
    let types = both(
        "pub struct Frame { pub index: u64, pub at: u32 }",
        "pub struct Tick { pub number: u64, pub at: u32 }",
    );
    let found = found(&types);
    assert_eq!(found.near_copies.len(), 1);
    let close = &found.near_copies[0].close;
    assert_eq!(close.shared, ["at"]);
    assert_eq!(close.renamed, [(
        "index".to_string(),
        "number".to_string(),
        "u64".to_string()
    )]);
    assert_eq!(close.percent(), 75);
}

#[test]
fn an_enum_with_one_variant_more_is_a_near_copy() {
    assert_eq!(
        near(
            "pub enum Run { Pause, Resume, Step }",
            "pub enum Go { Pause, Resume }"
        ),
        1
    );
}

#[test]
fn the_same_shape_under_another_name_is_a_near_copy_at_a_hundred() {
    let types = both(
        "pub struct Frame { pub a: u64, pub b: u32 }",
        "pub struct Tick { pub a: u64, pub b: u32 }",
    );
    let found = found(&types);
    assert_eq!(found.near_copies[0].close.percent(), 100);
}

#[test]
fn two_types_that_share_no_members_are_not_near_copies() {
    assert_eq!(
        near(
            "pub struct Table { pub rows: u32, pub cols: u32 }",
            "pub struct Colour { pub r: u8, pub g: u8 }"
        ),
        0
    );
}

#[test]
fn one_member_in_common_is_not_enough_to_call_a_copy() {
    assert_eq!(
        near(
            "pub struct A { pub id: u32, pub x: u8 }",
            "pub struct B { pub id: u32, pub y: u16 }"
        ),
        0
    );
}

#[test]
fn a_struct_and_an_enum_of_the_same_members_are_not_one_concept_copied() {
    assert!(
        close(
            &crate_of("a", "pub struct A { pub x: u8, pub y: u8 }")[0],
            &crate_of("b", "pub enum B { X, Y }")[0]
        )
        .is_none()
    );
    assert_eq!(
        near(
            "pub struct A { pub x: u8, pub y: u8 }",
            "pub enum B { X, Y }"
        ),
        0
    );
}

#[test]
fn the_minimum_is_a_knob_on_what_is_listed() {
    let types = both(
        "pub struct Frame { pub index: u64, pub at: u32, pub extra: u8 }",
        "pub struct Tick { pub index: u64, pub at: u32 }",
    );
    let strict = Options {
        min_percent: 90,
        pub_only:    false,
    };
    assert!(compare(&types, &|_| true, &strict).near_copies.is_empty());
    assert_eq!(compare(&types, &|_| true, &OPTIONS).near_copies.len(), 1);
}

#[test]
fn one_name_over_two_declarations_is_listed_with_both() {
    let types = both(
        "pub struct Table { pub rows: u32, pub cols: u32 }",
        "pub struct Table { pub name: String }",
    );
    let found = found(&types);
    assert_eq!(found.same_named.len(), 1);
    assert_eq!(
        found.same_named[0].close.percent(),
        0,
        "unrelated, so it sits at the bottom"
    );
    assert!(found.near_copies.is_empty());
}

#[test]
fn a_same_named_pair_that_drifted_ranks_above_an_unrelated_one() {
    let mut types = crate_of(
        "crate-a",
        "pub struct Frame { pub index: u64, pub at: u32 }\npub struct Table { pub rows: u32 }",
    );
    types.extend(crate_of(
        "crate-b",
        "pub struct Frame { pub index: u64, pub at: u32, pub more: u8 }",
    ));
    types.extend(crate_of("crate-c", "pub struct Table { pub name: String }"));
    let found = found(&types);
    let order: Vec<&str> = found
        .same_named
        .iter()
        .map(|p| p.first.name.as_str())
        .collect();
    assert_eq!(order, ["Frame", "Table"]);
}

#[test]
fn one_name_over_one_declaration_is_left_to_single_source() {
    let types = both(
        "pub struct Frame { pub index: u64 }",
        "pub struct Frame { pub index: u64 }",
    );
    let found = found(&types);
    assert!(found.same_named.is_empty() && found.near_copies.is_empty());
}

#[test]
fn a_pair_in_one_crate_is_not_a_pair() {
    let types = crate_of(
        "only",
        "pub struct A { pub x: u8, pub y: u8 }\npub struct B { pub x: u8, pub y: u8 }",
    );
    let found = found(&types);
    assert!(found.near_copies.is_empty());
}

#[test]
fn a_pair_with_neither_side_in_scope_is_not_read() {
    let types = both(
        "pub struct A { pub x: u8, pub y: u8 }",
        "pub struct B { pub x: u8, pub y: u8 }",
    );
    assert!(compare(&types, &|_| false, &OPTIONS).near_copies.is_empty());
    assert_eq!(
        compare(&types, &|t| t.crate_name == "crate-a", &OPTIONS)
            .near_copies
            .len(),
        1
    );
}

#[test]
fn a_private_type_is_read_unless_only_public_ones_are_asked_for() {
    let types = both(
        "struct A { x: u8, y: u8 }",
        "pub struct B { pub x: u8, pub y: u8 }",
    );
    assert_eq!(compare(&types, &|_| true, &OPTIONS).near_copies.len(), 1);
    let only_pub = Options {
        min_percent: 60,
        pub_only:    true,
    };
    assert!(compare(&types, &|_| true, &only_pub).near_copies.is_empty());
}

#[test]
fn nothing_found_prints_nothing() {
    assert_eq!(render::report(&Found::default(), 30), "");
}

#[test]
fn a_finding_prints_both_places_and_what_differs() {
    let types = both(
        "pub struct Frame { pub index: u64, pub at: u32, pub extra: u8 }",
        "pub struct Tick { pub index: u64, pub at: u32 }",
    );
    let text = render::report(&found(&types), 30);
    assert!(
        text.contains("struct Frame (crate-a src/lib.rs:1)"),
        "{text}"
    );
    assert!(
        text.contains("struct Tick (crate-b src/lib.rs:1)"),
        "{text}"
    );
    assert!(text.contains("shared: at, index"), "{text}");
    assert!(text.contains("only in crate-a: extra"), "{text}");
}

#[test]
fn the_limit_caps_a_list_and_says_how_many_it_left() {
    let mut types = crate_of("crate-a", "pub struct A1 { pub x: u8, pub y: u8 }");
    types.extend(crate_of(
        "crate-b",
        "pub struct B1 { pub x: u8, pub y: u8 }\npub struct B2 { pub x: u8, pub y: u8 }",
    ));
    let text = render::report(&found(&types), 1);
    assert!(text.contains("... 1 more"), "{text}");
}

#[test]
fn the_command_line_is_read_and_a_wrong_flag_is_refused() {
    let args =
        parse(&["alpha", "--min", "80", "--limit", "5", "--named", "--against", "dev"]).unwrap();
    assert_eq!(
        (
            args.crates.as_slice(),
            args.min,
            args.limit,
            args.named,
            args.against.as_str()
        ),
        (&["alpha".to_string()][..], 80, 5, true, "dev")
    );
    assert!(parse(&["--nope"]).is_err());
    assert!(parse(&["--min"]).is_err());
    assert!(parse(&["--min", "x"]).is_err());
}

pub(super) fn write(root: &std::path::Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn run_over_named_crates_reports_them_and_exits_clean() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "mock/crates/alpha/src/lib.rs",
        "pub struct Frame { pub index: u64, pub at: u32 }",
    );
    write(
        dir.path(),
        "mock/crates/beta/src/lib.rs",
        "pub struct Tick { pub index: u64, pub at: u32 }",
    );
    write(
        dir.path(),
        "mock/crates/beta/tests/it.rs",
        "pub struct Other { pub index: u64, pub at: u32 }",
    );
    let crates: BTreeSet<String> = ["alpha", "beta"].iter().map(|s| s.to_string()).collect();
    let args = ["alpha", "beta"];
    let registry = RegistryView::default();
    let mock = dir.path().join("mock");
    let src_dirs = [mock.join("crates")];
    let ctx = ToolContext {
        mock_dir:   &mock,
        repo_root:  dir.path(),
        all_crates: &crates,
        src_dirs:   &src_dirs,
        args:       &args,
        stdin:      None,
        registry:   &registry,
    };
    let report = NearCopies.run(&ctx);
    assert!(
        matches!(report.outcome, Outcome::Clean {
            examined: 2,
        }),
        "tests/ is not source: {:?}",
        report.outcome
    );
    assert!(
        report.output.contains("struct Frame") && report.output.contains("struct Tick"),
        "{}",
        report.output
    );
    assert!(
        !report.output.contains("Other"),
        "a file under tests/ declares nothing"
    );
}

#[test]
fn run_over_an_unread_crate_is_inconclusive_and_not_clean() {
    let dir = tempfile::tempdir().unwrap();
    let crates: BTreeSet<String> = BTreeSet::new();
    let args = ["ghost"];
    let registry = RegistryView::default();
    let mock = dir.path().join("mock");
    let ctx = ToolContext {
        mock_dir:   &mock,
        repo_root:  dir.path(),
        all_crates: &crates,
        src_dirs:   &[],
        args:       &args,
        stdin:      None,
        registry:   &registry,
    };
    assert!(matches!(
        NearCopies.run(&ctx).outcome,
        Outcome::Inconclusive { .. }
    ));
}

#[test]
fn a_single_member_in_common_is_not_a_copy_even_at_a_hundred_percent() {
    // `{ id: u32 }` twice is 100% by the count and nothing by the meaning: the floor of two members
    // lined up is what keeps every one-field wrapper in the workspace off the list.
    assert_eq!(
        near(
            "pub struct A { pub id: u32 }",
            "pub struct B { pub id: u32 }"
        ),
        0
    );
}

#[test]
fn a_flag_that_wants_a_value_without_one_is_refused_and_the_minimum_stops_at_a_hundred() {
    assert!(parse(&["--against"]).is_err());
    assert!(parse(&["--limit"]).is_err());
    assert_eq!(parse(&["--min", "250"]).unwrap().min, 100);
    assert!(parse(&["--pub-only"]).unwrap().pub_only);
}

#[test]
fn a_reordered_tuple_field_is_said_to_have_moved_and_not_to_be_renamed() {
    let types = both(
        "pub struct A(pub u8, pub u16, pub u32);",
        "pub struct B(pub u8, pub u32, pub u16);",
    );
    let text = render::report(&found(&types), 30);
    assert!(text.contains("moved: 1 -> 2 of u16"), "{text}");
    assert!(!text.contains("renamed: 1"), "{text}");
}

#[test]
fn every_same_named_pair_prints_what_differs_and_not_only_that_something_does() {
    // Mutation: the frame left out of the rendering, so a supertrait or a unit against a braced body
    // prints a pair at a hundred percent and nothing that tells them apart.
    const SHAPES: &[&str] = &[
        "pub struct X { pub a: u8 }",
        "pub struct X;",
        "pub struct X {}",
        "pub struct X(pub u8);",
        "pub enum X { A, B }",
        "#[repr(u8)] pub enum X { A, B }",
        "#[repr(u8)] pub enum X { B, A }",
        "pub trait X { fn put(&self); }",
        "pub trait X: Copy { fn put(&self); }",
        "pub trait X: Clone { fn put(&self); }",
        "pub struct X<T> { pub a: T }",
    ];
    for (i, a) in SHAPES.iter().enumerate() {
        for b in &SHAPES[i + 1 ..] {
            let types = both(a, b);
            if types[0].shape == types[1].shape {
                continue;
            }
            let found = found(&types);
            assert_eq!(found.same_named.len(), 1, "`{a}` against `{b}`");
            let text = render::report(&found, 30);
            let (x, y) = (&types[0].parts, &types[1].parts);
            let said = |s: &str| text.contains(s);
            let told = if types[0].kind != types[1].kind {
                said("keywords differ")
            } else if x.frame != y.frame {
                said(&x.frame) && said(&y.frame)
            } else if x.generics != y.generics {
                said(&x.generics) && said(&y.generics)
            } else {
                said("only in") || said("renamed") || said("moved")
            };
            assert!(
                told,
                "`{a}` against `{b}` does not print what differs:\n{text}"
            );
        }
    }
}

#[test]
fn crates_with_no_source_directory_to_read_them_from_are_inconclusive_and_not_clean() {
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "mock/crates/alpha/src/lib.rs",
        "pub struct Frame { pub index: u64, pub at: u32 }",
    );
    let crates: BTreeSet<String> = ["alpha"].iter().map(|s| s.to_string()).collect();
    let args: [&str; 0] = [];
    let registry = RegistryView::default();
    let mock = dir.path().join("mock");
    let ctx = ToolContext {
        mock_dir:   &mock,
        repo_root:  dir.path(),
        all_crates: &crates,
        src_dirs:   &[],
        args:       &args,
        stdin:      None,
        registry:   &registry,
    };
    let outcome = NearCopies.run(&ctx).outcome;
    assert!(
        matches!(&outcome, Outcome::Inconclusive { reason } if reason.contains("no source directory")),
        "{outcome:?}: a repository with crates and no source directory has read nothing, and nothing read is not clean"
    );
}
