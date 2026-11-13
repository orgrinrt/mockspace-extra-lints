//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! A question says whether it takes one pick or many, and the makers write and judge the field.

use crate::args;
use crate::testkit::Scratch;

const OPTIONS: &[&str] = &["a: first", "b: second"];
/// A question is about a subject before it is held.
const ABOUT: &[&str] = &["subject::tickets"];

fn question(s: &Scratch, choice: &[&str]) -> Result<String, String> {
    let mut fields: Vec<(&str, &[&str])> = vec![("options", OPTIONS), ("related", ABOUT)];
    if !choice.is_empty() {
        fields.push(("choice", choice));
    }
    s.add_as("Which", "question", Some("waiting"), "op", &fields)
}

#[test]
fn both_makers_take_the_flag_and_write_it_to_the_row() {
    assert!(args::ADD_FLAGS.contains(&"--choice"));
    assert!(args::MOVE_FLAGS.contains(&"--choice"));
    let add = args::add(&["Which", "--choice", "many"]).unwrap();
    assert_eq!(add.fields["choice"], ["many"]);
    let mv = args::move_(&["t", "--choice", "one"]).unwrap();
    assert_eq!(mv.links["choice"], ["one"]);
    let s = Scratch::new();
    let id = question(&s, &["many"]).unwrap();
    let store = crate::load(&s.root).unwrap();
    assert!(store.tickets[&id].takes_many());
    assert!(
        s.read(&format!("ticket/items/{id}.toml"))
            .contains("choice = \"many\"")
    );
}

#[test]
fn a_choice_that_is_neither_is_refused_on_add_and_on_a_record_and_writes_nothing() {
    let s = Scratch::new();
    let err = question(&s, &["several"]).unwrap_err();
    assert!(err.contains("`several`"), "{err}");
    assert!(crate::load(&s.root).unwrap().tickets.is_empty());
    let id = question(&s, &[]).unwrap();
    let file = format!("ticket/items/{id}.toml");
    let before = s.read(&file);
    let err = s
        .links(&id, "", None, &[("choice", &["several"])])
        .unwrap_err();
    assert!(err.contains("`several`"), "{err}");
    assert_eq!(s.read(&file), before, "a refusal writes nothing");
    s.links(&id, "", None, &[("choice", &["many"])]).unwrap();
    assert!(crate::load(&s.root).unwrap().tickets[&id].takes_many());
    s.links(&id, "", None, &[("choice", &[])]).unwrap();
    assert!(
        !crate::load(&s.root).unwrap().tickets[&id].takes_many(),
        "an empty value clears it"
    );
}

#[test]
fn a_question_with_a_dozen_options_is_added_and_a_choice_with_none_is_refused() {
    let s = Scratch::new();
    let twelve: Vec<String> = (0 .. 12)
        .map(|i| format!("{}: option {i}", (b'a' + i as u8) as char))
        .collect();
    let words: Vec<&str> = twelve.iter().map(String::as_str).collect();
    s.add_as("Many", "question", Some("waiting"), "op", &[
        ("options", &words),
        ("choice", &["many"]),
        ("related", ABOUT),
    ])
    .unwrap();
    let err = s
        .add_as("None", "question", Some("backlog"), "", &[("choice", &[
            "many",
        ])])
        .unwrap_err();
    assert!(err.contains("no `options`"), "{err}");
}
