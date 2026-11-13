//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The last line of an answer says what the verb read and what it showed, and the count the engine
//! closes on is the same number, so a question that found nothing does not read as a clean census.

use mockspace::tool::Outcome;

use crate::tests::{answered, fixture, run, tree};

fn closing(args: &[&str]) -> (String, usize) {
    let r = run(&fixture(), &tree(), args);
    let text = answered(&r).lines().last().unwrap_or_default().to_owned();
    let Outcome::Clean {
        examined,
    } = r.outcome
    else {
        panic!("{:?}", r.outcome);
    };
    (text, examined)
}

#[test]
fn a_find_that_finds_nothing_says_what_it_read_and_showed_none() {
    assert_eq!(
        closing(&["find", "zzz-not-a-word"]),
        ("read 6 rows of 2 namespaces, shown 0".to_owned(), 6)
    );
}

#[test]
fn a_find_narrowed_to_the_tenets_says_it_read_the_tenets() {
    assert_eq!(
        closing(&["find", "sun", "--in", "tenet"]),
        ("read 4 tenets, shown 3".to_owned(), 4)
    );
    assert_eq!(
        closing(&["find", "zzz-not-a-word", "--in", "tenet"]),
        ("read 4 tenets, shown 0".to_owned(), 4)
    );
}

#[test]
fn a_find_shows_the_tenets_it_listed() {
    assert_eq!(
        closing(&["find", "sun"]),
        ("read 6 rows of 2 namespaces, shown 3".to_owned(), 6)
    );
}

#[test]
fn the_subjects_are_rows_the_verb_read() {
    assert_eq!(
        closing(&["subjects"]),
        ("read 4 tenets and 2 subjects, shown 2".to_owned(), 6)
    );
    assert_eq!(
        closing(&["subject", "sky"]),
        ("read 4 tenets and 2 subjects, shown 2".to_owned(), 6)
    );
}

#[test]
fn a_verb_that_walks_the_tree_says_so() {
    assert_eq!(
        closing(&["uncited"]),
        ("read 4 tenets and the tree, shown 2".to_owned(), 4)
    );
    assert_eq!(
        closing(&["related", "the_sun_rises"]),
        ("read 6 rows of 2 namespaces, shown 3".to_owned(), 6)
    );
}
