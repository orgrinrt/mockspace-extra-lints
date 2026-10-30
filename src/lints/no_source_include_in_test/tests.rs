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

fn check_with(lint: &NoSourceIncludeInTest, root: &Path) -> Vec<LintError> {
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
        verdicts: None,
    };
    lint.check_repo(&ctx)
}

fn check(root: &Path) -> Vec<LintError> {
    check_with(&NoSourceIncludeInTest::default(), root)
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

fn paths(found: &[LintError]) -> Vec<String> {
    found.iter().map(|e| e.path.clone().unwrap()).collect()
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
fn an_item_that_is_not_under_test_is_not_a_test() {
    let found = run(concat!(
        "#[cfg(not(test))]\n",
        "fn a() { include_str!(\"a.rs\"); }\n",
        "#[cfg(feature = \"test\")]\n",
        "fn b() { include_str!(\"b.rs\"); }\n",
        "#[cfg(unix)]\n",
        "fn c() { include_str!(\"c.rs\"); }\n",
        "#[cfg(any(test, unix))]\n",
        "fn d() { include_str!(\"d.rs\"); }\n",
        "#[derive(Debug)]\n",
        "struct E;\n",
        "fn f() { include_str!(\"f.rs\"); }\n",
        "#[tests]\n",
        "fn g() { include_str!(\"g.rs\"); }\n",
    ));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn the_attributes_that_make_a_test_are_read_by_their_last_segment_with_or_without_arguments() {
    let found = run(concat!(
        "#[tokio::test]\n",
        "async fn a() { include_str!(\"a.rs\"); }\n",
        "#[tokio::test(flavor = \"multi_thread\")]\n",
        "async fn b() { include_str!(\"b.rs\"); }\n",
        "#[rstest]\n",
        "fn c() { include_str!(\"c.rs\"); }\n",
        "#[test_case(1, 2)]\n",
        "fn d() { include_str!(\"d.rs\"); }\n",
        "#[test_case]\n",
        "fn e() { include_str!(\"e.rs\"); }\n",
        "#[quickcheck]\n",
        "fn f() { include_str!(\"f.rs\"); }\n",
        "#[ test ]\n",
        "fn g() { include_str!(\"g.rs\"); }\n",
        "#[ cfg ( test ) ]\n",
        "fn h() { include_str!(\"h.rs\"); }\n",
    ));
    assert_eq!(lines(&found), [2, 4, 6, 8, 10, 12, 14, 16], "{found:?}");
}

#[test]
fn a_cfg_predicate_is_a_test_when_it_cannot_be_true_without_test() {
    let yes = [
        "test",
        "all(test, unix)",
        "all(unix, test)",
        "all(not(unix), test)",
        "all(feature = \"a,b\", test)",
        "any(test, all(test, unix))",
        "all(any(test), unix)",
    ];
    let no = [
        "not(test)",
        "any(test, unix)",
        "all(not(test), unix)",
        "all(unix, feature = \"test\")",
        "all()",
        "any()",
        "tests",
        "all(not(all(test)))",
    ];
    for p in yes {
        assert!(requires_test(p), "{p}");
    }
    for p in no {
        assert!(!requires_test(p), "{p}");
    }
    assert_eq!(split_terms("a,b(c,d),e=\"f,g\",h"), ["a", "b(c,d)", "e=\"f,g\"", "h"]);
}

#[test]
fn an_inner_cfg_test_makes_the_file_or_the_module_a_test() {
    let file = run("#![cfg(test)]\n\nfn helper() { include_str!(\"x.rs\"); }\n");
    assert_eq!(lines(&file), [3], "{file:?}");
    let module = run("mod m {\n    #![cfg(test)]\n    fn a() { include_str!(\"x.rs\"); }\n}\nfn b() { include_str!(\"y.rs\"); }\n");
    assert_eq!(lines(&module), [3], "{module:?}");
    let other = run("#![allow(dead_code)]\n#![cfg(not(test))]\nfn a() { include_str!(\"x.rs\"); }\n");
    assert!(other.is_empty(), "{other:?}");
}

#[test]
fn the_std_spellings_of_the_macros_are_found_and_other_paths_are_not() {
    let found = run(concat!(
        "#[test]\n",
        "fn t() {\n",
        "    std::include_str!(\"a.rs\");\n",
        "    core::include_bytes!(\"b.rs\");\n",
        "    ::std::include_str!(\"c.rs\");\n",
        "    assert!(std::include_str!(\"d.rs\").contains(\"x\"));\n",
        "    mine::include_str!(\"e.rs\");\n",
        "    std::format!(\"f.rs\");\n",
        "}\n",
    ));
    assert_eq!(lines(&found), [3, 4, 5, 6], "{found:?}");
}

#[test]
fn a_module_a_test_declares_is_test_code_and_so_is_every_module_it_declares() {
    let d = tempfile::tempdir().unwrap();
    let body = "fn helper() { include_str!(\"x.rs\"); }\n";
    write(d.path(), "mock/crates/alpha/src/lib.rs", "#[cfg(test)]\nmod support;\nmod live;\n#[cfg(test)]\nmod folder;\n#[cfg(test)]\nmod testing;\n");
    write(d.path(), "mock/crates/alpha/src/support.rs", &format!("mod deep;\n{body}"));
    write(d.path(), "mock/crates/alpha/src/support/deep.rs", body);
    write(d.path(), "mock/crates/alpha/src/live.rs", body);
    write(d.path(), "mock/crates/alpha/src/folder/mod.rs", body);
    write(d.path(), "mock/crates/alpha/src/testing.rs", body);
    write(d.path(), "mock/crates/alpha/src/tests_helper.rs", body);
    let found = check(d.path());
    assert_eq!(
        paths(&found),
        [
            "mock/crates/alpha/src/folder/mod.rs",
            "mock/crates/alpha/src/support.rs",
            "mock/crates/alpha/src/support/deep.rs",
            "mock/crates/alpha/src/testing.rs",
        ]
    );
}

#[test]
fn a_module_file_beside_a_non_root_file_is_found_in_the_folder_of_that_file() {
    assert_eq!(module_files("a/src/lib.rs", "m"), ["a/src/m.rs", "a/src/m/mod.rs"]);
    assert_eq!(module_files("a/src/main.rs", "m"), ["a/src/m.rs", "a/src/m/mod.rs"]);
    assert_eq!(module_files("a/src/x/mod.rs", "m"), ["a/src/x/m.rs", "a/src/x/m/mod.rs"]);
    assert_eq!(module_files("a/src/x.rs", "m"), ["a/src/x/m.rs", "a/src/x/m/mod.rs"]);
    assert_eq!(module_files("lib.rs", "m"), ["m.rs", "m/mod.rs"]);
    assert_eq!(module_files("x.rs", "m"), ["m.rs", "m/mod.rs"]);
}

#[test]
fn the_allow_globs_let_a_source_fixture_through_by_resolved_path_or_as_written() {
    let d = tempfile::tempdir().unwrap();
    let body = "#[test]\nfn t() { include_str!(\"../fixtures/a.rs\"); include_str!(\"../other/b.rs\"); }\n";
    write(d.path(), "mock/crates/alpha/tests/it.rs", body);
    let none = check(d.path());
    assert_eq!(none.len(), 2, "{none:?}");
    let mut lint = NoSourceIncludeInTest::default();
    lint.configure(&HashMap::from([(ALLOW_KEY.to_string(), " mock/crates/alpha/fixtures/*.rs , ,x ".to_string())]));
    assert_eq!(lint.allow, ["mock/crates/alpha/fixtures/*.rs", "x"]);
    let by_resolved = check_with(&lint, d.path());
    assert_eq!(by_resolved.len(), 1, "{by_resolved:?}");
    assert!(by_resolved[0].message.contains("b.rs"), "{:?}", by_resolved[0].message);
    lint.configure(&HashMap::from([(ALLOW_KEY.to_string(), "../other/*.rs".to_string())]));
    let as_written = check_with(&lint, d.path());
    assert_eq!(as_written.len(), 1, "{as_written:?}");
    assert!(as_written[0].message.contains("a.rs"), "{:?}", as_written[0].message);
}

#[test]
fn a_path_is_resolved_from_the_including_file() {
    assert_eq!(resolve("a/b/c.rs", "d.rs").as_deref(), Some("a/b/d.rs"));
    assert_eq!(resolve("a/b/c.rs", "../d.rs").as_deref(), Some("a/d.rs"));
    assert_eq!(resolve("a/b/c.rs", "./x//d.rs").as_deref(), Some("a/b/x/d.rs"));
    assert_eq!(resolve("c.rs", "d.rs").as_deref(), Some("d.rs"));
    assert_eq!(resolve("c.rs", "../d.rs"), None);
    assert_eq!(resolve("a/c.rs", "/abs/d.rs"), None);
}

#[test]
fn the_path_is_the_last_literal_so_a_source_name_followed_by_something_else_is_not_one() {
    let found = run(concat!(
        "#[test]\n",
        "fn t() {\n",
        "    include_str!(concat!(env!(\"DIR\"), \"/src/lib.rs\"));\n",
        "    include_str!(concat!(\"a.rs\", \".bak\"));\n",
        "    include_str!(concat!(\"a.rs\", env!(\"X\")));\n",
        "    include_str!(concat!(\"a/\", env!(\"X\"), \".rs\"));\n",
        "    include_str!(\"data.json\", \"x.rs\");\n",
        "}\n",
    ));
    assert_eq!(lines(&found), [3, 6, 7], "{found:?}");
}

#[test]
fn a_file_is_test_code_by_where_it_sits_and_not_by_a_name_that_only_looks_like_it() {
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
fn build_output_vendored_sources_and_the_record_directories_are_not_read() {
    let d = tempfile::tempdir().unwrap();
    let body = "#[test]\nfn t() { include_str!(\"x.rs\"); }\n";
    for rel in [
        "target/debug/x.rs",
        "mock/vendor/dep/tests/x.rs",
        "mock/research/sketches/x/probe.rs",
        "mock/design_rounds/1/probe.rs",
    ] {
        write(d.path(), rel, body);
    }
    assert!(check(d.path()).is_empty());
}

#[test]
fn the_default_blocks_every_gate_and_allow_is_the_only_key() {
    let lint = NoSourceIncludeInTest::default();
    assert_eq!(lint.default_severity(), Severity::HARD_ERROR);
    assert_eq!(lint.name(), "no-source-include-in-test");
    assert_eq!(lint.config_keys(), &["allow"]);
}

fn allowing(allow: &str) -> NoSourceIncludeInTest {
    let mut lint = NoSourceIncludeInTest::default();
    lint.configure(&HashMap::from([(ALLOW_KEY.to_string(), allow.to_string())]));
    lint
}

#[test]
fn an_allow_reaches_a_source_file_at_the_root_and_a_bare_name_means_only_that_one() {
    let d = tempfile::tempdir().unwrap();
    let body = "#[test]\nfn t() { include_str!(\"../../../../data.rs\"); include_str!(\"../fixtures/data.rs\"); }\n";
    write(d.path(), "mock/crates/alpha/tests/it.rs", body);
    let none = check(d.path());
    assert_eq!(none.len(), 2, "{none:?}");
    let root_only = check_with(&allowing("data.rs"), d.path());
    assert_eq!(root_only.len(), 1, "{root_only:?}");
    assert!(root_only[0].message.contains("fixtures/data.rs"), "{:?}", root_only[0].message);
    assert!(check_with(&allowing("**/data.rs"), d.path()).is_empty());
    assert!(check_with(&allowing("?ata.rs, mock/**/data.rs"), d.path()).is_empty());
}

#[test]
fn a_test_file_at_the_root_is_read_and_its_include_is_allowed_by_its_resolved_path() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "tests/it.rs", "#[test]\nfn t() { include_str!(\"../src/lib.rs\"); }\n");
    let none = check(d.path());
    assert_eq!(none.len(), 1, "{none:?}");
    assert!(check_with(&allowing("src/**/*.rs"), d.path()).is_empty());
    assert!(check_with(&allowing("src/**"), d.path()).is_empty());
    assert_eq!(check_with(&allowing("lib.rs"), d.path()).len(), 1);
}

/// A store that keeps what it is given, for the replay tests.
#[derive(Default)]
struct Kept(std::cell::RefCell<std::collections::HashMap<String, String>>);

impl mockspace_lint_rules::verdict::VerdictCache for Kept {
    fn digest(&self, parts: &[&[u8]]) -> String {
        parts.iter().map(|p| format!("{}:{}|", p.len(), String::from_utf8_lossy(p))).collect()
    }

    fn replay(&self, key: &str) -> Option<String> {
        self.0.borrow().get(key).cloned()
    }

    fn remember(&self, key: &str, encoded: &str) {
        self.0.borrow_mut().insert(key.to_string(), encoded.to_string());
    }
}

fn check_kept(root: &Path, store: &Kept) -> Vec<LintError> {
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
        verdicts: Some(store),
    };
    NoSourceIncludeInTest::default().check_repo(&ctx)
}

fn shown(found: &[LintError]) -> Vec<String> {
    found.iter().map(|e| format!("{}:{}:{}:{:?}", e.crate_name, e.line, e.message, e.path)).collect()
}

#[test]
fn a_saved_run_finds_what_an_unsaved_one_finds_across_an_edit_that_moves_a_test_module() {
    let d = tempfile::tempdir().unwrap();
    let store = Kept::default();
    write(d.path(), "mock/crates/alpha/src/lib.rs", "#[cfg(test)]\nmod probe;\n");
    write(d.path(), "mock/crates/alpha/src/probe.rs", "fn t() { let s = include_str!(\"lib.rs\"); }\n");
    let full = check(d.path());
    assert_eq!(full.len(), 1, "the control: the declared test module's include is reported");
    assert_eq!(shown(&check_kept(d.path(), &store)), shown(&full), "cold");
    assert_eq!(shown(&check_kept(d.path(), &store)), shown(&full), "warm");

    // The module stops being a test module: the include in the untouched file stops being a finding.
    write(d.path(), "mock/crates/alpha/src/lib.rs", "mod probe;\n");
    let full = check(d.path());
    assert!(full.is_empty(), "{full:?}");
    assert_eq!(shown(&check_kept(d.path(), &store)), shown(&full), "after the declaring file moved");
}
