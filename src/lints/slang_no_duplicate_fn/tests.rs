use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::*;
use crate::test_support::run_repo_lint;

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn check(root: &Path) -> Vec<LintError> {
    let crates = BTreeSet::from(["alpha".to_string(), "beta".to_string()]);
    run_repo_lint(&SlangNoDuplicateFn, root, &crates)
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
