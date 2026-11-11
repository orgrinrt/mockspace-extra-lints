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
    let lints: [&dyn Lint; 4] = [&file_size, &SlangSingleSource, &SlangNoDuplicateFn::default(), &SlangWritingStyle];
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

mod held_to_the_declaration {
    use std::collections::HashMap;

    use mockspace_extra_lints::lints::a_git_dependency_is_not_pinned::AGitDependencyIsNotPinned;
    use mockspace_extra_lints::lints::slang_file_size::SlangFileSize;
    use mockspace_extra_lints::lints::slang_no_duplicate_fn::SlangNoDuplicateFn;
    use mockspace_extra_lints::lints::slang_single_source::SlangSingleSource;
    use mockspace_extra_lints::lints::slang_writing_style::SlangWritingStyle;
    use mockspace_lint_rules::reach_check::repo_lint;
    use mockspace_lint_rules::{Lint, RepoLint};

    const FUNC: &str = "float shared(float x) { return x; }\n";
    const OUTSIDE: (&str, &str) = ("notes/b.txt", "nothing here\n");

    /// The lint over `tree`, which must give findings, and must give the same ones whatever moves
    /// outside the globs it declares.
    fn held(lint: &dyn RepoLint, tree: &[(&str, &str)]) {
        held_in(lint, tree, &[]);
    }

    fn held_in(lint: &dyn RepoLint, tree: &[(&str, &str)], crates: &[&str]) {
        let dir = tempfile::tempdir().unwrap();
        let seen = repo_lint(lint, tree, crates, dir.path()).unwrap_or_else(|e| panic!("{e}"));
        assert!(seen > 0, "{}: the fixture must make the lint find something", lint.name());
    }

    #[test]
    fn the_file_size_lint() {
        let mut lint = SlangFileSize::default();
        lint.configure(&HashMap::from([("max_lines".to_string(), "2".to_string())]));
        let long = "float a() { return 1; }\nfloat b() { return 2; }\nfloat c() { return 3; }\n";
        held(&lint, &[("mock/crates/a/shaders/long.slang", long), OUTSIDE]);
    }

    #[test]
    fn the_duplicate_fn_lint() {
        held(
            &SlangNoDuplicateFn::default(),
            &[("mock/crates/a/shaders/x.slang", FUNC), ("mock/crates/b/shaders/y.slang", FUNC), OUTSIDE],
        );
    }

    #[test]
    fn the_single_source_lint() {
        let a = "module a;\npublic struct Frame { public float4x4 view; public uint4 counts; };\n";
        let b = "module b;\nstruct Frame { float4x4 view; uint4 counts; };\n";
        held_in(
            &SlangSingleSource,
            &[("mock/crates/alpha/shaders/a.slang", a), ("mock/crates/beta/shaders/b.slang", b), OUTSIDE],
            &["alpha", "beta"],
        );
    }

    #[test]
    fn the_writing_style_lint() {
        let styled = "float a;\n// a **loud** claim\nfloat b;\n";
        held(&SlangWritingStyle, &[("mock/crates/a/shaders/x.slang", styled), OUTSIDE]);
    }

    #[test]
    fn the_pin_lint() {
        let mut lint = AGitDependencyIsNotPinned::default();
        lint.configure(&HashMap::from([("accounts".to_string(), "github.com/acme".to_string())]));
        let manifest = "[dependencies]\ntomlfuse = { git = \"https://github.com/acme/tomlfuse\", rev = \"1773dfb\" }\n";
        held(&lint, &[("mock/crates/alpha/Cargo.toml", manifest), OUTSIDE]);
    }
}
