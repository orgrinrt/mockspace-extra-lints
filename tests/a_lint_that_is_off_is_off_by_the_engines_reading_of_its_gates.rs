//! `a-lint-admits-no-crate-list` leaves a lint alone when it is off at every gate, and reads the gates
//! out of the table itself. The engine resolves the same gates when it loads `mockspace.toml`, so the
//! two are held to each other here: over a matrix of tables, the lint reports the `crates` key exactly
//! when the engine's own reading says the lint is on at some gate (or says nothing about it, which
//! means the lint's own default applies and it runs).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use mockspace::config::Config;
use mockspace_extra_lints::lints::a_lint_admits_no_crate_list::ALintAdmitsNoCrateList;
use mockspace_lint_rules::{Level, RegistryView, RepoContext, RepoLint};

const LINT: &str = "some-lint";

/// Whether the lint under test reports the `crates` key of `[lints.some-lint]` in `toml`.
fn reports(root: &Path) -> bool {
    let registry = RegistryView::new(BTreeMap::new(), BTreeMap::new());
    let crates = BTreeSet::from(["alpha".to_string()]);
    let ctx = RepoContext {
        mock_dir:    &root.join("mock"),
        repo_root:   root,
        all_crates:  &crates,
        src_dirs:    &[],
        invocation:  None,
        canon_paths: &[],
        open_panels: &[],
        registry:    &registry,
        verdicts:    None,
    };
    !ALintAdmitsNoCrateList.check_repo(&ctx).is_empty()
}

/// Whether the engine's own reading of the table leaves the lint on at some gate. A lint it holds
/// no severity for runs at the lint's default, which the lint under test cannot know, so it runs.
fn engine_runs(root: &Path) -> bool {
    let cfg = Config::from_dir(root);
    match cfg.lint_overrides.base.get(LINT) {
        Some(s) => [s.on_commit, s.on_build, s.on_push].iter().any(|l| *l != Level::Pass),
        None => true,
    }
}

#[test]
fn the_lint_and_the_engine_agree_on_whether_a_lint_is_on() {
    let bodies = [
        "",
        "severity = \"off\"",
        "severity = \"error\"",
        "severity = \"warn\"",
        "severity = \"ERROR\"",
        "severity = \" off \"",
        "commit = \"off\"",
        "commit = \"off\"\nbuild = \"off\"\npush = \"off\"",
        "commit = \"off\"\nbuild = \"off\"\npush = \"error\"",
        "commit = \"off\"\nbuild = \"warn\"",
        "severity = \"off\"\npush = \"error\"",
        "severity = \"error\"\ncommit = \"off\"\nbuild = \"off\"\npush = \"off\"",
        "severity = \"hard-error\"",
        "severity = \"advisory\"",
        "commit = \"Off\"\nbuild = \"OFF\"\npush = \"off\"",
        "build = \"error\"",
    ];
    let mut disagreements = Vec::new();
    for body in bodies {
        let d = tempfile::tempdir().unwrap();
        let toml = format!("[lints.{LINT}]\ncrates = [\"alpha\"]\n{body}\n");
        fs::write(d.path().join("mockspace.toml"), &toml).unwrap();
        let (lint, engine) = (reports(d.path()), engine_runs(d.path()));
        if lint != engine {
            disagreements.push(format!("{body:?}: lint reports {lint}, engine runs {engine}"));
        }
    }
    assert!(disagreements.is_empty(), "{disagreements:#?}");
}

#[test]
fn the_matrix_has_a_table_that_is_on_and_one_that_is_off() {
    // The control: an agreement that is true of every table proves nothing.
    let on = tempfile::tempdir().unwrap();
    fs::write(on.path().join("mockspace.toml"), "[lints.some-lint]\ncrates = [\"a\"]\nseverity = \"error\"\n").unwrap();
    let off = tempfile::tempdir().unwrap();
    fs::write(off.path().join("mockspace.toml"), "[lints.some-lint]\ncrates = [\"a\"]\nseverity = \"off\"\n").unwrap();
    assert!(reports(on.path()) && engine_runs(on.path()));
    assert!(!reports(off.path()) && !engine_runs(off.path()));
}
