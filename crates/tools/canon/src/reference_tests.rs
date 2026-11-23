//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `find`, `related`, `citers` and `show` read every namespace of the registry, and narrow only when
//! `--in` says which. A reference namespace such as `ikiuni-renderer` is asked with the same verbs as the
//! tenets, and `--in canon` narrows to the namespaces `canon_paths` names.

use std::collections::BTreeMap;
use std::path::Path;

use mockspace::RegistryView;
use mockspace::tool::Outcome;

use crate::tests::{Row, answered, run, scratch, write};

/// The fixture's reverse edges, by the row they point at.
pub(crate) const EDGES: &[(&str, &[&str])] = &[
    ("ikiuni-renderer::a_sky_pass", &[
        "ikiuni-renderer::a_sun_disc",
        "tenet::the_sun_rises",
    ]),
    ("ikiuni::scaffold", &["ikiuni-renderer::a_sky_pass"]),
    ("tenet::the_sun_rises", &["tenet::days_follow_the_sun"]),
];

pub(crate) const ROWS: &[Row<'static>] = &[
    ("subject::sky", &[("what", "The sky.")]),
    ("subject::time", &[("what", "The clock.")]),
    ("tenet::the_sun_rises", &[
        ("standing", "ratified"),
        ("subject", "sky"),
        ("what", "The sun rises in the east."),
        ("keywords", "sun, dawn"),
        ("ikiuni_renderer", "a_sky_pass"),
    ]),
    ("tenet::days_follow_the_sun", &[
        ("standing", "ratified"),
        ("subject", "time"),
        ("what", "A day is the_sun_rises again."),
        ("cites", "the_sun_rises"),
    ]),
    ("ikiuni-renderer::a_sky_pass", &[
        ("kind", "technique"),
        ("tier", "canon"),
        ("name", "Sky pass"),
        ("doc", "DESIGN.md:3-9"),
        ("rev", "243a537d6df9"),
        (
            "quote",
            "## The sky pass\n\nThe sun is drawn once, as a disc.\n",
        ),
        ("keywords", "sky, atmosphere, scaffold"),
        ("ikiuni", "scaffold"),
    ]),
    ("ikiuni-renderer::a_sun_disc", &[
        ("kind", "trick"),
        ("tier", "design"),
        ("name", "Sun disc"),
        ("doc", "DESIGN.md:40-44"),
        ("quote", "A disc of the sun.\n"),
        ("related", "ikiuni-renderer::a_sky_pass"),
    ]),
    ("ikiuni::scaffold", &[
        ("what", "A scaffold is worked to completion."),
        ("doc", "300.md:12"),
        ("layer", "300"),
    ]),
    ("ikiuni::the_sun_rises", &[
        ("what", "A sun rises on a schedule."),
        ("doc", "100.md:5"),
        ("layer", "100"),
    ]),
    ("bevy::sun_light", &[
        ("what", "Directional light, the sun."),
        ("kind", "component"),
        ("quote", "pub struct DirectionalLight"),
    ]),
];

/// Tenets, subjects and three reference namespaces, with the edges the engine would compute.
pub(crate) fn wide() -> RegistryView {
    let rows: BTreeMap<String, BTreeMap<String, String>> = ROWS
        .iter()
        .map(|(q, fields)| {
            let f = fields
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect();
            ((*q).to_string(), f)
        })
        .collect();
    let referrers = EDGES
        .iter()
        .map(|(to, from)| {
            (
                (*to).to_string(),
                from.iter().map(|s| (*s).to_string()).collect(),
            )
        })
        .collect();
    RegistryView::new(rows, referrers)
}

pub(crate) fn refused(reg: &RegistryView, repo: &Path, args: &[&str]) -> String {
    match run(reg, repo, args).outcome {
        Outcome::Inconclusive {
            reason,
        } => reason,
        other => panic!("{other:?}"),
    }
}

pub(crate) fn line_of<'a>(text: &'a str, key: &str) -> &'a str {
    text.lines()
        .find(|l| l.trim_start().starts_with(key))
        .unwrap_or_else(|| panic!("no `{key}` line in {text}"))
}

// --- find ----------------------------------------------------------------------------------------

#[test]
fn a_filter_naming_no_namespace_is_refused_with_the_ones_there_are() {
    let reg = wide();
    let repo = scratch();
    let why = refused(&reg, &repo, &["show", "sun", "--in", "nosuch"]);
    assert!(why.contains("`nosuch`"), "{why}");
    for ns in ["bevy", "ikiuni", "ikiuni-renderer", "subject", "tenet", "canon"] {
        assert!(why.contains(ns), "`{ns}` missing from {why}");
    }
    assert!(refused(&reg, &repo, &["show", "sun", "--in", "bevy,nosuch"]).contains("`nosuch`"));
    assert!(refused(&reg, &repo, &["show", "sun", "--in"]).contains("--in"));
    assert!(refused(&reg, &repo, &["show", "sun", "--in", ","]).contains("--in"));
}

#[test]
fn a_filter_is_refused_by_the_verbs_that_are_about_tenets_alone() {
    let reg = wide();
    let repo = scratch();
    for verb in ["subjects", "uncited", "forbids", "contradicts"] {
        let why = refused(&reg, &repo, &[verb, "--in", "tenet"]);
        assert!(why.contains("citers and show"), "{verb}: {why}");
    }
}

// --- the canon namespaces ------------------------------------------------------------------------

#[test]
fn in_canon_is_refused_where_the_repository_declares_no_canon() {
    let reg = wide();
    let none = scratch();
    let why = refused(&reg, &none, &["show", "sun", "--in", "canon"]);
    assert!(why.contains("canon_paths"), "{why}");
    write(&none, "mockspace.toml", "mock_dir = \"mock\"\n");
    assert!(refused(&reg, &none, &["show", "sun", "--in", "canon"]).contains("canon_paths"));
}

// --- show ----------------------------------------------------------------------------------------

#[test]
fn show_prints_a_reference_row_with_its_fields_its_doc_and_its_quote() {
    let reg = wide();
    let repo = scratch();
    let text = answered(&run(&reg, &repo, &["show", "a_sky_pass"])).to_owned();
    assert!(text.starts_with("ikiuni-renderer::a_sky_pass\n"), "{text}");
    for (k, v) in [
        ("kind", "technique"),
        ("tier", "canon"),
        ("name", "Sky pass"),
        ("doc", "DESIGN.md:3-9"),
    ] {
        let l = line_of(&text, &format!("{k}:"));
        assert!(l.trim_end().ends_with(v), "{k}: {l}");
    }
    assert!(
        text.contains("  quote:\n    ## The sky pass\n\n    The sun is drawn once, as a disc.\n"),
        "{text}"
    );
    // the fields come first and the doc and the quote after them, the quote last
    let at = |k: &str| text.find(k).unwrap_or_else(|| panic!("no `{k}` in {text}"));
    assert!(
        at("  tier:") < at("  doc:") && at("  doc:") < at("  quote:"),
        "{text}"
    );
    // the same row by its qualified name, and the id line is the header alone
    let by_name = answered(&run(&reg, &repo, &["show", "ikiuni-renderer::a_sky_pass"])).to_owned();
    assert_eq!(by_name, text);
    assert!(
        !text.lines().any(|l| l.trim_start().starts_with("id:")),
        "{text}"
    );
}

#[test]
fn show_prints_a_tenet_and_a_subject_as_well() {
    let reg = wide();
    let repo = scratch();
    let text = answered(&run(&reg, &repo, &["show", "tenet::the_sun_rises"])).to_owned();
    assert!(text.starts_with("tenet::the_sun_rises\n"), "{text}");
    assert!(line_of(&text, "what:").ends_with("The sun rises in the east."));
    assert!(line_of(&text, "ikiuni_renderer:").ends_with("a_sky_pass"));
    let sub = answered(&run(&reg, &repo, &["show", "subject::time"])).to_owned();
    assert!(line_of(&sub, "what:").ends_with("The clock."));
}

#[test]
fn a_bare_id_in_two_namespaces_is_refused_until_one_is_named() {
    let reg = wide();
    let repo = scratch();
    let why = refused(&reg, &repo, &["show", "the_sun_rises"]);
    assert!(
        why.contains("ikiuni::the_sun_rises") && why.contains("tenet::the_sun_rises"),
        "{why}"
    );
    let narrowed = answered(&run(&reg, &repo, &[
        "show",
        "the_sun_rises",
        "--in",
        "ikiuni",
    ]))
    .to_owned();
    assert!(
        narrowed.starts_with("ikiuni::the_sun_rises\n"),
        "{narrowed}"
    );
    let named = answered(&run(&reg, &repo, &["show", "ikiuni::the_sun_rises"])).to_owned();
    // what is read differs with the filter, so the two answers agree up to their closing line
    let body = |t: &str| {
        t.rsplit_once("\nread ")
            .map_or(t.to_owned(), |(b, _)| b.to_owned())
    };
    assert_eq!(body(&named), body(&narrowed));
}

#[test]
fn show_refuses_what_is_no_row_or_is_outside_the_filter() {
    let reg = wide();
    let repo = scratch();
    assert!(refused(&reg, &repo, &["show", "no_such_row"]).contains("`no_such_row`"));
    assert!(refused(&reg, &repo, &["show", "nosuch::a_sky_pass"]).contains("`nosuch::a_sky_pass`"));
    assert!(
        refused(&reg, &repo, &["show", "ikiuni-renderer::nope"])
            .contains("`ikiuni-renderer::nope`")
    );
    let outside = refused(&reg, &repo, &["show", "a_sky_pass", "--in", "tenet"]);
    assert!(
        outside.contains("`a_sky_pass`") && outside.contains("tenet"),
        "{outside}"
    );
    assert!(refused(&reg, &repo, &["show"]).contains("wants a row"));
}
