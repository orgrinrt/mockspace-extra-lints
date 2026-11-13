//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Each verb against a small registry and a scratch tree, then two questions put to the committed
//! canon: whether every tenet that existed before the canon was given subjects still does, and
//! whether asking for a subject returns the tenets it holds.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use mockspace::RegistryView;
use mockspace::tool::{Outcome, ToolContext, ToolReport, contract_faults, missing_required};

use crate::{Canon, Tier, mentions, named, tier_of};

pub(crate) type Row<'a> = (&'a str, &'a [(&'a str, &'a str)]);

pub(crate) fn view(rows: &[Row<'_>]) -> RegistryView {
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

/// Two subjects, four tenets: one superseded, one citing another, one named nowhere and the only one
/// provisional.
pub(crate) fn fixture() -> RegistryView {
    view(&[
        ("subject::sky", &[("what", "The sky.")]),
        ("subject::time", &[("what", "The clock.")]),
        ("tenet::the_sun_rises", &[
            ("standing", "ratified"),
            ("subject", "sky"),
            ("what", "The sun rises in the east."),
            ("keywords", "sun, dawn"),
        ]),
        ("tenet::the_sun_rose", &[
            ("standing", "ratified"),
            ("subject", "sky"),
            ("what", "The sun rose once."),
            ("superseded_by", "the_sun_rises"),
        ]),
        ("tenet::days_follow_the_sun", &[
            ("standing", "ratified"),
            ("subject", "time"),
            ("what", "A day is the_sun_rises again."),
            ("cites", "the_sun_rises"),
            ("keywords", "day"),
        ]),
        ("tenet::nobody_names_this", &[
            ("standing", "provisional"),
            ("subject", "time"),
            ("what", "A Moonless Night."),
            ("related", "tenet::the_sun_rises"),
        ]),
    ])
}

pub(crate) fn scratch() -> PathBuf {
    static N: AtomicU32 = AtomicU32::new(0);
    let d = std::env::temp_dir().join(format!(
        "canon-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("a scratch repository");
    d
}

pub(crate) fn write(repo: &Path, rel: &str, body: &str) {
    let p = repo.join(rel);
    std::fs::create_dir_all(p.parent().expect("a parent")).expect("its directory");
    std::fs::write(p, body).expect("the file");
}

pub(crate) fn run(reg: &RegistryView, repo: &Path, args: &[&str]) -> ToolReport {
    let crates = BTreeSet::new();
    let ctx = ToolContext {
        mock_dir: &repo.join("mock"),
        repo_root: repo,
        all_crates: &crates,
        src_dirs: &[],
        args,
        stdin: None,
        registry: reg,
    };
    Canon.run_with(&ctx, &|_| Vec::new())
}

pub(crate) fn answered(r: &ToolReport) -> &str {
    assert!(
        matches!(r.outcome, Outcome::Clean { .. }),
        "{:?}",
        r.outcome
    );
    &r.output
}

/// The tenet ids an answer lists, one per line, as the first word of an indented line.
fn listed(text: &str) -> Vec<&str> {
    text.lines()
        .filter(|l| l.starts_with("  "))
        .filter_map(|l| l.split_whitespace().next())
        .collect()
}

#[test]
fn the_declaration_is_a_check_the_engine_accepts() {
    assert!(
        contract_faults(&Canon, None).is_empty(),
        "{:?}",
        contract_faults(&Canon, None)
    );
}

#[test]
fn a_missing_or_unknown_verb_is_refused_naming_the_verbs() {
    let missing: Vec<&str> = missing_required(&Canon, &[])
        .iter()
        .map(|a| a.name)
        .collect();
    assert_eq!(missing, ["verb"]);
    let reg = fixture();
    let repo = scratch();
    for args in [&[][..], &["nosuch"][..]] {
        match run(&reg, &repo, args).outcome {
            Outcome::Inconclusive {
                reason,
            } => {
                assert!(reason.contains("uncited"), "{reason}");
            },
            other => panic!("{other:?}"),
        }
    }
}

#[test]
fn subjects_counts_each_subject_s_tenets_and_how_many_bind() {
    let reg = fixture();
    let out = run(&reg, &scratch(), &["subjects"]);
    let text = answered(&out);
    let sky = text
        .lines()
        .find(|l| l.trim_start().starts_with("sky"))
        .expect("sky");
    assert!(
        sky.contains("2 tenets") && sky.contains("1 binding"),
        "{sky}"
    );
    let time = text
        .lines()
        .find(|l| l.trim_start().starts_with("time"))
        .expect("time");
    assert!(
        time.contains("2 tenets") && time.contains("2 binding"),
        "{time}"
    );
}

#[test]
fn a_subject_lists_exactly_its_tenets_and_marks_the_superseded() {
    let reg = fixture();
    let out = run(&reg, &scratch(), &["subject", "sky"]);
    let text = answered(&out);
    assert_eq!(listed(text), ["the_sun_rises", "the_sun_rose"]);
    let rose = text
        .lines()
        .find(|l| l.contains("the_sun_rose "))
        .expect("listed");
    assert!(rose.contains("superseded by the_sun_rises"), "{rose}");
    match run(&reg, &scratch(), &["subject", "weather"]).outcome {
        Outcome::Inconclusive {
            reason,
        } => assert!(reason.contains("sky, time"), "{reason}"),
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        run(&reg, &scratch(), &["subject"]).outcome,
        Outcome::Inconclusive { .. }
    ));
}

#[test]
fn find_matches_an_id_a_keyword_or_the_words_ignoring_case() {
    let reg = fixture();
    let repo = scratch();
    assert_eq!(listed(answered(&run(&reg, &repo, &["find", "DAWN"]))), [
        "the_sun_rises"
    ]);
    assert_eq!(
        listed(answered(&run(&reg, &repo, &["find", "moonless"]))),
        ["nobody_names_this"]
    );
    assert_eq!(listed(answered(&run(&reg, &repo, &["find", "sun"]))), [
        "days_follow_the_sun",
        "the_sun_rises",
        "the_sun_rose"
    ]);
    assert!(listed(answered(&run(&reg, &repo, &["find", "comet"]))).is_empty());
}

#[test]
fn related_gives_both_directions_of_both_relations() {
    let reg = fixture();
    let out = run(&reg, &scratch(), &["related", "the_sun_rises"]);
    let text = answered(&out);
    let line = |k: &str| {
        text.lines()
            .find(|l| l.trim_start().starts_with(k))
            .unwrap_or_else(|| panic!("no `{k}` in {text}"))
            .to_string()
    };
    assert!(line("subject").contains("sky"));
    assert!(line("cites:").ends_with("nothing"), "{}", line("cites:"));
    assert!(line("cited by:").contains("days_follow_the_sun"));
    assert!(
        line("links to:").ends_with("nothing"),
        "{}",
        line("links to:")
    );
    assert!(line("related by:").contains("nobody_names_this"));
    assert!(line("supersedes:").contains("the_sun_rose"));
    assert!(line("superseded by:").ends_with("nothing"));
    let from = run(&reg, &scratch(), &["related", "nobody_names_this"]);
    assert!(
        answered(&from)
            .lines()
            .any(|l| l.trim_start().starts_with("links to:") && l.ends_with("the_sun_rises")),
        "{}",
        from.output
    );
    match run(&reg, &scratch(), &["related", "no_such_tenet"]).outcome {
        Outcome::Inconclusive {
            reason,
        } => assert!(reason.contains("no_such_tenet"), "{reason}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_name_is_a_whole_id_wherever_it_is_written() {
    let known: BTreeSet<&str> = ["the_sun", "the_sun_rises"].into_iter().collect();
    assert_eq!(named("after `the_sun_rises`, then the_sun.", &known), [
        "the_sun_rises",
        "the_sun"
    ]);
    assert_eq!(named("{{ tenet::the_sun }} and tenet::the_sun", &known), [
        "the_sun"
    ]);
    assert!(named("the_sun_rises_late and xthe_sun", &known).is_empty());
}

#[test]
fn a_file_s_tier_follows_where_it_sits() {
    for (p, t) in [
        ("mock/crates/kaski-starsystem/DESIGN.md.tmpl", Tier::Design),
        ("mock/WORKFLOW.md.tmpl", Tier::Design),
        ("mock/crates/kaski-starsystem/src/lib.rs", Tier::Code),
        ("mock/lints/slang_fresh.rs", Tier::Code),
        ("mock/research/x/NOTES.md", Tier::Research),
        ("mock/design_rounds/1_topic.a.md", Tier::Rounds),
        ("content/kaski/items.toml", Tier::Content),
        ("mock/registry/tenet/sky.toml", Tier::Canon),
        ("mockspace.toml", Tier::Other),
    ] {
        assert_eq!(tier_of(Path::new(p)), t, "{p}");
    }
}

/// A tree naming `the_sun_rises` once in each tier, and in the places that are not read.
pub(crate) fn tree() -> PathBuf {
    let repo = scratch();
    write(
        &repo,
        "mock/crates/a/DESIGN.md.tmpl",
        "Rests on the_sun_rises.\n",
    );
    write(
        &repo,
        "mock/crates/a/src/lib.rs",
        "// after `the_sun_rises`\n",
    );
    write(
        &repo,
        "mock/research/r/NOTES.md",
        "line one\nthe_sun_rises\n",
    );
    write(&repo, "mock/design_rounds/1_topic.t.md", "the_sun_rises\n");
    write(&repo, "content/kaski/x.toml", "# the_sun_rises\n");
    write(
        &repo,
        "mock/registry/tenet/sky.toml",
        "[[tenet]]\nid = \"the_sun_rises\"\n\n[[tenet]]\nid = \"days_follow_the_sun\"\ncites = [\"the_sun_rises\"]\n",
    );
    write(
        &repo,
        "docs/TENET.md",
        "the_sun_rises the_sun_rose nobody_names_this\n",
    );
    write(&repo, "mock/target/x.rs", "the_sun_rose\n");
    write(&repo, "mock/research/r/ids.txt", "nobody_names_this\n");
    repo
}

#[test]
fn citers_finds_every_tier_and_skips_what_is_generated_or_built() {
    let reg = fixture();
    let repo = tree();
    let known: BTreeSet<&str> = ["the_sun_rises", "the_sun_rose", "nobody_names_this"]
        .into_iter()
        .collect();
    let found = mentions(&repo, &known);
    let tiers: Vec<Tier> = found["the_sun_rises"].iter().map(|l| l.tier).collect();
    assert_eq!(
        tiers.iter().copied().collect::<BTreeSet<_>>(),
        [
            Tier::Design,
            Tier::Code,
            Tier::Research,
            Tier::Rounds,
            Tier::Content,
            Tier::Canon
        ]
        .into_iter()
        .collect(),
    );
    assert_eq!(
        tiers.len(),
        6,
        "the row's own `id =` line is not a citation"
    );
    assert!(!found.contains_key("the_sun_rose") && !found.contains_key("nobody_names_this"));
    let research = found["the_sun_rises"]
        .iter()
        .find(|l| l.tier == Tier::Research)
        .expect("research");
    assert_eq!(research.line, 2);

    let out = run(&reg, &repo, &["citers", "the_sun_rises"]);
    let text = answered(&out);
    assert!(text.contains("named 6 times"), "{text}");
    for tier in ["design", "code", "research", "rounds", "content", "canon"] {
        assert!(
            text.lines().any(|l| l == tier),
            "no `{tier}` heading in {text}"
        );
    }
    assert!(text.contains("mock/research/r/NOTES.md:2"), "{text}");
}

#[test]
fn uncited_lists_the_binding_tenets_nothing_names() {
    let reg = fixture();
    let out = run(&reg, &tree(), &["uncited"]);
    // `the_sun_rose` is named nowhere read but no longer binds; `days_follow_the_sun` is named only
    // by its own `id =` line; `the_sun_rises` is named everywhere
    assert_eq!(listed(answered(&out)), [
        "days_follow_the_sun",
        "nobody_names_this"
    ]);
}

// --- the committed canon -------------------------------------------------------------------------
