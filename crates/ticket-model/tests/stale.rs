//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The three lists of a `Store` that name a ticket as possibly stale. None refuses anything: each is a
//! finding a person reads.

mod common;

use common::{closed, open, store};
use kaski_tickets::{Tree, stale};

/// A tree that once held the names it is given, each deleted by a commit, and holds nothing else of them.
struct Gone(&'static [(&'static str, &'static str)]);

impl Tree for Gone {
    fn gone(&self, name: &str) -> Option<String> {
        self.0
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, by)| (*by).to_string())
    }
}

#[test]
fn an_open_ticket_naming_a_crate_the_tree_lost_is_listed_with_the_commit_that_deleted_it() {
    let s = store(&[open(
        "a",
        "title = \"port the pass of kaski-render\"\nwhat = \"nothing\"",
    )]);
    let out = stale::gone_names(&s, &Gone(&[("kaski-render", "ef19a8e0c")]));
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].row, "ticket::a");
    assert!(out[0].message.contains("kaski-render") && out[0].message.contains("ef19a8e0c"));
}

#[test]
fn a_name_the_tree_never_held_is_not_listed() {
    let s = store(&[open("a", "title = \"a word like kaski-nothing\"")]);
    assert!(stale::gone_names(&s, &Gone(&[("kaski-render", "ef19a8e0c")])).is_empty());
}

#[test]
fn a_path_in_what_is_listed_as_a_crate_is() {
    let s = store(&[open(
        "a",
        "title = \"t\"\nwhat = \"see `mock/crates/kaski-render/src/lib.rs` and kaski-render\"",
    )]);
    let out = stale::gone_names(
        &s,
        &Gone(&[("mock/crates/kaski-render/src/lib.rs", "aaa1111"), ("kaski-render", "bbb2222")]),
    );
    assert_eq!(out.len(), 1, "one finding names the ticket once: {out:?}");
    assert!(out[0].message.contains("aaa1111") && out[0].message.contains("bbb2222"));
}

#[test]
fn a_closed_ticket_naming_a_lost_crate_is_not_listed() {
    let s = store(&[closed("a", "title = \"kaski-render\"")]);
    assert!(stale::gone_names(&s, &Gone(&[("kaski-render", "ef19a8e0c")])).is_empty());
}

#[test]
fn an_open_parent_with_every_child_closed_is_listed() {
    let s = store(&[
        open("parent", ""),
        closed("c1", "part_of = \"parent\""),
        closed("c2", "part_of = \"parent\""),
    ]);
    let out = stale::left_open(&s);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].row, "ticket::parent");
}

#[test]
fn an_open_parent_with_one_open_child_is_not_listed() {
    let s = store(&[
        open("parent", ""),
        closed("c1", "part_of = \"parent\""),
        open("c2", "part_of = \"parent\""),
    ]);
    assert!(stale::left_open(&s).is_empty());
}

#[test]
fn an_open_parent_whose_children_are_closed_but_a_need_is_open_is_not_listed() {
    let s = store(&[
        open("parent", "needs = [\"other\"]"),
        open("other", ""),
        closed("c1", "part_of = \"parent\""),
    ]);
    assert!(stale::left_open(&s).is_empty());
}

#[test]
fn an_open_ticket_with_no_child_and_its_needs_closed_is_free_to_take_and_not_listed() {
    let s = store(&[open("a", "needs = [\"b\"]"), closed("b", "")]);
    assert!(stale::left_open(&s).is_empty());
}

#[test]
fn a_closed_ticket_with_an_open_child_is_listed() {
    let s = store(&[closed("parent", ""), open("c1", "part_of = \"parent\"")]);
    let out = stale::left_open(&s);
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].row, "ticket::parent");
    assert!(out[0].message.contains("c1"));
}

#[test]
fn the_count_is_the_findings_of_the_two_lists() {
    let s = store(&[
        open("p1", ""),
        closed("c1", "part_of = \"p1\""),
        closed("p2", ""),
        open("c2", "part_of = \"p2\""),
    ]);
    assert_eq!(stale::stale_count(&s, &Gone(&[])), 2);
}

#[test]
fn an_absolute_path_and_a_parent_path_are_no_names() {
    let names =
        stale::names_in("see /home/user/kaski and ../x/y and mock/crates/kaski-a/src/lib.rs.");
    assert_eq!(names, ["mock/crates/kaski-a/src/lib.rs"]);
}

#[test]
fn a_directory_is_named_without_its_closing_slash() {
    assert_eq!(stale::names_in("under mock/benches/ only"), [
        "mock/benches"
    ]);
}
