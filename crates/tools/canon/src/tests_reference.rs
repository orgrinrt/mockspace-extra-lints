//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `find`, `related`, `citers` and `show` read every namespace of the registry, and narrow only when
//! `--in` says which. A reference namespace such as `ikiuni-renderer` is asked with the same verbs as the
//! tenets, and `--in canon` narrows to the namespaces `canon_paths` names.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mockspace::RegistryView;
use mockspace::tool::Outcome;

use crate::tests::{Row, answered, run, scratch, view, write};

/// The fixture's reverse edges, by the row they point at.
const EDGES: &[(&str, &[&str])] = &[
    ("ikiuni-renderer::a_sky_pass", &[
        "ikiuni-renderer::a_sun_disc",
        "tenet::the_sun_rises",
    ]),
    ("ikiuni::scaffold", &["ikiuni-renderer::a_sky_pass"]),
    ("tenet::the_sun_rises", &["tenet::days_follow_the_sun"]),
];

const ROWS: &[Row<'static>] = &[
    ("subject::sky", &[("what", "The sky.")]),
    ("subject::time", &[("what", "The clock.")]),
    ("tenet::the_sun_rises", &[
        ("subject", "sky"),
        ("what", "The sun rises in the east."),
        ("keywords", "sun, dawn"),
        ("ikiuni_renderer", "a_sky_pass"),
    ]),
    ("tenet::days_follow_the_sun", &[
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
fn wide() -> RegistryView {
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

/// A repository whose `canon_paths` reach the subjects and the tenets, as kaski's do, and not the
/// reference namespaces.
fn canon_repo() -> PathBuf {
    let repo = scratch();
    write(
        &repo,
        "mockspace.toml",
        "canon_paths = [\"mock/registry/*.toml\", \"mock/registry/tenet/*.toml\", \
         \"mock/registry/tenet/**/*.toml\"]\n",
    );
    write(
        &repo,
        "mock/registry/subject.toml",
        "[[subject]]\nid = \"sky\"\n",
    );
    write(
        &repo,
        "mock/registry/tenet/sky.toml",
        "[[tenet]]\nid = \"the_sun_rises\"\n",
    );
    write(
        &repo,
        "mock/registry/tenet/deeper/more.toml",
        "[[tenet]]\nid = \"days_follow_the_sun\"\n",
    );
    write(
        &repo,
        "mock/registry/ikiuni-renderer/100_core.toml",
        "[[ikiuni-renderer]]\nid = \"a_sky_pass\"\n",
    );
    write(
        &repo,
        "mock/registry/bevy/100.toml",
        "[[bevy]]\nid = \"sun_light\"\n",
    );
    repo
}

/// What an answer lists, by section: the header line and the ids under it.
fn sections(text: &str) -> Vec<(String, Vec<String>)> {
    let mut out: Vec<(String, Vec<String>)> = Vec::new();
    for l in text.lines() {
        if l.is_empty() || l.starts_with("read ") {
            continue;
        }
        if let Some(entry) = l.strip_prefix("  ") {
            let id = entry.split_whitespace().next().unwrap_or_default();
            out.last_mut()
                .expect("a header first")
                .1
                .push(id.to_owned());
        } else {
            out.push((l.to_owned(), Vec::new()));
        }
    }
    out
}

/// The ids listed under the section for `ns`, or nothing where it has none.
fn listed_in(text: &str, ns: &str) -> Vec<String> {
    let want = if ns == "tenet" {
        " tenets carry ".to_owned()
    } else {
        format!(" {ns} rows carry ")
    };
    sections(text)
        .into_iter()
        .find(|(h, _)| h.contains(&want))
        .map(|(_, ids)| ids)
        .unwrap_or_default()
}

fn refused(reg: &RegistryView, repo: &Path, args: &[&str]) -> String {
    match run(reg, repo, args).outcome {
        Outcome::Inconclusive {
            reason,
        } => reason,
        other => panic!("{other:?}"),
    }
}

fn line_of<'a>(text: &'a str, key: &str) -> &'a str {
    text.lines()
        .find(|l| l.trim_start().starts_with(key))
        .unwrap_or_else(|| panic!("no `{key}` line in {text}"))
}

// --- find ----------------------------------------------------------------------------------------

#[test]
fn find_reads_every_namespace_when_nothing_narrows_it() {
    let text = answered(&run(&wide(), &scratch(), &["find", "sun"])).to_owned();
    assert_eq!(listed_in(&text, "tenet"), [
        "days_follow_the_sun",
        "the_sun_rises"
    ]);
    assert_eq!(listed_in(&text, "ikiuni-renderer"), [
        "a_sky_pass",
        "a_sun_disc"
    ]);
    assert_eq!(listed_in(&text, "ikiuni"), ["the_sun_rises"]);
    assert_eq!(listed_in(&text, "bevy"), ["sun_light"]);
    assert!(
        sections(&text).iter().all(|(h, _)| !h.contains("subject")),
        "no subject carries the word: {text}"
    );
}

#[test]
fn find_matches_a_reference_row_by_the_words_of_its_quote_and_its_name() {
    let reg = wide();
    let repo = scratch();
    // the word is in the quote of `a_sky_pass` and nowhere in its id, name or keywords
    assert_eq!(
        listed_in(
            answered(&run(&reg, &repo, &["find", "DRAWN"])),
            "ikiuni-renderer"
        ),
        ["a_sky_pass"]
    );
    // and in the name of `a_sun_disc` only
    assert_eq!(
        listed_in(
            answered(&run(&reg, &repo, &["find", "sun disc"])),
            "ikiuni-renderer"
        ),
        ["a_sun_disc"]
    );
}

#[test]
fn find_narrowed_by_in_reads_only_the_namespaces_it_names() {
    let reg = wide();
    let repo = scratch();
    let only = |args: &[&str]| {
        sections(answered(&run(&reg, &repo, args)))
            .into_iter()
            .map(|(h, ids)| (h.split_once(" carry").expect("a header").0.to_owned(), ids))
            .collect::<Vec<_>>()
    };
    assert_eq!(only(&["find", "sun", "--in", "ikiuni-renderer"]), [(
        "2 ikiuni-renderer rows".to_owned(),
        vec!["a_sky_pass".to_owned(), "a_sun_disc".to_owned()]
    )]);
    assert_eq!(only(&["find", "sun", "--in=bevy"]), [(
        "1 bevy rows".to_owned(),
        vec!["sun_light".to_owned()]
    )]);
    // the flag may come first, and several namespaces are one comma list
    let both = only(&["--in", "tenet,bevy", "find", "sun"]);
    assert_eq!(both.iter().map(|(h, _)| h.as_str()).collect::<Vec<_>>(), [
        "2 tenets",
        "1 bevy rows"
    ]);
    // narrowed to the tenets alone, the answer is the one the tenets had before it read the rest
    let tenets = run(&reg, &repo, &["find", "sun", "--in", "tenet"]);
    assert_eq!(
        answered(&tenets),
        "2 tenets carry `sun`\n\n  days_follow_the_sun  A day is the_sun_rises again.\n  the_sun_rises        \
         The sun rises in the east.\n\nread 2 tenets, shown 2\n"
    );
}

#[test]
fn a_filter_naming_no_namespace_is_refused_with_the_ones_there_are() {
    let reg = wide();
    let repo = scratch();
    let why = refused(&reg, &repo, &["find", "sun", "--in", "nosuch"]);
    assert!(why.contains("`nosuch`"), "{why}");
    for ns in ["bevy", "ikiuni", "ikiuni-renderer", "subject", "tenet", "canon"] {
        assert!(why.contains(ns), "`{ns}` missing from {why}");
    }
    assert!(refused(&reg, &repo, &["find", "sun", "--in", "bevy,nosuch"]).contains("`nosuch`"));
    assert!(refused(&reg, &repo, &["find", "sun", "--in"]).contains("--in"));
    assert!(refused(&reg, &repo, &["find", "sun", "--in", ","]).contains("--in"));
}

#[test]
fn a_filter_is_refused_by_the_verbs_that_are_about_tenets_alone() {
    let reg = wide();
    let repo = scratch();
    for verb in ["subjects", "uncited", "forbids", "contradicts"] {
        let why = refused(&reg, &repo, &[verb, "--in", "tenet"]);
        assert!(
            why.contains("find, related, citers and show"),
            "{verb}: {why}"
        );
    }
}

// --- the canon namespaces ------------------------------------------------------------------------

#[test]
fn in_canon_narrows_to_the_namespaces_canon_paths_reach() {
    let reg = wide();
    let repo = canon_repo();
    // the word is a subject, and the id of one reference row and a field of another that relates to it
    let all = answered(&run(&reg, &repo, &["find", "sky"])).to_owned();
    assert_eq!(listed_in(&all, "ikiuni-renderer"), [
        "a_sky_pass",
        "a_sun_disc"
    ]);
    assert_eq!(listed_in(&all, "subject"), ["sky"]);
    let canon = answered(&run(&reg, &repo, &["find", "sky", "--in", "canon"])).to_owned();
    assert_eq!(listed_in(&canon, "subject"), ["sky"]);
    assert!(listed_in(&canon, "ikiuni-renderer").is_empty(), "{canon}");
    // `canon` is a name among the others, so it can be listed with one a person adds
    let more = answered(&run(&reg, &repo, &["find", "sun", "--in", "canon,bevy"])).to_owned();
    assert_eq!(listed_in(&more, "bevy"), ["sun_light"]);
    assert_eq!(listed_in(&more, "tenet"), [
        "days_follow_the_sun",
        "the_sun_rises"
    ]);
    assert!(listed_in(&more, "ikiuni-renderer").is_empty(), "{more}");
}

#[test]
fn in_canon_is_refused_where_the_repository_declares_no_canon() {
    let reg = wide();
    let none = scratch();
    let why = refused(&reg, &none, &["find", "sun", "--in", "canon"]);
    assert!(why.contains("canon_paths"), "{why}");
    write(&none, "mockspace.toml", "mock_dir = \"mock\"\n");
    assert!(refused(&reg, &none, &["find", "sun", "--in", "canon"]).contains("canon_paths"));
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

// --- related -------------------------------------------------------------------------------------

#[test]
fn related_of_a_reference_row_names_both_directions_across_namespaces() {
    let reg = wide();
    let repo = scratch();
    let text = answered(&run(&reg, &repo, &["related", "a_sky_pass"])).to_owned();
    assert!(text.starts_with("ikiuni-renderer::a_sky_pass\n"), "{text}");
    let to = line_of(&text, "refers to:");
    assert!(to.contains("ikiuni::scaffold [ikiuni]"), "{to}");
    let from = line_of(&text, "referred by:");
    assert!(
        from.contains("ikiuni-renderer::a_sun_disc [related]"),
        "{from}"
    );
    assert!(
        from.contains("tenet::the_sun_rises [ikiuni_renderer]"),
        "{from}"
    );
    // narrowed to the tenets, only the tenet's edge is left
    let narrow = answered(&run(&reg, &repo, &[
        "related",
        "a_sky_pass",
        "--in",
        "ikiuni-renderer,tenet",
    ]))
    .to_owned();
    assert!(
        line_of(&narrow, "refers to:")
            .trim_end()
            .ends_with("nothing"),
        "{narrow}"
    );
    assert!(
        !line_of(&narrow, "referred by:").contains("scaffold"),
        "{narrow}"
    );
}

#[test]
fn related_of_a_tenet_keeps_its_own_block_and_adds_the_rows_it_reaches() {
    let reg = wide();
    let repo = scratch();
    let text = answered(&run(&reg, &repo, &["related", "days_follow_the_sun"])).to_owned();
    assert!(text.starts_with("days_follow_the_sun\n"), "{text}");
    assert!(
        line_of(&text, "cites:").ends_with("the_sun_rises"),
        "{text}"
    );
    let text = answered(&run(&reg, &repo, &["related", "tenet::the_sun_rises"])).to_owned();
    assert!(text.starts_with("the_sun_rises\n"), "{text}");
    assert!(
        line_of(&text, "cited by:").contains("days_follow_the_sun"),
        "{text}"
    );
    assert!(
        line_of(&text, "refers to:").contains("ikiuni-renderer::a_sky_pass [ikiuni_renderer]"),
        "{text}"
    );
    // a tenet's relations to other tenets are the block above, not repeated as rows
    assert!(!line_of(&text, "refers to:").contains("tenet::"), "{text}");
    // narrowed to the tenets the answer is the one the tenets had
    let narrow = answered(&run(&reg, &repo, &[
        "related",
        "tenet::the_sun_rises",
        "--in",
        "tenet",
    ]))
    .to_owned();
    assert!(
        !narrow.contains("refers to:") && !narrow.contains("referred by:"),
        "{narrow}"
    );
}

#[test]
fn related_refuses_what_no_namespace_holds() {
    let reg = wide();
    let repo = scratch();
    assert!(refused(&reg, &repo, &["related", "no_such_row"]).contains("`no_such_row`"));
    assert!(refused(&reg, &repo, &["related", "the_sun_rises"]).contains("ikiuni::the_sun_rises"));
}

// --- citers --------------------------------------------------------------------------------------

#[test]
fn citers_of_a_reference_row_walks_the_tree_for_its_name() {
    let reg = wide();
    let repo = scratch();
    write(
        &repo,
        "mock/crates/a/DESIGN.md.tmpl",
        "Departs from a_sky_pass.\n",
    );
    write(
        &repo,
        "mock/research/r/NOTES.md",
        "one\ntwo\nikiuni-renderer::a_sky_pass\n",
    );
    write(
        &repo,
        "mock/registry/tenet/sky.toml",
        "[[tenet]]\nid = \"the_sun_rises\"\nikiuni_renderer = [\"a_sky_pass\"]\n",
    );
    write(
        &repo,
        "mock/registry/ikiuni-renderer/100.toml",
        "[[ikiuni-renderer]]\nid = \"a_sky_pass\"\n",
    );
    let text = answered(&run(&reg, &repo, &["citers", "a_sky_pass"])).to_owned();
    assert!(
        text.starts_with("ikiuni-renderer::a_sky_pass is named 3 times\n"),
        "{text}"
    );
    for at in [
        "mock/crates/a/DESIGN.md.tmpl:1",
        "mock/research/r/NOTES.md:3",
        "mock/registry/tenet/sky.toml:3",
    ] {
        assert!(text.lines().any(|l| l.trim() == at), "no `{at}` in {text}");
    }
    assert!(
        !text.contains("mock/registry/ikiuni-renderer/100.toml"),
        "the row's own `id =` line is not a citation"
    );
    assert!(refused(&reg, &repo, &["citers", "no_such_row"]).contains("`no_such_row`"));
}

// --- the closing line ----------------------------------------------------------------------------

#[test]
fn the_closing_line_says_how_many_rows_of_how_many_namespaces_were_read() {
    let reg = wide();
    let repo = scratch();
    let last = |args: &[&str]| {
        let r = run(&reg, &repo, args);
        let line = answered(&r).lines().last().unwrap_or_default().to_owned();
        let Outcome::Clean {
            examined,
        } = r.outcome
        else {
            panic!("{:?}", r.outcome);
        };
        (line, examined)
    };
    assert_eq!(
        last(&["find", "sun"]),
        ("read 9 rows of 5 namespaces, shown 6".to_owned(), 9)
    );
    assert_eq!(
        last(&["find", "sun", "--in", "bevy"]),
        ("read 1 row of 1 namespace, shown 1".to_owned(), 1)
    );
    assert_eq!(
        last(&["find", "sun", "--in", "tenet"]),
        ("read 2 tenets, shown 2".to_owned(), 2)
    );
    assert_eq!(
        last(&["show", "a_sky_pass"]),
        ("read 9 rows of 5 namespaces, shown 1".to_owned(), 9)
    );
}

// --- the committed registry ----------------------------------------------------------------------

#[test]
fn a_long_id_does_not_push_the_listing_s_words_off_the_screen() {
    let long = "y".repeat(120);
    let long_row = format!("bevy::{long}");
    let reg = view(&[
        ("bevy::x", &[("what", "alpha one")]),
        (long_row.as_str(), &[("what", "alpha two")]),
    ]);
    let text = answered(&run(&reg, &scratch(), &["find", "alpha", "--in", "bevy"])).to_owned();
    let short = line_of(&text, "x ");
    assert_eq!(short.find("alpha one"), Some(2 + 64 + 2), "{short}");
    let wide = line_of(&text, &long);
    assert!(
        wide.ends_with("alpha two") && wide.contains(&format!("{long}  alpha")),
        "{wide}"
    );
}
