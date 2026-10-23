//! A key under `[lints.catalogue-ignore]` in `mockspace.toml` is read by the engine's own config
//! reader, handed to the lint the way a run hands it, and changes what the lint does.
//!
//! The lint's own unit tests call `configure` with a map they build, which is a path no project
//! can reach: the engine reserves some keys for itself and never forwards them, so a key the lint
//! reads under a reserved name is configurable in the test and nowhere else.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use mockspace::config::Config;
use mockspace_extra_lints::lints::catalogue_ignore::CatalogueIgnore;
use mockspace_lint_rules::{RegistryView, RepoContext, RepoLint};

const BARE: &str = "#[test]\n#[ignore]\nfn t() {}\n";

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

/// The paths the lint finds a bare ignore at, for a repository whose `mockspace.toml` is `toml`.
fn found_with(toml: &str) -> (Vec<String>, bool) {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "mockspace.toml", toml);
    write(d.path(), "scratch.rs", BARE);
    write(d.path(), "kept.rs", BARE);
    let cfg = Config::from_dir(d.path());
    let mut lint = CatalogueIgnore::default();
    cfg.lint_overrides.apply_params_to(&mut lint);
    let registry = RegistryView::new(BTreeMap::new(), BTreeMap::new());
    let crates = BTreeSet::from(["alpha".to_string()]);
    let ctx = RepoContext {
        mock_dir: &d.path().join("mock"),
        repo_root: d.path(),
        all_crates: &crates,
        src_dirs: &[],
        invocation: None,
        canon_paths: &[],
        open_panels: &[],
        registry: &registry,
    };
    let mut paths: Vec<String> =
        lint.check_repo(&ctx).into_iter().filter_map(|e| e.path).collect();
    paths.sort();
    // That the table survived the read at all: one the engine could not deserialise is gone from
    // the result, severity included, or the read refuses it outright, by engine version.
    (paths, cfg.lint_overrides.base.contains_key("catalogue-ignore"))
}

#[test]
fn a_skip_in_mockspace_toml_reaches_the_lint_and_a_matching_path_is_left_alone() {
    let control = "[lints.catalogue-ignore]\nseverity = \"error\"\n";
    let (all, kept) = found_with(control);
    assert!(kept, "the control table did not survive the read");
    assert_eq!(all, ["kept.rs", "scratch.rs"], "the control must see both files");

    let skipped = "[lints.catalogue-ignore]\nseverity = \"error\"\nskip = \"scratch.rs\"\n";
    let (found, kept) = found_with(skipped);
    assert!(kept, "setting `skip` lost the table");
    assert_eq!(found, ["kept.rs"], "`skip` set in mockspace.toml did not reach the lint");
}
