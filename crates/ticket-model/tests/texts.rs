//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The store read from texts alone: a move is read wherever its file sits, so a move in a file of its
//! own and a move beside its ticket are one store, and what cannot be read is refused by name.

use mockspace_extra_ticket_model::{Move, Store, rules};

const STATES: &str = r#"
[[ticket_state]]
id = "waiting"
column = 1
needs_holder = true

[[ticket_state]]
id = "done"
column = 2
closed = true
satisfies = true
"#;

const KINDS: &str = r#"
[[ticket_kind]]
id = "question"
states = ["waiting", "done"]
initial = "waiting"
satisfied_by = ["answer", "picks"]
view = "question"
"#;

const TICKET: &str = r#"
[[ticket]]
id = "dig"
title = "Does digging need a tool"
kind = "question"
options = ["Faster with one", "Required"]

[[ticket_move]]
id = "m1"
ticket = "dig"
to = "waiting"
by = "coord"
at = "2026-10-27T10:00:00Z"
holder = "op"
"#;

const CLOSE: &str = r#"
[[ticket_move]]
id = "m2"
ticket = "dig"
to = "done"
by = "op"
at = "2026-10-27T11:00:00Z"
answer = "Faster, and a hand is a tool"
picks = ["Faster with one"]
"#;

fn store(files: &[(&str, &str)]) -> Result<Store, String> {
    Store::from_texts(files.iter().copied())
}

#[test]
fn a_move_in_a_file_of_its_own_is_read_with_its_ticket_and_closes_it() {
    let s = store(&[
        ("ticket/states.toml", STATES),
        ("ticket/kinds.toml", KINDS),
        ("ticket/items/dig.toml", TICKET),
        ("ticket/moves/dig/m2.toml", CLOSE),
    ])
    .unwrap();
    let now = s.now_of("dig").unwrap();
    assert_eq!(now.id, "m2");
    assert_eq!(now.answer, "Faster, and a hand is a tool");
    assert_eq!(now.picks, ["Faster with one"]);
    assert_eq!(s.state_of("dig").unwrap().id, "done");
    assert_eq!(s.tickets["dig"].file, "ticket/items/dig.toml");
    assert_eq!(
        s.kinds["question"].one("view"),
        "question",
        "a field the rules never read is kept"
    );
    assert!(
        rules::check_store(&s).is_empty(),
        "{:?}",
        rules::check_store(&s)
    );
}

#[test]
fn the_newest_move_wins_whatever_order_the_files_come_in() {
    let s = store(&[
        ("ticket/moves/dig/m2.toml", CLOSE),
        ("ticket/items/dig.toml", TICKET),
        ("ticket/kinds.toml", KINDS),
        ("ticket/states.toml", STATES),
    ])
    .unwrap();
    assert_eq!(s.now_of("dig").unwrap().id, "m2");
}

#[test]
fn an_id_held_twice_a_row_with_no_id_and_a_text_that_is_not_toml_are_refused_by_name() {
    let twice = store(&[("a.toml", TICKET), ("b.toml", TICKET)]).unwrap_err();
    assert!(twice.contains("`ticket::dig` is declared twice"), "{twice}");
    let none = store(&[("c.toml", "[[ticket_move]]\nticket = \"dig\"\n")]).unwrap_err();
    assert!(
        none.contains("a `ticket_move` row in c.toml has no id"),
        "{none}"
    );
    let broken = store(&[("d.toml", "[[ticket]\n")]).unwrap_err();
    assert!(broken.starts_with("cannot parse d.toml"), "{broken}");
}

#[test]
fn a_state_whose_column_is_no_whole_number_or_is_missing_is_refused_by_name() {
    for (column, said) in
        [("column = \"left\"\n", "left"), ("column = 1.5\n", "1.5"), ("", "no column")]
    {
        let text = format!("[[ticket_state]]\nid = \"waiting\"\n{column}needs_holder = true\n");
        let err = store(&[("ticket/states.toml", &text)]).unwrap_err();
        assert!(
            err.contains("`ticket_state::waiting`") && err.contains("column") && err.contains(said),
            "{err}"
        );
    }
    let ok = "[[ticket_state]]\nid = \"waiting\"\ncolumn = -3\n";
    assert_eq!(
        store(&[("ticket/states.toml", ok)]).unwrap().states["waiting"].column,
        -3
    );
}

/// The `dig` question with this line added to its row.
fn dig_with(line: &str) -> String {
    TICKET.replacen(
        "options = [\"Faster with one\", \"Required\"]\n",
        &format!("options = [\"Faster with one\", \"Required\"]\n{line}\n"),
        1,
    )
}

fn dig_store(ticket: &str) -> Store {
    store(&[
        ("ticket/states.toml", STATES),
        ("ticket/kinds.toml", KINDS),
        ("ticket/items/dig.toml", ticket),
    ])
    .unwrap()
}

#[test]
fn a_choice_that_is_neither_one_nor_many_and_one_on_a_ticket_with_no_options_are_named() {
    for good in ["", "choice = \"one\"", "choice = \"many\""] {
        let s = dig_store(&dig_with(good));
        assert!(
            rules::check_store(&s).is_empty(),
            "{good}: {:?}",
            rules::check_store(&s)
        );
    }
    let bad = rules::check_store(&dig_store(&dig_with("choice = \"several\"")));
    assert!(
        bad.iter()
            .any(|f| f.row == "ticket::dig" && f.message.contains("`several`")),
        "{bad:?}"
    );
    let no_options = TICKET.replace(
        "options = [\"Faster with one\", \"Required\"]\n",
        "choice = \"many\"\n",
    );
    let bare = rules::check_store(&dig_store(&no_options));
    assert!(
        bare.iter().any(|f| f.message.contains("no `options`")),
        "{bare:?}"
    );
    assert!(rules::check_links(&dig_store(&dig_with("choice = \"several\"")), "dig").is_err());
}

#[test]
fn a_question_is_closed_with_several_picks_only_where_it_says_many() {
    let two = |s: &Store| {
        let mut m = dig_move("m3", "done", "2026-10-27T12:00:00Z");
        m.answer = String::new();
        m.picks = vec!["Faster with one".into(), "Required".into()];
        rules::check_move(s, &m)
    };
    let err = two(&dig_store(&dig_with(""))).unwrap_err();
    assert!(err.contains("`dig`") && err.contains("one pick"), "{err}");
    assert!(two(&dig_store(&dig_with("choice = \"one\""))).is_err());
    assert!(two(&dig_store(&dig_with("choice = \"many\""))).is_ok());
    let mut one = dig_move("m3", "done", "2026-10-27T12:00:00Z");
    one.answer = String::new();
    one.picks = vec!["Required".into()];
    assert!(rules::check_move(&dig_store(&dig_with("")), &one).is_ok());
}

#[test]
fn a_ticket_takes_several_picks_only_where_its_choice_is_many() {
    let takes = |line: &str| dig_store(&dig_with(line)).tickets["dig"].takes_many();
    assert!(!takes(""));
    assert!(!takes("choice = \"one\""));
    assert!(takes("choice = \"many\""));
}

#[test]
fn a_question_may_hold_any_number_of_options() {
    let options: Vec<String> = (0 .. 12).map(|i| format!("\"Option {i}\"")).collect();
    let text = TICKET.replace(
        "options = [\"Faster with one\", \"Required\"]",
        &format!("options = [{}]", options.join(", ")),
    );
    let s = dig_store(&text);
    assert_eq!(s.tickets["dig"].many("options").len(), 12);
    assert!(
        rules::check_store(&s).is_empty(),
        "{:?}",
        rules::check_store(&s)
    );
}

#[test]
fn a_move_closing_a_question_is_checked_by_what_it_gives() {
    let s = store(&[
        ("ticket/states.toml", STATES),
        ("ticket/kinds.toml", KINDS),
        ("ticket/items/dig.toml", TICKET),
    ])
    .unwrap();
    let close = |answer: &str, picks: &[&str]| {
        Move {
            id: "m3".into(),
            ticket: "dig".into(),
            to: "done".into(),
            by: "op".into(),
            at: "2026-10-27T12:00:00Z".into(),
            answer: answer.into(),
            picks: picks.iter().map(|p| (*p).to_string()).collect(),
            ..Move::default()
        }
    };
    assert!(rules::check_move(&s, &close("", &[])).is_err());
    assert!(rules::check_move(&s, &close("Yes", &[])).is_ok());
    assert!(rules::check_move(&s, &close("", &["Required"])).is_ok());
    assert!(rules::check_move(&s, &close("", &["Neither"])).is_err());
}

/// A move of `dig` to `to` at `at`, under the id `id`, for a ticket that is waiting at `10:00:00Z`.
fn dig_move(id: &str, to: &str, at: &str) -> Move {
    Move {
        id: id.into(),
        ticket: "dig".into(),
        to: to.into(),
        by: "coord".into(),
        at: at.into(),
        holder: if to == "waiting" { "op".into() } else { String::new() },
        answer: if to == "done" { "Yes".into() } else { String::new() },
        ..Move::default()
    }
}

#[test]
fn two_moves_of_one_second_keep_the_order_the_file_has_them_in() {
    // the ids sort the other way round from the file, so an id is not what orders them
    let tied = r#"
[[ticket_move]]
id = "m9_later_id"
ticket = "dig"
to = "done"
by = "op"
at = "2026-10-27T10:00:00Z"
answer = "Yes"

[[ticket_move]]
id = "m1_earlier_id"
ticket = "dig"
to = "waiting"
by = "coord"
at = "2026-10-27T10:00:00Z"
holder = "op"
"#;
    let s = store(&[
        ("ticket/states.toml", STATES),
        ("ticket/kinds.toml", KINDS),
        ("ticket/items/dig.toml", TICKET),
        ("ticket/moves/dig/tied.toml", tied),
    ])
    .unwrap();
    let order: Vec<&str> = s.moves_of("dig").map(|m| m.id.as_str()).collect();
    assert_eq!(order, ["m1", "m9_later_id", "m1_earlier_id"], "{order:?}");
    assert_eq!(s.now_of("dig").unwrap().id, "m1_earlier_id");
    assert_eq!(s.state_of("dig").unwrap().id, "waiting");
}

#[test]
fn a_move_earlier_than_the_newest_is_refused_and_one_of_its_second_is_not() {
    let s = store(&[
        ("ticket/states.toml", STATES),
        ("ticket/kinds.toml", KINDS),
        ("ticket/items/dig.toml", TICKET),
        ("ticket/moves/dig/m2.toml", CLOSE),
    ])
    .unwrap();
    // the newest move of `dig` is its close at 11:00:00Z
    let earlier = rules::check_move(&s, &dig_move("m3", "done", "2026-10-27T10:59:59Z"));
    let err = earlier.unwrap_err();
    assert!(
        err.contains("2026-10-27T10:59:59Z") && err.contains("2026-10-27T11:00:00Z"),
        "the refusal says both times: {err}"
    );
    assert!(rules::check_move(&s, &dig_move("m3", "done", "2026-10-27T11:00:00Z")).is_ok());
    assert!(rules::check_move(&s, &dig_move("m3", "done", "2026-10-27T11:00:01Z")).is_ok());
}
