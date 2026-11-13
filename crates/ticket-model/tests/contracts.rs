//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The contracts a ticket changes: a value of `contracts` is a crate and a type joined by `::` and
//! nothing else, the makers and the check refuse any other, and `Store::changing` gives the open
//! tickets that hold a path, a bare type name matching the type in any crate.

mod common;

use common::{closed, open, store};
use kaski_tickets::contracts::shape;
use kaski_tickets::rules::{self, NewRequest};
use kaski_tickets::{Fields, Store};

fn ids(s: &Store, path: &str) -> Vec<String> {
    s.changing(path).map(|t| t.id.clone()).collect()
}

#[test]
fn a_crate_and_a_type_joined_by_two_colons_is_a_contract() {
    assert_eq!(shape("kaski-view-contracts::SeaStateView"), Ok(()));
    assert_eq!(shape("kaski-tickets::Store"), Ok(()));
}

#[test]
fn every_other_spelling_is_refused_naming_the_value() {
    for bad in [
        "SeaStateView",
        "::SeaStateView",
        "kaski-view-contracts::",
        "kaski-view-contracts::Sea::State",
        "kaski view::Sea",
        "Kaski-View::Sea",
        "kaski-view::Sea State",
        "kaski-view::1Sea",
        "",
    ] {
        let err = shape(bad).expect_err(bad);
        assert!(err.contains(&format!("`{bad}`")), "{err}");
    }
}

#[test]
fn an_open_ticket_that_holds_a_path_is_found_by_it() {
    let s = store(&[
        open("a", "contracts = [\"kaski-view-contracts::SeaStateView\"]"),
        open("b", "contracts = [\"kaski-view-contracts::Other\"]"),
        open("c", ""),
    ]);
    assert_eq!(ids(&s, "kaski-view-contracts::SeaStateView"), ["a"]);
}

#[test]
fn a_bare_type_name_matches_the_type_in_any_crate() {
    let s = store(&[
        open("a", "contracts = [\"kaski-view-contracts::SeaStateView\"]"),
        open("b", "contracts = [\"kaski-other::SeaStateView\"]"),
        open("c", "contracts = [\"kaski-other::SeaStateViewer\"]"),
    ]);
    assert_eq!(ids(&s, "SeaStateView"), ["a", "b"]);
}

#[test]
fn a_closed_ticket_changes_nothing_any_more() {
    let s = store(&[
        closed("a", "contracts = [\"kaski-view-contracts::SeaStateView\"]"),
        open("b", "contracts = [\"kaski-view-contracts::SeaStateView\"]"),
    ]);
    assert_eq!(ids(&s, "kaski-view-contracts::SeaStateView"), ["b"]);
}

#[test]
fn a_ticket_holding_several_paths_is_found_by_each() {
    let s = store(&[open("a", "contracts = [\"kaski-x::One\", \"kaski-y::Two\"]")]);
    assert_eq!(ids(&s, "kaski-x::One"), ["a"]);
    assert_eq!(ids(&s, "kaski-y::Two"), ["a"]);
    assert!(ids(&s, "kaski-x::Two").is_empty());
}

fn request(contracts: &[&str]) -> NewRequest {
    let mut fields = Fields::new();
    fields.insert(
        "contracts".to_string(),
        contracts.iter().map(ToString::to_string).collect(),
    );
    NewRequest {
        title: "Retire the read".into(),
        kind: "task".into(),
        by: "op".into(),
        at: "2026-11-08T10:00:00Z".into(),
        move_id: "m_new".into(),
        fields,
        ..NewRequest::default()
    }
}

#[test]
fn a_new_ticket_with_a_malformed_contract_is_refused() {
    let s = store(&[]);
    let err = rules::new_ticket(&s, &request(&["SeaStateView"])).unwrap_err();
    assert!(
        err.contains("`contracts`") && err.contains("`SeaStateView`"),
        "{err}"
    );
}

#[test]
fn a_new_ticket_with_a_good_contract_keeps_it() {
    let s = store(&[]);
    let made = rules::new_ticket(&s, &request(&["kaski-x::One"])).unwrap();
    assert_eq!(made.fields["contracts"], ["kaski-x::One"]);
}

#[test]
fn the_check_names_a_committed_row_holding_a_malformed_contract() {
    let s = store(&[open("a", "contracts = [\"nonsense\"]")]);
    let found = rules::check_store(&s);
    assert!(
        found
            .iter()
            .any(|f| f.row == "ticket::a" && f.message.contains("`nonsense`")),
        "{found:?}"
    );
}
