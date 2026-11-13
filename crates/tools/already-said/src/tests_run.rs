use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use corpus::repo::{Ready, ready_at};
use mockspace::tool::Outcome;
use mockspace::{RegistryView, RowFields};

use super::*;

/// A registry from rows given as `("ns::slug", &[("field", "value")])`.
///
/// No reverse edges, which this tool never asks for. A tool that did would
/// get a view answering empty for every row, indistinguishable from a
/// project whose rows reference nothing, and would need them passed.
fn registry(rows: &[(&str, &[(&str, &str)])]) -> RegistryView {
    let mut map: BTreeMap<String, RowFields> = BTreeMap::new();
    for (q, fields) in rows {
        let mut f = RowFields::new();
        for (k, v) in *fields {
            f.insert((*k).to_string(), (*v).to_string());
        }
        map.insert((*q).to_string(), f);
    }
    RegistryView::new(map, BTreeMap::new())
}

/// Both excluded fields, on one row, so the staleness check inside `run` is
/// satisfied and the ordinary path is reached. A fixture without them
/// refuses, which is a different test and is below.
const CITED: &[(&str, &str)] = &[
    ("provenance", "research::somewhere::NOTES::#anchor"),
    ("cites", "another"),
    ("superseded_by", "another"),
    ("related", "tenet::another"),
    ("subject", "somewhere"),
];

/// A mock directory the corpus is loaded from: the named roots, and files
/// written under them. Removed on drop, so a failing test leaves nothing.
struct Mock(PathBuf);

impl Mock {
    fn new(name: &str, roots: &[&str], docs: &[(&str, &str)]) -> Self {
        let dir = std::env::temp_dir().join(format!("kaski-already-said-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a mock directory");
        for r in roots {
            std::fs::create_dir_all(dir.join(r)).expect("a root");
        }
        for (path, body) in docs {
            std::fs::write(dir.join(path), body).expect("a document");
        }
        Self(dir)
    }
}

impl Drop for Mock {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run(mock: &Mock, reg: &RegistryView, args: &[&str]) -> ToolReport {
    run_with(mock, reg, &ready_at(None, &mock.0, &mock.0, None), args)
}

fn run_with(mock: &Mock, reg: &RegistryView, index: &Ready, args: &[&str]) -> ToolReport {
    let crates = BTreeSet::new();
    let dirs: Vec<PathBuf> = Vec::new();
    AlreadySaid.answer(
        &ToolContext {
            mock_dir: &mock.0,
            repo_root: &mock.0,
            all_crates: &crates,
            src_dirs: &dirs,
            args,
            stdin: None,
            registry: reg,
        },
        index,
    )
}

/// The reason an inconclusive run gives, or a failure naming what came back
/// instead. This tool's whole product is the difference between an answer
/// and a refusal, so a test reading one as the other is worth nothing.
fn reason(r: &ToolReport) -> String {
    match &r.outcome {
        Outcome::Inconclusive {
            reason,
        } => reason.clone(),
        other => panic!("expected inconclusive, got {other:?}"),
    }
}

/// The population the run says it read.
fn examined(r: &ToolReport) -> usize {
    match &r.outcome {
        Outcome::Clean {
            examined,
        } => *examined,
        other => panic!("expected clean, got {other:?}"),
    }
}

/// A corpus with nothing in it, for the tests that are about the registry
/// half. The research sentence then says the corpus could not be read,
/// which is true and is asserted where it is the subject.
fn no_corpus(name: &str) -> Mock {
    Mock::new(name, &[], &[])
}

#[test]
fn a_term_of_nothing_is_refused_rather_than_searched_for() {
    // The engine refuses a missing required argument before `run` is
    // entered, so this is reachable as `already-said ""` and as a direct
    // call. Both are the same question, which is no question.
    let mock = no_corpus("empty-term");
    let reg = registry(&[("ruling::a", CITED)]);
    assert!(reason(&run(&mock, &reg, &[])).contains("no term given"));
    assert!(reason(&run(&mock, &reg, &[""])).contains("no term given"));
    assert!(
        reason(&run(&mock, &reg, &["  ", " "])).contains("no term given"),
        "spaces are not a term either, and untrimmed this searched for a run \
             of them and answered that no row carries it"
    );

    // The control: a real term gets past this and is answered.
    assert_eq!(examined(&run(&mock, &reg, &["licensing"])), 1);
}

#[test]
fn an_empty_registry_says_nothing_was_searched_rather_than_naming_an_exclusion() {
    // With no rows every exclusion is unseen, so the staleness branch
    // answered for the empty registry and the branch that says what actually
    // happened could not be reached at all. Both sentences are inconclusive,
    // so nothing was ever wrongly trusted; what was wrong is that the reason
    // named the wrong thing, and the reason is the whole of an inconclusive
    // report.
    let mock = no_corpus("no-rows");
    let why = reason(&run(&mock, &registry(&[]), &["licensing"]));
    assert!(why.contains("no rows were searched"), "{why}");
    assert!(
        !why.contains("excluded from the search"),
        "nothing is stale here, there is simply nothing: {why}"
    );
}

#[test]
fn an_exclusion_no_row_carries_is_reported_as_stale() {
    // The check the ordering above must not break. A field excluded from the
    // search and carried by no row is an exclusion nobody has checked, which
    // is exactly how the hand-written table this replaced came to name a
    // field that did not exist.
    let mock = no_corpus("stale");
    let plain = registry(&[("ruling::a", &[("says", "prose")])]);
    let why = reason(&run(&mock, &plain, &["licensing"]));
    assert!(
        why.contains("provenance, cites, superseded_by, subject, related"),
        "all of them: {why}"
    );

    // Half of them present, and only the other half is named.
    let one = registry(&[("ruling::a", &[
        ("says", "prose"),
        ("provenance", "panel::x::#y"),
    ])]);
    let why = reason(&run(&mock, &one, &["licensing"]));
    assert!(
        why.contains("no row carries one: cites, superseded_by, subject, related."),
        "{why}"
    );

    // The control: both carried, and the run answers instead of refusing.
    let cited = registry(&[("ruling::a", CITED)]);
    assert_eq!(examined(&run(&mock, &cited, &["licensing"])), 1);
}

#[test]
fn the_whole_argument_is_one_phrase_rather_than_a_word_each() {
    // Splitting on spaces would search for each word and report a row that
    // carries all of them somewhere and the phrase nowhere, which for a tool
    // whose empty answer licenses interrupting a person is the expensive
    // direction to be wrong in.
    let mock = no_corpus("phrase");
    let reg = registry(&[
        ("slot::the_passphrase_is_gone", CITED),
        ("ruling::scattered", &[(
            "says",
            "the passphrase is one thing and the screen is gone",
        )]),
    ]);
    let out = run(&mock, &reg, &["passphrase", "is", "gone"]).output;
    assert!(out.contains("slot::the_passphrase_is_gone  (id)"), "{out}");
    assert!(
        !out.contains("ruling::scattered"),
        "every word of the phrase, and the phrase nowhere: {out}"
    );
    assert!(out.contains("1 of 2 rows carry it."), "{out}");
}

#[test]
fn a_hit_names_the_row_and_every_field_of_it_that_carries_the_term() {
    // Field order is a fact about what a person reads, and it is not the
    // order the row was written in: `run` hands over a row, which is a map,
    // so `keywords` comes before `says` however the toml has it.
    let mock = no_corpus("hit-shape");
    let reg = registry(&[
        ("ruling::a", &[
            ("says", "conveyance obliges corresponding source"),
            ("keywords", "convey, third party"),
            ("provenance", "panel::x::#y"),
            ("cites", "b"),
            ("superseded_by", "b"),
            ("related", "tenet::b"),
            ("subject", "x"),
        ]),
        ("ruling::b", &[("says", "the compositor consumes actions")]),
    ]);
    let out = run(&mock, &reg, &["convey"]).output;
    assert!(out.contains("ruling::a  (keywords, says)"), "{out}");
    assert!(
        !out.contains("provenance"),
        "a citation field is not a mention: {out}"
    );
    assert!(out.contains("1 of 2 rows carry it."), "{out}");

    // The control: the count is over every row walked rather than over the
    // hits, so it says how much of the corpus the answer rests on.
    assert_eq!(examined(&run(&mock, &reg, &["convey"])), 2);
    assert_eq!(examined(&run(&mock, &reg, &["nothing_carries_this"])), 2);
}

#[test]
fn the_research_half_reports_what_the_corpus_holds_and_where() {
    // The wiring, which is the part no test could reach while `run` was
    // never called. `research_line` takes four counts in a row and each is a
    // different fact: how many documents carry it, how many were read, how
    // many are listed. Two of them being swapped compiles and reads as a
    // measurement, which is the defect that sentence exists to have fixed.
    let mock = Mock::new("research", &["research", "design_rounds"], &[
        (
            "research/a.md",
            "# a\n\nthe quaternion is mentioned here.\n",
        ),
        ("research/b.md", "# b\n\nnothing of the sort.\n"),
    ]);
    let reg = registry(&[("ruling::a", CITED)]);
    let out = run(&mock, &reg, &["quaternion"]).output;
    assert!(
        out.contains("in 1 of 2 documents"),
        "one carries it, two were read: {out}"
    );
    assert!(out.contains("The nearest 1:"), "and one is listed: {out}");
    assert!(out.contains("research/a.md:3"), "{out}");
    assert!(out.contains("Run `mock unearth`"), "{out}");
    assert!(
        !out.contains("roots absent"),
        "all three roots are on disk here: {out}"
    );

    // The control: a term the same corpus does not carry is a real absence,
    // and it is the answer that licenses interrupting a person, so it must
    // not read like the two sentences above.
    let out = run(&mock, &reg, &["palindrome"]).output;
    assert!(out.contains("Nothing in the 2 research documents"), "{out}");
    assert!(!out.contains("The nearest"), "{out}");
    assert!(!out.contains("Run `mock unearth`"), "{out}");
}

#[test]
fn a_root_absent_from_disk_is_named_under_the_count() {
    // A count over a corpus with a root missing is a smaller claim in the
    // same words, and this tool once said "in at least 3 of 3920 documents"
    // with two whole trees unread and nothing saying so.
    let mock = Mock::new("one-root", &["research"], &[(
        "research/a.md",
        "# a\n\nthe quaternion is mentioned here.\n",
    )]);
    let reg = registry(&[("ruling::a", CITED)]);
    let out = run(&mock, &reg, &["quaternion"]).output;
    assert!(out.contains("in 1 of 1 documents"), "{out}");
    assert!(
        out.contains("Not read: design_rounds is not on disk and was not read"),
        "{out}"
    );
}

#[test]
fn a_corpus_that_could_not_be_read_says_so_rather_than_reporting_no_hits() {
    // The state that has to stay apart from a real absence: no document was
    // read, so nothing here is evidence about the term either way, and the
    // registry rows above are the whole of the answer.
    let mock = no_corpus("unreadable");
    let reg = registry(&[("ruling::a", CITED)]);
    let out = run(&mock, &reg, &["quaternion"]).output;
    assert!(out.contains("could not be read"), "{out}");
    assert!(
        !out.contains("Nothing in the"),
        "an unread corpus is not an absence: {out}"
    );
    assert!(
        out.contains("research is not on disk and was not read"),
        "and it says why: {out}"
    );
}

// -----------------------------------------------------------------------
// kaski's shape: op's captured words sit outside the repository, and a
// superseded tenet no longer binds.
// -----------------------------------------------------------------------

#[test]
fn op_s_captured_words_beside_the_repository_are_searched_and_named_as_his() {
    // The question this tool exists for is whether op has answered it. His
    // words sit in the workspace's `.data/op-responses`, outside the mock
    // directory, and a tool that never reads them says nothing about what
    // he said.
    let ws = std::env::temp_dir().join("kaski-already-said-op-ws");
    let _ = std::fs::remove_dir_all(&ws);
    let repo = ws.join("kaski");
    let mock = repo.join("mock");
    std::fs::create_dir_all(mock.join("research/202610031300_minds")).unwrap();
    std::fs::create_dir_all(mock.join("design_rounds")).unwrap();
    std::fs::write(
        mock.join("research/202610031300_minds/NOTES.md"),
        "# Minds\n\nA mind weighs its options.\n",
    )
    .unwrap();
    let op = ws.join("clause-dev/.data/op-responses");
    std::fs::create_dir_all(&op).unwrap();
    std::fs::write(
        op.join("202610040900_op-on-kaski-research.md"),
        "# kaski: who directs research\n\n> the realm sets what is studied, a scholar chooses how\n",
    )
    .unwrap();

    let reg = registry(&[("tenet::a", CITED)]);
    let crates = BTreeSet::new();
    let dirs: Vec<PathBuf> = Vec::new();
    let index = ready_at(None, &mock, &repo, Some(&op));
    let out = AlreadySaid
        .answer(
            &ToolContext {
                mock_dir:   &mock,
                repo_root:  &repo,
                all_crates: &crates,
                src_dirs:   &dirs,
                args:       &["scholar"],
                stdin:      None,
                registry:   &reg,
            },
            &index,
        )
        .output;
    let _ = std::fs::remove_dir_all(&ws);
    assert!(
        out.contains("op-responses/202610040900_op-on-kaski-research.md:3  [human,"),
        "op's capture is found and named as his: {out}"
    );
    assert!(!out.contains("op's captured words were not read"), "{out}");
}

#[test]
fn a_superseded_tenet_is_marked_as_no_longer_binding() {
    // A superseded tenet stays whole in the canon so its replacement is read
    // with it in view. Listed bare, it reads as an answer that still binds.
    let mock = no_corpus("superseded");
    let reg = registry(&[
        ("tenet::the_old_one", &[
            ("what", "a scholar picks the subject"),
            ("superseded_by", "the_new_one"),
            ("related", "tenet::the_new_one"),
            ("provenance", "research::x::NOTES::#a"),
            ("cites", "the_new_one"),
            ("subject", "knowledge"),
        ]),
        ("tenet::the_new_one", &[
            (
                "what",
                "the realm picks the subject and a scholar the method",
            ),
            ("subject", "knowledge"),
        ]),
    ]);
    let out = run(&mock, &reg, &["scholar"]).output;
    assert!(
        out.contains("tenet::the_old_one  (what)  superseded by the_new_one"),
        "{out}"
    );
    let new_line = out
        .lines()
        .find(|l| l.starts_with("tenet::the_new_one"))
        .unwrap_or_default();
    assert!(!new_line.contains("superseded"), "{out}");
}

/// A clone of a bare origin on `dev`, and a second clone that pushes after it.
fn behind_world(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let git = |dir: &std::path::Path, args: &[&str]| {
        let out = std::process::Command::new("git")
            .current_dir(dir)
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@example.org",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .expect("git runs");
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    let base = std::env::temp_dir().join(format!("kaski-already-said-behind-{name}"));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(base.join("origin.git")).unwrap();
    git(&base.join("origin.git"), &[
        "init", "-q", "--bare", "-b", "dev",
    ]);
    git(&base, &["clone", "-q", "origin.git", "other"]);
    git(&base.join("other"), &["switch", "-q", "-c", "dev"]);
    std::fs::write(base.join("other/a.txt"), "a").unwrap();
    git(&base.join("other"), &["add", "-A"]);
    git(&base.join("other"), &["commit", "-q", "-m", "chore: a"]);
    git(&base.join("other"), &["push", "-q", "origin", "dev"]);
    git(&base, &["clone", "-q", "-b", "dev", "origin.git", "tree"]);
    std::fs::write(base.join("other/b.txt"), "b").unwrap();
    git(&base.join("other"), &["add", "-A"]);
    git(&base.join("other"), &["commit", "-q", "-m", "chore: b"]);
    git(&base.join("other"), &["push", "-q", "origin", "dev"]);
    (base.clone(), base.join("tree"), base.join("other"))
}

fn run_at(tree: &std::path::Path, reg: &RegistryView) -> ToolReport {
    let crates = BTreeSet::new();
    let dirs: Vec<PathBuf> = Vec::new();
    AlreadySaid.run_over(
        &ToolContext {
            mock_dir:   tree,
            repo_root:  tree,
            all_crates: &crates,
            src_dirs:   &dirs,
            args:       &["term"],
            stdin:      None,
            registry:   reg,
        },
        &ready_at(None, tree, tree, None),
    )
}

#[test]
fn a_tree_behind_the_trunk_is_said_to_be_before_the_answer_and_a_current_one_is_not() {
    let (base, tree, other) = behind_world("notice");
    let reg = registry(&[("tenet::a", CITED)]);
    let behind = run_at(&tree, &reg);
    assert!(
        behind
            .output
            .starts_with("this answer reads the working tree of "),
        "{}",
        behind.output
    );
    assert!(
        behind.output.contains("1 commit behind origin/dev"),
        "{}",
        behind.output
    );
    // the answer itself follows the notice, unchanged
    assert!(
        behind.output.contains("rows carry it."),
        "{}",
        behind.output
    );
    // the control: the other clone is the trunk, so it is behind nothing and says nothing
    let current = run_at(&other, &reg);
    assert!(
        !current.output.contains("behind origin/dev"),
        "{}",
        current.output
    );
    assert!(
        !current.output.starts_with("this answer"),
        "{}",
        current.output
    );
    let _ = std::fs::remove_dir_all(base);
}
