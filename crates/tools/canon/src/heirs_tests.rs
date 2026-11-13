//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What binds in the place of a superseded tenet, the chain of replacements followed to its end.

use crate::rows::{Standing, Tenet, heirs, tenets};
use crate::tests::fixture;

fn row<'a>(id: &'a str, superseded_by: &[&'a str]) -> Tenet<'a> {
    Tenet {
        id,
        standing: Standing::Provisional,
        subject: None,
        superseded_by: superseded_by.to_vec(),
        cites: vec![],
        links: vec![],
        keywords: vec![],
        rules_out: vec![],
        what: "",
        because: "",
    }
}

#[test]
fn a_superseded_tenet_has_the_row_that_replaced_it_as_its_heir() {
    let reg = fixture();
    let h = heirs(&tenets(&reg));
    assert_eq!(h["the_sun_rose"], ["the_sun_rises"]);
    assert!(
        !h.contains_key("the_sun_rises"),
        "a row that binds is no key"
    );
}

#[test]
fn a_replacement_that_is_itself_superseded_is_followed_to_the_row_that_binds() {
    let all = [row("a", &["b"]), row("b", &["c"]), row("c", &[])];
    let h = heirs(&all);
    assert_eq!(h["a"], ["c"]);
    assert_eq!(h["b"], ["c"]);
}

#[test]
fn two_replacements_are_two_heirs_each_once() {
    let all = [row("a", &["b", "c"]), row("b", &["c"]), row("c", &[])];
    assert_eq!(heirs(&all)["a"], ["c"]);
    let all = [row("a", &["b", "c"]), row("b", &[]), row("c", &[])];
    assert_eq!(heirs(&all)["a"], ["b", "c"]);
}

#[test]
fn a_chain_that_loops_ends_without_an_heir_and_without_hanging() {
    let all = [row("a", &["b"]), row("b", &["a"])];
    let h = heirs(&all);
    assert!(h["a"].is_empty() && h["b"].is_empty(), "{h:?}");
}
