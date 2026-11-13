//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What git says of a name a ticket mentions, in a real repository: held once and deleted, held still,
//! and never held.

use kaski_tickets::Tree;

use crate::landing::Git;
use crate::tests_branch::{commit, git};

fn repository(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("tickets-tree-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("mock/crates/kaski-lost")).unwrap();
    std::fs::create_dir_all(dir.join("mock/crates/kaski-kept")).unwrap();
    git(&dir, &["init", "-q", "-b", "dev"]);
    commit(&dir, "base");
    for krate in ["kaski-lost", "kaski-kept"] {
        let file = format!("mock/crates/{krate}/Cargo.toml");
        commit(&dir, &file);
    }
    git(&dir, &["rm", "-q", "-r", "mock/crates/kaski-lost"]);
    git(&dir, &["commit", "-q", "-m", "delete the lost crate"]);
    dir
}

#[test]
fn a_crate_the_trunk_deleted_is_gone_with_the_commit_that_deleted_it() {
    let dir = repository("lost");
    let said = Git::at(&dir).gone("kaski-lost").expect("gone");
    assert!(said.contains("delete the lost crate"), "{said}");
}

#[test]
fn a_crate_the_trunk_still_holds_is_not_gone() {
    let dir = repository("kept");
    assert_eq!(Git::at(&dir).gone("kaski-kept"), None);
}

#[test]
fn a_name_the_trunk_never_held_is_not_gone() {
    let dir = repository("never");
    assert_eq!(Git::at(&dir).gone("kaski-never"), None);
}

#[test]
fn a_path_is_gone_when_deleted_and_held_when_it_stands() {
    let dir = repository("path");
    let git = Git::at(&dir);
    assert!(git.gone("mock/crates/kaski-lost/Cargo.toml").is_some());
    assert_eq!(git.gone("mock/crates/kaski-kept/Cargo.toml"), None);
}

#[test]
fn a_name_asked_twice_is_answered_alike() {
    let dir = repository("twice");
    let git = Git::at(&dir);
    assert_eq!(git.gone("kaski-lost"), git.gone("kaski-lost"));
}
