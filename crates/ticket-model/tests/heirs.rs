//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! A superseded tenet named by a ticket is followed to the row that binds in its place: `repoint`
//! gives the links a ticket should hold, `left_naming` the tickets still naming a superseded row.

mod common;

use std::collections::BTreeMap;

use common::{open, store};
use mockspace_extra_ticket_model::{Heirs, heirs};

fn heirs_of(rows: &[(&str, &[&str])]) -> Heirs {
    rows.iter()
        .map(|(old, new)| {
            (
                (*old).to_string(),
                new.iter().map(|s| (*s).to_string()).collect(),
            )
        })
        .collect::<BTreeMap<_, _>>()
}

fn only(lines: &str) -> mockspace_extra_ticket_model::Ticket {
    let s = store(&[open("a", lines)]);
    s.tickets["a"].clone()
}

#[test]
fn a_related_entry_naming_a_superseded_tenet_is_replaced_by_its_one_heir() {
    let t = only("title = \"t\"\nrelated = [\"tenet::old\", \"subject::engine\"]");
    let r = heirs::repoint(&t, &heirs_of(&[("old", &["new"])]));
    assert_eq!(r.related, ["tenet::new", "subject::engine"]);
    assert!(r.left.is_empty());
}

#[test]
fn a_repointed_list_is_deduplicated_keeping_the_first() {
    let t = only("title = \"t\"\nrelated = [\"tenet::old\", \"tenet::new\"]");
    let r = heirs::repoint(&t, &heirs_of(&[("old", &["new"])]));
    assert_eq!(r.related, ["tenet::new"]);
}

#[test]
fn a_bare_id_in_tenets_is_replaced_too() {
    let t = only("title = \"t\"\ntenets = [\"old\"]");
    let r = heirs::repoint(&t, &heirs_of(&[("old", &["new"])]));
    assert_eq!(r.tenets, ["new"]);
}

#[test]
fn a_row_with_several_heirs_is_left_and_named() {
    let t = only("title = \"t\"\nrelated = [\"tenet::old\"]");
    let r = heirs::repoint(&t, &heirs_of(&[("old", &["a", "b"])]));
    assert_eq!(r.related, ["tenet::old"]);
    assert_eq!(r.left, ["old"]);
}

#[test]
fn a_ticket_about_the_demotion_keeps_the_old_id_by_its_title() {
    let t = only("title = \"demote old and move its tickets\"\nrelated = [\"tenet::old\"]");
    let r = heirs::repoint(&t, &heirs_of(&[("old", &["new"])]));
    assert_eq!(r.related, ["tenet::old"]);
    assert!(r.left.is_empty());
}

#[test]
fn a_ticket_tagged_demoted_row_keeps_the_old_id() {
    let t = only("title = \"t\"\ntags = [\"demoted-row\"]\nrelated = [\"tenet::old\"]");
    let r = heirs::repoint(&t, &heirs_of(&[("old", &["new"])]));
    assert_eq!(r.related, ["tenet::old"]);
}

#[test]
fn a_ticket_naming_no_superseded_row_comes_back_as_it_is() {
    let t = only("title = \"t\"\nrelated = [\"tenet::fine\"]");
    let r = heirs::repoint(&t, &heirs_of(&[("old", &["new"])]));
    assert_eq!(r.related, ["tenet::fine"]);
    assert!(!r.changed);
}

#[test]
fn the_check_names_a_ticket_still_naming_a_superseded_row_with_its_heir() {
    let s = store(&[
        open("a", "title = \"t\"\nrelated = [\"tenet::old\"]"),
        open("b", "title = \"t\"\nrelated = [\"tenet::fine\"]"),
        open("c", "title = \"about old\"\nrelated = [\"tenet::old\"]"),
    ]);
    let out = heirs::left_naming(&s, &heirs_of(&[("old", &["new"])]));
    assert_eq!(out.len(), 1, "{out:?}");
    assert_eq!(out[0].row, "ticket::a");
    assert!(out[0].message.contains("old") && out[0].message.contains("new"));
}
