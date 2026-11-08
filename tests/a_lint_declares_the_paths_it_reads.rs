//! The lints that read a fixed set of files declare it, so the commit gate runs them only for a
//! commit that changes one. A lint declaring nothing is run by every commit, so each arm below
//! is held to the declaration and to the commit that must not run it.

use mockspace_extra_lints::lints::a_git_dependency_is_not_pinned::AGitDependencyIsNotPinned;
use mockspace_extra_lints::lints::slang_file_size::SlangFileSize;
use mockspace_extra_lints::lints::slang_no_duplicate_fn::SlangNoDuplicateFn;
use mockspace_extra_lints::lints::slang_single_source::SlangSingleSource;
use mockspace_extra_lints::lints::slang_writing_style::SlangWritingStyle;
use mockspace_lint_rules::{Lint, Reach, Touched};

fn runs(lint: &dyn Lint, changed: &[&str]) -> bool {
    let files: Vec<String> = changed.iter().map(|f| (*f).to_string()).collect();
    lint.reach().runs_for(&Touched::Files(&files))
}

#[test]
fn a_slang_lint_runs_for_a_shader_and_not_for_a_rust_file() {
    let file_size = SlangFileSize::default();
    let lints: [&dyn Lint; 4] = [&file_size, &SlangSingleSource, &SlangNoDuplicateFn, &SlangWritingStyle];
    for lint in lints {
        assert!(matches!(lint.reach(), Reach::Paths(_)), "{} declares its reads", lint.name());
        assert!(runs(lint, &["mock/crates/kaski-sky/shaders/sky.slang"]), "{}", lint.name());
        assert!(runs(lint, &["top.slang"]), "{} must see a shader at the root", lint.name());
        assert!(!runs(lint, &["mock/crates/kaski-sky/src/lib.rs", "README.md"]), "{}", lint.name());
    }
}

#[test]
fn the_pin_lint_runs_for_a_manifest_and_not_for_a_shader() {
    let lint = AGitDependencyIsNotPinned::default();
    assert!(matches!(lint.reach(), Reach::Paths(_)));
    assert!(runs(&lint, &["mock/crates/a/Cargo.toml"]));
    assert!(runs(&lint, &["Cargo.toml"]));
    assert!(!runs(&lint, &["mock/crates/a/shaders/x.slang"]));
}
