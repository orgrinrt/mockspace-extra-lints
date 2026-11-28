use super::{Owner, crate_prelude, manifest_is_of_the_stack, manifest_reaches};

#[test]
fn an_attribute_past_a_long_module_doc_is_still_found() {
    let doc = "//! prose\n".repeat(80);
    let source = format!("{doc}\n#![no_std]\n\npub mod thing;\n");
    assert!(
        crate_prelude(&source).contains("#![no_std]"),
        "a module doc of any length may precede the attribute"
    );
}

#[test]
fn an_attribute_on_the_very_first_line_is_found() {
    assert!(crate_prelude("#![no_std]\npub mod thing;\n").contains("#![no_std]"));
}

#[test]
fn the_prelude_stops_at_the_first_item_so_a_later_line_cannot_pass_for_one() {
    // The string appears, in a position the language would not accept an
    // inner attribute in. Reading the whole file would find it; reading the
    // prelude does not, which is the difference this function is for.
    let source = "//! prose\npub mod thing;\nconst NOTE: &str = \"#![no_std]\";\n";
    assert!(!crate_prelude(source).contains("#![no_std]"));
}

#[test]
fn a_crate_with_no_attribute_reports_none() {
    let source = "//! prose\n//! more\n\npub mod thing;\n";
    assert!(!crate_prelude(source).contains("#![no_std]"));
}

#[test]
fn a_block_comment_header_does_not_end_the_prelude() {
    let source = "/* a banner\n * over several lines\n */\n#![no_std]\npub mod thing;\n";
    assert!(crate_prelude(source).contains("#![no_std]"));
}

#[test]
fn an_outer_attribute_on_the_first_item_ends_the_prelude() {
    // `#[derive(...)]` opens an item rather than the crate, so anything
    // after it is not a place an inner attribute may sit.
    let source = "//! prose\n#[derive(Debug)]\npub struct Thing;\n#![no_std]\n";
    assert!(!crate_prelude(source).contains("#![no_std]"));
}

#[test]
fn an_empty_source_is_a_prelude_with_no_attribute_in_it() {
    assert!(!crate_prelude("").contains("#![no_std]"));
}

fn manifest(text: &str) -> toml::Table {
    text.parse().unwrap()
}

#[test]
fn a_crate_of_the_vocabulary_is_of_the_stack_by_its_own_name_whatever_its_root_declares() {
    assert!(manifest_is_of_the_stack(
        &manifest("[package]\nname = \"arvo\"\n"),
        true
    ));
    assert!(manifest_is_of_the_stack(
        &manifest("[package]\nname = \"notko-build\"\n"),
        false
    ));
}

#[test]
fn a_no_std_crate_depending_on_the_vocabulary_is_of_the_stack() {
    let m = "[package]\nname = \"hilavitkutin-ctx\"\n\n[dependencies]\narvo-bits = { path = \"../arvo-bits\" }\n";
    assert!(manifest_is_of_the_stack(&manifest(m), true));
    let renamed = "[package]\nname = \"x\"\n\n[dependencies]\nnum = { package = \"arvo\", version = \"0.1\" }\n";
    assert!(manifest_is_of_the_stack(&manifest(renamed), true));
    let target = "[package]\nname = \"x\"\n\n[target.'cfg(unix)'.dependencies]\nnotko = \"0.1\"\n";
    assert!(manifest_is_of_the_stack(&manifest(target), true));
}

#[test]
fn a_std_crate_depending_on_the_vocabulary_is_a_consumer_and_not_of_the_stack() {
    // muisti-engine's shape: a std crate taking notko for its outcome types.
    let m = "[package]\nname = \"muisti-engine\"\n\n[dependencies]\nnotko = { version = \"0.0.1\", default-features = false }\n";
    assert!(!manifest_is_of_the_stack(&manifest(m), false));
    assert!(
        manifest_is_of_the_stack(&manifest(m), true),
        "the same crate declaring no_std is of the stack"
    );
}

#[test]
fn a_crate_with_no_edge_to_the_vocabulary_is_not_of_the_stack() {
    let m = "[package]\nname = \"muisti\"\n\n[dependencies]\nrusqlite = \"0.41\"\nblake3 = \"1.8\"\n\n[dev-dependencies]\narvo = \"0.1\"\n";
    assert!(
        !manifest_is_of_the_stack(&manifest(m), true),
        "a dev-dependency builds tests, not the crate"
    );
    // A name that only opens like the vocabulary's is not one of its crates.
    let near = "[package]\nname = \"arvoisa\"\n\n[dependencies]\nnotkoja = \"1\"\n";
    assert!(!manifest_is_of_the_stack(&manifest(near), true));
}

#[test]
fn a_build_or_optional_edge_to_the_vocabulary_does_not_make_a_crate_of_the_stack() {
    let build = "[package]\nname = \"x\"\n\n[build-dependencies]\narvo = \"0.1\"\n";
    assert!(
        !manifest_is_of_the_stack(&manifest(build), true),
        "a build-dependency builds build.rs, not the crate"
    );
    let optional = "[package]\nname = \"x\"\n\n[dependencies]\nnotko = { version = \"0.1\", optional = true }\n";
    assert!(!manifest_is_of_the_stack(&manifest(optional), true));
    let string = "[package]\nname = \"x\"\n\n[dependencies]\nhilavitkutin-str = \"0.1\"\n";
    assert!(
        manifest_is_of_the_stack(&manifest(string), true),
        "the string crate is one of the vocabulary's"
    );
}

#[test]
fn a_crate_of_the_owners_family_reaches_the_owner() {
    assert!(manifest_reaches(
        &manifest("[package]\nname = \"arvo\"\n"),
        Owner::ARVO
    ));
    assert!(manifest_reaches(
        &manifest("[package]\nname = \"arvo-bits\"\n"),
        Owner::ARVO
    ));
    assert!(manifest_reaches(
        &manifest("[package]\nname = \"hilavitkutin-str\"\n"),
        Owner::HILAVITKUTIN_STR
    ));
    assert!(
        !manifest_reaches(&manifest("[package]\nname = \"notko\"\n"), Owner::ARVO),
        "another vocabulary crate is not arvo's family"
    );
    assert!(
        !manifest_reaches(
            &manifest("[package]\nname = \"hilavitkutin-api\"\n"),
            Owner::HILAVITKUTIN_STR
        ),
        "a sibling of the string crate is not of its family"
    );
}

#[test]
fn a_normal_dependency_on_any_target_reaches_the_owner() {
    let plain = "[package]\nname = \"x\"\n\n[dependencies]\narvo = \"0.1\"\n";
    assert!(manifest_reaches(&manifest(plain), Owner::ARVO));
    let member =
        "[package]\nname = \"x\"\n\n[dependencies]\narvo-bits = { path = \"../arvo-bits\" }\n";
    assert!(manifest_reaches(&manifest(member), Owner::ARVO));
    let renamed = "[package]\nname = \"x\"\n\n[dependencies]\nfallible = { package = \"notko\", version = \"0.1\" }\n";
    assert!(manifest_reaches(&manifest(renamed), Owner::NOTKO));
    let inherited = "[package]\nname = \"x\"\n\n[dependencies]\nnotko = { workspace = true }\n";
    assert!(manifest_reaches(&manifest(inherited), Owner::NOTKO));
    let target = "[package]\nname = \"x\"\n\n[target.'cfg(unix)'.dependencies]\nhilavitkutin-str = \"0.1\"\n";
    assert!(manifest_reaches(&manifest(target), Owner::HILAVITKUTIN_STR));
}

#[test]
fn a_dev_build_or_optional_edge_does_not_reach_the_owner() {
    let dev = "[package]\nname = \"x\"\n\n[dependencies]\nnotko = \"0.1\"\n\n[dev-dependencies]\narvo = \"0.1\"\n";
    assert!(
        !manifest_reaches(&manifest(dev), Owner::ARVO),
        "a dev-dependency builds the tests"
    );
    let build = "[package]\nname = \"x\"\n\n[build-dependencies]\narvo = \"0.1\"\n";
    assert!(
        !manifest_reaches(&manifest(build), Owner::ARVO),
        "a build-dependency builds build.rs"
    );
    let target_build =
        "[package]\nname = \"x\"\n\n[target.'cfg(unix)'.build-dependencies]\narvo = \"0.1\"\n";
    assert!(!manifest_reaches(&manifest(target_build), Owner::ARVO));
    let target_dev =
        "[package]\nname = \"x\"\n\n[target.'cfg(unix)'.dev-dependencies]\narvo = \"0.1\"\n";
    assert!(!manifest_reaches(&manifest(target_dev), Owner::ARVO));
    let optional = "[package]\nname = \"x\"\n\n[dependencies]\narvo = { version = \"0.1\", optional = true }\n";
    assert!(
        !manifest_reaches(&manifest(optional), Owner::ARVO),
        "source written for a build without the feature cannot name it"
    );
    let near = "[package]\nname = \"x\"\n\n[dependencies]\narvoisa = \"1\"\n";
    assert!(!manifest_reaches(&manifest(near), Owner::ARVO));
}
