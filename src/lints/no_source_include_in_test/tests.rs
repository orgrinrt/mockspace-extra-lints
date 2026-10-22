use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use mockspace_lint_rules::RegistryView;

use super::*;

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn check(root: &Path) -> Vec<LintError> {
    let crates = BTreeSet::from(["alpha".to_string()]);
    let reg = RegistryView::new(BTreeMap::new(), BTreeMap::new());
    let ctx = RepoContext {
        mock_dir: &root.join("mock"),
        repo_root: root,
        all_crates: &crates,
        src_dirs: &[],
        invocation: None,
        canon_paths: &[],
        open_panels: &[],
        registry: &reg,
    };
    NoSourceIncludeInTest.check_repo(&ctx)
}

/// The findings for `body` written to `rel` in a repository of one crate.
fn at(rel: &str, body: &str) -> Vec<LintError> {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), rel, body);
    check(d.path())
}

/// The findings for `body` written to a file that is not test code by where it sits.
fn run(body: &str) -> Vec<LintError> {
    at("mock/crates/alpha/src/lib.rs", body)
}

fn lines(found: &[LintError]) -> Vec<usize> {
    found.iter().map(|e| e.line).collect()
}

#[test]
fn a_test_function_including_source_text_is_reported_with_its_place() {
    let found = run("fn helper() {}\n\n#[test]\nfn t() {\n    let s = include_str!(\"../src/lib.rs\");\n}\n");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, 5);
    assert_eq!(found[0].crate_name, "alpha");
    assert_eq!(found[0].path.as_deref(), Some("mock/crates/alpha/src/lib.rs"));
    assert_eq!(found[0].severity, Severity::HARD_ERROR);
    assert!(found[0].message.contains("`include_str!`") && found[0].message.contains("../src/lib.rs"));
}

#[test]
fn include_bytes_of_source_is_reported_the_same() {
    let found = run("#[test]\nfn t() { let b = include_bytes!(\"lib.rs\"); }\n");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.contains("`include_bytes!`"), "{:?}", found[0].message);
}

#[test]
fn a_data_fixture_is_included_freely_and_so_is_code_by_include() {
    let body = concat!(
        "#[test]\n",
        "fn t() {\n",
        "    let _ = include_str!(\"fixtures/a.ron\");\n",
        "    let _ = include_str!(\"fixtures/a.toml\");\n",
        "    let _ = include_bytes!(\"fixtures/a.json\");\n",
        "    let _ = include_str!(\"shaders/a.slang\");\n",
        "    let _ = include_str!(\"README.md\");\n",
        "    let _ = include_str!(\"rs\");\n",
        "    let _ = include_str!(\"a.rs.bak\");\n",
        "    include!(\"generated.rs\");\n",
        "}\n",
    );
    let found = run(body);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn source_included_outside_a_test_is_not_reported() {
    let found = run("fn banner() -> &'static str { include_str!(\"main.rs\") }\n");
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn an_attribute_belongs_to_the_next_item_only() {
    let found = run("#[test]\nfn a() {}\n\nfn b() { include_str!(\"x.rs\"); }\n");
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_comment_between_the_attribute_and_the_function_does_not_cut_them_apart() {
    let found = run("#[test]\n/// Reads the source.\n// and says so\nfn t() { include_str!(\"x.rs\"); }\n");
    assert_eq!(lines(&found), [4], "{found:?}");
}

#[test]
fn a_cfg_test_item_is_a_test_and_so_is_everything_in_it() {
    let found = run(concat!(
        "#[cfg(test)]\n",
        "mod tests {\n",
        "    fn helper() -> &'static str { include_str!(\"a.rs\") }\n",
        "    mod nested { const S: &str = include_str!(\"b.rs\"); }\n",
        "}\n",
        "#[cfg(all(test, unix))]\n",
        "fn c() { include_str!(\"c.rs\"); }\n",
    ));
    assert_eq!(lines(&found), [3, 4, 7], "{found:?}");
}

#[test]
fn a_cfg_that_is_not_test_is_not_one() {
    let found = run(concat!(
        "#[cfg(not(test))]\n",
        "fn a() { include_str!(\"a.rs\"); }\n",
        "#[cfg(feature = \"test\")]\n",
        "fn b() { include_str!(\"b.rs\"); }\n",
        "#[cfg(unix)]\n",
        "fn c() { include_str!(\"c.rs\"); }\n",
        "#[test_case]\n",
        "fn d() { include_str!(\"d.rs\"); }\n",
        "#[derive(Debug)]\n",
        "struct E;\n",
        "fn f() { include_str!(\"f.rs\"); }\n",
    ));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_runtime_test_attribute_makes_a_test_too() {
    let found = run("#[tokio::test]\nasync fn t() { include_str!(\"x.rs\"); }\n");
    assert_eq!(lines(&found), [2], "{found:?}");
}

#[test]
fn the_include_is_found_inside_another_macro_and_by_the_last_literal_of_a_path() {
    let found = run(concat!(
        "#[test]\n",
        "fn t() {\n",
        "    assert!(include_str!(\"../src/lib.rs\").contains(\"x\"));\n",
        "    let s = include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/src/lib.rs\"));\n",
        "    let r = include_str!(r\"raw.rs\");\n",
        "}\n",
    ));
    assert_eq!(lines(&found), [3, 4, 5], "{found:?}");
}

#[test]
fn a_file_is_test_code_by_where_it_sits() {
    let body = "fn helper() -> &'static str { include_str!(\"x.rs\") }\n";
    for rel in [
        "mock/crates/alpha/tests/it.rs",
        "mock/crates/alpha/tests/sub/it.rs",
        "mock/crates/alpha/src/tests/mod.rs",
        "mock/crates/alpha/src/tests.rs",
        "mock/crates/alpha/src/walk_tests.rs",
    ] {
        let found = at(rel, body);
        assert_eq!(lines(&found), [1], "{rel}: {found:?}");
    }
    for rel in [
        "mock/crates/alpha/src/lib.rs",
        "mock/crates/alpha/src/attests.rs",
        "mock/crates/alpha/src/protests/mod.rs",
        "mock/crates/alpha/src/tests_helper.rs",
        "mock/crates/alpha/src/testing.rs",
    ] {
        let found = at(rel, body);
        assert!(found.is_empty(), "{rel}: {found:?}");
    }
}

#[test]
fn an_include_in_a_comment_or_a_string_is_not_one() {
    let body = concat!(
        "#[test]\n",
        "fn t() {\n",
        "    // include_str!(\"x.rs\")\n",
        "    let s = \"include_str!(\\\"x.rs\\\")\";\n",
        "    let n = include_str;\n",
        "    let m = include_str!();\n",
        "}\n",
    );
    let found = run(body);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn build_output_and_vendored_sources_are_not_read() {
    let d = tempfile::tempdir().unwrap();
    let body = "#[test]\nfn t() { include_str!(\"x.rs\"); }\n";
    write(d.path(), "target/debug/x.rs", body);
    write(d.path(), "mock/vendor/dep/tests/x.rs", body);
    assert!(check(d.path()).is_empty());
}

#[test]
fn the_default_blocks_every_gate_and_the_lint_takes_no_configuration() {
    let lint = NoSourceIncludeInTest;
    assert_eq!(lint.default_severity(), Severity::HARD_ERROR);
    assert_eq!(lint.name(), "no-source-include-in-test");
    assert!(lint.config_keys().is_empty());
}
