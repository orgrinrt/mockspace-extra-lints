//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `--repoint-superseded` as the command line gives it: no ticket named, the sweep written, nothing printed.

use std::collections::{BTreeMap, BTreeSet};

use mockspace::RegistryView;
use mockspace::tool::{Outcome, Tool, ToolContext, ToolReport};
use tickets::testkit::Scratch;
use tickets::{load, registry_dir};

use super::TicketMove;

fn tenets() -> RegistryView {
    let old: BTreeMap<String, String> =
        BTreeMap::from([("superseded_by".to_string(), "new".to_string())]);
    RegistryView::new(
        BTreeMap::from([
            ("tenet::old".to_string(), old),
            ("tenet::new".to_string(), BTreeMap::new()),
        ]),
        BTreeMap::new(),
    )
}

fn run(s: &Scratch, args: &[&str]) -> ToolReport {
    let crates = BTreeSet::new();
    let registry = tenets();
    TicketMove.run(&ToolContext {
        mock_dir: &s.mock,
        repo_root: &s.mock,
        all_crates: &crates,
        src_dirs: &[],
        args,
        stdin: None,
        registry: &registry,
    })
}

#[test]
fn the_flag_alone_repoints_every_ticket_naming_a_superseded_tenet_and_prints_nothing() {
    let s = Scratch::new();
    let a = s.add("Uses it", &[("related", &["tenet::old"])]).unwrap();
    let r = run(&s, &[
        "--repoint-superseded",
        "--by",
        "tester",
        "--at",
        "2026-11-08T10:00:00Z",
    ]);
    assert!(
        matches!(r.outcome, Outcome::Clean {
            examined: 1,
        }),
        "{:?}",
        r.outcome
    );
    assert!(r.output.is_empty(), "{}", r.output);
    let store = load(&registry_dir(&s.mock)).unwrap();
    assert_eq!(store.tickets[&a].many("related"), ["tenet::new"]);
}

#[test]
fn without_the_flag_a_ticket_is_still_asked_for() {
    let s = Scratch::new();
    let r = run(&s, &["--by", "tester"]);
    assert!(
        matches!(r.outcome, Outcome::Inconclusive { .. }),
        "{:?}",
        r.outcome
    );
}
