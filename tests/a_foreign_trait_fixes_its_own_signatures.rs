//! A signature a crate copies from a trait it does not define is judged at the
//! trait's owner, so the bare-type lints leave it alone.
//!
//! Each lint gets the same four fixtures: the signature in an impl of a foreign
//! trait passes, the same signature in an inherent impl reports, the same
//! signature in an impl of a trait the crate declares reports, and a bare type
//! in the body of a method of a foreign-trait impl still reports.

use mockspace_extra_lints::lints::arvo_types_only::ArvoTypesOnly;
use mockspace_extra_lints::lints::no_bare_numeric::NoBareNumeric;
use mockspace_extra_lints::lints::no_bare_option::NoBareOption;
use mockspace_extra_lints::lints::no_bare_result::NoBareResult;
use mockspace_extra_lints::lints::no_bare_static_str::NoBareStaticStr;
use mockspace_extra_lints::lints::no_bare_string::NoBareString;
use mockspace_lint_rules::testkit::LintFixture;
use mockspace_lint_rules::{CrateLint, LintError};

/// A crate of the stack's vocabulary, which every stack lint holds.
const VOCABULARY_MANIFEST: &str = "[package]\nname = \"notko-fixture\"\n";

/// The lines of `src/lib.rs` the lint reports, for a crate of `lib` and, when
/// given, a `src/contract.rs` declaring the crate's own trait.
fn reported(lint: &dyn CrateLint, lib: &str, contract: Option<&str>) -> Vec<usize> {
    let mut fixture = LintFixture::new(lib)
        .with_doc("Cargo.toml", VOCABULARY_MANIFEST)
        .with_workspace_root(std::path::Path::new("/tmp"));
    if let Some(contract) = contract {
        fixture = fixture.with_module("src/contract.rs", contract);
    }
    let hits: Vec<LintError> = lint.check(&fixture.ctx());
    hits.iter()
        .filter(|e| e.path.as_deref().is_none_or(|p| p.ends_with("lib.rs")))
        .map(|e| e.line)
        .collect()
}

/// The four arms for one lint. `sig` is a method or associated-const signature
/// carrying the bare type, `trait_decl` the matching trait item declaration, and
/// `body_item` an impl item of the foreign trait whose body carries one.
struct Case {
    lint:       &'static dyn CrateLint,
    sig:        &'static str,
    trait_decl: &'static str,
    body_item:  &'static str,
}

/// Each fixture puts the item under test on line 3 of `src/lib.rs`.
fn foreign_impl(item: &str) -> String {
    format!("struct S;\nimpl parser::de::Visitor<'t> for S {{\n    {item}\n}}\n")
}

fn inherent_impl(item: &str) -> String {
    format!("struct S;\nimpl S {{\n    {item}\n}}\n")
}

fn own_trait_impl(item: &str) -> String {
    format!("struct S;\nimpl crate::contract::Visitor<'t> for S {{\n    {item}\n}}\n")
}

fn own_trait(decl: &str) -> String {
    format!("pub trait Visitor<'t> {{\n    {decl}\n}}\n")
}

fn skipped(case: &Case) {
    let lines = reported(case.lint, &foreign_impl(case.sig), None);
    assert!(
        lines.is_empty(),
        "a foreign trait's signature must pass, got lines {lines:?}"
    );
}

fn inherent_reported(case: &Case) {
    let lines = reported(case.lint, &inherent_impl(case.sig), None);
    assert_eq!(lines, vec![3], "an inherent impl's signature must report");
}

fn own_trait_reported(case: &Case) {
    let lines = reported(
        case.lint,
        &own_trait_impl(case.sig),
        Some(&own_trait(case.trait_decl)),
    );
    assert_eq!(
        lines,
        vec![3],
        "an impl of the crate's own trait must report"
    );
}

fn body_reported(case: &Case) {
    let lines = reported(case.lint, &foreign_impl(case.body_item), None);
    assert_eq!(
        lines,
        vec![3],
        "a bare type in a foreign-trait method's body must report"
    );
}

const RESULT: Case = Case {
    lint:       &NoBareResult,
    sig:        "fn visit(&mut self) -> core::result::Result<(), Self::Error> { self.done() }",
    trait_decl: "fn visit(&mut self) -> core::result::Result<(), Self::Error>;",
    body_item:  "fn visit(&mut self) -> Self::Out { let r: Result<(), ()> = self.run(); self.wrap(r) }",
};

const OPTION: Case = Case {
    lint:       &NoBareOption,
    sig:        "fn next(&mut self) -> Option<Self::Item> { self.step() }",
    trait_decl: "fn next(&mut self) -> Option<Self::Item>;",
    body_item:  "fn next(&mut self) -> Self::Out { let x: Option<Self::Item> = self.step(); self.wrap(x) }",
};

const STRING: Case = Case {
    lint:       &NoBareString,
    sig:        "fn put(&self, key: &'t str) { self.store(key) }",
    trait_decl: "fn put(&self, key: &'t str);",
    body_item:  "fn put(&self, key: Key) { let k: &str = key.text(); self.store(k) }",
};

const NUMERIC: Case = Case {
    lint:       &NoBareNumeric,
    sig:        "fn width(&self) -> u32 { self.w() }",
    trait_decl: "fn width(&self) -> u32;",
    body_item:  "fn width(&self) -> Width { let n: u32 = self.w(); Width::from(n) }",
};

const ARVO: Case = Case {
    lint:       &ArvoTypesOnly,
    sig:        "fn ready(&self, at: usize) -> bool { self.at(at) }",
    trait_decl: "fn ready(&self, at: usize) -> bool;",
    body_item:  "fn ready(&self, at: Index) -> Ready { let b: bool = self.at(at); Ready::from(b) }",
};

/// `no-bare-static-str` reads `const` and `static` items, so what a foreign trait
/// fixes for it is the type of an associated constant.
const STATIC_STR: Case = Case {
    lint:       &NoBareStaticStr,
    sig:        "const NAME: &'static str = \"visitor\";",
    trait_decl: "const NAME: &'static str;",
    body_item:  "fn name(&self) -> Name { const N: &str = \"visitor\"; Name::of(N) }",
};

macro_rules! four_arms {
    ($case:ident, $skipped:ident, $inherent:ident, $own:ident, $body:ident) => {
        #[test]
        fn $skipped() {
            skipped(&$case);
        }

        #[test]
        fn $inherent() {
            inherent_reported(&$case);
        }

        #[test]
        fn $own() {
            own_trait_reported(&$case);
        }

        #[test]
        fn $body() {
            body_reported(&$case);
        }
    };
}

four_arms!(
    RESULT,
    no_bare_result_skips_a_foreign_trait_signature,
    no_bare_result_reports_an_inherent_signature,
    no_bare_result_reports_an_own_trait_signature,
    no_bare_result_reports_a_foreign_trait_method_body
);

four_arms!(
    OPTION,
    no_bare_option_skips_a_foreign_trait_signature,
    no_bare_option_reports_an_inherent_signature,
    no_bare_option_reports_an_own_trait_signature,
    no_bare_option_reports_a_foreign_trait_method_body
);

four_arms!(
    STRING,
    no_bare_string_skips_a_foreign_trait_signature,
    no_bare_string_reports_an_inherent_signature,
    no_bare_string_reports_an_own_trait_signature,
    no_bare_string_reports_a_foreign_trait_method_body
);

four_arms!(
    NUMERIC,
    no_bare_numeric_skips_a_foreign_trait_signature,
    no_bare_numeric_reports_an_inherent_signature,
    no_bare_numeric_reports_an_own_trait_signature,
    no_bare_numeric_reports_a_foreign_trait_method_body
);

four_arms!(
    ARVO,
    arvo_types_only_skips_a_foreign_trait_signature,
    arvo_types_only_reports_an_inherent_signature,
    arvo_types_only_reports_an_own_trait_signature,
    arvo_types_only_reports_a_foreign_trait_method_body
);

four_arms!(
    STATIC_STR,
    no_bare_static_str_skips_a_foreign_trait_associated_const,
    no_bare_static_str_reports_an_inherent_associated_const,
    no_bare_static_str_reports_an_own_trait_associated_const,
    no_bare_static_str_reports_a_const_in_a_foreign_trait_method_body
);

/// A signature spanning lines passes as a whole, and the body under it does not.
#[test]
fn a_multi_line_foreign_signature_passes_and_its_body_reports() {
    let lib = "struct S;\nimpl parser::de::Visitor<'t> for S {\n    fn visit(\n        &mut self,\n    ) -> core::result::Result<(), Self::Error>\n    {\n        let r: Result<(), ()> = self.run();\n        self.wrap(r)\n    }\n}\n";
    assert_eq!(reported(&NoBareResult, lib, None), vec![7]);
}
