//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `related` and `citers` across every namespace of the registry, and the closing line and the width
//! of a listing, on the wide registry of `reference_tests`.

use mockspace::tool::Outcome;

use crate::reference_tests::*;
use crate::tests::{answered, run, scratch, write};

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
        last(&["show", "a_sky_pass"]),
        ("read 9 rows of 5 namespaces, shown 1".to_owned(), 9)
    );
}

// --- the committed registry ----------------------------------------------------------------------
