//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The question kind: a question put to op is a ticket that waits under his name with the options
//! it was put with, and his answer closes it. The scratch registry holds the kinds the repository
//! ships, so these read the same `question` row the board reads.

use kaski_tickets::rules;

use crate::args;
use crate::testkit::{Scratch, committed_registry};

const T0: &str = "2026-10-26T10:00:00Z";
const T1: &str = "2026-10-26T11:00:00Z";
const T2: &str = "2026-10-26T12:00:00Z";

const DIG: &[&str] = &[
    "Tool makes it faster: bare hands can dig, slowly, and a spade speeds it by its quality",
    "Tool required: no digging without a digging tool in hand",
];

/// The subject a question is put under: the kind requires one before op holds it.
const SUBJECT: &[&str] = &["subject::engine"];

#[test]
fn the_question_kind_is_shipped_with_the_states_a_question_passes_through() {
    let store = crate::load(&committed_registry()).expect("the committed registry reads");
    let kind = store
        .kinds
        .get("question")
        .expect("a `question` kind is shipped");
    assert_eq!(kind.states, ["backlog", "waiting", "done", "dropped"]);
    assert_eq!(kind.initial, "backlog");
}

#[test]
fn a_question_is_put_to_op_held_by_him_and_closed_by_his_answer() {
    let s = Scratch::new();
    let id = s
        .add_as("Does digging need a tool", "question", None, "", &[
            ("options", DIG),
            ("related", SUBJECT),
        ])
        .unwrap();
    s.mv(&id, "waiting", Some("op"), T1).unwrap();
    let store = crate::load(&s.root).unwrap();
    assert_eq!(store.state_of(&id).unwrap().id, "waiting");
    assert_eq!(store.now_of(&id).unwrap().holder, "op");
    s.answer(&id, "done", "", &[DIG[1]]).unwrap();
    let store = crate::load(&s.root).unwrap();
    assert_eq!(store.state_of(&id).unwrap().id, "done");
    assert_eq!(store.now_of(&id).unwrap().picks, [DIG[1]]);
}

#[test]
fn a_question_put_to_op_under_no_subject_is_refused_and_writes_nothing() {
    let s = Scratch::new();
    // put in one call
    let refused = s
        .add_as(
            "Does digging need a tool",
            "question",
            Some("waiting"),
            "op",
            &[("options", DIG)],
        )
        .unwrap_err();
    assert!(
        refused.contains("a `subject` link in `related`"),
        "{refused}"
    );
    assert!(crate::load(&s.root).unwrap().tickets.is_empty());
    // put later, from the backlog
    let id = s
        .add_as("Does digging need a tool", "question", None, "", &[(
            "options", DIG,
        )])
        .unwrap();
    let refused = s.mv(&id, "waiting", Some("op"), T1).unwrap_err();
    assert!(
        refused.contains("a `subject` link in `related`"),
        "{refused}"
    );
    assert_eq!(
        crate::load(&s.root).unwrap().state_of(&id).unwrap().id,
        "backlog",
        "the refused move was written"
    );
    // a question not put is held to nothing: it can be dropped under none
    s.mv(&id, "dropped", None, T1).unwrap();
}

#[test]
fn a_question_waiting_on_nobody_is_refused() {
    let s = Scratch::new();
    let id = s
        .add_as("Who holds this", "question", None, "", &[])
        .unwrap();
    let refused = s.mv(&id, "waiting", None, T1);
    assert!(refused.is_err(), "a question with no holder was let wait");
}

#[test]
fn a_question_does_not_start_or_review_as_work() {
    let s = Scratch::new();
    let id = s.add_as("Is this work", "question", None, "", &[]).unwrap();
    for state in ["doing", "review", "ready"] {
        assert!(
            s.mv(&id, state, Some("op"), T1).is_err(),
            "a question moved to `{state}`, which its kind does not list"
        );
    }
}

#[test]
fn the_options_are_kept_whole_with_their_commas_and_in_the_order_given() {
    let s = Scratch::new();
    let id = s
        .add_as("Does digging need a tool", "question", None, "", &[(
            "options", DIG,
        )])
        .unwrap();
    let store = crate::load(&s.root).unwrap();
    assert_eq!(store.tickets[&id].many("options"), DIG);
}

#[test]
fn the_options_flag_splits_on_a_vertical_bar_and_never_on_a_comma() {
    let r = args::add(&[
        "Does digging need a tool",
        "--kind",
        "question",
        "--options",
        "Tool makes it faster: bare hands dig, slowly | Tool required",
    ])
    .unwrap();
    assert_eq!(r.fields["options"], [
        "Tool makes it faster: bare hands dig, slowly",
        "Tool required"
    ]);
    let r = args::add(&["Needs", "--needs", "a, b"]).unwrap();
    assert_eq!(
        r.fields["needs"],
        ["a", "b"],
        "the other lists keep the comma"
    );
}

#[test]
fn the_preview_flag_names_the_pages_a_question_depends_on_and_the_row_keeps_them() {
    let r = args::add(&[
        "Is the barn the right size",
        "--kind",
        "question",
        "--preview",
        "https://barn.example.dev/a, https://barn.example.dev/b",
    ])
    .unwrap();
    assert_eq!(r.fields["preview"], [
        "https://barn.example.dev/a",
        "https://barn.example.dev/b"
    ]);
    let s = Scratch::new();
    let id = s
        .add_as("Is the barn the right size", "question", None, "", &[(
            "preview",
            &["https://barn.example.dev/a"],
        )])
        .unwrap();
    let store = crate::load(&s.root).expect("the registry reads with the field in it");
    assert_eq!(store.tickets[&id].many("preview"), [
        "https://barn.example.dev/a"
    ]);
    assert!(
        s.read(&format!("ticket/items/{id}.toml"))
            .contains("preview = [\"https://barn.example.dev/a\"]")
    );
}

/// A question titled `title`, put and waiting under op's name, as every test of its closing starts
/// from.
fn waiting(s: &Scratch, title: &str) -> String {
    let id = s
        .add_as(title, "question", None, "", &[
            ("options", DIG),
            ("related", SUBJECT),
        ])
        .unwrap();
    s.mv(&id, "waiting", Some("op"), T0).unwrap();
    id
}

#[test]
fn a_question_is_not_closed_without_what_answered_it() {
    let s = Scratch::new();
    let id = waiting(&s, "Does digging need a tool");
    let refused = s.mv(&id, "done", None, T1);
    assert!(
        refused
            .as_ref()
            .is_err_and(|e| e.contains("`answer` or `picks`")),
        "a question closed with no answer and no picks: {refused:?}"
    );
    let store = crate::load(&s.root).unwrap();
    assert_eq!(
        store.state_of(&id).unwrap().id,
        "waiting",
        "nothing is written when refused"
    );
}

#[test]
fn the_words_alone_or_the_picks_alone_close_a_question_and_both_are_kept_as_given() {
    let s = Scratch::new();
    let words = waiting(&s, "Does digging need a tool");
    s.answer(
        &words,
        "done",
        "Neither: a tool only changes the pace, and a hand is a tool",
        &[],
    )
    .unwrap();
    let both = waiting(&s, "Does a spade speed it");
    s.links(&both, "", None, &[("choice", &["many"])]).unwrap();
    s.answer(&both, "done", "The second, for now", &[DIG[1], DIG[0]])
        .unwrap();
    let store = crate::load(&s.root).unwrap();
    let now = store.now_of(&words).unwrap();
    assert_eq!(
        now.answer,
        "Neither: a tool only changes the pace, and a hand is a tool"
    );
    assert!(now.picks.is_empty());
    let now = store.now_of(&both).unwrap();
    assert_eq!(now.answer, "The second, for now");
    assert_eq!(
        now.picks,
        [DIG[1], DIG[0]],
        "the picks keep the order they were chosen in"
    );
    assert!(
        rules::check_store(&store).is_empty(),
        "{:?}",
        rules::check_store(&store)
    );
}

#[test]
fn an_answer_on_a_move_that_does_not_satisfy_is_refused() {
    let s = Scratch::new();
    let id = s
        .add_as("Is digging slow", "question", None, "", &[])
        .unwrap();
    let refused = s.answer(&id, "waiting", "Already", &[]);
    assert!(refused.is_err(), "an answer rode on a move to `waiting`");
    let id = waiting(&s, "Does digging need a tool");
    let refused = s.answer(&id, "dropped", "", &[DIG[0]]);
    assert!(
        refused.is_err(),
        "picks rode on a move to `dropped`, which does not satisfy"
    );
}

#[test]
fn a_question_dropped_or_a_task_done_needs_no_answer() {
    let s = Scratch::new();
    let id = waiting(&s, "Does digging need a tool");
    s.mv(&id, "dropped", None, T1).unwrap();
    let task = s.add("Dig a ditch", &[]).unwrap();
    s.mv(&task, "done", None, T2).unwrap();
    let store = crate::load(&s.root).unwrap();
    assert_eq!(store.state_of(&id).unwrap().id, "dropped");
    assert_eq!(store.state_of(&task).unwrap().id, "done");
}

#[test]
fn a_link_recorded_on_an_answered_question_keeps_the_answer_standing() {
    let s = Scratch::new();
    let id = waiting(&s, "Does digging need a tool");
    s.answer(&id, "done", "", &[DIG[0]]).unwrap();
    s.links(&id, "", None, &[("branch", &["work/dig"])])
        .unwrap();
    let store = crate::load(&s.root).unwrap();
    assert_eq!(store.tickets[&id].one("branch"), "work/dig");
    assert_eq!(store.now_of(&id).unwrap().picks, [DIG[0]]);
}

#[test]
fn a_closing_move_written_by_hand_without_an_answer_is_found() {
    let s = Scratch::new();
    let id = waiting(&s, "Does digging need a tool");
    s.append(
        &format!("ticket/items/{id}.toml"),
        &format!(
            "\n[[ticket_move]]\nid = \"m20261026120000_op_hand\"\nticket = \"{id}\"\nto = \"done\"\nby = \"op\"\nat = \"{T2}\"\n"
        ),
    );
    let found = rules::check_store(&crate::load(&s.root).unwrap());
    assert!(
        found.iter().any(|f| {
            f.row == "ticket_move::m20261026120000_op_hand"
                && f.message.contains("`answer` or `picks`")
        }),
        "{found:?}"
    );
}

#[test]
fn the_move_flags_take_the_answer_whole_and_split_the_picks_on_a_vertical_bar() {
    let r = args::move_(&[
        "dig",
        "done",
        "--answer",
        "A tool, yes, but slowly without",
        "--picks",
        "Tool makes it faster: bare hands dig, slowly | Tool required",
    ])
    .unwrap();
    assert_eq!(r.answer, "A tool, yes, but slowly without");
    assert_eq!(r.picks, [
        "Tool makes it faster: bare hands dig, slowly",
        "Tool required"
    ]);
}

#[test]
fn a_pick_that_is_not_one_of_the_options_is_refused() {
    let s = Scratch::new();
    let id = waiting(&s, "Does digging need a tool");
    let refused = s.answer(&id, "done", "", &["Tool required, or a strong back"]);
    assert!(
        refused
            .as_ref()
            .is_err_and(|e| e.contains("not one of the options")),
        "a pick no option reads was taken: {refused:?}"
    );
}

#[test]
fn a_stamp_is_tagged_for_the_console_s_chip_when_it_is_added_or_called_one() {
    // 7 November: four stamps were missing from the console's stamp chip for want of the tag (proposal A7).
    // Mutation: the tag not written on a move whose why calls it a stamp, or not on an add whose id says so.
    use crate::tests_tags::{added, moved};
    let s = Scratch::new();
    let tags = |id: &str| {
        crate::load(&s.root).unwrap().tickets[id]
            .many("tags")
            .to_vec()
    };
    let id = s
        .add_as("Does digging need a tool", "question", None, "", &[
            ("options", DIG),
            ("related", SUBJECT),
        ])
        .unwrap();
    assert!(tags(&id).is_empty());
    moved(&s, &[
        &id,
        "waiting",
        "--holder",
        "op",
        "--why",
        "Provisional stamp, holds as ratified until op confirms it",
    ])
    .unwrap();
    assert_eq!(tags(&id), ["provisional-stamp"]);
    let stamp = added(&s, &[
        "Coord stamped a",
        "--id",
        "stamp_coord_stamped_a",
        "--kind",
        "question",
        "--options",
        "a: yes|b: no",
        "--related",
        "subject::engine",
        "--tags",
        "kept",
    ])
    .unwrap();
    assert_eq!(tags(&stamp), ["kept", "provisional-stamp"]);
    // control: a task called a stamp in its why is not a question, and is left untagged
    let task = added(&s, &["A task"]).unwrap();
    moved(&s, &[&task, "ready", "--why", "provisional stamp"]).unwrap();
    assert!(tags(&task).is_empty());
}
