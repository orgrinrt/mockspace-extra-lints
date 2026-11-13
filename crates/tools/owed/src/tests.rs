//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The maker run on fixture trees: a repository with the committed ticket vocabularies and the files a
//! test writes into it. The marker word is written `§` in this file, so the file holds no marker of its
//! own, and `t` puts the word in.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use ticket_model::Store;
use mockspace::tool::{Outcome, ToolContext, ToolReport};
use mockspace::{RegistryView, RowFields};
use registry_lints::tree_walk;

use super::*;

fn t(s: &str) -> String {
    s.replace('§', "FIXME")
}

/// A repository on disk: `mock/registry/ticket/` holds the shipped states and kinds and no ticket.
struct Repo {
    root: PathBuf,
}

impl Repo {
    fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let root = std::env::temp_dir().join(format!(
            "owed-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join("mock/registry/ticket");
        std::fs::create_dir_all(&dir).unwrap();
        // the project the fixtures name, so a crate `kaski-<x>` is about the subject `<x>`
        std::fs::write(
            root.join("mockspace.toml"),
            "project_name = \"kaski\"\ncrate_prefix = \"kaski\"\n",
        )
        .unwrap();
        for f in ["states.toml", "kinds.toml"] {
            std::fs::copy(
                tickets::testkit::shipped_registry()
                    .join("ticket")
                    .join(f),
                dir.join(f),
            )
            .unwrap();
        }
        Repo {
            root,
        }
    }

    fn write(&self, rel: &str, text: &str) {
        let path = self.root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, t(text)).unwrap();
    }

    fn remove(&self, rel: &str) {
        std::fs::remove_file(self.root.join(rel)).unwrap();
    }

    fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.root.join(rel)).unwrap()
    }

    fn run_with(&self, args: &[&str], subjects: &[&str]) -> ToolReport {
        let crates = std::collections::BTreeSet::new();
        let rows: BTreeMap<String, RowFields> = subjects
            .iter()
            .map(|s| (format!("subject::{s}"), RowFields::new()))
            .collect();
        let registry = RegistryView::new(rows, BTreeMap::new());
        let mock = self.root.join("mock");
        let parsed = parse(args).unwrap();
        Owed.work(
            &ToolContext {
                mock_dir: &mock,
                repo_root: &self.root,
                all_crates: &crates,
                src_dirs: &[],
                args,
                stdin: None,
                registry: &registry,
            },
            &parsed,
        )
    }

    fn run(&self, args: &[&str]) -> String {
        let r = self.run_with(args, &[]);
        if let Outcome::Inconclusive {
            reason,
        } = &r.outcome
        {
            panic!("owed refused: {reason}");
        }
        r.output
    }

    fn store(&self) -> Store {
        tickets::load(&self.root.join("mock/registry")).unwrap()
    }

    /// Every file under the repository with its text, to compare a run against.
    fn snapshot(&self) -> BTreeMap<String, String> {
        tree_walk::walk_files(&self.root, |_, _| false, |_| true)
            .into_iter()
            .map(|p| {
                let rel = p
                    .strip_prefix(&self.root)
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                (rel, std::fs::read_to_string(&p).unwrap_or_default())
            })
            .collect()
    }

    /// The tickets, by id: kind, state, holder.
    fn board(&self) -> BTreeMap<String, (String, String, String)> {
        let s = self.store();
        s.tickets
            .keys()
            .map(|id| {
                let now = s.now_of(id).unwrap();
                (
                    id.clone(),
                    (
                        s.tickets[id].one("kind").to_string(),
                        now.to.clone(),
                        now.holder.clone(),
                    ),
                )
            })
            .collect()
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn first_cited(text: &str) -> String {
    let all = fixme_markers::scan(text, fixme_markers::Kind::Slashes);
    all[0].tag.clone().expect("a cited marker")
}

const SIM: &str = "mock/crates/kaski-sim/src/lib.rs";
const WEATHER: &str = "mock/crates/kaski-weather/src/lib.rs";

#[test]
fn an_uncited_marker_gets_a_task_and_cites_it_with_its_reason_kept() {
    let r = Repo::new();
    r.write(
        SIM,
        "fn a() {}\n// §(stand-in): a flat roof for now\nfn b() {}\n",
    );
    let out = r.run(&[]);
    assert!(out.contains("filed 1 tasks covering 1 markers"), "{out}");
    let board = r.board();
    assert_eq!(board.len(), 1);
    let (id, (kind, state, holder)) = board.iter().next().unwrap();
    assert_eq!(
        (kind.as_str(), state.as_str(), holder.as_str()),
        ("task", "backlog", "coord")
    );
    assert_eq!(
        r.read(SIM),
        t(&format!(
            "fn a() {{}}\n// §({id}): a flat roof for now\nfn b() {{}}\n"
        ))
    );
    let what = r.store().tickets[id].one("what").to_string();
    assert!(
        what.contains("Covers:\n- mock/crates/kaski-sim/src/lib.rs:2: a flat roof for now"),
        "{what}"
    );
    assert!(
        r.store().tickets[id]
            .one("title")
            .starts_with("kaski-sim: ")
    );
}

#[test]
fn a_second_run_with_nothing_changed_writes_nothing() {
    let r = Repo::new();
    r.write(
        SIM,
        "// §(stand-in): a flat roof for now\n// §: the wind is not read\n",
    );
    r.write(
        WEATHER,
        "// §(canon silent): how a front moves over a ridge\n",
    );
    r.run(&[]);
    let before = r.snapshot();
    assert_eq!(r.run(&[]), "");
    assert_eq!(r.snapshot(), before, "a second run changed the tree");
}

#[test]
fn a_marker_whose_line_moved_changes_nothing_and_an_unrewritten_one_is_adopted() {
    let r = Repo::new();
    r.write(SIM, "// §(stand-in): a flat roof for now\n");
    r.run(&[]);
    let id = first_cited(&r.read(SIM));
    let before = r.board();
    // lines added above: the marker is cited, so nothing is asked of it
    r.write(
        SIM,
        &format!("fn x() {{}}\nfn y() {{}}\n// §({id}): a flat roof for now\n"),
    );
    let snap = r.snapshot();
    assert_eq!(r.run(&[]), "");
    assert_eq!(r.snapshot(), snap);
    // a run that filed the ticket and stopped before rewriting: the marker is uncited again and has
    // moved, and it is found by its words and cites the ticket it was listed on
    r.write(
        SIM,
        "fn x() {}\nfn y() {}\n\n// §(stand-in): a flat roof for now\n",
    );
    let out = r.run(&[]);
    assert!(out.contains("filed 0 tasks covering 0 markers"), "{out}");
    assert!(out.contains("cited 1 markers"), "{out}");
    assert_eq!(first_cited(&r.read(SIM)), id);
    assert_eq!(
        r.board(),
        before,
        "a ticket was filed for a marker already listed"
    );
}

#[test]
fn markers_that_say_one_thing_share_one_ticket_and_an_unlike_one_does_not() {
    let r = Repo::new();
    r.write(
        SIM,
        "// §: the wind pushes branches only on the first frame\n",
    );
    r.write(
        WEATHER,
        "// §: wind pushes the branches only on first frame\n// §: rain never wets the ground\n",
    );
    r.run(&[]);
    let board = r.board();
    assert_eq!(board.len(), 2, "{board:?}");
    let wind = first_cited(&r.read(SIM));
    let weather = fixme_markers::scan(&r.read(WEATHER), fixme_markers::Kind::Slashes);
    assert_eq!(weather[0].tag.as_deref(), Some(wind.as_str()));
    assert_ne!(weather[1].tag.as_deref(), Some(wind.as_str()));
    let what = r.store().tickets[&wind].one("what").to_string();
    assert_eq!(what.matches("\n- mock/crates/").count(), 2, "{what}");
}

#[test]
fn a_tag_naming_a_seam_gathers_its_markers_into_one_task() {
    let r = Repo::new();
    r.write(SIM, "// §(gpu-pass): the pass is a copy\n");
    r.write(WEATHER, "// §(gpu-pass): and this one reads the wrong buffer\n// §(other-seam): something else entirely\n");
    r.run(&[]);
    let s = r.store();
    let titles: Vec<&str> = s.tickets.values().map(|t| t.one("title")).collect();
    assert_eq!(titles.len(), 2, "{titles:?}");
    assert!(
        titles.contains(&"Placeholders tagged gpu-pass"),
        "{titles:?}"
    );
    let seam = first_cited(&r.read(SIM));
    assert_eq!(
        s.tickets[&seam].one("title"),
        "Placeholders tagged gpu-pass"
    );
}

/// Every class of marker and a candidate file, once each: nothing `owed` files is a question, held by
/// the maintainer, waiting, or carrying options, whatever the marker says of the canon or of the
/// maintainer.
const EVERY_CLASS: &str = "// §(canon silent): how a front moves over a ridge\n// §(maintainer): whether rain wets the ground\n// §(stand-in): a flat roof for now\n// §: the wind is not read\n// §(gpu-pass): the pass is a copy\n";
const CANDIDATE_FILE: &str = "mock/research/20261028_weather/CANDIDATE_weather.md";

#[test]
fn nothing_owed_files_is_a_question_whatever_the_marker_says() {
    let r = Repo::new();
    r.write(SIM, EVERY_CLASS);
    r.write(
        CANDIDATE_FILE,
        "# The weather canon\n\nA front crosses a ridge.\n",
    );
    let out = r.run(&[]);
    let s = r.store();
    assert_eq!(
        s.tickets.len(),
        6,
        "{:?}",
        s.tickets.keys().collect::<Vec<_>>()
    );
    assert!(out.contains("filed 6 tasks covering 5 markers"), "{out}");
    for (id, (kind, state, holder)) in r.board() {
        assert_eq!(
            (kind.as_str(), state.as_str(), holder.as_str()),
            ("task", "backlog", "coord"),
            "{id}"
        );
        assert!(
            s.tickets[&id].many("options").is_empty(),
            "{id} was filed with options"
        );
    }
}

#[test]
fn a_task_for_a_canon_silence_asks_an_agent_to_evaluate_each_marker_and_lists_its_line() {
    let r = Repo::new();
    r.write(SIM, "fn a() {}\n// §(canon silent): how a front moves over a ridge\n// §(maintainer): whether rain wets the ground\n");
    r.run(&[]);
    let s = r.store();
    assert_eq!(s.tickets.len(), 2);
    for t in s.tickets.values() {
        let what = t.one("what");
        assert!(
            what.contains("An agent is to evaluate each marker"),
            "{what}"
        );
        assert!(what.contains("a question to the maintainer"), "{what}");
    }
    let listed: Vec<String> = s
        .tickets
        .values()
        .flat_map(|t| {
            t.one("what")
                .lines()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|l| l.starts_with("- mock/crates/"))
        .collect();
    assert!(
        listed.contains(
            &"- mock/crates/kaski-sim/src/lib.rs:2: how a front moves over a ridge".to_string()
        ),
        "{listed:?}"
    );
    assert!(
        listed
            .iter()
            .any(|l| l.starts_with("- mock/crates/kaski-sim/src/lib.rs:3: ")),
        "{listed:?}"
    );
}

#[test]
fn a_ticket_whose_markers_are_gone_is_closed_and_one_with_a_marker_left_is_not() {
    let r = Repo::new();
    r.write(
        SIM,
        "// §: the wind pushes branches only on the first frame\n",
    );
    r.write(WEATHER, "// §: wind pushes the branches only on first frame\n// §(canon silent): how a front moves over a ridge\n");
    r.run(&[]);
    let board = r.board();
    assert_eq!(board.len(), 2);
    let wind = first_cited(&r.read(SIM));
    let front = fixme_markers::scan(&r.read(WEATHER), fixme_markers::Kind::Slashes)[1]
        .tag
        .clone()
        .unwrap();
    // one of the two markers of the wind ticket goes: it stays open
    r.write(SIM, "fn done() {}\n");
    assert_eq!(r.run(&[]), "");
    assert_eq!(r.board()[&wind].1, "backlog");
    // the other goes, and the canon silence with it: both tasks are done
    r.write(WEATHER, "fn done() {}\n");
    let out = r.run(&[]);
    assert!(out.contains("closed 2 tickets"), "{out}");
    let board = r.board();
    assert_eq!(board[&wind].1, "done");
    assert_eq!(board[&front].1, "done");
    let again = r.snapshot();
    assert_eq!(r.run(&[]), "");
    assert_eq!(r.snapshot(), again, "a closed ticket was touched again");
}

#[test]
fn a_question_filed_before_owed_filed_tasks_is_still_closed_as_dropped_when_its_markers_go() {
    let r = Repo::new();
    let store = r.store();
    let registry = r.root.join("mock/registry");
    let mut fields = ticket_model::Fields::new();
    fields.insert(
        "what".to_string(),
        vec!["The canon silences in kaski-weather, 1 marker.\n\nCovers:\n- mock/crates/kaski-weather/src/lib.rs: how a front moves over a ridge\n".to_string()],
    );
    fields.insert("options".to_string(), vec![
        "settle: x".to_string(),
        "keep: y".to_string(),
    ]);
    fields.insert("related".to_string(), vec!["subject::weather".to_string()]);
    tickets::write::add_with(&store, &registry, &tickets::write::AddRequest {
        title: "kaski-weather: how a front moves over a ridge".to_string(),
        id: Some("an_old_question".to_string()),
        kind: "question".to_string(),
        state: Some("waiting".to_string()),
        holder: "op".to_string(),
        by: plan::BY.to_string(),
        at: "2026-10-30T10:00:00Z".to_string(),
        why: "filed by mock owed for 1 marker(s)".to_string(),
        fields,
    })
    .unwrap();
    assert_eq!(r.board()["an_old_question"].1, "waiting");
    let out = r.run(&[]);
    assert!(out.contains("closed 1 tickets"), "{out}");
    assert_eq!(r.board()["an_old_question"].1, "dropped");
}

#[test]
fn a_candidate_file_becomes_one_task_that_says_how_to_put_it_to_op() {
    let r = Repo::new();
    let path = "mock/research/20261028_weather/CANDIDATE_weather.md";
    r.write(
        path,
        "# The weather canon\n\nA front crosses a ridge and sheds rain on the lee.\n",
    );
    let out = r.run(&[]);
    assert!(out.contains("filed 1 tasks covering 0 markers"), "{out}");
    let s = r.store();
    let (id, ticket) = s.tickets.iter().next().unwrap();
    assert_eq!(ticket.one("kind"), "task");
    assert_eq!(
        ticket.one("title"),
        "Ratify the candidate The weather canon"
    );
    assert!(ticket.many("options").is_empty());
    let what = ticket.one("what");
    assert!(
        what.starts_with(&format!("Candidate file: {path}")),
        "{what}"
    );
    assert!(
        what.contains("ratify, ratify changed and refuse"),
        "the options an agent puts it to the maintainer with: {what}"
    );
    let (_, state, holder) = &r.board()[id];
    assert_eq!((state.as_str(), holder.as_str()), ("backlog", "coord"));
    // not filed twice, and the file itself is not rewritten
    let snap = r.snapshot();
    assert_eq!(r.run(&[]), "");
    assert_eq!(r.snapshot(), snap);
    // the file goes: the task is dropped
    r.remove(path);
    let out = r.run(&[]);
    assert!(out.contains("closed 1 tickets"), "{out}");
    assert_eq!(r.board()[id].1, "dropped");
}

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

#[allow(dead_code)]
fn _paths(_: &Path) {}

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
