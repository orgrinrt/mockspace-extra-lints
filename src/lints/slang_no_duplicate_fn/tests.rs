use std::collections::{BTreeMap, BTreeSet};
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
    run(&SlangNoDuplicateFn::default(), root)
}

fn run(lint: &SlangNoDuplicateFn, root: &Path) -> Vec<LintError> {
    let crates = BTreeSet::from(["alpha".to_string(), "beta".to_string()]);
    let reg = RegistryView::new(BTreeMap::new(), BTreeMap::new());
    let ctx = RepoContext {
        mock_dir: &root.join("mock"),
        repo_root: root,
        all_crates: &crates,
        src_dirs: &[],
        invocation: None,
        canon_paths: &[],
        open_panels: &[],
        ref_files: &mockspace_lint_rules::src_layout::NO_REF_FILES,
        registry: &reg,
        verdicts: None,
    };
    lint.check_repo(&ctx)
}

fn tree(files: &[(&str, &str)]) -> Vec<LintError> {
    let d = tempfile::tempdir().unwrap();
    for (rel, text) in files {
        write(d.path(), rel, text);
    }
    check(d.path())
}

const ALPHA: &str = "mock/crates/alpha/shaders/light.slang";
const BETA: &str = "mock/crates/beta/shaders/shade.slang";

#[test]
fn distinct_functions_in_two_crates_are_clean() {
    let found = tree(&[
        (ALPHA, "module light;\npublic float falloff(float d) { return 1.0 / d; }\n"),
        (BETA, "module shade;\npublic float glow(float d) { return d; }\n"),
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn the_same_function_copied_into_a_second_crate_is_reported_at_the_copy() {
    let found = tree(&[
        (ALPHA, "module light;\npublic float falloff(float d) { return 1.0 / d; }\n"),
        (BETA, "module shade;\n\nfloat falloff(float d) { return 1.0 / d; }\n"),
    ]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].path.as_deref(), Some(BETA));
    assert_eq!(found[0].line, 3);
    assert_eq!(found[0].crate_name, "beta");
}

#[test]
fn the_same_name_with_another_signature_is_an_overload_and_clean() {
    let found = tree(&[
        (ALPHA, "module light;\nfloat falloff(float d) { return d; }\n"),
        (BETA, "module shade;\nfloat falloff(float3 d, float k) { return k; }\n"),
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn entries_and_probes_are_not_counted() {
    let found = tree(&[
        (ALPHA, "[shader(\"compute\")]\nvoid cs(uint3 id : SV_DispatchThreadID) { }\n"),
        (BETA, "[shader(\"compute\")]\nvoid cs(uint3 id : SV_DispatchThreadID) { }\n"),
        ("mock/crates/beta/src/tests/probe/p.slang", "float falloff(float d) { return d; }\n"),
        ("mock/crates/alpha/shaders/other.slang", "float falloff(float d) { return d; }\n"),
    ]);
    // the probe is not shipped, so `falloff` is defined once among the shipped files
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_marked_copy_is_let_through() {
    let found = tree(&[
        (ALPHA, "module light;\nfloat falloff(float d) { return d; }\n"),
        (BETA, "module shade;\nfloat falloff(float d) { return d; } // lint:allow(slang-no-duplicate-fn)\n"),
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_comment_that_names_a_function_is_not_a_copy() {
    let found = tree(&[
        (ALPHA, "module light;\nfloat falloff(float d) { return d; }\n"),
        (BETA, "module shade;\n// float falloff(float d) { return d; }\n"),
    ]);
    assert!(found.is_empty(), "{found:?}");
}

/// A body of well over the default 24 tokens.
const LONG: &str = "{ float a = d * d; float b = a + 1.0; float c = b * 0.25; return saturate(c - a / b); }";

fn with(min: &str) -> SlangNoDuplicateFn {
    let mut lint = SlangNoDuplicateFn::default();
    lint.configure(&HashMap::from([(MIN_BODY_TOKENS_KEY.to_string(), min.to_string())]));
    lint
}

#[test]
fn a_long_body_copied_under_another_name_is_reported_as_a_body_copy() {
    let found = tree(&[
        (ALPHA, &format!("module light;\npublic float falloff(float d) {LONG}\n")),
        (BETA, &format!("module shade;\nfloat glow(float d)\n{LONG}\n")),
    ]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].finding_kind, Some(BODY));
    assert_eq!(found[0].path.as_deref(), Some(BETA));
    assert!(found[0].message.contains("falloff"), "{}", found[0].message);
}

#[test]
fn a_copy_that_differs_only_in_comments_and_spacing_is_the_same_body() {
    let spaced = LONG.replace("; ", ";\n    // a note\n    ");
    let found = tree(&[
        (ALPHA, &format!("module light;\npublic float falloff(float d) {LONG}\n")),
        (BETA, &format!("module shade;\nfloat glow(float d) {spaced}\n")),
    ]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].finding_kind, Some(BODY));
}

#[test]
fn a_short_body_shared_by_two_functions_is_not_a_copy() {
    // the control: `return d;` is the whole of many unrelated functions
    let found = tree(&[
        (ALPHA, "module light;\npublic float falloff(float d) { return d; }\n"),
        (BETA, "module shade;\nfloat glow(float d) { return d; }\n"),
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_different_long_body_is_not_a_copy() {
    let other = LONG.replace("0.25", "0.5");
    let found = tree(&[
        (ALPHA, &format!("module light;\npublic float falloff(float d) {LONG}\n")),
        (BETA, &format!("module shade;\nfloat glow(float d) {other}\n")),
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_same_signature_copy_is_reported_once_as_a_signature_copy() {
    let found = tree(&[
        (ALPHA, &format!("module light;\npublic float falloff(float d) {LONG}\n")),
        (BETA, &format!("module shade;\nfloat falloff(float d) {LONG}\n")),
    ]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].finding_kind, Some(SIGNATURE));
}

#[test]
fn the_allow_marker_lets_a_body_copy_through() {
    let found = tree(&[
        (ALPHA, &format!("module light;\npublic float falloff(float d) {LONG}\n")),
        (BETA, &format!("module shade;\nfloat glow(float d) {LONG} // lint:allow(slang-no-duplicate-fn)\n")),
    ]);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn the_shortest_body_compared_is_the_projects_to_set() {
    // `return d * d;` is five tokens, so a threshold of five compares it and six does not
    let d = tempfile::tempdir().unwrap();
    write(d.path(), ALPHA, "module light;\npublic float falloff(float d) { return d * d; }\n");
    write(d.path(), BETA, "module shade;\nfloat glow(float d) { return d * d; }\n");
    assert_eq!(run(&with("5"), d.path()).len(), 1);
    assert!(run(&with("6"), d.path()).is_empty());
    assert!(SlangNoDuplicateFn::default().config_keys().contains(&MIN_BODY_TOKENS_KEY));
}

#[test]
fn a_shortest_body_that_is_not_a_number_is_reported() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), ALPHA, &format!("module light;\npublic float falloff(float d) {LONG}\n"));
    let found = run(&with("2x"), d.path());
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.contains("`2x`"), "{}", found[0].message);
    // the control: a number is taken and the clean tree stays clean
    assert!(run(&with("24"), d.path()).is_empty());
}

#[test]
fn a_function_reported_as_a_signature_copy_is_not_also_a_body_copy() {
    let found = tree(&[
        (ALPHA, &format!("module light;\nfloat glow(float d) {LONG}\nfloat falloff(float3 d) {LONG}\n")),
        (BETA, &format!("module shade;\nfloat falloff(float3 d) {LONG}\n")),
    ]);
    let mut kinds: Vec<(&str, Option<&str>)> =
        found.iter().map(|e| (e.path.as_deref().unwrap(), e.finding_kind)).collect();
    kinds.sort();
    assert_eq!(kinds, vec![(ALPHA, Some(BODY)), (BETA, Some(SIGNATURE))], "{found:?}");
}

#[test]
fn an_overload_with_the_same_body_is_one_body_copy() {
    let found = tree(&[
        (ALPHA, &format!("module light;\nfloat falloff(float d) {LONG}\n")),
        (BETA, &format!("module shade;\nfloat falloff(float3 d) {LONG}\n")),
    ]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].finding_kind, Some(BODY));
}

#[test]
fn a_threshold_of_zero_does_not_call_two_declarations_copies() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), ALPHA, "module light;\nfloat f(float d);\n");
    write(d.path(), BETA, "module shade;\nfloat g(float3 d);\n");
    assert!(run(&with("0"), d.path()).is_empty());
    // the control: two real bodies are still compared at zero
    write(d.path(), ALPHA, "module light;\nfloat f(float d) { return d * d; }\n");
    write(d.path(), BETA, "module shade;\nfloat g(float3 d) { return d * d; }\n");
    assert_eq!(run(&with("0"), d.path()).len(), 1);
}
