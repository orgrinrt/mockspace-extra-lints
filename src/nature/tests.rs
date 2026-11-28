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

/// A crate's manifest read on its own, with no workspace to inherit from.
fn of_the_stack(text: &str, no_std: bool) -> bool {
    manifest_is_of_the_stack(&manifest(text), None, no_std)
}

/// A crate's manifest read on its own, with no workspace to inherit from.
fn reaches(text: &str, owner: Owner) -> bool {
    manifest_reaches(&manifest(text), None, owner)
}

/// The `[workspace.dependencies]` table of a root manifest.
fn workspace_table(root: &str) -> toml::Table {
    manifest(root)["workspace"]["dependencies"]
        .as_table()
        .unwrap()
        .clone()
}

#[test]
fn a_crate_of_the_vocabulary_is_of_the_stack_by_its_own_name_whatever_its_root_declares() {
    assert!(of_the_stack("[package]\nname = \"arvo\"\n", true));
    assert!(of_the_stack("[package]\nname = \"notko-build\"\n", false));
}

#[test]
fn a_no_std_crate_depending_on_the_vocabulary_is_of_the_stack() {
    let m = "[package]\nname = \"hilavitkutin-ctx\"\n\n[dependencies]\narvo-bits = { path = \"../arvo-bits\" }\n";
    assert!(of_the_stack(m, true));
    let renamed = "[package]\nname = \"x\"\n\n[dependencies]\nnum = { package = \"arvo\", version = \"0.1\" }\n";
    assert!(of_the_stack(renamed, true));
    let target = "[package]\nname = \"x\"\n\n[target.'cfg(unix)'.dependencies]\nnotko = \"0.1\"\n";
    assert!(of_the_stack(target, true));
    let optional = "[package]\nname = \"x\"\n\n[dependencies]\nnotko = { version = \"0.1\", optional = true }\n";
    assert!(
        of_the_stack(optional, true),
        "an optional dependency is a dependency"
    );
}

#[test]
fn a_std_crate_depending_on_the_vocabulary_is_a_consumer_and_not_of_the_stack() {
    // muisti-engine's shape: a std crate taking notko for its outcome types.
    let m = "[package]\nname = \"muisti-engine\"\n\n[dependencies]\nnotko = { version = \"0.0.1\", default-features = false }\n";
    assert!(!of_the_stack(m, false));
    assert!(
        of_the_stack(m, true),
        "the same crate declaring no_std is of the stack"
    );
}

#[test]
fn a_crate_with_no_edge_to_the_vocabulary_is_not_of_the_stack() {
    let m = "[package]\nname = \"muisti\"\n\n[dependencies]\nrusqlite = \"0.41\"\nblake3 = \"1.8\"\n\n[dev-dependencies]\narvo = \"0.1\"\n";
    assert!(
        !of_the_stack(m, true),
        "a dev-dependency builds tests, not the crate"
    );
    // A name that only opens like the vocabulary's is not one of its crates.
    let near = "[package]\nname = \"arvoisa\"\n\n[dependencies]\nnotkoja = \"1\"\n";
    assert!(!of_the_stack(near, true));
}

#[test]
fn a_build_edge_to_the_vocabulary_no_longer_makes_a_crate_of_the_stack() {
    let build = "[package]\nname = \"x\"\n\n[build-dependencies]\narvo = \"0.1\"\n";
    assert!(
        !of_the_stack(build, true),
        "a build-dependency builds build.rs, not the crate"
    );
}

#[test]
fn a_no_std_crate_on_the_string_crate_alone_is_now_of_the_stack() {
    let string = "[package]\nname = \"x\"\n\n[dependencies]\nhilavitkutin-str = \"0.1\"\n";
    assert!(
        of_the_stack(string, true),
        "the string crate is one of the vocabulary's"
    );
}

#[test]
fn a_crate_of_the_owners_family_reaches_the_owner() {
    assert!(reaches("[package]\nname = \"arvo\"\n", Owner::ARVO));
    assert!(reaches("[package]\nname = \"arvo-bits\"\n", Owner::ARVO));
    assert!(reaches(
        "[package]\nname = \"hilavitkutin-str\"\n",
        Owner::HILAVITKUTIN_STR
    ));
    assert!(
        !reaches("[package]\nname = \"notko\"\n", Owner::ARVO),
        "another vocabulary crate is not arvo's family"
    );
    assert!(
        !reaches(
            "[package]\nname = \"hilavitkutin-api\"\n",
            Owner::HILAVITKUTIN_STR
        ),
        "a sibling of the string crate is not of its family"
    );
}

#[test]
fn a_normal_dependency_on_any_target_reaches_the_owner_optional_or_not() {
    let plain = "[package]\nname = \"x\"\n\n[dependencies]\narvo = \"0.1\"\n";
    assert!(reaches(plain, Owner::ARVO));
    let member =
        "[package]\nname = \"x\"\n\n[dependencies]\narvo-bits = { path = \"../arvo-bits\" }\n";
    assert!(reaches(member, Owner::ARVO));
    let renamed = "[package]\nname = \"x\"\n\n[dependencies]\nfallible = { package = \"notko\", version = \"0.1\" }\n";
    assert!(reaches(renamed, Owner::NOTKO));
    let inherited = "[package]\nname = \"x\"\n\n[dependencies]\nnotko = { workspace = true }\n";
    assert!(reaches(inherited, Owner::NOTKO));
    let target = "[package]\nname = \"x\"\n\n[target.'cfg(unix)'.dependencies]\nhilavitkutin-str = \"0.1\"\n";
    assert!(reaches(target, Owner::HILAVITKUTIN_STR));
    let optional = "[package]\nname = \"x\"\n\n[dependencies]\narvo = { version = \"0.1\", optional = true }\n";
    assert!(
        reaches(optional, Owner::ARVO),
        "an optional dependency is a dependency, and marking it so steps out of nothing"
    );
}

#[test]
fn a_dev_or_build_edge_does_not_reach_the_owner() {
    let dev = "[package]\nname = \"x\"\n\n[dependencies]\nnotko = \"0.1\"\n\n[dev-dependencies]\narvo = \"0.1\"\n";
    assert!(
        !reaches(dev, Owner::ARVO),
        "a dev-dependency builds the tests"
    );
    let build = "[package]\nname = \"x\"\n\n[build-dependencies]\narvo = \"0.1\"\n";
    assert!(
        !reaches(build, Owner::ARVO),
        "a build-dependency builds build.rs"
    );
    let target_build =
        "[package]\nname = \"x\"\n\n[target.'cfg(unix)'.build-dependencies]\narvo = \"0.1\"\n";
    assert!(!reaches(target_build, Owner::ARVO));
    let target_dev =
        "[package]\nname = \"x\"\n\n[target.'cfg(unix)'.dev-dependencies]\narvo = \"0.1\"\n";
    assert!(!reaches(target_dev, Owner::ARVO));
    let near = "[package]\nname = \"x\"\n\n[dependencies]\narvoisa = \"1\"\n";
    assert!(!reaches(near, Owner::ARVO));
}

#[test]
fn a_workspace_entry_renamed_at_the_root_counts_by_the_package_the_root_names() {
    let member =
        manifest("[package]\nname = \"x\"\n\n[dependencies]\nnum = { workspace = true }\n");
    let root = workspace_table(
        "[workspace]\nmembers = [\"x\"]\n\n[workspace.dependencies]\nnum = { package = \"arvo\", version = \"0.1\" }\n",
    );
    assert!(
        !manifest_reaches(&member, None, Owner::ARVO),
        "read by its key alone, `num` is not arvo"
    );
    assert!(
        manifest_reaches(&member, Some(&root), Owner::ARVO),
        "the root's entry says `num` is arvo"
    );
    assert!(manifest_is_of_the_stack(&member, Some(&root), true));
    let plain = workspace_table("[workspace]\n\n[workspace.dependencies]\nnum = \"0.1\"\n");
    assert!(
        !manifest_reaches(&member, Some(&plain), Owner::ARVO),
        "a root entry that renames nothing leaves the key"
    );
}

#[test]
fn a_dependency_naming_its_own_package_is_not_read_through_the_root() {
    // Cargo refuses `package` beside `workspace = true`, so a member's own `package` only ever sits
    // on an entry that does not inherit, and it is what names the package.
    let member = manifest(
        "[package]\nname = \"x\"\n\n[dependencies]\nnum = { package = \"notko\", version = \"0.1\" }\n",
    );
    let root = workspace_table(
        "[workspace]\n\n[workspace.dependencies]\nnum = { package = \"arvo\", version = \"0.1\" }\n",
    );
    assert!(!manifest_reaches(&member, Some(&root), Owner::ARVO));
    assert!(manifest_reaches(&member, Some(&root), Owner::NOTKO));
}
