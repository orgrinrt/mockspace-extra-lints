//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The maker run on the fixture trees of `tests`: what a run reads and leaves alone, its flags, its
//! titles, and the markers citing a question already answered.

use mockspace::tool::Outcome;

use super::*;
use crate::tests::*;

#[test]
fn closed_rounds_the_research_and_generated_files_are_read_for_no_ticket() {
    let r = Repo::new();
    r.write(
        "mock/design_rounds/2026/202610010000_topic.x.md",
        "§(canon silent): a quotation\n",
    );
    r.write("mock/research/20261028_x/NOTES.md", "§(maintainer): a quotation\n");
    r.write("docs/DESIGN.md", "§: generated\n");
    r.write("mock/vendor/a/src/lib.rs", "// §: vendored\n");
    let snap = r.snapshot();
    assert_eq!(r.run(&[]), "");
    assert_eq!(r.snapshot(), snap);
}

#[test]
fn a_marker_citing_a_ticket_that_exists_is_left_alone_and_one_citing_none_is_not() {
    let r = Repo::new();
    r.write(SIM, "// §(stand-in): a flat roof for now\n");
    r.run(&[]);
    let id = first_cited(&r.read(SIM));
    r.write(WEATHER, &format!("// §({id}): the same gap, cited by hand\n// §(no_such_ticket): cites a ticket nobody opened\n"));
    r.run(&[]);
    let weather = fixme_markers::scan(&r.read(WEATHER), fixme_markers::Kind::Slashes);
    assert_eq!(weather[0].tag.as_deref(), Some(id.as_str()));
    assert_ne!(weather[1].tag.as_deref(), Some("no_such_ticket"));
    assert!(
        r.store()
            .tickets
            .contains_key(weather[1].tag.as_deref().unwrap())
    );
}

#[test]
fn a_marker_that_lost_its_colon_is_rewritten_with_one() {
    let r = Repo::new();
    r.write(SIM, "// § the sun is a constant\n");
    r.run(&[]);
    let id = first_cited(&r.read(SIM));
    assert_eq!(
        r.read(SIM),
        t(&format!("// §({id}): the sun is a constant\n"))
    );
}

#[test]
fn only_limits_the_run_to_the_files_it_names_and_closes_nothing() {
    let r = Repo::new();
    r.write(SIM, "// §: the wind is not read\n");
    r.run(&[]);
    let board = r.board();
    // the marker goes and another file gains one, and a run over the second file alone leaves the
    // first ticket open though nothing cites it
    r.write(SIM, "fn done() {}\n");
    r.write(WEATHER, "// §: rain never wets the ground\n");
    r.run(&["--only", "*kaski-weather*"]);
    let after = r.board();
    assert_eq!(after.len(), 2);
    let old = board.keys().next().unwrap();
    assert_eq!(after[old].1, "backlog", "a partial run closed a ticket");
    // and a file it does not name is left as it is
    r.write(SIM, "// § another one\n");
    let snap = r.read(SIM);
    r.run(&["--only", "*.md.tmpl"]);
    assert_eq!(r.read(SIM), snap);
}

#[test]
fn dry_says_what_it_would_do_and_writes_nothing() {
    let r = Repo::new();
    r.write(SIM, "// §(stand-in): a flat roof for now\n");
    let snap = r.snapshot();
    let out = r.run(&["--dry"]);
    assert!(out.starts_with("would file task "), "{out}");
    assert!(out.contains("mock/crates/kaski-sim/src/lib.rs:1"), "{out}");
    assert_eq!(r.snapshot(), snap);
}

#[test]
fn a_ticket_is_about_the_subject_a_crate_is_named_for_and_none_otherwise() {
    let r = Repo::new();
    r.write(WEATHER, "// §: rain never wets the ground\n");
    r.write(SIM, "// §: the clock is read twice\n");
    let out = r.run_with(&[], &["weather"]);
    assert!(matches!(out.outcome, Outcome::Clean { .. }));
    let s = r.store();
    for (_, t) in &s.tickets {
        let about = t.many("related");
        if t.one("title").starts_with("kaski-weather") {
            assert_eq!(about, ["subject::weather"]);
        } else {
            assert!(about.is_empty(), "{about:?}");
        }
    }
}

#[test]
fn two_gaps_with_the_same_title_get_two_ids() {
    let r = Repo::new();
    r.write(SIM, "// §: wind\n");
    r.write(
        "mock/crates/kaski-sim/src/other.rs",
        "// §: wind\n// §: wind\n",
    );
    r.run(&[]);
    let ids = r.board().keys().cloned().collect::<Vec<_>>();
    assert!(!ids.is_empty());
    let cited: Vec<String> = ["mock/crates/kaski-sim/src/other.rs", SIM]
        .iter()
        .flat_map(|p| fixme_markers::scan(&r.read(p), fixme_markers::Kind::Slashes))
        .map(|m| m.tag.unwrap())
        .collect();
    assert!(cited.iter().all(|c| ids.contains(c)), "{cited:?} {ids:?}");
}

#[test]
fn the_command_line_takes_the_flags_it_names_and_no_other() {
    assert_eq!(
        parse(&["--dry", "--only", "a*, b*", "--no-fetch"]).unwrap(),
        Args {
            dry:      true,
            no_fetch: true,
            only:     vec!["a*".to_string(), "b*".to_string()],
        }
    );
    assert!(parse(&["--wat"]).unwrap_err().contains("--wat"));
    assert!(parse(&["--only"]).is_err());
    assert!(glob("*.md.tmpl", "mock/WORKFLOW.md.tmpl"));
    assert!(!glob("*.md.tmpl", "mock/lib.rs"));
    assert!(glob("mock/*/lib.rs", "mock/crates/a/lib.rs"));
}

#[test]
fn a_title_cut_from_a_sentence_does_not_end_on_a_word_that_carries_nothing() {
    let r = Repo::new();
    r.write(
        SIM,
        "// §: every grant here is a first take for op to decide\n",
    );
    r.run(&[]);
    let s = r.store();
    let title = s.tickets.values().next().unwrap().one("title").to_string();
    assert_eq!(title, "kaski-sim: every grant here is a first take for op");
    let id = s.tickets.keys().next().unwrap();
    assert!(!id.ends_with("_to") && !id.ends_with("_for"), "{id}");
    // marks the cut leaves behind go as well
    assert_eq!(
        plan::tidy_for_test("what a stint, of use,"),
        "what a stint, of use"
    );
}

#[test]
fn a_seam_tag_found_later_cites_the_one_task_of_the_tag_and_files_no_second() {
    let r = Repo::new();
    r.write(SIM, "// §(gpu-pass): the pass is a copy\n");
    r.run(&[]);
    let id = first_cited(&r.read(SIM));
    r.write(
        WEATHER,
        "// §(gpu-pass): a later one reads the wrong buffer\n",
    );
    let out = r.run(&[]);
    assert!(out.contains("filed 0 tasks covering 0 markers"), "{out}");
    assert_eq!(first_cited(&r.read(WEATHER)), id);
    assert_eq!(r.board().len(), 1, "a second task was filed for the seam");
}

#[test]
fn a_list_written_before_lines_were_listed_is_matched_as_it_was() {
    let r = Repo::new();
    r.write(SIM, "// §(stand-in): a flat roof for now\n");
    let store = r.store();
    let mut fields = ticket_model::Fields::new();
    fields.insert("what".to_string(), vec![format!(
        "The stand-ins in kaski-sim, 1 marker.\n\nCovers:\n- {SIM}: a flat roof for now\n"
    )]);
    tickets::write::add_with(
        &store,
        &r.root.join("mock/registry"),
        &tickets::write::AddRequest {
            title: "kaski-sim: a flat roof for now".to_string(),
            id: Some("an_old_task".to_string()),
            kind: "task".to_string(),
            state: None,
            holder: String::new(),
            by: plan::BY.to_string(),
            at: "2026-10-30T10:00:00Z".to_string(),
            why: "filed by mock owed for 1 marker(s)".to_string(),
            fields,
        },
    )
    .unwrap();
    let out = r.run(&[]);
    assert!(out.contains("filed 0 tasks covering 0 markers"), "{out}");
    assert!(out.contains("cited 1 markers"), "{out}");
    assert_eq!(first_cited(&r.read(SIM)), "an_old_task");
    assert_eq!(r.board().len(), 1);
}

const FREE: &str = "mock/tools/roofs/src/lib.rs";

/// A question put to the maintainer, then moved to `to`, with the answer and picks a move carries.
fn question_closed(r: &Repo, id: &str, to: &str, answer: &str, picks: &[&str]) {
    let store = r.store();
    let registry = r.root.join("mock/registry");
    let mut fields = ticket_model::Fields::new();
    fields.insert("options".to_string(), vec![
        "flat: a flat roof".to_string(),
        "pitched: a pitched roof".to_string(),
    ]);
    fields.insert("related".to_string(), vec!["subject::weather".to_string()]);
    tickets::write::add_with(&store, &registry, &tickets::write::AddRequest {
        title: "Which roof".to_string(),
        id: Some(id.to_string()),
        kind: "question".to_string(),
        state: Some("waiting".to_string()),
        holder: "op".to_string(),
        by: "coord".to_string(),
        at: "2026-11-01T10:00:00Z".to_string(),
        why: "asked".to_string(),
        fields,
    })
    .unwrap();
    tickets::write::move_ticket(&registry, &tickets::write::MoveRequest {
        ticket: id.to_string(),
        to: to.to_string(),
        release: true,
        by: "op".to_string(),
        at: "2026-11-02T10:00:00Z".to_string(),
        why: "answered".to_string(),
        answer: answer.to_string(),
        picks: picks.iter().map(|p| (*p).to_string()).collect(),
        ..Default::default()
    })
    .unwrap();
}

#[test]
fn a_marker_citing_an_answered_question_gets_a_task_carrying_the_answer_and_cites_it() {
    let r = Repo::new();
    question_closed(&r, "which_roof", "done", "Any roof is fine.", &[
        "flat: a flat roof",
    ]);
    r.write(FREE, "// §(which_roof): the roof waits on op\n");
    r.write(
        WEATHER,
        "// §(which_roof): the same wait, in a crate under the gate\n",
    );
    r.run(&[]);
    let task = r
        .store()
        .tickets
        .values()
        .find(|t| t.one("title").starts_with("Markers citing the closed"))
        .map(|t| (t.id.clone(), t.one("what").to_string()))
        .expect("a task was filed");
    assert!(
        task.1.contains("Answer, verbatim: Any roof is fine."),
        "{}",
        task.1
    );
    assert!(task.1.contains("Picks: flat: a flat roof"), "{}", task.1);
    assert!(task.1.contains("- pitched: a pitched roof"), "{}", task.1);
    assert!(
        !task.1.contains(WEATHER),
        "a gated file is not covered: {}",
        task.1
    );
    let free = fixme_markers::scan(&r.read(FREE), fixme_markers::Kind::Slashes);
    assert_eq!(free[0].tag.as_deref(), Some(task.0.as_str()));
    // the control: the marker in the gated file is left citing the question
    let gated = fixme_markers::scan(&r.read(WEATHER), fixme_markers::Kind::Slashes);
    assert_eq!(gated[0].tag.as_deref(), Some("which_roof"));
    // and a second run files nothing more
    let before = r.snapshot();
    r.run(&[]);
    assert_eq!(before, r.snapshot());
}

#[test]
fn a_marker_citing_a_question_closed_with_no_answer_is_left_alone() {
    let r = Repo::new();
    question_closed(&r, "which_roof", "dropped", "", &[]);
    r.write(
        FREE,
        "// §(which_roof): waits on a question nobody answered\n",
    );
    let before = r.snapshot();
    r.run(&[]);
    assert_eq!(before, r.snapshot());
}

#[test]
fn a_task_is_held_by_the_holder_the_table_names_and_by_nobody_where_it_names_none() {
    let mut r = Repo::new();
    r.write(SIM, "// §: the wind is not read\n");
    r.owed = Owed::default();
    r.run(&[]);
    let board = r.board();
    let (_, (_, state, holder)) = board.iter().next().expect("a task was filed");
    assert_eq!((state.as_str(), holder.as_str()), ("backlog", ""));

    // THE CONTROL: the same marker with `holder` set is held by it.
    let mut held = Repo::new();
    held.write(SIM, "// §: the wind is not read\n");
    let mut params = mockspace::tool::ToolParams::new();
    params.set(plan::HOLDER, vec!["helpers".to_string()]);
    held.owed = Owed::default();
    held.owed.configure(&params);
    held.run(&[]);
    let board = held.board();
    assert_eq!(board.values().next().expect("a task").2, "helpers");
}
