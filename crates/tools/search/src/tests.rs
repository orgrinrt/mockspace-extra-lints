//! The search over a planted checkout: a question that never builds the index, a refresh that
//! does, a question that says how far behind the index is, and each filter beside the same
//! question unfiltered.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use corpus::repo::{Opened, open_at, refresh_at};
use mockspace::tool::{Outcome, ToolContext, ToolReport};
use mockspace::{RegistryView, RowFields};

use super::*;
use crate::registry::rank_keywords;

/// A checkout under git, and a place for its index outside it, both removed when the test ends.
struct Planted {
    repo:  PathBuf,
    index: PathBuf,
}

impl Planted {
    fn new(name: &str) -> Self {
        let base = std::env::temp_dir().join(format!("search-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let repo = base.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        let ok = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(&repo)
            .status()
            .unwrap()
            .success();
        assert!(ok, "git init");
        Self {
            index: base.join("index").join("search.sqlite"),
            repo,
        }
    }

    fn file(&self, rel: &str, body: &str) -> &Self {
        let p = self.repo.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
        self
    }

    fn mock(&self) -> PathBuf {
        self.repo.join("mock")
    }

    fn refresh(&self) {
        let done = refresh_at(&self.index, &self.mock(), &self.repo, None).unwrap();
        assert!(done.is_some(), "the refresh held the index");
    }

    /// The index opened as a question opens it, and how many refreshes it started.
    fn open(&self) -> (Opened, usize) {
        let started = Cell::new(0);
        let opened = open_at(&self.index, &self.mock(), &self.repo, None, &|_: &Path| {
            started.set(started.get() + 1);
        });
        (opened, started.get())
    }

    fn ask(&self, reg: &RegistryView, args: &[&str]) -> ToolReport {
        let crates = BTreeSet::new();
        let dirs: Vec<PathBuf> = Vec::new();
        let ctx = ToolContext {
            mock_dir: &self.mock(),
            repo_root: &self.repo,
            all_crates: &crates,
            src_dirs: &dirs,
            args,
            stdin: None,
            registry: reg,
        };
        let spec = Spec::parse(args).unwrap();
        answer(&ctx, &spec, self.open().0)
    }
}

impl Drop for Planted {
    fn drop(&mut self) {
        if let Some(base) = self.repo.parent() {
            let _ = std::fs::remove_dir_all(base);
        }
    }
}

fn no_rows() -> RegistryView {
    RegistryView::new(BTreeMap::<String, RowFields>::new(), BTreeMap::new())
}

fn output(r: &ToolReport) -> &str {
    assert!(
        matches!(r.outcome, Outcome::Clean { .. }),
        "not answered: {:?} {}",
        r.outcome,
        r.output
    );
    &r.output
}

fn shadow_tree(p: &Planted) {
    p.file(
        "mock/research/202611120700_data-ref-architecture/017_shadow.md",
        "# The shadow tree\n\nThe shadow tree holds templates and proposals.\n",
    )
    .file(
        "mock/design_rounds/202611010000_topic.other.md",
        "# Other\n\nNothing of that here.\n",
    )
    .file(
        "src/lib.rs",
        "//! The crate.\npub fn shadow_tree_path() {}\n",
    );
}

#[test]
fn a_question_never_builds_the_index_and_starts_a_refresh_instead() {
    let p = Planted::new("absent");
    shadow_tree(&p);
    let (opened, started) = p.open();
    assert!(opened.index.is_none());
    assert_eq!(
        started, 1,
        "an absent index starts one refresh in the background"
    );
    assert!(!p.index.exists(), "the question wrote no index");
    let r = p.ask(&no_rows(), &["shadow", "tree"]);
    assert!(
        matches!(r.outcome, Outcome::Inconclusive { .. }),
        "{}",
        r.output
    );

    // THE CONTROL: once refreshed, the same question is answered and starts nothing.
    p.refresh();
    let (opened, started) = p.open();
    assert!(opened.index.is_some() && opened.behind.is_empty());
    assert_eq!(started, 0);
    let out = p.ask(&no_rows(), &["shadow", "tree"]).output;
    assert!(
        out.contains("mock/research/202611120700_data-ref-architecture/017_shadow.md"),
        "{out}"
    );
}

#[test]
fn a_file_changed_since_the_refresh_is_named_as_behind_and_the_answer_still_comes() {
    let p = Planted::new("behind");
    shadow_tree(&p);
    p.refresh();
    p.file(
        "mock/research/new.md",
        "# A new note on the shadow tree, longer than before\n",
    );
    let (opened, started) = p.open();
    assert_eq!(opened.behind.missing, ["mock/research/new.md"]);
    assert_eq!(started, 1, "a behind index starts a refresh");
    let out = output(&p.ask(&no_rows(), &["shadow", "tree"])).to_string();
    assert!(out.contains("NOTE: the index is behind by 1 file"), "{out}");
    assert!(out.contains("017_shadow.md"), "{out}");

    // THE CONTROL: refreshed again, nothing is behind and nothing is said of it.
    p.refresh();
    let out = output(&p.ask(&no_rows(), &["shadow", "tree"])).to_string();
    assert!(!out.contains("behind"), "{out}");
}

#[test]
fn every_tracked_file_is_read_under_the_class_its_place_gives_it() {
    let p = Planted::new("classes");
    shadow_tree(&p);
    p.refresh();
    let research = output(&p.ask(&no_rows(), &["shadow", "--class", "research"])).to_string();
    assert!(
        research.contains("017_shadow.md") && !research.contains("src/lib.rs"),
        "{research}"
    );
    let source = output(&p.ask(&no_rows(), &["shadow", "--class", "source"])).to_string();
    assert!(
        source.contains("src/lib.rs") && !source.contains("017_shadow.md"),
        "{source}"
    );
    // THE CONTROL: unfiltered, both come back.
    let both = output(&p.ask(&no_rows(), &["shadow", "--limit", "10"])).to_string();
    assert!(
        both.contains("src/lib.rs") && both.contains("017_shadow.md"),
        "{both}"
    );
}

#[test]
fn a_path_filter_keeps_what_is_under_it_and_drops_the_rest() {
    let p = Planted::new("under");
    shadow_tree(&p);
    p.refresh();
    let out = output(&p.ask(&no_rows(), &["shadow", "--under", "src"])).to_string();
    assert!(
        out.contains("src/lib.rs") && !out.contains("017_shadow.md"),
        "{out}"
    );
}

#[test]
fn the_row_filters_answer_in_rows() {
    let p = Planted::new("rows");
    shadow_tree(&p);
    p.file(
        "mock/registry/tenet/data.toml",
        "[[tenet]]\nid = \"the_shadow_tree_holds_templates\"\nsubject = \"data\"\nwhat = \"The shadow tree holds templates.\"\n\n[[tenet]]\nid = \"a_crate_holds_its_own_thing\"\nsubject = \"workflow\"\nwhat = \"Each crate holds its own thing.\"\n",
    );
    p.refresh();
    let out = output(&p.ask(&no_rows(), &["--namespace", "tenet"])).to_string();
    assert!(out.contains("IN THE REGISTRY (2 rows)"), "{out}");
    let out = output(&p.ask(&no_rows(), &["shadow", "--namespace", "tenet", "--rows"])).to_string();
    assert!(
        out.contains("tenet::the_shadow_tree_holds_templates"),
        "{out}"
    );
    assert!(!out.contains("a_crate_holds_its_own_thing"), "{out}");
    let out = output(&p.ask(&no_rows(), &["--subject", "workflow"])).to_string();
    assert!(
        out.contains("a_crate_holds_its_own_thing") && !out.contains("shadow"),
        "{out}"
    );
}

#[test]
fn a_flag_that_is_no_filter_is_refused_with_the_filters_named() {
    let e = Spec::parse(&["shadow", "--unearth"]).unwrap_err();
    assert!(
        e.contains("`--unearth` is no filter") && e.contains("--class"),
        "{e}"
    );
    assert!(Spec::parse(&["--since", "yesterday"]).is_err());
    assert!(
        Spec::parse(&["--class", "notes"])
            .unwrap_err()
            .contains("the classes are")
    );
    // THE CONTROL: the filters read as filters and the rest as the question.
    let s = Spec::parse(&["shadow", "tree", "--class", "research,design", "--since", "2026-11-01"])
        .unwrap();
    assert_eq!(s.question, "shadow tree");
    assert_eq!(s.classes.len(), 2);
    assert_eq!(s.since, corpus::day::day_of("2026-11-01"));
}

#[test]
fn the_cap_keeps_the_keywords_that_discriminate_rather_than_the_early_alphabet() {
    let candidates = vec![
        (1, "aggregate".to_string()),
        (1, "bundling".to_string()),
        (1, "copyleft".to_string()),
        (1, "spdx".to_string()),
    ];
    let df = |k: &str| {
        match k {
            "aggregate" => 900,
            "bundling" => 700,
            "copyleft" => 400,
            "spdx" => 4,
            _ => 0,
        }
    };
    assert_eq!(rank_keywords(candidates, df, 2), ["spdx", "copyleft"]);
}

#[test]
fn a_row_carrying_more_of_the_question_still_wins() {
    let candidates = vec![(2, "common but on point".to_string()), (1, "rare".to_string())];
    let df = |k: &str| if k == "rare" { 2 } else { 800 };
    assert_eq!(rank_keywords(candidates, df, 1), ["common but on point"]);
}

#[test]
fn the_engine_registers_the_tool_as_it_is_declared() {
    // A tool the engine refuses at registration takes every tool of the consumer down with it, which
    // is how a search with no required question failed as `takes-a-question`.
    let faults = mockspace::tool::contract_faults(&Search::default(), None);
    assert!(faults.is_empty(), "{faults:?}");
}
