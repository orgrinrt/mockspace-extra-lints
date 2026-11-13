//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The sweep that names an heir in every ticket naming a superseded tenet.

use std::collections::BTreeMap;

use mockspace::RegistryView;

use crate::repoint::repoint_superseded;
use crate::testkit::Scratch;

fn tenets() -> RegistryView {
    let row = |pairs: &[(&str, &str)]| -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    };
    RegistryView::new(
        BTreeMap::from([
            ("tenet::old".to_string(), row(&[("superseded_by", "new")])),
            ("tenet::new".to_string(), row(&[])),
        ]),
        BTreeMap::new(),
    )
}

#[test]
fn a_ticket_naming_a_superseded_tenet_is_written_to_name_its_heir() {
    let s = Scratch::new();
    let a = s
        .add("Uses the row", &[("related", &[
            "tenet::old",
            "subject::engine",
        ])])
        .unwrap();
    let written = repoint_superseded(
        &crate::registry_dir(&s.mock),
        &tenets(),
        "tester",
        "2026-11-08T10:00:00Z",
    )
    .unwrap();
    assert_eq!(written, [a.clone()]);
    let store = crate::load(&crate::registry_dir(&s.mock)).unwrap();
    assert_eq!(store.tickets[&a].many("related"), [
        "tenet::new",
        "subject::engine"
    ]);
}

#[test]
fn a_ticket_about_the_demotion_is_left_as_it_is() {
    let s = Scratch::new();
    let a = s
        .add("Demote old", &[
            ("related", &["tenet::old"]),
            ("tags", &["demoted-row"]),
        ])
        .unwrap();
    let written = repoint_superseded(
        &crate::registry_dir(&s.mock),
        &tenets(),
        "tester",
        "2026-11-08T10:00:00Z",
    )
    .unwrap();
    assert!(written.is_empty(), "{written:?}");
    let store = crate::load(&crate::registry_dir(&s.mock)).unwrap();
    assert_eq!(store.tickets[&a].many("related"), ["tenet::old"]);
}

#[test]
fn a_second_sweep_writes_nothing() {
    let s = Scratch::new();
    s.add("Uses the row", &[("related", &["tenet::old"])])
        .unwrap();
    let root = crate::registry_dir(&s.mock);
    repoint_superseded(&root, &tenets(), "tester", "2026-11-08T10:00:00Z").unwrap();
    let again = repoint_superseded(&root, &tenets(), "tester", "2026-11-08T10:00:01Z").unwrap();
    assert!(again.is_empty(), "{again:?}");
}
