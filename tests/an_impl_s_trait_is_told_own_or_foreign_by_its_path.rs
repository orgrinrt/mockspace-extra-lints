//! Whether an impl's trait is the crate's own or another crate's is read off
//! the path the impl names it by, through the crate's `use` declarations, and
//! only a bare name the file imports from nowhere is asked of the crate's
//! `trait` items. A trait that only a macro expands to is judged at the macro,
//! and a method that refines a return type its trait leaves open is the
//! implementer's choice and judged as one.

use mockspace_extra_lints::lints::no_bare_numeric::NoBareNumeric;
use mockspace_extra_lints::lints::no_bare_option::NoBareOption;
use mockspace_lint_rules::testkit::LintFixture;
use mockspace_lint_rules::{CrateLint, LintError};

/// A crate of the stack's vocabulary, which every stack lint holds.
const VOCABULARY_MANIFEST: &str = "[package]\nname = \"notko-fixture\"\n";

/// Each file the lint reports, with the lines, for a crate of `lib` and the
/// `modules` beside it.
fn reported(lint: &dyn CrateLint, lib: &str, modules: &[(&str, &str)]) -> Vec<(String, usize)> {
    let mut fixture = LintFixture::new(lib)
        .with_doc("Cargo.toml", VOCABULARY_MANIFEST)
        .with_workspace_root(std::path::Path::new("/tmp"));
    for (path, text) in modules {
        fixture = fixture.with_module(path, text);
    }
    let hits: Vec<LintError> = lint.check(&fixture.ctx());
    let mut out: Vec<(String, usize)> = hits
        .iter()
        .map(|e| {
            let file = e.path.as_deref().unwrap_or("src/lib.rs");
            (file.rsplit('/').next().unwrap_or(file).to_owned(), e.line)
        })
        .collect();
    out.sort();
    out
}

const OWN: (&str, &str) = (
    "src/contract.rs",
    "pub trait Width {\n    fn width(&self) -> u32;\n}\n",
);

fn with_own(line: usize) -> Vec<(String, usize)> {
    vec![("contract.rs".to_owned(), 2), ("lib.rs".to_owned(), line)]
}

fn on_lib(line: usize) -> Vec<(String, usize)> {
    vec![("lib.rs".to_owned(), line)]
}

#[test]
fn an_own_trait_under_another_name_is_the_crate_s_own() {
    let lib = "use crate::contract::Width as Wide;\nstruct S;\nimpl Wide for S {\n    fn width(&self) -> u32 { 1 }\n}\n";
    assert_eq!(reported(&NoBareNumeric, lib, &[OWN]), with_own(4));
    // and in a list, and through `self` and `super` paths
    let lib = "use crate::{contract::{Width as Wide}};\nstruct S;\nimpl Wide for S {\n    fn width(&self) -> u32 { 1 }\n}\n";
    assert_eq!(reported(&NoBareNumeric, lib, &[OWN]), with_own(4));
    let lib = "mod inner {\n    use super::contract::Width as Wide;\n    struct S;\n    impl Wide for S {\n        fn width(&self) -> u32 { 1 }\n    }\n}\n";
    assert_eq!(reported(&NoBareNumeric, lib, &[OWN]), with_own(5));
}

#[test]
fn a_trait_named_by_a_path_of_the_crate_is_the_crate_s_own_whatever_it_is_called() {
    for path in ["crate::contract::Width", "self::contract::Width", "contract::Width"] {
        let lib = format!("mod contract;\nstruct S;\nimpl {path} for S {{\n    fn width(&self) -> u32 {{ 1 }}\n}}\n");
        assert_eq!(reported(&NoBareNumeric, &lib, &[OWN]), with_own(4), "{path}");
    }
}

#[test]
fn a_foreign_trait_sharing_a_name_with_an_own_one_is_foreign() {
    let own = (
        "src/contract.rs",
        "pub trait Iterator {\n    fn step(&self) -> u32;\n}\n",
    );
    // named by a path rooted in another crate
    let lib = "struct S;\nimpl core::iter::Iterator for S {\n    type Item = u8;\n    fn next(&mut self) -> Option<Self::Item> { None }\n}\n";
    assert_eq!(reported(&NoBareOption, lib, &[own]), vec![]);
    // and by a bare name the file imports from another crate
    let lib = "use core::iter::Iterator;\nstruct S;\nimpl Iterator for S {\n    type Item = u8;\n    fn next(&mut self) -> Option<Self::Item> { None }\n}\n";
    assert_eq!(reported(&NoBareOption, lib, &[own]), vec![]);
    // the control: the bare name the file does not import is the crate's own
    let lib = "struct S;\nimpl Iterator for S {\n    fn step(&self) -> u32 { 1 }\n}\n";
    assert_eq!(reported(&NoBareNumeric, lib, &[own]), with_own(3));
}

#[test]
fn a_trait_only_a_macro_declares_is_judged_where_the_macro_writes_it() {
    // the trait's signature is reported in the macro's body, which the line
    // scan reads, and the impl's copy of it is not reported twice
    let lib = "macro_rules! declare {\n    () => {\n        pub trait Make { fn make(&self) -> u32; }\n    };\n}\ndeclare!();\nstruct S;\nimpl Make for S {\n    fn make(&self) -> u32 { 1 }\n}\n";
    assert_eq!(reported(&NoBareNumeric, lib, &[]), on_lib(3));
}

#[test]
fn a_return_type_an_impl_refines_is_its_own_choice() {
    // a trait method returning `impl Trait` leaves the type open, and an impl
    // naming a type of its own says so to the compiler
    let lib = "struct S;\nimpl parser::Items for S {\n    #[allow(refining_impl_trait)]\n    fn items(&self) -> core::option::IntoIter<u32> { None.into_iter() }\n}\n";
    assert_eq!(reported(&NoBareNumeric, lib, &[]), on_lib(4));
    let lib = "struct S;\nimpl parser::Items for S {\n    #[expect(refining_impl_trait_reachable)]\n    fn items(&self) -> core::option::IntoIter<u32> { None.into_iter() }\n}\n";
    assert_eq!(reported(&NoBareNumeric, lib, &[]), on_lib(4));
    // the control: with no such word the signature is the trait's
    let lib = "struct S;\nimpl parser::Items for S {\n    #[inline]\n    fn items(&self) -> core::option::IntoIter<u32> { None.into_iter() }\n}\n";
    assert_eq!(reported(&NoBareNumeric, lib, &[]), vec![]);
}

#[test]
fn what_the_reading_misjudges_is_held_as_it_is() {
    // a `use` is read for the whole file: a module bringing the name in from
    // another crate makes the bare name foreign in a sibling module too
    let lib = "mod a {\n    use parser::Width;\n}\nmod b {\n    struct S;\n    impl Width for S {\n        fn width(&self) -> u32 { 1 }\n    }\n}\n";
    assert_eq!(reported(&NoBareNumeric, lib, &[OWN]), vec![("contract.rs".to_owned(), 2)]);
    // a foreign trait re-exported and named by a path of the crate is read as
    // the crate's own, so its signature is still checked
    let lib = "pub mod reexport {\n    pub use parser::Width;\n}\nstruct S;\nimpl crate::reexport::Width for S {\n    fn width(&self) -> u32 { 1 }\n}\n";
    assert_eq!(reported(&NoBareNumeric, lib, &[]), on_lib(6));
    // an impl a macro body writes is no impl in the parse, so it is all read
    let lib = "macro_rules! implement {\n    () => {\n        impl parser::Width for S { fn width(&self) -> u32 { 1 } }\n    };\n}\n";
    assert_eq!(reported(&NoBareNumeric, lib, &[]), on_lib(3));
}
