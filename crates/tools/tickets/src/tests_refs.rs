//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! A ticket that links a tenet or a subject the canon does not hold is refused where it is written and
//! named by the check where a hand edit left it.

use std::collections::BTreeMap;

use ticket_model::Tree;
use mockspace::RegistryView;
use mockspace::tool::Outcome;

use crate::landing::{Landing, Trunk};
use crate::refs::Known;
use crate::testkit::Scratch;

/// A repository that knows no branch and has lost nothing.
struct Nothing;

impl Trunk for Nothing {
    fn landing(&self, _: &str) -> Landing {
        Landing::Unknown
    }
}

impl Tree for Nothing {
    fn crate_prefix(&self) -> &str {
        "kaski"
    }

    fn gone(&self, _: &str) -> Option<String> {
        None
    }
}

fn canon() -> RegistryView {
    RegistryView::new(
        BTreeMap::from([
            ("tenet::real".to_string(), BTreeMap::new()),
            ("subject::engine".to_string(), BTreeMap::new()),
        ]),
        BTreeMap::new(),
    )
}

fn fields(k: &str, v: &[&str]) -> BTreeMap<String, Vec<String>> {
    BTreeMap::from([(k.to_string(), v.iter().map(|s| (*s).to_string()).collect())])
}

#[test]
fn a_link_to_a_tenet_or_subject_no_row_carries_is_refused_by_name_and_a_real_one_is_not() {
    let known = Known::of(&canon());
    assert!(
        known
            .refuse(&fields("related", &["tenet::real", "subject::engine"]))
            .is_ok()
    );
    let err = known
        .refuse(&fields("related", &[
            "tenet::real",
            "subject::enigne",
            "tenet::made_up",
        ]))
        .unwrap_err();
    assert!(
        err.contains("`subject::enigne`")
            && err.contains("`tenet::made_up`")
            && !err.contains("`tenet::real`"),
        "{err}"
    );
}

#[test]
fn rows_of_other_namespaces_and_the_tenets_field_are_judged_each_by_its_own_kind() {
    let known = Known::of(&canon());
    // another ticket, a design: not the canon's to say
    assert!(
        known
            .refuse(&fields("related", &[
                "ticket::anything",
                "reference::paper"
            ]))
            .is_ok()
    );
    assert!(
        known
            .refuse(&fields("tenets", &["real", "tenet::real"]))
            .is_ok()
    );
    assert!(known.refuse(&fields("tenets", &["nothing"])).is_err());
    // a subject slug is no tenet
    assert!(known.refuse(&fields("tenets", &["engine"])).is_err());
    assert!(
        known
            .refuse(&fields("related", &["tenet::engine"]))
            .is_err()
    );
}

#[test]
fn the_check_names_a_ticket_a_hand_edit_left_linking_a_row_that_is_not_there() {
    let s = Scratch::new();
    let good = s.add("Good", &[("related", &["subject::engine"])]).unwrap();
    let bad = s.add("Bad", &[("related", &["subject::enigne"])]).unwrap();
    let store = crate::load(&s.root).unwrap();
    let plan = crate::load_plan(&s.root).unwrap();
    let found = Known::of(&canon()).findings(&store);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].row, format!("ticket::{bad}"));
    assert!(found[0].message.contains("`subject::enigne`"), "{found:?}");
    assert!(!found.iter().any(|f| f.row.contains(&good)));
    let report = crate::check(
        &store,
        &plan,
        &Nothing,
        &ticket_model::Heirs::new(),
        &Known::of(&canon()),
    );
    assert!(
        matches!(report.outcome, Outcome::Findings(ref f) if f.len() == 1),
        "{:?}",
        report.outcome
    );
}
