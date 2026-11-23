use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::process::Command;

use mockspace::tool::ToolParams;
use muisti::repo::{DocClass, Query};

use super::*;

/// A checkout under git, and a place for its index outside it, both removed when the test ends.
struct Planted {
    repo:  PathBuf,
    index: PathBuf,
}

impl Planted {
    fn new(name: &str, files: &[(&str, &str)]) -> Self {
        let base = std::env::temp_dir().join(format!("corpus-repo-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let repo = base.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        git(&repo, &["init", "-q"], &[]);
        let me = Self {
            index: base.join("index").join("search.sqlite"),
            repo,
        };
        for (rel, text) in files {
            me.file(rel, text);
        }
        me
    }

    fn file(&self, rel: &str, body: &str) {
        let p = self.repo.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }

    fn mock(&self) -> PathBuf {
        self.repo.join("mock")
    }

    fn refresh(&self, maintainer: Option<&Maintainer>) {
        let done = refresh_at(&self.index, &self.mock(), &self.repo, maintainer).unwrap();
        assert!(done.is_some(), "the refresh held the index");
    }

    fn open(&self, maintainer: Option<&Maintainer>) -> (Opened, usize) {
        let started = Cell::new(0);
        let opened = open_at(
            &self.index,
            &self.mock(),
            &self.repo,
            maintainer,
            &|_: &Path| {
                started.set(started.get() + 1);
            },
        );
        (opened, started.get())
    }
}

impl Drop for Planted {
    fn drop(&mut self) {
        if let Some(base) = self.repo.parent() {
            let _ = std::fs::remove_dir_all(base);
        }
    }
}

fn git(dir: &Path, args: &[&str], env: &[(&str, &str)]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .envs(env.iter().copied())
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn ask(index: &RepoIndex, text: &str, phrase: bool) -> muisti::repo::RepoReport {
    index.search(&Query {
        text,
        expand: &[],
        phrase,
        limit: 5,
        today: 20_700,
    })
}

fn shown(listing: &Listing) -> Vec<(DocClass, String)> {
    let mut out: Vec<(DocClass, String)> = listing
        .roots
        .iter()
        .flat_map(|r| {
            r.files
                .iter()
                .flatten()
                .map(move |l| (r.class, l.rel.clone()))
        })
        .collect();
    out.sort();
    out
}

#[test]
fn a_file_is_read_under_the_class_its_place_gives_it() {
    let c = |p: &str| class_of(p, "mock", Some("data/words"));
    assert_eq!(c("mock/registry/tenet/a.toml"), DocClass::Canon);
    assert_eq!(
        c("mock/design_rounds/202611010000_topic.a.md"),
        DocClass::Design
    );
    assert_eq!(
        c("mock/research/sketches/202611010000_a/FINDINGS.md"),
        DocClass::Sketch
    );
    assert_eq!(c("mock/research/a.md"), DocClass::Research);
    assert_eq!(c("data/words/202611010000_said.md"), DocClass::Human);
    assert_eq!(c("src/lib.rs"), DocClass::Source);
    // A name that only opens like a root's is not under it.
    assert_eq!(c("mock/researchers.md"), DocClass::Source);
    assert_eq!(class_of("research/a.md", "", None), DocClass::Research);
}

#[test]
fn the_listing_is_every_file_git_knows_and_every_file_of_a_declared_data_ref() {
    let p = Planted::new("listing", &[
        (".gitignore", "target/\nmock/research/\n"),
        (
            "mockspace.toml",
            "[[data_ref]]\nnamespace = \"mock\"\npath = \".\"\n",
        ),
        ("src/lib.rs", "pub fn a() {}\n"),
        ("target/debug/out.txt", "built\n"),
        ("mock/research/a.md", "# A\n\non the data ref\n"),
        (
            "mock/research/b.md",
            "# B\n\nnot committed to the ref yet\n",
        ),
    ]);
    // The data ref holds `research/a.md`, from the mock directory, as the engine keeps it.
    let index_file = p.repo.join(".git").join("data-ref-index");
    let env = [("GIT_INDEX_FILE", index_file.to_str().unwrap())];
    let mock = p.mock();
    let work = format!("--work-tree={}", mock.display());
    git(&p.repo, &[&work, "add", "-f", "research/a.md"], &env);
    let tree = git(&p.repo, &[&work, "write-tree"], &env);
    let commit = git(&p.repo, &["commit-tree", &tree, "-m", "data"], &[]);
    git(&p.repo, &["update-ref", "refs/mock/mock", &commit], &[]);

    let got = list(&p.mock(), &p.repo, None);
    assert!(got.notes.is_empty(), "{:?}", got.notes);
    let paths: Vec<String> = shown(&got).into_iter().map(|(_, p)| p).collect();
    assert!(paths.contains(&"src/lib.rs".to_string()), "{paths:?}");
    assert!(
        paths.contains(&"mock/research/a.md".to_string()),
        "{paths:?}"
    );
    assert!(
        !paths.iter().any(|p| p.starts_with("target/")),
        "an ignored build is not read: {paths:?}"
    );
    // THE CONTROL: an ignored file on no data ref is not listed.
    assert!(
        !paths.contains(&"mock/research/b.md".to_string()),
        "{paths:?}"
    );
}

#[test]
fn a_question_opens_the_index_read_only_and_never_builds_it() {
    let p = Planted::new("open", &[(
        "mock/research/a.md",
        "# A\n\nthe shadow tree holds templates\n",
    )]);
    let (opened, started) = p.open(None);
    assert!(opened.index.is_none());
    assert_eq!(started, 1);
    assert!(!p.index.exists(), "a question wrote no index");
    assert!(
        opened.notes.iter().any(|n| n.contains("not built")),
        "{:?}",
        opened.notes
    );

    // THE CONTROL: refreshed, the index answers and nothing is started.
    p.refresh(None);
    let (opened, started) = p.open(None);
    assert_eq!(started, 0);
    let index = opened.index.unwrap();
    let r = ask(&index, "shadow tree", false);
    assert_eq!(
        r.hits.first().map(|h| h.path.as_str()),
        Some("mock/research/a.md")
    );
}

#[test]
fn a_second_refresh_over_unchanged_files_reads_none_of_them() {
    let p = Planted::new("incremental", &[
        ("mock/research/a.md", "# A\n\nfirst\n"),
        ("mock/research/b.md", "# B\n\nsecond\n"),
    ]);
    p.refresh(None);
    let again = refresh_at(&p.index, &p.mock(), &p.repo, None)
        .unwrap()
        .unwrap();
    assert_eq!(again.analysed, 0, "{again:?}");
    // THE CONTROL: a changed file is read, and only it.
    p.file("mock/research/a.md", "# A\n\nfirst, and longer now\n");
    let third = refresh_at(&p.index, &p.mock(), &p.repo, None)
        .unwrap()
        .unwrap();
    assert_eq!(third.changed, 1, "{third:?}");
}

#[test]
fn the_near_words_of_a_question_are_the_forms_in_the_documents_and_the_synonyms_in_the_keywords() {
    let p = Planted::new("near", &[
        (
            "mock/research/a.md",
            "# Note\n\nthe air is saturated and saturates with heat\n",
        ),
        (
            "mock/registry/tenet/air.toml",
            "[[tenet]]\nid = \"heat_is_a_field\"\nkeywords = [\"warmth\", \"heat\", \"thermal\"]\n",
        ),
    ]);
    p.refresh(None);
    let index = p.open(None).0.index.unwrap();
    let near = near_words(&index, "air saturation heat", 5);
    let find = |w: &str| near.iter().find(|n| n.word == w);
    assert_eq!(
        find("saturated").map(|n| n.synonym),
        Some(false),
        "{near:?}"
    );
    assert_eq!(find("warmth").map(|n| n.synonym), Some(true), "{near:?}");
    let row = index.row("tenet::heat_is_a_field").unwrap().map(|r| r.path);
    assert_eq!(row.as_deref(), Some("mock/registry/tenet/air.toml"));
}

const QUOTING_NOTE: &str = "# A note quoting the maintainer\n\nThe agent writes that the weighing of offers is a curve.\n\nLead: the pheasant ledger is settled and carries no tax.\n\nA closing agent paragraph about moonlit orchards.\n";

#[test]
fn a_phrase_the_maintainer_said_only_in_a_note_comes_back_as_theirs() {
    let p = Planted::new("quoting", &[(
        "mock/research/202606231730_quoting.md",
        QUOTING_NOTE,
    )]);
    let mut m = Maintainer::default();
    let mut params = ToolParams::new();
    params.set(crate::maintainer::NAMES, vec!["lead".to_string()]);
    m.configure(&params);
    p.refresh(Some(&m));
    let index = p.open(Some(&m)).0.index.unwrap();
    let r = ask(&index, "pheasant ledger", true);
    assert_eq!(r.hits.len(), 1, "{:?}", r.hits);
    assert_eq!(r.hits[0].class, DocClass::Human);
    assert!(
        found_by(&r.hits[0]).contains("the maintainer's words"),
        "{:?}",
        r.hits[0]
    );
    // THE CONTROL: what only the agent wrote in the same note is not theirs.
    let r = ask(&index, "moonlit orchards", true);
    assert_eq!(
        r.hits.first().map(|h| h.class),
        Some(DocClass::Research),
        "{:?}",
        r.hits
    );
}

#[test]
fn maintainer_words_named_outside_the_repository_are_not_read_and_the_note_says_so() {
    let p = Planted::new("outside", &[("mock/research/a.md", "# A\n\nprose\n")]);
    let mut m = Maintainer::default();
    let mut params = ToolParams::new();
    params.set(crate::maintainer::WORDS, vec![
        "../.data/op-responses".to_string(),
    ]);
    m.configure(&params);
    p.refresh(Some(&m));
    let notes = p.open(Some(&m)).0.notes;
    assert!(
        notes
            .iter()
            .any(|n| n.contains("not a path inside the repository")),
        "{notes:?}"
    );
}

#[test]
fn a_subject_asks_its_name_as_words_then_its_keywords_and_a_question_of_stop_words_is_not_askable()
{
    assert_eq!(
        question_of("air_is_one", &["lattice".to_string()]),
        "air is one lattice"
    );
    assert!(askable("the shadow tree"));
    assert!(!askable("the of and"));
}
