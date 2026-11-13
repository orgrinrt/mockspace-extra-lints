use std::collections::{BTreeMap, BTreeSet, HashMap};

use tempfile::TempDir;

use super::*;
use crate::test_support::run_repo_lint_in;

fn registry(rows: &[(&str, &[(&str, &str)])]) -> RegistryView {
    let rows = rows
        .iter()
        .map(|(q, f)| {
            let f = f
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect::<BTreeMap<_, _>>();
            ((*q).to_string(), f)
        })
        .collect::<BTreeMap<_, _>>();
    RegistryView::new(rows, BTreeMap::new())
}

fn run_with(
    lint: &AProvisionalStampIsNotARatification,
    rows: &[(&str, &[(&str, &str)])],
) -> Vec<LintError> {
    let dir = TempDir::new().unwrap();
    run_repo_lint_in(lint, dir.path(), &BTreeSet::new(), &registry(rows), &[])
}

fn run(rows: &[(&str, &[(&str, &str)])]) -> Vec<LintError> {
    run_with(&AProvisionalStampIsNotARatification::default(), rows)
}

fn kinds(found: &[LintError]) -> Vec<&'static str> {
    found.iter().filter_map(|e| e.finding_kind).collect()
}

const WHOLE: &str = "Stamped by the three coordinators converging; question ticket `abc123def456`.";

#[test]
fn a_whole_stamp_on_an_unratified_row_passes() {
    for q in ["ruling::r", "proposal::p"] {
        let f = run(&[(q, &[
            ("rung", "stated"),
            ("stamped_by", "coordinators"),
            ("stamp", WHOLE),
        ])]);
        assert!(f.is_empty(), "{q}: {f:?}");
    }
}

#[test]
fn a_stamp_with_no_owner_is_half_a_stamp() {
    let f = run(&[("proposal::p", &[("stamp", WHOLE)])]);
    assert_eq!(kinds(&f), ["half"], "{f:?}");
}

#[test]
fn an_owner_with_no_stamp_is_half_a_stamp() {
    let f = run(&[("proposal::p", &[("stamped_by", "experts"), ("stamp", "  ")])]);
    assert_eq!(kinds(&f), ["half"], "{f:?}");
}

#[test]
fn a_stamp_naming_no_ticket_is_reported() {
    // The planted case: the stamp recorded only in prose, which no tool can follow to him.
    for s in [
        "Stamped by the coordinators; see PROVISIONAL.md.",
        "Stamped; a ticket was filed.",
        "Stamped; ticket ``.",
    ] {
        let f = run(&[("proposal::p", &[
            ("stamped_by", "coordinators"),
            ("stamp", s),
        ])]);
        assert_eq!(kinds(&f), ["no-ticket"], "{s:?}: {f:?}");
    }
}

#[test]
fn a_stamp_on_a_ratified_row_is_reported() {
    let f = run(&[("ruling::r", &[
        ("rung", "ratified"),
        ("stamped_by", "experts"),
        ("stamp", WHOLE),
    ])]);
    assert_eq!(kinds(&f), ["on-a-ratified-row"], "{f:?}");
}

#[test]
fn a_stamped_row_a_ratified_row_promotes_is_reported() {
    let f = run(&[
        ("ruling::blessing", &[
            ("rung", "ratified"),
            ("ratifies", "other, p"),
        ]),
        ("proposal::p", &[
            ("stamped_by", "coordinators"),
            ("stamp", WHOLE),
        ]),
    ]);
    assert_eq!(kinds(&f), ["ratified-still-stamped"], "{f:?}");
    assert!(
        f[0].message.contains("ruling::blessing"),
        "{}",
        f[0].message
    );
}

#[test]
fn control_a_promotion_below_ratified_leaves_the_stamp_alone() {
    let f = run(&[
        ("ruling::ack", &[("rung", "stated"), ("ratifies", "p")]),
        ("proposal::p", &[
            ("stamped_by", "coordinators"),
            ("stamp", WHOLE),
        ]),
    ]);
    assert!(f.is_empty(), "{f:?}");
}

#[test]
fn control_a_row_with_neither_field_is_not_read() {
    let f = run(&[("ruling::r", &[("rung", "ratified")]), ("proposal::p", &[("says", "x")])]);
    assert!(f.is_empty(), "{f:?}");
}

#[test]
fn a_namespace_not_named_is_not_read() {
    let f = run(&[("law::l", &[("stamp", "no owner")])]);
    assert!(f.is_empty(), "{f:?}");
}

#[test]
fn a_project_names_its_own_words() {
    let mut lint = AProvisionalStampIsNotARatification::default();
    lint.configure(&HashMap::from([
        (NAMESPACES_KEY.to_string(), "tenet, decision".to_string()),
        (TIERED_KEY.to_string(), "decision".to_string()),
        (TIER_FIELD_KEY.to_string(), "status".to_string()),
        (RATIFIED_KEY.to_string(), "settled".to_string()),
    ]));
    let f = run_with(&lint, &[("decision::d", &[
        ("status", "settled"),
        ("stamped_by", "experts"),
        ("stamp", WHOLE),
    ])]);
    assert_eq!(kinds(&f), ["on-a-ratified-row"], "{f:?}");
    let f = run_with(&lint, &[("ruling::r", &[("stamp", "no owner")])]);
    assert!(
        f.is_empty(),
        "the default namespaces were still read: {f:?}"
    );
}

#[test]
fn every_finding_blocks() {
    let f = run(&[("proposal::p", &[("stamp", WHOLE)])]);
    assert!(
        f.iter().all(|e| e.severity == Severity::HARD_ERROR),
        "{f:?}"
    );
}

#[test]
fn it_answers_to_its_name() {
    assert_eq!(AProvisionalStampIsNotARatification::default().name(), NAME);
}
