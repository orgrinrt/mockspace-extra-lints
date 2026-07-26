//! Smoke test: the pack's one entry point contributes its lints, and every
//! contributed lint is well-formed whatever its input kind.

use mockspace_extra_lints as pack;
use mockspace_lint_rules::{Lint, LintPack};

/// Collect the pack once, as the host does.
fn collected() -> LintPack {
    let mut p = LintPack::default();
    pack::collect(&mut p);
    p
}

/// Every lint the pack contributes, as trait objects over the shared supertrait.
///
/// That supertrait is what lets a check like "every name is kebab-case" be
/// written once for all kinds rather than once per kind.
fn all_lints(p: &LintPack) -> Vec<&dyn Lint> {
    let mut v: Vec<&dyn Lint> = Vec::new();
    v.extend(p.crate_lints.iter().map(|l| l.as_ref() as &dyn Lint));
    v.extend(p.workspace_lints.iter().map(|l| l.as_ref() as &dyn Lint));
    v.extend(p.repo_lints.iter().map(|l| l.as_ref() as &dyn Lint));
    v.extend(p.message_lints.iter().map(|l| l.as_ref() as &dyn Lint));
    v
}

#[test]
fn the_pack_contributes_crate_lints() {
    let p = collected();
    assert!(
        !p.crate_lints.is_empty(),
        "the pack should contribute at least one crate lint"
    );
}

#[test]
fn the_pack_contributes_a_workspace_lint() {
    let p = collected();
    assert!(
        !p.workspace_lints.is_empty(),
        "writing-style compares across crates, so it belongs to the workspace kind"
    );
}

#[test]
fn every_lint_has_a_kebab_case_name() {
    let p = collected();
    for lint in all_lints(&p) {
        let name = lint.name();
        assert!(!name.is_empty(), "lint name should not be empty");
        assert!(
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit()),
            "lint name `{name}` should be kebab-case"
        );
    }
}

#[test]
fn lint_names_are_unique_across_every_kind() {
    // Names key the config, so a collision between two kinds would leave one of
    // them unconfigurable. Checking within each kind separately would miss it.
    let p = collected();
    let names: Vec<&'static str> = all_lints(&p).iter().map(|l| l.name()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(names.len(), sorted.len(), "lint names collide: {names:?}");
}

#[test]
fn collecting_twice_accumulates_rather_than_replacing() {
    // The host collects every pack into one shared value, so `collect` appends.
    let mut p = LintPack::default();
    pack::collect(&mut p);
    let first = p.crate_lints.len();
    pack::collect(&mut p);
    assert_eq!(p.crate_lints.len(), first * 2);
}
