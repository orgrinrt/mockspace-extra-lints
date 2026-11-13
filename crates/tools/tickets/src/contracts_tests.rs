//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The contracts a ticket changes: the makers write and clear the field and refuse a malformed path,
//! and `tickets contract` gives the open tickets that change a path, refusing a crate the tree does
//! not hold.

use crate::args::{self, Verb};
use crate::testkit::Scratch;

const SEA: &str = "kaski-view-contracts::SeaStateView";

fn held_crate(s: &Scratch, krate: &str) {
    std::fs::create_dir_all(s.mock.join("crates").join(krate)).unwrap();
}

fn ask(s: &Scratch, paths: &[&str]) -> Result<String, String> {
    let store = crate::load(&s.root).unwrap();
    let plan = crate::load_plan(&s.root).unwrap();
    let paths: Vec<String> = paths.iter().map(|p| (*p).to_string()).collect();
    crate::contract_lines(&store, &plan, &s.mock, &paths).map(|(out, _)| out)
}

#[test]
fn the_flag_is_taken_by_both_makers() {
    assert!(args::ADD_FLAGS.contains(&"--contracts"));
    assert!(args::MOVE_FLAGS.contains(&"--contracts"));
    let r = args::move_(&["a", "--contracts", SEA]).unwrap();
    assert_eq!(r.links["contracts"], [SEA]);
}

#[test]
fn the_verb_wants_a_path_and_takes_several() {
    assert!(args::tickets(&["contract"]).is_err());
    let r = args::tickets(&["contract", SEA, "Other"]).unwrap();
    assert_eq!(
        r.verb,
        Verb::Contract(vec![SEA.to_string(), "Other".to_string()])
    );
}

#[test]
fn a_contract_is_written_and_cleared_on_the_row() {
    let s = Scratch::new();
    let a = s.add("Retire the read", &[("contracts", &[SEA])]).unwrap();
    assert!(
        s.read(&format!("ticket/items/{a}.toml"))
            .contains(&format!("contracts = [\"{SEA}\"]"))
    );
    s.links(&a, "", None, &[("contracts", &[])]).unwrap();
    assert!(
        !s.read(&format!("ticket/items/{a}.toml"))
            .contains("contracts")
    );
}

#[test]
fn a_malformed_contract_is_refused_by_both_makers_and_writes_nothing() {
    let s = Scratch::new();
    let err = s
        .add("Bad", &[("contracts", &["SeaStateView"])])
        .unwrap_err();
    assert!(err.contains("`SeaStateView`"), "{err}");
    let a = s.add("Good", &[]).unwrap();
    let file = format!("ticket/items/{a}.toml");
    let before = s.read(&file);
    let err = s
        .links(&a, "", None, &[("contracts", &["no way"])])
        .unwrap_err();
    assert!(err.contains("`no way`"), "{err}");
    assert_eq!(s.read(&file), before, "a refusal writes nothing");
}

#[test]
fn the_verb_gives_the_open_tickets_that_change_a_path_and_names_the_path() {
    let s = Scratch::new();
    held_crate(&s, "kaski-view-contracts");
    let a = s.add("Retire the read", &[("contracts", &[SEA])]).unwrap();
    s.add("Elsewhere", &[("contracts", &[
        "kaski-view-contracts::Other",
    ])])
    .unwrap();
    s.add_as("Done long ago", "task", Some("done"), "coord", &[(
        "contracts",
        &[SEA],
    )])
    .unwrap();
    let out = ask(&s, &[SEA]).unwrap();
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(
        out.starts_with(&format!("{a} backlog - Retire the read")),
        "{out}"
    );
    assert!(out.trim_end().ends_with(SEA), "{out}");
    // a bare type name finds it too, and a path nothing changes gives nothing
    assert_eq!(ask(&s, &["SeaStateView"]).unwrap().lines().count(), 1);
    assert_eq!(ask(&s, &["kaski-view-contracts::Nothing"]).unwrap(), "");
}

#[test]
fn a_crate_the_tree_does_not_hold_is_refused_and_not_read_as_nothing_changing() {
    let s = Scratch::new();
    let err = ask(&s, &["kaski-view-contract::SeaStateView"]).unwrap_err();
    assert!(
        err.contains("`kaski-view-contract`") && err.contains("does not hold"),
        "{err}"
    );
}
