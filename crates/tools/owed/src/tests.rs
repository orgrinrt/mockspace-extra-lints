//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The maker run on fixture trees: a repository with the committed ticket vocabularies and the files a
//! test writes into it. The marker word is written `§` in this file, so the file holds no marker of its
//! own, and `t` puts the word in.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use ticket_model::Store;
use mockspace::tool::{Outcome, ToolContext, ToolReport};
use mockspace::{RegistryView, RowFields};
use registry_lints::tree_walk;

use super::*;

pub(crate) fn t(s: &str) -> String {
    s.replace('§', "FIXME")
}

/// A repository on disk: `mock/registry/ticket/` holds the shipped states and kinds and no ticket.
pub(crate) struct Repo {
    pub(crate) root: PathBuf,
}

impl Repo {
    pub(crate) fn new() -> Self {
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

    pub(crate) fn write(&self, rel: &str, text: &str) {
        let path = self.root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, t(text)).unwrap();
    }

    pub(crate) fn remove(&self, rel: &str) {
        std::fs::remove_file(self.root.join(rel)).unwrap();
    }

    pub(crate) fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.root.join(rel)).unwrap()
    }

    pub(crate) fn run_with(&self, args: &[&str], subjects: &[&str]) -> ToolReport {
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

    pub(crate) fn run(&self, args: &[&str]) -> String {
        let r = self.run_with(args, &[]);
        if let Outcome::Inconclusive {
            reason,
        } = &r.outcome
        {
            panic!("owed refused: {reason}");
        }
        r.output
    }

    pub(crate) fn store(&self) -> Store {
        tickets::load(&self.root.join("mock/registry")).unwrap()
    }

    /// Every file under the repository with its text, to compare a run against.
    pub(crate) fn snapshot(&self) -> BTreeMap<String, String> {
        tree_walk::walk_files(&mockspace::reading::Reading::new(&self.root).view(), &self.root, |_, _| false, |_| true)
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
    pub(crate) fn board(&self) -> BTreeMap<String, (String, String, String)> {
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

pub(crate) fn first_cited(text: &str) -> String {
    let all = fixme_markers::scan(text, fixme_markers::Kind::Slashes);
    all[0].tag.clone().expect("a cited marker")
}

pub(crate) const SIM: &str = "mock/crates/kaski-sim/src/lib.rs";
pub(crate) const WEATHER: &str = "mock/crates/kaski-weather/src/lib.rs";

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
pub(crate) const EVERY_CLASS: &str = "// §(canon silent): how a front moves over a ridge\n// §(maintainer): whether rain wets the ground\n// §(stand-in): a flat roof for now\n// §: the wind is not read\n// §(gpu-pass): the pass is a copy\n";
pub(crate) const CANDIDATE_FILE: &str = "mock/research/20261028_weather/CANDIDATE_weather.md";

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
