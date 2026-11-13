//! `tickets answerable`: beside each open question, the candidates the repository index holds, so a
//! question is not put to a person that a row or a note already answers.

use crate::answerable::{self, Candidate};
use crate::args::{self, Verb};
use crate::testkit::Scratch;
use crate::view::{self, Filter};

fn asked(s: &Scratch, ask: &dyn Fn(&str) -> Vec<Candidate>) -> String {
    let store = crate::load(&s.root).unwrap();
    let plan = crate::load_plan(&s.root).unwrap();
    let rows = view::list(
        &store,
        &plan,
        &answerable::open_questions(&Filter::default()),
    );
    answerable::report(&store, &rows, ask).0
}

fn question(s: &Scratch, title: &str, options: &[&str]) -> String {
    s.add_as(title, "question", Some("waiting"), "op", &[
        ("options", options),
        ("related", &["subject::engine"]),
    ])
    .unwrap()
}

fn cand(path: &str) -> Candidate {
    Candidate {
        path:    path.to_string(),
        line:    3,
        class:   "human".to_string(),
        excerpt: "said it".to_string(),
        score:   1.0,
    }
}

#[test]
fn each_open_question_is_listed_with_the_candidates_asked_of_its_title_and_options() {
    let s = Scratch::new();
    let id = question(&s, "Which air curve", &[
        "steep: a steep one",
        "flat: a flat one",
    ]);
    let seen = std::cell::RefCell::new(Vec::new());
    let out = asked(&s, &|text| {
        seen.borrow_mut().push(text.to_string());
        vec![cand("op-responses/air.md")]
    });
    assert!(out.contains(&id), "{out}");
    assert!(out.contains("op-responses/air.md:3"), "{out}");
    let text = &seen.borrow()[0];
    assert!(
        text.contains("Which air curve") && text.contains("steep"),
        "{text}"
    );
}

#[test]
fn a_question_with_no_candidate_says_so_and_a_task_is_not_listed() {
    let s = Scratch::new();
    let id = question(&s, "Which sea", &["deep: deep"]);
    let task = s.add("Build a thing", &[]).unwrap();
    let out = asked(&s, &|_| Vec::new());
    assert!(out.contains(&id) && out.contains("no candidate"), "{out}");
    // THE CONTROL: only the question is asked about.
    assert!(!out.contains(&task), "{out}");
}

#[test]
fn the_verb_is_parsed() {
    assert_eq!(
        args::tickets(&["answerable"]).unwrap().verb,
        Verb::Answerable
    );
}
