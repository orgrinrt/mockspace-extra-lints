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


// the three routes onto a stamp, from the crate that names them
use mockspace_extra_stamp::ROUTES;

/// A whole stamp for a route: its ticket, and for the maintainer his passage.
fn whole(by: &str) -> &'static str {
    if by == MAINTAINER {
        "He directed it onto a stamp at `research/OP-ANSWERS-18.md:5`; question ticket \
         `abc123def456`."
    } else {
        "Stamped by the route converging; question ticket `abc123def456`."
    }
}

#[test]
fn a_whole_stamp_on_an_unratified_row_passes() {
    for by in ROUTES {
        for q in ["ruling::r", "proposal::p"] {
            let f = run(&[(q, &[
                ("rung", "stated"),
                ("stamped_by", by),
                ("stamp", whole(by)),
            ])]);
            assert!(f.is_empty(), "{by} {q}: {f:?}");
        }
    }
}

#[test]
fn a_stamp_with_no_owner_is_half_a_stamp() {
    let f = run(&[("proposal::p", &[("stamp", whole("experts"))])]);
    assert_eq!(kinds(&f), ["half"], "{f:?}");
}

#[test]
fn an_owner_with_no_stamp_is_half_a_stamp() {
    for by in ROUTES {
        let f = run(&[("proposal::p", &[("stamped_by", by), ("stamp", "  ")])]);
        assert_eq!(kinds(&f), ["half"], "{by}: {f:?}");
    }
}

#[test]
fn a_stamp_naming_no_ticket_is_reported() {
    // The planted case: the stamp recorded only in prose, which no tool can follow to him.
    for by in ROUTES {
        for s in [
            "Stamped; see PROVISIONAL.md and `research/OP-ANSWERS-18.md:5`.",
            "Stamped; a ticket was filed, `research/OP-ANSWERS-18.md:5`.",
            "Stamped; ticket ``, `research/OP-ANSWERS-18.md:5`.",
        ] {
            let f = run(&[("proposal::p", &[("stamped_by", by), ("stamp", s)])]);
            assert_eq!(kinds(&f), ["no-ticket"], "{by} {s:?}: {f:?}");
        }
    }
}

#[test]
fn a_maintainer_stamp_citing_no_passage_is_reported() {
    for s in [
        "Stamped, ticket `000000000000`.",
        "He said so in OP-ANSWERS-18.md; ticket `000000000000`.",
        "He said so in `OP-ANSWERS-18.md`; ticket `000000000000`.",
        "He said so at `line:5`; ticket `000000000000`.",
    ] {
        let f = run(&[("proposal::p", &[("stamped_by", "maintainer"), ("stamp", s)])]);
        assert_eq!(kinds(&f), ["no-passage"], "{s:?}: {f:?}");
    }
}

#[test]
fn control_a_passage_with_a_line_range_is_a_passage() {
    let s = "He directed it at `research/OP-ANSWERS-18.md:5-9`; ticket `000000000000`.";
    let f = run(&[("proposal::p", &[("stamped_by", "maintainer"), ("stamp", s)])]);
    assert!(f.is_empty(), "{f:?}");
}

#[test]
fn control_a_convergence_owes_no_passage() {
    for by in ["coordinators", "experts"] {
        let f = run(&[("proposal::p", &[
            ("stamped_by", by),
            ("stamp", "Stamped, ticket `000000000000`."),
        ])]);
        assert!(f.is_empty(), "{by}: {f:?}");
    }
}

#[test]
fn a_stamp_on_a_ratified_row_is_reported() {
    for by in ROUTES {
        let f = run(&[("ruling::r", &[
            ("rung", "ratified"),
            ("stamped_by", by),
            ("stamp", whole(by)),
        ])]);
        assert_eq!(kinds(&f), ["on-a-ratified-row"], "{by}: {f:?}");
    }
}

/// A lint whose registry declares which tiers a stamp may sit on.
fn with_stampable() -> AProvisionalStampIsNotARatification {
    let mut lint = AProvisionalStampIsNotARatification::default();
    lint.configure(&HashMap::from([
        (TIER_FIELD_KEY.to_string(), "standing".to_string()),
        (RATIFIED_KEY.to_string(), "ratified".to_string()),
        (STAMPABLE_KEY.to_string(), "provisional".to_string()),
    ]));
    lint
}

#[test]
fn a_stamp_on_each_standing_but_the_stampable_one_is_reported() {
    let lint = with_stampable();
    for by in ROUTES {
        for (standing, kind) in [
            ("stated", Some("not-stampable")),
            ("open", Some("not-stampable")),
            ("held", Some("not-stampable")),
            ("ratified", Some("on-a-ratified-row")),
            ("provisional", None),
        ] {
            let f = run_with(&lint, &[("ruling::r", &[
                ("standing", standing),
                ("stamped_by", by),
                ("stamp", whole(by)),
            ])]);
            assert_eq!(kinds(&f), kind.into_iter().collect::<Vec<_>>(), "{by} {standing}: {f:?}");
        }
    }
}

#[test]
fn control_with_no_stampable_list_every_tier_but_ratified_takes_a_stamp() {
    for tier in ["stated", "open", "in_force"] {
        let f = run(&[("ruling::r", &[
            ("rung", tier),
            ("stamped_by", "experts"),
            ("stamp", whole("experts")),
        ])]);
        assert!(f.is_empty(), "{tier}: {f:?}");
    }
}

#[test]
fn a_stamp_on_a_superseded_row_is_reported() {
    for by in ROUTES {
        for newer in ["ruling::newer", "proposal::newer"] {
            let f = run(&[
                (newer, &[("supersedes", "other, p")]),
                ("proposal::p", &[("stamped_by", by), ("stamp", whole(by))]),
            ]);
            assert_eq!(kinds(&f), ["on-a-superseded-row"], "{by} {newer}: {f:?}");
            assert!(f[0].message.contains(newer), "{}", f[0].message);
        }
    }
}

#[test]
fn control_a_row_superseding_another_may_carry_a_stamp() {
    let f = run(&[
        ("proposal::newer", &[
            ("supersedes", "old"),
            ("stamped_by", "experts"),
            ("stamp", whole("experts")),
        ]),
        ("proposal::old", &[("says", "x")]),
    ]);
    assert!(f.is_empty(), "{f:?}");
}

#[test]
fn a_stamp_on_a_row_naming_its_heir_is_reported() {
    // The relation as the pack's tenet schema writes it: on the replaced row, naming what replaces it.
    for by in ROUTES {
        for heir in ["newer", "tenet::newer", "other, newer"] {
            let f = run(&[
                ("proposal::newer", &[("says", "x")]),
                ("proposal::p", &[
                    ("superseded_by", heir),
                    ("stamped_by", by),
                    ("stamp", whole(by)),
                ]),
            ]);
            let want = heir.split(", ").map(|_| "on-a-superseded-row").collect::<Vec<_>>();
            assert_eq!(kinds(&f), want, "{by} {heir}: {f:?}");
            assert!(f.iter().any(|e| e.message.contains("newer")), "{f:?}");
        }
    }
}

#[test]
fn a_relation_written_at_both_ends_is_one_finding() {
    let f = run(&[
        ("proposal::newer", &[("supersedes", "p")]),
        ("proposal::p", &[
            ("superseded_by", "newer"),
            ("stamped_by", "experts"),
            ("stamp", whole("experts")),
        ]),
    ]);
    assert_eq!(kinds(&f), ["on-a-superseded-row"], "{f:?}");
}

#[test]
fn control_the_heir_named_by_a_replaced_row_may_carry_a_stamp() {
    // The other direction: the row that replaces is read as unreplaced, so its stamp stands.
    let f = run(&[
        ("proposal::old", &[("superseded_by", "newer")]),
        ("proposal::newer", &[("stamped_by", "experts"), ("stamp", whole("experts"))]),
    ]);
    assert!(f.is_empty(), "{f:?}");
}

#[test]
fn control_a_blank_superseded_by_supersedes_nothing() {
    for blank in ["", "  ", ","] {
        let f = run(&[("proposal::p", &[
            ("superseded_by", blank),
            ("stamped_by", "experts"),
            ("stamp", whole("experts")),
        ])]);
        assert!(f.is_empty(), "{blank:?}: {f:?}");
    }
}

#[test]
fn a_project_names_its_own_superseded_by_field() {
    let mut lint = AProvisionalStampIsNotARatification::default();
    lint.configure(&HashMap::from([(SUPERSEDED_BY_KEY.to_string(), "replaced_by".to_string())]));
    let stamped = |field| -> Vec<LintError> {
        run_with(&lint, &[("proposal::p", &[
            (field, "newer"),
            ("stamped_by", "experts"),
            ("stamp", whole("experts")),
        ])])
    };
    assert_eq!(kinds(&stamped("replaced_by")), ["on-a-superseded-row"]);
    assert!(stamped("superseded_by").is_empty(), "the default field was still read");
}

#[test]
fn a_stamped_row_a_ratified_row_promotes_is_reported() {
    for by in ROUTES {
        let f = run(&[
            ("ruling::blessing", &[
                ("rung", "ratified"),
                ("ratifies", "other, p"),
            ]),
            ("proposal::p", &[("stamped_by", by), ("stamp", whole(by))]),
        ]);
        assert_eq!(kinds(&f), ["ratified-still-stamped"], "{by}: {f:?}");
        assert!(f[0].message.contains("ruling::blessing"), "{}", f[0].message);
    }
}

#[test]
fn control_a_promotion_below_ratified_leaves_the_stamp_alone() {
    for by in ROUTES {
        let f = run(&[
            ("ruling::ack", &[("rung", "stated"), ("ratifies", "p")]),
            ("proposal::p", &[("stamped_by", by), ("stamp", whole(by))]),
        ]);
        assert!(f.is_empty(), "{by}: {f:?}");
    }
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
        (SUPERSEDES_KEY.to_string(), "replaces".to_string()),
    ]));
    let f = run_with(&lint, &[("decision::d", &[
        ("status", "settled"),
        ("stamped_by", "experts"),
        ("stamp", whole("experts")),
    ])]);
    assert_eq!(kinds(&f), ["on-a-ratified-row"], "{f:?}");
    let f = run_with(&lint, &[
        ("tenet::new", &[("replaces", "d")]),
        ("decision::d", &[("stamped_by", "experts"), ("stamp", whole("experts"))]),
    ]);
    assert_eq!(kinds(&f), ["on-a-superseded-row"], "{f:?}");
    let f = run_with(&lint, &[("ruling::r", &[("stamp", "no owner")])]);
    assert!(f.is_empty(), "the default namespaces were still read: {f:?}");
}

#[test]
fn every_finding_blocks() {
    // One planted row per kind, so a kind that stopped blocking is caught.
    let plants: [(&str, Vec<(&str, &[(&str, &str)])>); 7] = [
        ("half", vec![("proposal::p", &[("stamp", "x")])]),
        ("no-ticket", vec![("proposal::p", &[("stamped_by", "experts"), ("stamp", "x")])]),
        ("no-passage", vec![("proposal::p", &[
            ("stamped_by", "maintainer"),
            ("stamp", "ticket `000000000000`"),
        ])]),
        ("on-a-ratified-row", vec![("ruling::r", &[
            ("rung", "ratified"),
            ("stamped_by", "experts"),
            ("stamp", "ticket `000000000000`"),
        ])]),
        ("not-stampable", vec![]),
        ("on-a-superseded-row", vec![
            ("ruling::n", &[("supersedes", "p")]),
            ("proposal::p", &[("stamped_by", "experts"), ("stamp", "ticket `000000000000`")]),
        ]),
        ("ratified-still-stamped", vec![
            ("ruling::b", &[("rung", "ratified"), ("ratifies", "p")]),
            ("proposal::p", &[("stamped_by", "experts"), ("stamp", "ticket `000000000000`")]),
        ]),
    ];
    for (kind, rows) in plants {
        let f = if kind == "not-stampable" {
            run_with(&with_stampable(), &[("ruling::r", &[
                ("standing", "stated"),
                ("stamped_by", "experts"),
                ("stamp", "ticket `000000000000`"),
            ])])
        } else {
            run(&rows)
        };
        assert_eq!(kinds(&f), [kind], "{kind}: {f:?}");
        assert!(f.iter().all(|e| e.severity == Severity::HARD_ERROR), "{kind}: {f:?}");
    }
}
