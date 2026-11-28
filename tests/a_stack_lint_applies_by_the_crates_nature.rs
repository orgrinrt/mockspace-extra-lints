//! Where a stack lint applies is read off the crate, never a list.
//!
//! `no-std` and `no-alloc` hold a crate that declares `#![no_std]`. The bare-primitive lints,
//! `arvo-types-only` and `no-public-raw-field` hold a crate of the stack: one of the vocabulary's
//! own crates, or a `#![no_std]` crate built on one. A `std` crate taking the vocabulary is a
//! consumer, and the ban is not enforced on consumers. A bare-type lint further wants the crate its
//! replacement lives in, which `a_bare_type_lint_holds_a_crate_only_where_its_replacement_is_reached`
//! covers lint by lint.
//!
//! Each case runs all nine lints over the same drift, so a crate of the stack is shown to fail and
//! a crate outside it to pass, on one source.

use mockspace_extra_lints::lints::arvo_types_only::ArvoTypesOnly;
use mockspace_extra_lints::lints::no_alloc::NoAlloc;
use mockspace_extra_lints::lints::no_bare_numeric::NoBareNumeric;
use mockspace_extra_lints::lints::no_bare_option::NoBareOption;
use mockspace_extra_lints::lints::no_bare_result::NoBareResult;
use mockspace_extra_lints::lints::no_bare_static_str::NoBareStaticStr;
use mockspace_extra_lints::lints::no_bare_string::NoBareString;
use mockspace_extra_lints::lints::no_public_raw_field::NoPublicRawField;
use mockspace_extra_lints::lints::no_std::NoStd;
use mockspace_lint_rules::testkit::LintFixture;
use mockspace_lint_rules::CrateLint;

/// Drift every one of the nine lints reports in a crate it holds.
const DRIFT: &str = "\
use std::vec::Vec;
pub struct Handle { pub count: u32, name: String }
pub const NAME: &str = \"x\";
pub fn f(v: Vec<u8>) -> Option<Result<bool, ()>> { None }
";

fn fixture(no_std: bool, manifest: &str) -> LintFixture {
    let root = if no_std { format!("#![no_std]\n{DRIFT}") } else { DRIFT.to_string() };
    LintFixture::new(&root).with_doc("Cargo.toml", manifest)
}

fn nine() -> Vec<Box<dyn CrateLint>> {
    vec![
        Box::new(NoStd),
        Box::new(NoAlloc),
        Box::new(ArvoTypesOnly),
        Box::new(NoBareNumeric),
        Box::new(NoBareOption),
        Box::new(NoBareResult),
        Box::new(NoBareString),
        Box::new(NoBareStaticStr),
        Box::new(NoPublicRawField),
    ]
}

/// The names of the lints among the nine that report anything.
fn firing(f: &LintFixture) -> Vec<&'static str> {
    nine()
        .into_iter()
        .filter(|l| !l.check(&f.ctx()).is_empty())
        .map(|l| l.name())
        .collect()
}

const ON_ARVO: &str = "[package]\nname = \"stack-crate\"\n\n[dependencies]\narvo = \"0.1\"\n";
const ON_ALL_THREE: &str = "[package]\nname = \"stack-crate\"\n\n[dependencies]\narvo = \"0.1\"\nnotko = \"0.1\"\nhilavitkutin-str = \"0.1\"\n";
const ON_NOTKO: &str = "[package]\nname = \"muisti-engine\"\n\n[dependencies]\nnotko = { version = \"0.0.1\", default-features = false }\n";
const ON_NOTHING: &str = "[package]\nname = \"muisti\"\n\n[dependencies]\nrusqlite = \"0.41\"\n\n[dev-dependencies]\narvo = \"0.1\"\n";

#[test]
fn a_no_std_crate_built_on_all_three_owners_fails_all_nine() {
    let fired = firing(&fixture(true, ON_ALL_THREE));
    assert_eq!(
        fired.len(),
        9,
        "every stack lint holds a crate of the stack reaching every owner; fired: {fired:?}"
    );
}

#[test]
fn a_no_std_crate_built_on_arvo_alone_fails_the_lints_whose_replacement_is_arvos() {
    let mut fired = firing(&fixture(true, ON_ARVO));
    fired.sort_unstable();
    assert_eq!(
        fired,
        [
            "arvo-types-only",
            "no-alloc",
            "no-bare-numeric",
            "no-bare-static-str",
            "no-public-raw-field",
            "no-std",
        ],
        "notko's and the string crate's lints wait on their crates; the static-string gate needs none"
    );
}

#[test]
fn a_crate_of_the_vocabulary_fails_its_own_familys_lints_whatever_its_root_declares() {
    let mut fired = firing(&fixture(false, "[package]\nname = \"notko-build\"\n"));
    fired.sort_unstable();
    assert_eq!(
        fired,
        ["no-bare-option", "no-bare-result", "no-bare-static-str"],
        "notko's family reaches notko and no other owner, and the no-std lints wait on the attribute"
    );
}

#[test]
fn a_std_consumer_of_the_vocabulary_passes_all_nine() {
    let fired = firing(&fixture(false, ON_NOTKO));
    assert!(
        fired.is_empty(),
        "a std consumer is held by none of them; fired: {fired:?}"
    );
}

#[test]
fn a_std_crate_with_no_edge_to_the_vocabulary_passes_all_nine() {
    let fired = firing(&fixture(false, ON_NOTHING));
    assert!(
        fired.is_empty(),
        "a std tool over a database is not of the stack; fired: {fired:?}"
    );
}

#[test]
fn a_no_std_crate_with_no_edge_to_the_vocabulary_is_held_only_to_no_std() {
    let fired = firing(&fixture(true, ON_NOTHING));
    let mut sorted = fired.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, ["no-alloc", "no-std"], "fired: {fired:?}");
}

#[test]
fn a_crate_whose_manifest_cannot_be_read_is_not_taken_to_be_of_the_stack() {
    let fired = firing(&LintFixture::new(DRIFT));
    assert!(fired.is_empty(), "fired: {fired:?}");
}
