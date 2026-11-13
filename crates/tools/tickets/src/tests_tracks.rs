//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `tickets tracks`: a block a track with its steps in order and the waits between tracks, narrowed by the
//! filters of `list`, over the committed `track` kind.

use crate::args::{self, Verb};
use crate::testkit::Scratch;
use crate::view::{self, Filter};

fn text(s: &Scratch, filter: &Filter) -> String {
    let store = crate::load(&s.root).unwrap();
    let plan = crate::load_plan(&s.root).unwrap();
    view::tracks(&store, &plan, filter).0
}

/// Two tracks: `sea` held by `water-coord` with a first step that waits on a step of `sky`, held by
/// `render-coord`.
fn two(s: &Scratch) {
    let sea = s
        .add_as("Sea", "track", Some("doing"), "water-coord", &[(
            "shows",
            &["a ship rides a wave"],
        )])
        .unwrap();
    let sky = s
        .add_as("Sky", "track", Some("doing"), "render-coord", &[(
            "shows",
            &["a night sky"],
        )])
        .unwrap();
    let stars = s.add("Stars", &[("part_of", &[&sky])]).unwrap();
    let wave = s
        .add("Wave", &[("part_of", &[&sea]), ("needs", &[&stars])])
        .unwrap();
    s.add("Ship", &[("part_of", &[&sea]), ("needs", &[&wave])])
        .unwrap();
}

#[test]
fn the_verb_is_read_and_takes_the_filters_of_list() {
    let r = args::tickets(&["tracks", "--holder", "water-coord"]).unwrap();
    assert_eq!(r.verb, Verb::Tracks);
    assert_eq!(r.filter.holder, ["water-coord"]);
    assert!(args::tickets(&["tracks", "extra"]).is_err());
}

#[test]
fn a_block_gives_the_track_its_steps_in_order_and_the_wait_between_tracks() {
    let s = Scratch::new();
    two(&s);
    let out = text(&s, &Filter::default());
    let sea = out.split("track sea ").nth(1).expect(&out);
    assert!(
        sea.starts_with("doing water-coord shows: a ship rides a wave [first steps waiting: 1]\n"),
        "{out}"
    );
    assert!(sea.contains("  1 wave backlog - Wave\n    waits on stars (track sky) backlog -\n  2 ship backlog - Ship\n"), "{out}");
    let sky = out.split("track sky ").nth(1).unwrap();
    assert!(
        sky.starts_with("doing render-coord shows: a night sky\n  1 stars backlog - Stars\n"),
        "{out}"
    );
    assert!(!sky.contains("waits"), "{out}");
}

#[test]
fn a_holder_narrows_the_tracks_by_the_track_ticket() {
    let s = Scratch::new();
    two(&s);
    let only = text(&s, &Filter {
        holder: vec!["render-coord".into()],
        ..Filter::default()
    });
    assert!(
        only.contains("track sky ") && !only.contains("track sea "),
        "{only}"
    );
}

#[test]
fn a_track_with_no_steps_is_one_line_and_a_registry_with_none_is_silent() {
    let s = Scratch::new();
    assert_eq!(text(&s, &Filter::default()), "");
    s.add_as("Lonely", "track", Some("doing"), "coord", &[("shows", &[
        "nothing yet",
    ])])
    .unwrap();
    assert_eq!(
        text(&s, &Filter::default()),
        "track lonely doing coord shows: nothing yet\n"
    );
}

#[test]
fn a_track_is_refused_without_the_result_it_shows() {
    let s = Scratch::new();
    let err = s
        .add_as("Bare", "track", Some("doing"), "coord", &[])
        .unwrap_err();
    assert!(err.contains("shows"), "{err}");
}

#[test]
fn the_flag_is_taken_by_both_makers_and_written_to_the_row() {
    assert!(args::ADD_FLAGS.contains(&"--shows") && args::MOVE_FLAGS.contains(&"--shows"));
    let s = Scratch::new();
    let id = s.add("Plain", &[("shows", &["a thing"])]).unwrap();
    assert!(
        s.read(&format!("ticket/items/{id}.toml"))
            .contains("shows = \"a thing\"")
    );
}
