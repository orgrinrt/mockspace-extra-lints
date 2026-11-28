//! A bare-type lint holds a crate of the stack only where the crate that owns the replacement it
//! names is reachable from the crate's own source: the crate is of the owner's family, or names the
//! owner under `[dependencies]`, on any target, `optional` or not, a `workspace = true` entry read
//! through the root manifest's `[workspace.dependencies]`. A dev-dependency builds the tests and a
//! build-dependency builds `build.rs`, so neither reaches it.
//!
//! Every case is a `#![no_std]` crate, so it is of the stack whenever it has an edge to any of the
//! vocabulary's crates, and the only thing that moves between a refusal and a pass is the edge to
//! the one owner the lint asks for.

use mockspace_extra_lints::lints::arvo_types_only::ArvoTypesOnly;
use mockspace_extra_lints::lints::no_bare_numeric::NoBareNumeric;
use mockspace_extra_lints::lints::no_bare_option::NoBareOption;
use mockspace_extra_lints::lints::no_bare_result::NoBareResult;
use mockspace_extra_lints::lints::no_bare_static_str::NoBareStaticStr;
use mockspace_extra_lints::lints::no_bare_string::NoBareString;
use mockspace_extra_lints::lints::no_public_raw_field::NoPublicRawField;
use mockspace_lint_rules::testkit::LintFixture;
use mockspace_lint_rules::{CrateLint, LintError};

const NUMBER: &str = "#![no_std]\npub fn f(x: u32) -> u32 { x }\n";
const OPTION: &str = "#![no_std]\npub fn f() -> Option<()> { None }\n";
const RESULT: &str = "#![no_std]\npub fn f() -> Result<(), ()> { Ok(()) }\n";
const STRING: &str = "#![no_std]\npub fn f(s: String) {}\n";
const NUMBER_FIELD: &str = "#![no_std]\npub struct Handle { pub count: u32 }\n";
const STRING_FIELD: &str = "#![no_std]\npub struct Handle { pub name: String }\n";
const STATIC_STR: &str = "#![no_std]\npub const NAME: &str = \"x\";\n";

/// A `#![no_std]` crate whose manifest is the package line and `deps` verbatim.
fn run(lint: &dyn CrateLint, source: &str, deps: &str) -> Vec<LintError> {
    let manifest = format!("[package]\nname = \"fixture\"\n\n{deps}");
    let fixture = LintFixture::new(source).with_doc("Cargo.toml", &manifest);
    lint.check(&fixture.ctx())
}

/// The owner as a normal dependency, beside another vocabulary crate so the crate is of the stack
/// whichever edge is taken away.
fn with(owner: &str, other: &str) -> String {
    format!("[dependencies]\n{owner} = \"0.1\"\n{other} = \"0.1\"\n")
}

/// The crate of the stack through `other` alone, with the owner on the edge `table` gives it.
fn owner_on(table: &str, owner: &str, other: &str) -> String {
    format!("[dependencies]\n{other} = \"0.1\"\n\n[{table}]\n{owner} = \"0.1\"\n")
}

fn without(other: &str) -> String {
    format!("[dependencies]\n{other} = \"0.1\"\n")
}

/// The cases one lint is held to: refused with the owner, plain or optional, and passing with the
/// owner absent, dev-only and build-only.
fn holds_only_where_reached(lint: &dyn CrateLint, source: &str, owner: &str, other: &str) {
    let name = lint.name();
    assert!(
        !run(lint, source, &with(owner, other)).is_empty(),
        "{name}: a crate depending on `{owner}` with the bare type is refused"
    );
    let optional = format!(
        "[dependencies]\n{other} = \"0.1\"\n{owner} = {{ version = \"0.1\", optional = true }}\n"
    );
    assert!(
        !run(lint, source, &optional).is_empty(),
        "{name}: an optional `{owner}` is a dependency, and the crate is refused"
    );
    let none = run(lint, source, &without(other));
    assert!(
        none.is_empty(),
        "{name}: the same source without `{owner}` passes: {none:?}"
    );
    for table in ["dev-dependencies", "build-dependencies"] {
        let found = run(lint, source, &owner_on(table, owner, other));
        assert!(
            found.is_empty(),
            "{name}: `{owner}` under `[{table}]` is not in scope in `src/`: {found:?}"
        );
    }
}

#[test]
fn arvo_types_only_holds_a_crate_only_where_arvo_is_reached() {
    holds_only_where_reached(&ArvoTypesOnly, NUMBER, "arvo", "notko");
}

#[test]
fn no_bare_numeric_holds_a_crate_only_where_arvo_is_reached() {
    holds_only_where_reached(&NoBareNumeric, NUMBER, "arvo", "notko");
}

#[test]
fn no_bare_option_holds_a_crate_only_where_notko_is_reached() {
    holds_only_where_reached(&NoBareOption, OPTION, "notko", "arvo");
}

#[test]
fn no_bare_result_holds_a_crate_only_where_notko_is_reached() {
    holds_only_where_reached(&NoBareResult, RESULT, "notko", "arvo");
}

#[test]
fn no_bare_string_holds_a_crate_only_where_the_string_crate_is_reached() {
    holds_only_where_reached(&NoBareString, STRING, "hilavitkutin-str", "arvo");
}

#[test]
fn no_public_raw_field_holds_a_number_field_only_where_arvo_is_reached() {
    holds_only_where_reached(&NoPublicRawField, NUMBER_FIELD, "arvo", "notko");
}

#[test]
fn no_public_raw_field_holds_a_string_field_only_where_the_string_crate_is_reached() {
    holds_only_where_reached(&NoPublicRawField, STRING_FIELD, "hilavitkutin-str", "arvo");
}

#[test]
fn no_public_raw_field_holds_each_field_type_to_its_own_owner() {
    let both =
        "#![no_std]\npub struct Handle { pub count: u32 }\npub struct Name { pub name: String }\n";
    let found = run(&NoPublicRawField, both, &without("arvo"));
    assert_eq!(found.len(), 1, "the number field only: {found:?}");
    assert!(
        found[0].message.contains("`arvo`"),
        "the finding names the owner: {found:?}"
    );
}

#[test]
fn a_crate_of_the_owners_family_is_held_without_naming_the_owner() {
    let fixture =
        LintFixture::new(NUMBER).with_doc("Cargo.toml", "[package]\nname = \"arvo-bits\"\n");
    assert!(!ArvoTypesOnly.check(&fixture.ctx()).is_empty());
    let fixture =
        LintFixture::new(OPTION).with_doc("Cargo.toml", "[package]\nname = \"arvo-bits\"\n");
    assert!(
        NoBareOption.check(&fixture.ctx()).is_empty(),
        "arvo's family is not notko's"
    );
}

#[test]
fn no_bare_static_str_holds_every_crate_of_the_stack_and_offers_str_only_where_reached() {
    let with_str = run(
        &NoBareStaticStr,
        STATIC_STR,
        &with("hilavitkutin-str", "arvo"),
    );
    assert_eq!(with_str.len(), 1, "{with_str:?}");
    assert!(
        with_str[0].message.contains("hilavitkutin_str::Str"),
        "{with_str:?}"
    );
    for deps in [
        without("arvo"),
        owner_on("dev-dependencies", "hilavitkutin-str", "arvo"),
        owner_on("build-dependencies", "hilavitkutin-str", "arvo"),
    ] {
        let found = run(&NoBareStaticStr, STATIC_STR, &deps);
        assert_eq!(
            found.len(),
            1,
            "the gate is the fix without the string crate: {found:?}"
        );
        assert!(
            !found[0].message.contains("hilavitkutin_str"),
            "a crate that cannot name `Str` is not told to: {found:?}"
        );
        assert!(found[0].message.contains("debug_assertions"), "{found:?}");
    }
}

#[test]
fn no_bare_static_str_now_holds_a_crate_on_the_string_crate_alone() {
    let found = run(&NoBareStaticStr, STATIC_STR, &without("hilavitkutin-str"));
    assert_eq!(
        found.len(),
        1,
        "the string crate makes it a crate of the stack: {found:?}"
    );
    assert!(
        found[0].message.contains("hilavitkutin_str::Str"),
        "{found:?}"
    );
}

#[test]
fn no_bare_static_str_no_longer_holds_a_crate_whose_only_vocabulary_edge_is_a_build_dependency() {
    let build_only = "[build-dependencies]\narvo = \"0.1\"\n";
    let found = run(&NoBareStaticStr, STATIC_STR, build_only);
    assert!(
        found.is_empty(),
        "a build-dependency does not make a crate of the stack: {found:?}"
    );
}

/// A crate inheriting `num` from a real workspace on disk whose root renames it to `root_package`.
/// The crate's own files are the fixture's; the root manifest is read off the disk as a run reads it.
fn in_a_workspace(root_package: &str, lint: &dyn CrateLint, source: &str) -> Vec<LintError> {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        format!(
            "[workspace]\nmembers = [\"member\"]\n\n[workspace.dependencies]\nnum = {{ package = \"{root_package}\", version = \"0.1\" }}\n"
        ),
    )
    .unwrap();
    let member = dir.path().join("member");
    let fixture = LintFixture::new(source)
        .with_crate_name(member.to_str().unwrap(), "member")
        .with_doc(
            "Cargo.toml",
            "[package]\nname = \"member\"\n\n[dependencies]\nnotko = \"0.1\"\nnum = { workspace = true }\n",
        );
    lint.check(&fixture.ctx())
}

#[test]
fn a_workspace_entry_the_root_renames_to_the_owner_holds_the_crate() {
    let found = in_a_workspace("arvo", &ArvoTypesOnly, NUMBER);
    assert!(
        !found.is_empty(),
        "`num` is arvo by the root's entry, so the bare number is refused"
    );
    let found = in_a_workspace("rusqlite", &ArvoTypesOnly, NUMBER);
    assert!(
        found.is_empty(),
        "the root names another package: {found:?}"
    );
}

/// A crate outside its workspace's directory, naming the root through `package.workspace`, and
/// inheriting `num` from it. No directory above the crate holds the root, so only the key finds it.
fn named_by_package_workspace(
    root_package: &str,
    lint: &dyn CrateLint,
    source: &str,
) -> Vec<LintError> {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("root");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("Cargo.toml"),
        format!(
            "[workspace]\nmembers = [\"../elsewhere/member\"]\n\n[workspace.dependencies]\nnum = {{ package = \"{root_package}\", version = \"0.1\" }}\n"
        ),
    )
    .unwrap();
    let member = dir.path().join("elsewhere").join("member");
    let fixture = LintFixture::new(source)
        .with_crate_name(member.to_str().unwrap(), "member")
        .with_doc(
            "Cargo.toml",
            "[package]\nname = \"member\"\nworkspace = \"../../root\"\n\n[dependencies]\nnotko = \"0.1\"\nnum = { workspace = true }\n",
        );
    lint.check(&fixture.ctx())
}

#[test]
fn a_root_named_by_package_workspace_renames_the_entry_for_a_crate_outside_it() {
    let found = named_by_package_workspace("arvo", &ArvoTypesOnly, NUMBER);
    assert!(
        !found.is_empty(),
        "`package.workspace` finds the root, whose entry says `num` is arvo"
    );
    let found = named_by_package_workspace("rusqlite", &ArvoTypesOnly, NUMBER);
    assert!(
        found.is_empty(),
        "the root names another package: {found:?}"
    );
}
