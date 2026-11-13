//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Marking a ticket with free-string `tags` and filing one under another by `ticket-move --part-of`:
//! each path that writes, each case the rules refuse with the file shown unchanged, and the listing
//! that finds a ticket by its tag.

use kaski_tickets::rules;

use crate::testkit::{Branches, Scratch};
use crate::view::{self, Filter};
use crate::{args, load, load_plan, write};

const T0: &str = "2026-10-23T10:00:00Z";
const T1: &str = "2026-10-23T11:00:00Z";

pub(crate) fn file(s: &Scratch, id: &str) -> String {
    s.read(&format!("ticket/items/{id}.toml"))
}

pub(crate) fn moves(s: &Scratch, id: &str) -> usize {
    load(&s.root).unwrap().moves_of(id).count()
}

/// `ticket-add`, from its command line.
pub(crate) fn added(s: &Scratch, words: &[&str]) -> Result<String, String> {
    let mut all = words.to_vec();
    all.extend(["--by", "tester", "--at", T0]);
    write::add(&s.root, &args::add(&all)?)
}

/// `ticket-move`, from its command line.
pub(crate) fn moved(s: &Scratch, words: &[&str]) -> Result<String, String> {
    let mut all = words.to_vec();
    all.extend(["--by", "tester", "--at", T1]);
    write::move_ticket(&s.root, &args::move_(&all)?)
}

fn tags_of(s: &Scratch, id: &str) -> Vec<String> {
    load(&s.root).unwrap().tickets[id].many("tags").to_vec()
}

fn listed(s: &Scratch, filter: &Filter) -> Vec<String> {
    let store = load(&s.root).unwrap();
    let plan = load_plan(&s.root).unwrap();
    view::list(&store, &plan, filter)
        .into_iter()
        .map(|r| r.id)
        .collect()
}

fn tagged(words: &[&str]) -> Filter {
    Filter {
        tag: words.iter().map(|w| (*w).to_string()).collect(),
        ..Filter::default()
    }
}

#[test]
fn a_ticket_added_with_tags_keeps_them_in_the_order_given_and_drops_an_empty_one() {
    let s = Scratch::new();
    let id = added(&s, &[
        "Mark",
        "--tags",
        "provisional-stamp,,second tag, third",
    ])
    .unwrap();
    assert_eq!(tags_of(&s, &id), [
        "provisional-stamp",
        "second tag",
        "third"
    ]);
    assert!(
        file(&s, &id).contains("tags = [\"provisional-stamp\", \"second tag\", \"third\"]\n"),
        "{}",
        file(&s, &id)
    );
    let bare = added(&s, &["Bare"]).unwrap();
    assert!(
        !file(&s, &bare).contains("tags"),
        "a ticket given none writes no line"
    );
}

#[test]
fn ticket_move_records_replaces_and_clears_tags_alone_and_with_a_move() {
    let s = Scratch::new();
    let id = added(&s, &["Mark"]).unwrap();
    assert_eq!(
        moved(&s, &[&id, "--tags", "provisional-stamp,other"]).unwrap(),
        "",
        "tags alone make no move"
    );
    assert_eq!(tags_of(&s, &id), ["provisional-stamp", "other"]);
    assert_eq!(moves(&s, &id), 1);
    moved(&s, &[&id, "--tags", "replaced"]).unwrap();
    assert_eq!(
        tags_of(&s, &id),
        ["replaced"],
        "a value replaces what the field held"
    );
    let made = moved(&s, &[&id, "ready", "--tags", "kept,with-a-move"]).unwrap();
    assert!(!made.is_empty(), "a state makes a move");
    assert_eq!(moves(&s, &id), 2);
    assert_eq!(tags_of(&s, &id), ["kept", "with-a-move"]);
    moved(&s, &[&id, "--tags", ""]).unwrap();
    assert!(
        tags_of(&s, &id).is_empty(),
        "an empty value clears the field"
    );
    assert!(!file(&s, &id).contains("tags"), "{}", file(&s, &id));
    let store = load(&s.root).unwrap();
    assert!(
        rules::check_store(&store).is_empty(),
        "{:?}",
        rules::check_store(&store)
    );
}

#[test]
fn ticket_move_files_a_ticket_that_exists_under_another_and_a_later_value_replaces_it() {
    let s = Scratch::new();
    let epic = added(&s, &["Epic"]).unwrap();
    let other = added(&s, &["Other"]).unwrap();
    let part = added(&s, &["Part"]).unwrap();
    assert_eq!(moved(&s, &[&part, "--part-of", &epic]).unwrap(), "");
    assert_eq!(load(&s.root).unwrap().tickets[&part].one("part_of"), epic);
    assert_eq!(moves(&s, &part), 1, "a link alone appends no move");
    moved(&s, &[&part, "--part-of", &other]).unwrap();
    assert_eq!(load(&s.root).unwrap().tickets[&part].one("part_of"), other);
    moved(&s, &[&part, "--part-of", ""]).unwrap();
    assert_eq!(load(&s.root).unwrap().tickets[&part].one("part_of"), "");
    let store = load(&s.root).unwrap();
    assert!(
        rules::check_store(&store).is_empty(),
        "{:?}",
        rules::check_store(&store)
    );
}

#[test]
fn a_part_of_that_a_rule_refuses_is_refused_and_the_file_is_as_it_was() {
    let s = Scratch::new();
    let epic = added(&s, &["Epic"]).unwrap();
    let part = added(&s, &["Part", "--part-of", &epic]).unwrap();
    let before = (file(&s, &epic), file(&s, &part));

    let err = moved(&s, &[&part, "--part-of", "ghost"]).unwrap_err();
    assert!(
        err.contains("`part_of` names `ghost`, which is no ticket"),
        "{err}"
    );
    let err = moved(&s, &[&part, "--part-of", &part]).unwrap_err();
    assert!(err.contains("`part_of` names the ticket itself"), "{err}");
    // the epic is part of its own part: a ring of two
    let err = moved(&s, &[&epic, "--part-of", &part]).unwrap_err();
    assert!(err.contains("`part_of` would go round"), "{err}");
    let err = moved(&s, &["ghost", "--part-of", &epic]).unwrap_err();
    assert!(err.contains("`ghost` is no ticket"), "{err}");

    assert_eq!(
        (file(&s, &epic), file(&s, &part)),
        before,
        "a refusal writes nothing"
    );
}

#[test]
fn the_listing_narrows_to_the_tickets_holding_any_tag_named_and_to_that_tag_whole() {
    let s = Scratch::new();
    let a = added(&s, &["Alpha", "--tags", "provisional-stamp"]).unwrap();
    let b = added(&s, &["Beta", "--tags", "other,more"]).unwrap();
    let c = added(&s, &["Gamma"]).unwrap();
    assert_eq!(
        listed(&s, &Filter::default()).len(),
        3,
        "no tag named lists all"
    );
    assert_eq!(listed(&s, &tagged(&["provisional-stamp"])), [a.clone()]);
    assert_eq!(listed(&s, &tagged(&["more"])), [b.clone()]);
    let mut both = listed(&s, &tagged(&["provisional-stamp", "more"]));
    both.sort();
    assert_eq!(both, [a.clone(), b.clone()]);
    assert!(
        listed(&s, &tagged(&["stamp"])).is_empty(),
        "a tag is matched whole and never as part of another"
    );
    assert!(listed(&s, &tagged(&["held-by-nobody"])).is_empty());
    assert!(!listed(&s, &tagged(&["more"])).contains(&c));
}

#[test]
fn next_narrows_on_a_tag_as_the_listing_does() {
    let s = Scratch::new();
    let a = added(&s, &["Alpha", "--tags", "provisional-stamp"]).unwrap();
    let b = added(&s, &["Beta"]).unwrap();
    let store = load(&s.root).unwrap();
    let plan = load_plan(&s.root).unwrap();
    let ids = |f: &Filter| -> Vec<String> {
        let mut v: Vec<String> = view::next(&store, &plan, f, &Branches::default())
            .into_iter()
            .map(|r| r.id)
            .collect();
        v.sort();
        v
    };
    let mut all = vec![a.clone(), b.clone()];
    all.sort();
    assert_eq!(ids(&Filter::default()), all);
    assert_eq!(ids(&tagged(&["provisional-stamp"])), [a]);
}

#[test]
fn the_flags_are_read_and_the_other_commands_flag_is_no_flag_here() {
    let l = args::tickets(&["list", "--tag", "a,b"]).unwrap();
    assert_eq!(l.filter.tag, ["a", "b"]);
    assert!(args::tickets(&["list"]).unwrap().filter.tag.is_empty());
    assert!(
        args::tickets(&["list", "--tags", "a"])
            .unwrap_err()
            .contains("`--tags` is no flag of tickets")
    );
    assert!(
        args::add(&["T", "--tag", "a"])
            .unwrap_err()
            .contains("`--tag` is no flag of ticket-add")
    );
    assert!(
        args::move_(&["t", "--tag", "a"])
            .unwrap_err()
            .contains("`--tag` is no flag of ticket-move")
    );
    let m = args::move_(&["t", "--tags", "a,b", "--part-of", "p"]).unwrap();
    assert_eq!(m.links["tags"], ["a", "b"]);
    assert_eq!(m.links["part_of"], ["p"]);
}

#[test]
fn add_tags_and_remove_tags_edit_the_list_the_ticket_holds_and_keep_the_rest() {
    // 7 November: `--tags` replaced a ticket's list, and the sweep lost four tags without a word
    // (ticket_move_tags_replaces_a_ticket_s_tags_instead_of_adding).
    // Mutation: the edit applied to an empty list instead of the held one, so `tooling` is lost.
    let s = Scratch::new();
    let id = added(&s, &["Mark", "--tags", "tooling,registry"]).unwrap();
    assert_eq!(
        moved(&s, &[&id, "--add-tags", "names-deleted-crate,tooling"]).unwrap(),
        ""
    );
    assert_eq!(tags_of(&s, &id), [
        "tooling",
        "registry",
        "names-deleted-crate"
    ]);
    let made = moved(&s, &[
        &id,
        "ready",
        "--remove-tags",
        "registry,never-held",
        "--add-tags",
        "x",
    ])
    .unwrap();
    assert!(!made.is_empty(), "a state with an edit makes a move");
    assert_eq!(tags_of(&s, &id), ["tooling", "names-deleted-crate", "x"]);
    moved(&s, &[&id, "--remove-tags", "tooling,names-deleted-crate,x"]).unwrap();
    assert!(
        tags_of(&s, &id).is_empty(),
        "taking every tag off clears the list"
    );
}

#[test]
fn tags_and_a_tag_edit_together_are_refused_and_nothing_is_written() {
    // Mutation: the edit overrides --tags or the reverse, so one of the two is silently dropped.
    let s = Scratch::new();
    let id = added(&s, &["Mark", "--tags", "kept"]).unwrap();
    let before = file(&s, &id);
    let err = moved(&s, &[&id, "--tags", "a", "--add-tags", "b"]).unwrap_err();
    assert!(err.contains("one or the other"), "{err}");
    assert_eq!(file(&s, &id), before);
}
