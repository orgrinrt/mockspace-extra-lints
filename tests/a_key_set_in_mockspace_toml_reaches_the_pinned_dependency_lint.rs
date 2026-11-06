//! `accounts`, `branch` and `exempt` under `[lints.a-git-dependency-is-not-pinned]` in `mockspace.toml` are read
//! by the engine's own config reader, handed to the lint the way a run hands it, and change what the
//! lint does. The lint's own unit tests call `configure` with a map they build, a path no project
//! can reach, since the engine reserves some keys and never forwards them.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use mockspace::config::Config;
use mockspace_extra_lints::lints::a_git_dependency_is_not_pinned::AGitDependencyIsNotPinned;
use mockspace_lint_rules::{RegistryView, RepoContext, RepoLint};

const PINNED: &str = "[dependencies]\na = { git = \"https://github.com/acme/a\", rev = \"1\" }\n";
const UNPINNED: &str = "[dependencies]\na = { git = \"https://github.com/acme/a\" }\n";

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

/// The manifests the lint reports, for a repository whose `mockspace.toml` is `toml`.
fn found_with(toml: &str) -> Vec<String> {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "mockspace.toml", toml);
    write(d.path(), "mock/crates/pinned/Cargo.toml", PINNED);
    write(d.path(), "mock/research/old/Cargo.toml", PINNED);
    write(d.path(), "mock/crates/loose/Cargo.toml", UNPINNED);
    let cfg = Config::from_dir(d.path());
    let mut lint = AGitDependencyIsNotPinned::default();
    cfg.lint_overrides.apply_params_to(&mut lint);
    let registry = RegistryView::new(BTreeMap::new(), BTreeMap::new());
    let crates = BTreeSet::from(["alpha".to_string()]);
    let ctx = RepoContext {
        mock_dir:    &d.path().join("mock"),
        repo_root:   d.path(),
        all_crates:  &crates,
        src_dirs:    &[],
        invocation:  None,
        canon_paths: &[],
        open_panels: &[],
        registry:    &registry,
        verdicts:    None,
    };
    let mut paths: Vec<String> = lint.check_repo(&ctx).into_iter().filter_map(|e| e.path).collect();
    paths.sort();
    paths
}

#[test]
fn a_table_that_names_no_account_finds_nothing() {
    let none = found_with("[lints.a-git-dependency-is-not-pinned]\nseverity = \"error\"\n");
    assert!(none.is_empty(), "{none:?}");
}

#[test]
fn the_accounts_key_switches_the_lint_on_for_those_accounts_only() {
    let on = found_with("[lints.a-git-dependency-is-not-pinned]\naccounts = \"github.com/acme\"\n");
    assert_eq!(on, ["mock/crates/pinned/Cargo.toml", "mock/research/old/Cargo.toml"]);
    let off = found_with("[lints.a-git-dependency-is-not-pinned]\naccounts = \"github.com/elsewhere\"\n");
    assert!(off.is_empty(), "{off:?}");
}

#[test]
fn the_exempt_key_spares_the_paths_it_names_and_the_branch_key_adds_the_branch_finding() {
    let exempt =
        found_with("[lints.a-git-dependency-is-not-pinned]\naccounts = \"github.com/acme\"\nexempt = \"mock/research/**\"\n");
    assert_eq!(exempt, ["mock/crates/pinned/Cargo.toml"]);
    let branch =
        found_with("[lints.a-git-dependency-is-not-pinned]\naccounts = \"github.com/acme\"\nbranch = \"main\"\nexempt = \"mock/research/**\"\n");
    assert_eq!(branch, ["mock/crates/loose/Cargo.toml", "mock/crates/pinned/Cargo.toml"]);
}
