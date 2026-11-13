//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `related` and `citers` across every namespace of the registry, and the closing line and the width
//! of a listing, on the wide registry of `reference_tests`.

use mockspace::tool::Outcome;

use crate::tests::{answered, run, scratch, view, write};
use crate::reference_tests::*;


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
