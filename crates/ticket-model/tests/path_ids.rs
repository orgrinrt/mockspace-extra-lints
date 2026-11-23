//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! A ticket's id as a path where the consumer's `ticket` namespace declares `path = ["kind"]`: the
//! kind is read from the id, a row writing it is refused, a new id is made from `--under` and a leaf
//! and may be no prefix of another's. A namespace without `path` is unchanged.

use mockspace_extra_ticket_model::path::{self, Declaration};
use mockspace_extra_ticket_model::rules::{self, NewRequest};
use mockspace_extra_ticket_model::Store;

const FOLDING: &str = r#"
[[registry.namespace]]
key = "ticket_kind"

[[registry.namespace]]
key = "ticket"
path = ["kind"]

[[registry.namespace.field]]
name = "kind"
type = "ticket_kind"
required = true
"#;

const PLAIN: &str = r#"
[[registry.namespace]]
key = "ticket"
"#;

const VOCABULARY: &str = r#"
[[ticket_state]]
id = "backlog"
column = 1

[[ticket_kind]]
id = "task"
states = ["backlog"]
initial = "backlog"

[[ticket_kind]]
id = "question"
states = ["backlog"]
initial = "backlog"
"#;

fn ticket(id: &str, extra: &str) -> String {
    format!(
        "[[ticket]]\nid = \"{id}\"\ntitle = \"t\"\n{extra}\n[[ticket_move]]\nid = \"m_{}\"\nticket = \"{id}\"\nto = \"backlog\"\nby = \"coord\"\nat = \"2026-11-02T10:00:00Z\"\n",
        id.replace("::", "_")
    )
}

fn store(config: &str, tickets: &[String]) -> Result<Store, String> {
    let decl = Declaration::from_config(config).unwrap();
    let mut files = vec![("vocab.toml".to_string(), VOCABULARY.to_string())];
    for (i, t) in tickets.iter().enumerate() {
        files.push((format!("t{i}.toml"), t.clone()));
    }
    Store::from_texts_declared(&decl, files.iter().map(|(p, s)| (p.as_str(), s.as_str())))
}

fn request(under: Option<&str>, leaf: Option<&str>, title: &str) -> NewRequest {
    NewRequest {
        title: title.into(),
        kind: "task".into(),
        under: under.map(String::from),
        leaf: leaf.map(String::from),
        by: "op".into(),
        at: "2026-11-08T10:00:00Z".into(),
        move_id: "m20261108100000_op_ab12".into(),
        ..NewRequest::default()
    }
}

#[test]
fn the_kind_is_read_from_the_id_and_not_from_the_row() {
    let s = store(FOLDING, &[ticket("task::engine::lattice::reads", "")]).unwrap();
    assert_eq!(s.tickets["task::engine::lattice::reads"].one("kind"), "task");
    assert_eq!(s.state_of("task::engine::lattice::reads").unwrap().id, "backlog");
}

#[test]
fn a_row_writing_the_folded_kind_is_refused() {
    let e = store(FOLDING, &[ticket("task::engine::reads", "kind = \"task\"\n")]).unwrap_err();
    assert!(e.contains("folded-field-written") && e.contains("`kind`"), "{e}");
}

#[test]
fn an_id_whose_first_name_is_no_kind_is_refused() {
    let e = store(FOLDING, &[ticket("chore::engine::reads", "")]).unwrap_err();
    assert!(e.contains("does not read"), "{e}");
}

#[test]
fn without_path_the_row_writes_its_kind_as_before() {
    let s = store(PLAIN, &[ticket("plain_id", "kind = \"task\"\n")]).unwrap();
    assert_eq!(s.tickets["plain_id"].one("kind"), "task");
    assert!(!s.decl.folds());
}

#[test]
fn under_and_leaf_make_the_id() {
    let s = store(FOLDING, &[]).unwrap();
    let made = rules::new_ticket(&s, &request(Some("engine::lattice"), Some("reads"), "Reads")).unwrap();
    assert_eq!(made.id, "task::engine::lattice::reads");
    assert_eq!(made.first.ticket, made.id);
}

#[test]
fn the_leaf_is_the_first_four_words_of_the_title_that_are_not_function_words() {
    assert_eq!(path::leaf_of_title("Add the cost of a neighbour read to the lattice"), "add_cost_neighbour_read");
    assert_eq!(path::leaf_of_title("2026 plan"), "t_2026_plan");
    let s = store(FOLDING, &[]).unwrap();
    let made = rules::new_ticket(&s, &request(Some("engine"), None, "Wire the lamp to the wall")).unwrap();
    assert_eq!(made.id, "task::engine::wire_lamp_wall");
}

#[test]
fn a_folding_namespace_wants_under_or_a_whole_id() {
    let s = store(FOLDING, &[]).unwrap();
    let e = rules::new_ticket(&s, &request(None, None, "Reads")).unwrap_err();
    assert!(e.contains("--under"), "{e}");
}

#[test]
fn a_whole_id_is_checked_and_must_agree_with_the_kind() {
    let s = store(FOLDING, &[]).unwrap();
    let mut r = request(None, None, "Reads");
    r.id = Some("task::engine::reads".into());
    assert_eq!(rules::new_ticket(&s, &r).unwrap().id, "task::engine::reads");
    r.id = Some("question::engine::reads".into());
    let e = rules::new_ticket(&s, &r).unwrap_err();
    assert!(e.contains("reads `kind` as `question`"), "{e}");
    r.id = Some("task::Engine::reads".into());
    assert!(rules::new_ticket(&s, &r).is_err());
}

#[test]
fn an_id_that_is_a_prefix_of_another_or_has_one_as_its_prefix_is_refused() {
    let s = store(FOLDING, &[ticket("task::engine::lattice::reads", "")]).unwrap();
    let e = rules::new_ticket(&s, &request(Some("engine"), Some("lattice"), "x")).unwrap_err();
    assert!(e.contains("is a proper prefix of"), "{e}");
    let e = rules::new_ticket(&s, &request(Some("engine::lattice::reads"), Some("deeper"), "x")).unwrap_err();
    assert!(e.contains("is a proper prefix of"), "{e}");
    assert!(rules::new_ticket(&s, &request(Some("engine::lattice"), Some("writes"), "x")).is_ok());
}

#[test]
fn a_committed_prefix_clash_is_a_finding() {
    let s = store(FOLDING, &[
        ticket("task::engine::lattice", ""),
        ticket("task::engine::lattice::reads", ""),
    ])
    .unwrap();
    let found = rules::check_store(&s);
    assert!(found.iter().any(|f| f.message.contains("is a proper prefix of")), "{found:?}");
}

#[test]
fn under_and_leaf_are_refused_where_nothing_folds() {
    let s = store(PLAIN, &[]).unwrap();
    let e = rules::new_ticket(&s, &request(Some("engine"), None, "Reads")).unwrap_err();
    assert!(e.contains("declares no `path`"), "{e}");
}

#[test]
fn a_twelve_to_forty_hex_value_looks_like_a_sha() {
    assert!(path::refuse_sha("217cb11a9f3e").unwrap_err().contains("looks like a commit sha; a ticket is named by its id, never a sha"));
    assert!(path::refuse_sha(&"a".repeat(40)).is_err());
    assert!(path::refuse_sha(&"a".repeat(41)).is_ok());
    assert!(path::refuse_sha("deadbeef").is_ok());
    assert!(path::refuse_sha("task::engine::reads").is_ok());
}

#[test]
fn the_file_of_an_id_nests_by_its_names_and_its_leaf_is_the_last() {
    assert_eq!(path::file_stem("task::engine::reads"), "task/engine/reads");
    assert_eq!(path::file_stem("plain"), "plain");
    assert_eq!(path::leaf("task::engine::reads"), "reads");
    assert_eq!(path::leaf("plain"), "plain");
}
