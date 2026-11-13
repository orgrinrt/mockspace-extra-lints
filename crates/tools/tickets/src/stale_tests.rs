//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What `tickets check` prints of the tickets that may have stopped being true: a line for each, the count,
//! and no failure for any of them.

use ticket_model::{Heirs, Tree};
use mockspace::tool::Outcome;

use crate::landing::{Landing, Trunk};
use crate::testkit::Scratch;

struct World(&'static [(&'static str, &'static str)]);

impl Trunk for World {
    fn landing(&self, _branch: &str) -> Landing {
        Landing::Unknown
    }
}

impl Tree for World {
    fn crate_prefix(&self) -> &str {
        "kaski"
    }

    fn gone(&self, name: &str) -> Option<String> {
        self.0
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, by)| (*by).to_string())
    }
}

fn report(s: &Scratch, world: &World, heirs: &Heirs) -> mockspace::tool::ToolReport {
    let store = crate::load(&s.root).unwrap();
    let plan = crate::load_plan(&s.root).unwrap();
    crate::check(&store, &plan, world, heirs, &crate::refs::Known::default())
}

#[test]
fn the_lists_are_printed_with_a_count_and_the_check_stays_clean() {
    let s = Scratch::new();
    let parent = s.add("Parent", &[]).unwrap();
    let child = s.add("Child", &[("part_of", &[&parent])]).unwrap();
    s.mv(&child, "done", None, "2026-11-03T10:00:00Z").unwrap();
    s.add("Port kaski-render", &[]).unwrap();
    let r = report(&s, &World(&[("kaski-render", "ef19a8e0c")]), &Heirs::new());
    assert!(
        matches!(r.outcome, Outcome::Clean { .. }),
        "{:?}",
        r.outcome
    );
    assert!(
        r.output.contains("kaski-render") && r.output.contains("ef19a8e0c"),
        "{}",
        r.output
    );
    assert!(r.output.contains("all 1 of its children"), "{}", r.output);
    assert!(
        r.output
            .ends_with("2 tickets may be stale: tickets check\n"),
        "{}",
        r.output
    );
}

#[test]
fn the_list_says_how_many_of_its_tickets_the_sweep_would_close_and_only_where_one_would() {
    let s = Scratch::new();
    s.add("Port kaski-render", &[]).unwrap();
    let r = report(&s, &World(&[("kaski-render", "ef19a8e0c")]), &Heirs::new());
    assert!(
        r.output
            .contains("1 of them name only what the tree lost: ticket-move --close-gone\n"),
        "{}",
        r.output
    );
    assert!(
        r.output
            .ends_with("1 tickets may be stale: tickets check\n"),
        "{}",
        r.output
    );
    let s = Scratch::new();
    s.add("Move kaski-render into kaski-frame", &[]).unwrap();
    let r = report(&s, &World(&[("kaski-render", "ef19a8e0c")]), &Heirs::new());
    assert!(r.output.contains("kaski-render"), "{}", r.output);
    assert!(!r.output.contains("--close-gone"), "{}", r.output);
}

#[test]
fn nothing_is_printed_where_nothing_may_be_stale() {
    let s = Scratch::new();
    s.add("Alone", &[]).unwrap();
    let r = report(&s, &World(&[]), &Heirs::new());
    assert!(r.output.is_empty(), "{}", r.output);
}

#[test]
fn a_ticket_still_naming_a_superseded_row_is_listed_with_its_heir_and_not_counted_as_stale() {
    let s = Scratch::new();
    s.add("Uses it", &[("related", &["tenet::old"])]).unwrap();
    let heirs = Heirs::from([("old".to_string(), vec!["new".to_string()])]);
    let r = report(&s, &World(&[]), &heirs);
    assert!(
        r.output.contains("`old`") && r.output.contains("`new`"),
        "{}",
        r.output
    );
    assert!(!r.output.contains("may be stale"), "{}", r.output);
}
