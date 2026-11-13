//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The counts against a small registry, the probe set's parsing and running, the tool through the
//! engine's context, and the committed probe set against the committed canon.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use mockspace::RegistryView;
use mockspace::tool::{Outcome as ToolOutcome, Tool, ToolContext, ToolReport, contract_faults};

use crate::{CanonLinks, PROBES, Via, counts, parse, run, totals};

type Row<'a> = (&'a str, &'a [(&'a str, &'a str)]);

fn view(rows: &[Row<'_>]) -> RegistryView {
    let rows: BTreeMap<String, BTreeMap<String, String>> = rows
        .iter()
        .map(|(q, fields)| {
            let f = fields
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect();
            ((*q).to_string(), f)
        })
        .collect();
    RegistryView::new(rows, BTreeMap::new())
}

/// `a` cites `b`, `c` links `b`, `d` is joined to nothing, `e` repeats itself and `b`.
fn fixture() -> RegistryView {
    view(&[
        ("tenet::a", &[
            ("subject", "s"),
            ("what", "As b says."),
            ("cites", "b"),
            ("keywords", "dawn, sun"),
        ]),
        ("tenet::b", &[
            ("subject", "s"),
            ("what", "The sun rises."),
            ("keywords", "sun"),
        ]),
        ("tenet::c", &[
            ("subject", "t"),
            ("what", "A day."),
            ("related", "tenet::b"),
        ]),
        ("tenet::d", &[
            ("subject", "t"),
            ("what", "A moonless night."),
        ]),
        ("tenet::e", &[
            ("subject", "t"),
            ("what", "x"),
            ("cites", "b"),
            ("related", "tenet::b, tenet::e"),
        ]),
    ])
}

fn scratch() -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let d = std::env::temp_dir().join(format!(
        "canon-links-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("a scratch mock directory");
    d
}

fn put(dir: &Path, rel: &str, body: &str) {
    let p = dir.join(rel);
    std::fs::create_dir_all(p.parent().expect("a parent")).expect("its directory");
    std::fs::write(p, body).expect("the file");
}

fn ask(reg: &RegistryView, mock: &Path, args: &[&str]) -> ToolReport {
    let crates = BTreeSet::new();
    let ctx = ToolContext {
        mock_dir: mock,
        repo_root: mock,
        all_crates: &crates,
        src_dirs: &[],
        args,
        stdin: None,
        registry: reg,
    };
    CanonLinks.run(&ctx)
}

#[test]
fn the_declaration_is_a_check_the_engine_accepts() {
    assert!(contract_faults(&CanonLinks, None).is_empty());
}

#[test]
fn a_row_is_counted_by_the_distinct_tenets_it_joins_in_each_direction() {
    let reg = fixture();
    let all = canon::tenets(&reg);
    let c = counts(&all);
    let of = |id: &str| c.iter().find(|r| r.id == id).expect("a counted row");
    // `a` cites `b`; `b` is cited by `a`, `c` and `e`, and `e` naming it twice is one tenet
    assert_eq!(
        (of("a").keywords, of("a").outgoing, of("a").incoming),
        (2, 1, 0)
    );
    assert_eq!((of("b").outgoing, of("b").incoming), (0, 3));
    assert_eq!((of("c").outgoing, of("c").incoming), (1, 0));
    // `e` relating to itself is no edge
    assert_eq!((of("e").outgoing, of("e").incoming), (1, 0));
    assert!(of("d").is_island());
    assert!(!of("a").is_island());
}

#[test]
fn the_totals_count_both_kinds_of_edge_and_the_rows_joined_to_nothing() {
    let reg = fixture();
    let all = canon::tenets(&reg);
    let c = counts(&all);
    let every: Vec<_> = c.iter().collect();
    let t = totals(&every, &all);
    assert_eq!((t.rows, t.binding, t.keywords), (5, 5, 3));
    assert_eq!((t.cites_edges, t.link_edges), (2, 3));
    assert_eq!((t.islands, t.no_keywords), (1, 3));
    let t_only: Vec<_> = c.iter().filter(|r| r.subject == "t").collect();
    assert_eq!(totals(&t_only, &all).rows, 3);
}

#[test]
fn the_probe_set_is_refused_where_a_probe_is_malformed() {
    for (text, why) in [
        ("[[probe]]\nfind = \"x\"\nexpect = [\"a\"]\n", "no `ask`"),
        (
            "[[probe]]\nask = \"q\"\nexpect = [\"a\"]\n",
            "asks `find` or `related`",
        ),
        (
            "[[probe]]\nask = \"q\"\nfind = \"x\"\nrelated = \"a\"\nexpect = [\"a\"]\n",
            "asks `find` or `related`",
        ),
        ("[[probe]]\nask = \"q\"\nfind = \"x\"\n", "expects nothing"),
        ("nothing = 1\n", "no `[[probe]]`"),
        ("[[probe", "not TOML"),
    ] {
        let e = parse(text).expect_err(why);
        assert!(e.contains(why), "{e}");
    }
    let ok = parse("[[probe]]\nask = \"q\"\nfind = \"X\"\nexpect = [\"a\"]\n").expect("a probe");
    assert_eq!(ok[0].via, Via::Find("X".to_string()));
}

#[test]
fn a_probe_finds_what_a_word_reaches_and_names_what_it_misses() {
    let reg = fixture();
    let all = canon::tenets(&reg);
    let probes = parse(
        "[[probe]]\nask = \"sun\"\nfind = \"SUN\"\nexpect = [\"a\", \"b\", \"c\"]\n\
         [[probe]]\nask = \"next to b\"\nrelated = \"b\"\nexpect = [\"a\", \"c\", \"e\"]\n\
         [[probe]]\nask = \"next to c\"\nrelated = \"c\"\nexpect = [\"d\"]\n",
    )
    .expect("probes");
    let out = run(&all, &probes);
    // `sun` is a keyword of `a` and `b` and in `b`'s words, and `c` is about a day: the miss is named
    assert_eq!(out[0].found, ["a", "b"]);
    assert_eq!(out[0].missing, ["c"]);
    assert_eq!(out[0].returned, 2);
    // `b` is joined to the three that cite or relate to it, from the other direction
    assert!(out[1].missing.is_empty(), "{:?}", out[1]);
    assert_eq!(out[1].returned, 3);
    // `d` is joined to nothing, so `c` does not reach it
    assert_eq!(out[2].missing, ["d"]);
}

#[test]
fn a_probe_naming_no_tenet_is_flagged_rather_than_counted_as_a_miss() {
    let reg = fixture();
    let all = canon::tenets(&reg);
    let probes = parse(
        "[[probe]]\nask = \"q\"\nfind = \"sun\"\nexpect = [\"a\", \"nosuch\"]\n\
         [[probe]]\nask = \"r\"\nrelated = \"nosuch\"\nexpect = [\"a\"]\n",
    )
    .expect("probes");
    let out = run(&all, &probes);
    assert_eq!(out[0].unknown, ["nosuch"]);
    assert!(out[0].missing.is_empty(), "{:?}", out[0]);
    assert_eq!(out[1].unknown, ["nosuch"]);
}

#[test]
fn the_views_answer_and_an_unknown_one_is_refused() {
    let reg = fixture();
    let mock = scratch();
    let totals = ask(&reg, &mock, &[]);
    assert!(
        matches!(totals.outcome, ToolOutcome::Clean { .. }),
        "{:?}",
        totals.outcome
    );
    assert!(
        totals.output.contains("joined to nothing"),
        "{}",
        totals.output
    );
    assert!(totals.output.contains("\n  d\n"), "{}", totals.output);
    let rows = ask(&reg, &mock, &["rows"]);
    assert_eq!(
        rows.output.lines().filter(|l| l.starts_with("  ")).count(),
        6,
        "{}",
        rows.output
    );
    match ask(&reg, &mock, &["nosuch"]).outcome {
        ToolOutcome::Inconclusive {
            reason,
        } => {
            assert!(reason.contains("totals, rows, probes"), "{reason}")
        },
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_probes_view_reads_the_set_from_the_mock_directory() {
    let reg = fixture();
    let mock = scratch();
    match ask(&reg, &mock, &["probes"]).outcome {
        ToolOutcome::Inconclusive {
            reason,
        } => {
            assert!(reason.contains("could not be read"), "{reason}")
        },
        other => panic!("{other:?}"),
    }
    put(
        &mock,
        PROBES,
        "[[probe]]\nask = \"sun\"\nfind = \"sun\"\nexpect = [\"a\", \"c\"]\n",
    );
    let r = ask(&reg, &mock, &["probes"]);
    assert!(
        matches!(r.outcome, ToolOutcome::Clean { .. }),
        "{:?}",
        r.outcome
    );
    assert!(
        r.output.contains("sun: find sun: 1 of 2 expected"),
        "{}",
        r.output
    );
    assert!(r.output.contains("missing  c"), "{}", r.output);
    assert!(
        r.output.contains("1 probes, 0 answered whole, 1 of 2"),
        "{}",
        r.output
    );
}
