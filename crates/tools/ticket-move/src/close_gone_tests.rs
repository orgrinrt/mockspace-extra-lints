//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `--close-gone` as the command line gives it, over a repository whose trunk deleted a crate.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use mockspace::RegistryView;
use mockspace::tool::{Outcome, Tool, ToolContext, ToolReport};
use tickets::testkit::Scratch;
use tickets::{load, registry_dir};

use super::TicketMove;

/// The marker word is written `§` here, so this file holds no marker of its own.
fn t(s: &str) -> String {
    s.replace('§', "FIXME")
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A scratch whose mock directory is a repository on `dev` that held `kaski-lost` and deleted it.
fn scratch_with_a_deleted_crate() -> Scratch {
    let s = Scratch::new();
    let krate = s.mock.join("mock/crates/kaski-lost");
    std::fs::create_dir_all(&krate).unwrap();
    std::fs::write(krate.join("Cargo.toml"), "x").unwrap();
    // the prefix a crate is named by, which a ticket naming `kaski-lost` is read against
    std::fs::write(s.mock.join("mockspace.toml"), "crate_prefix = \"kaski\"\n").unwrap();
    git(&s.mock, &["init", "-q", "-b", "dev"]);
    git(&s.mock, &["add", "mock/crates/kaski-lost/Cargo.toml"]);
    git(&s.mock, &["commit", "-q", "-m", "add the lost crate"]);
    git(&s.mock, &["rm", "-q", "-r", "mock/crates/kaski-lost"]);
    git(&s.mock, &["commit", "-q", "-m", "delete the lost crate"]);
    s
}

fn run(s: &Scratch, args: &[&str]) -> ToolReport {
    let crates = BTreeSet::new();
    let dirs = [s.mock.join("mock/crates")];
    let registry = RegistryView::default();
    TicketMove.run(&ToolContext {
        mock_dir: &s.mock,
        repo_root: &s.mock,
        all_crates: &crates,
        src_dirs: &dirs,
        args,
        stdin: None,
        registry: &registry,
    })
}

fn state(s: &Scratch, id: &str) -> String {
    let store = load(&registry_dir(&s.mock)).unwrap();
    store.now_of(id).map(|m| m.to.clone()).unwrap_or_default()
}

const AT: [&str; 4] = ["--by", "tester", "--at", "2026-11-08T10:00:00Z"];

fn sweep(s: &Scratch, extra: &[&str]) -> ToolReport {
    let mut args = vec!["--close-gone"];
    args.extend(extra);
    args.extend(AT);
    run(s, &args)
}

#[test]
fn the_flag_alone_drops_a_ticket_naming_only_a_deleted_crate_and_says_how_many_it_closed_apart_from_how_many_it_looked_at()
 {
    let s = scratch_with_a_deleted_crate();
    let gone = s.add("Port kaski-lost", &[]).unwrap();
    let live = s.add("Port kaski-lost into kaski-live", &[]).unwrap();
    s.add("Something else", &[]).unwrap();
    let r = sweep(&s, &[]);
    assert!(
        matches!(r.outcome, Outcome::Clean {
            examined: 3,
        }),
        "three open tickets were examined: {:?}",
        r.outcome
    );
    assert_eq!(r.output, "closed 1\n", "one of the three was closed");
    assert_eq!(state(&s, &gone), "dropped");
    assert_eq!(state(&s, &live), "backlog");
}

#[test]
fn a_ticket_a_marker_cites_stays_open_and_is_named_until_a_successor_takes_the_marker() {
    let s = scratch_with_a_deleted_crate();
    let old = s.add("Port kaski-lost", &[]).unwrap();
    let new = s.add("Replacement", &[]).unwrap();
    let src = s.mock.join("src/a.rs");
    std::fs::create_dir_all(src.parent().unwrap()).unwrap();
    std::fs::write(&src, t(&format!("// §({old}): the port\n"))).unwrap();
    let r = sweep(&s, &[]);
    assert!(r.output.contains(&old), "{}", r.output);
    assert_eq!(state(&s, &old), "backlog");
    assert_eq!(
        std::fs::read_to_string(&src).unwrap(),
        t(&format!("// §({old}): the port\n"))
    );
    let r = sweep(&s, &["--for", &new]);
    assert_eq!(r.output, "closed 1\n");
    assert_eq!(state(&s, &old), "dropped");
    assert_eq!(
        std::fs::read_to_string(&src).unwrap(),
        t(&format!("// §({new}): the port\n"))
    );
}

#[test]
fn with_no_trunk_to_say_what_was_deleted_nothing_is_closed() {
    let s = Scratch::new();
    let a = s.add("Port kaski-lost", &[]).unwrap();
    let r = sweep(&s, &[]);
    assert!(
        matches!(r.outcome, Outcome::Clean {
            examined: 1,
        }),
        "the one open ticket was looked at and none closed: {:?}",
        r.outcome
    );
    assert_eq!(r.output, "closed 0\n");
    assert_eq!(state(&s, &a), "backlog");
}
