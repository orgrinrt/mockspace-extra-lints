//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Where a Bevy row cites, and the three ways a place is read, each beside the quote that must fail.

use super::Memory;
use crate::bevy::{Outcome, Place, check, entries, first_place};

const CRATE: &str = "bevy_ecs-0.20.0-rc.2/src/message/messages.rs:3-4";
const BLOB: &str =
    "https://github.com/bevyengine/bevy/blob/v0.20.0-rc.2/crates/bevy_app/Cargo.toml#L2-L3";
const PAGE: &str = "https://bevy.org/learn/book/control-flow/messages/ (Message Lifespan)";

fn crates() -> Memory {
    Memory::default().with(
        "bevy_ecs-0.20.0-rc.2",
        "src/message/messages.rs",
        "//! first\n//! second\n//! Messages live two frames.\n//! They are then dropped.\n//! last\n",
    )
}

fn blobs() -> Memory {
    Memory::default().with(
        "v0.20.0-rc.2",
        "crates/bevy_app/Cargo.toml",
        "[features]\nbevy_reflect = [\"dep:bevy_reflect\"]\nstd = []\n",
    )
}

#[test]
fn a_comma_inside_a_heading_does_not_split_an_entry() {
    let doc = format!(
        "{PAGE}, https://github.com/bevyengine/bevy/blob/v0.20.1/docs/cargo_features.md#L131 (Feature List, dynamic_linking), {CRATE}"
    );
    let e = entries(&doc);
    assert_eq!(e.len(), 3, "{e:?}");
    assert!(e[1].ends_with("dynamic_linking)"), "{e:?}");
    assert_eq!(e[2], CRATE);
}

#[test]
fn the_three_kinds_of_place_are_told_apart() {
    assert!(matches!(first_place(CRATE).unwrap(), Place::Crate { .. }));
    assert!(matches!(first_place(BLOB).unwrap(), Place::Blob { .. }));
    assert_eq!(first_place(PAGE), None);
    assert_eq!(first_place(""), None);
}

#[test]
fn a_single_line_is_a_range_of_one() {
    let Place::Crate {
        cited,
        ..
    } = first_place("bevy_app-0.20.0/src/lib.rs:12").unwrap()
    else {
        panic!("not a crate place")
    };
    assert_eq!((cited.lines.first(), cited.lines.last()), (12, 12));
    let Place::Blob {
        tag,
        cited,
    } = first_place("https://github.com/bevyengine/bevy/blob/v0.20.1/Cargo.toml#L9 (x)").unwrap()
    else {
        panic!("not a blob")
    };
    assert_eq!(
        (tag.as_str(), cited.lines.first(), cited.lines.last()),
        ("v0.20.1", 9, 9)
    );
}

#[test]
fn an_excerpt_inside_the_cited_range_is_held() {
    let o = check(
        CRATE,
        "//! Messages live two frames.\n",
        &mut crates(),
        None::<&mut Memory>,
    );
    assert_eq!(o, Outcome::Held);
    let o = check(
        CRATE,
        "//!   Messages  live two frames.\n//! They are then dropped.\n",
        &mut crates(),
        None::<&mut Memory>,
    );
    assert_eq!(o, Outcome::Held);
}

#[test]
fn a_quote_with_a_changed_word_is_absent() {
    let o = check(
        CRATE,
        "//! Messages live three frames.\n",
        &mut crates(),
        None::<&mut Memory>,
    );
    assert_eq!(
        o,
        Outcome::Absent("//! Messages live three frames.".to_owned())
    );
    assert!(o.is_finding());
}

#[test]
fn lines_out_of_order_are_scattered_and_not_held() {
    let o = check(
        CRATE,
        "//! They are then dropped.\n//! Messages live two frames.\n",
        &mut crates(),
        None::<&mut Memory>,
    );
    assert_eq!(o, Outcome::Scattered);
    assert!(!o.is_finding());
}

#[test]
fn a_line_outside_the_cited_range_is_absent() {
    let o = check(CRATE, "//! last\n", &mut crates(), None::<&mut Memory>);
    assert!(matches!(o, Outcome::Absent(_)), "{o:?}");
}

#[test]
fn a_range_past_the_end_of_the_file_is_reported() {
    let o = check(
        "bevy_ecs-0.20.0-rc.2/src/message/messages.rs:9-10",
        "x",
        &mut crates(),
        None::<&mut Memory>,
    );
    assert!(
        matches!(o, Outcome::PastEnd {
            lines: 5,
            ..
        }),
        "{o:?}"
    );
}

#[test]
fn a_file_the_source_does_not_hold_is_reported_and_a_missing_source_is_not_a_finding() {
    let o = check(
        "bevy_ecs-0.20.0-rc.2/src/gone.rs:1-2",
        "x",
        &mut crates(),
        None::<&mut Memory>,
    );
    assert!(matches!(o, Outcome::NoFile(_)), "{o:?}");
    let o = check(
        "bevy_ecs-0.20.1/src/lib.rs:1-2",
        "x",
        &mut crates(),
        None::<&mut Memory>,
    );
    assert_eq!(o, Outcome::NoSource("bevy_ecs-0.20.1".to_owned()));
    assert!(!o.is_finding());
}

#[test]
fn a_blob_is_read_from_the_clone_at_its_tag_or_counted_as_needing_one() {
    let quote = "bevy_reflect = [\"dep:bevy_reflect\"]\nstd = []\n";
    assert_eq!(
        check(BLOB, quote, &mut crates(), None::<&mut Memory>),
        Outcome::NeedsClone
    );
    assert_eq!(
        check(BLOB, quote, &mut crates(), Some(&mut blobs())),
        Outcome::Held
    );
    let wrong = "bevy_reflect = [\"dep:bevy_reflect\"]\nstd = [\"alloc\"]\n";
    let o = check(BLOB, wrong, &mut crates(), Some(&mut blobs()));
    assert!(matches!(o, Outcome::Absent(_)), "{o:?}");
}

#[test]
fn a_page_is_never_passed() {
    assert_eq!(
        check(PAGE, "anything", &mut crates(), None::<&mut Memory>),
        Outcome::NotLocal
    );
}

#[test]
fn prose_absent_from_the_crate_may_be_the_page_the_row_also_cites_but_a_doc_comment_may_not() {
    let doc = format!("{CRATE}, {PAGE}");
    assert_eq!(
        check(
            &doc,
            "Messages live for ever.\n",
            &mut crates(),
            None::<&mut Memory>
        ),
        Outcome::NotLocal
    );
    let o = check(
        &doc,
        "//! Messages live for ever.\n",
        &mut crates(),
        None::<&mut Memory>,
    );
    assert!(matches!(o, Outcome::Absent(_)), "{o:?}");
}

#[test]
fn an_excerpt_may_begin_and_end_inside_a_line_but_only_there() {
    assert_eq!(
        check(CRATE, "live two\n", &mut crates(), None::<&mut Memory>),
        Outcome::Held
    );
    assert_eq!(
        check(
            CRATE,
            "two frames.\n//! They are\n",
            &mut crates(),
            None::<&mut Memory>
        ),
        Outcome::Held
    );
    // a line in the middle of the quote has to be whole
    let o = check(
        "bevy_ecs-0.20.0-rc.2/src/message/messages.rs:1-5",
        "second\n//! Messages live two frames.\n//! last\n",
        &mut crates(),
        None::<&mut Memory>,
    );
    assert_ne!(o, Outcome::Held);
}
