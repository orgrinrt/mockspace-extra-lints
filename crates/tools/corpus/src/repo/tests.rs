use std::path::{Path, PathBuf};

use muisti::repo::{DocClass, Query};

use super::*;

fn plant(name: &str, files: &[(&str, &str)]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kaski_repo_{}_{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    for (rel, text) in files {
        let path = dir.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    dir
}

#[test]
fn the_roots_are_the_research_and_the_rounds_and_op_words_only_where_a_directory_was_found() {
    let mock = Path::new("/m");
    let without = roots(mock, None);
    assert_eq!(
        without.iter().map(|r| r.shown.as_str()).collect::<Vec<_>>(),
        ["research", "design_rounds"]
    );
    assert_eq!(without[0].class, DocClass::Research);
    assert_eq!(without[1].class, DocClass::Design);
    let with = roots(mock, Some(Path::new("/words")));
    assert_eq!(with.len(), 3);
    assert_eq!(with[2].shown, "op-responses");
    assert_eq!(with[2].class, DocClass::Human);
}

#[test]
fn a_row_is_found_in_the_file_whose_table_carries_it() {
    let repo = plant("rows", &[
        (
            "mock/registry/tenet/air.toml",
            "# the air\n\n[[tenet]]\nid = \"the_air_is_one_lattice\"\nwhat = \"x\"\n\n[[tenet]]\nid = \"heat_is_a_field\"\n",
        ),
        (
            "mock/registry/ikiuni/core.toml",
            "[[ikiuni]]\nid = \"fold_once\"\n",
        ),
        ("mock/registry/subject.toml", "[[subject]]\nid = \"air\"\n"),
    ]);
    let cache = plant("rows_cache", &[]);
    let got = ready_at(Some(&cache), &repo.join("mock"), &repo, None);
    let at = |key: &str| got.index.row(key).unwrap().map(|r| r.path);
    assert_eq!(
        at("tenet::the_air_is_one_lattice").as_deref(),
        Some("registry/tenet/air.toml")
    );
    assert_eq!(
        at("tenet::heat_is_a_field").as_deref(),
        Some("registry/tenet/air.toml")
    );
    assert_eq!(
        at("ikiuni::fold_once").as_deref(),
        Some("registry/ikiuni/core.toml")
    );
    assert_eq!(at("subject::air").as_deref(), Some("registry/subject.toml"));
    // THE CONTROL: a slug written under another namespace is not the row of this one.
    assert_eq!(at("tenet::fold_once"), None);
    // The git path a caller asks the day of is the shown path below the repository.
    assert_eq!(
        repo_path(&repo.join("mock"), &repo, "registry/subject.toml"),
        "mock/registry/subject.toml"
    );
    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn an_index_is_ready_over_the_roots_and_answers_from_them() {
    let repo = plant("ready", &[(
        "mock/research/a.md",
        "# Note\n\nthe saturation curve of the air bends with heat\n",
    )]);
    let cache = plant("ready_cache", &[]);
    let got = ready_at(Some(&cache), &repo.join("mock"), &repo, None);
    let report = got.index.search(&Query {
        text:   "saturation curve",
        expand: &[],
        phrase: false,
        limit:  5,
        today:  20_000,
    });
    assert_eq!(report.hits.len(), 1, "{report:?}");
    assert_eq!(report.hits[0].path, "research/a.md");
    // The index is in the cache directory that was given, and not in the user's.
    assert!(cache.join("muisti/repos").is_dir(), "{:?}", got.notes);
    let _ = std::fs::remove_dir_all(&repo);
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn an_index_that_cannot_be_kept_on_disk_says_so_and_still_answers() {
    let repo = plant("nocache", &[(
        "mock/research/a.md",
        "# Note\n\nthe saturation curve of the air bends with heat\n",
    )]);
    let got = ready_at(None, &repo.join("mock"), &repo, None);
    assert!(
        got.notes.iter().any(|n| n.contains("not kept on disk")),
        "{:?}",
        got.notes
    );
    assert_eq!(got.index.len().unwrap(), 1);
    let _ = std::fs::remove_dir_all(&repo);
}

#[test]
fn the_near_words_of_a_question_are_the_forms_in_the_documents_and_the_synonyms_in_the_keywords() {
    let repo = plant("near", &[
        (
            "mock/research/a.md",
            "# Note\n\nthe air is saturated and saturates with heat\n",
        ),
        (
            "mock/registry/tenet/air.toml",
            "[[tenet]]\nid = \"heat_is_a_field\"\nkeywords = [\"warmth\", \"heat\", \"thermal\"]\n",
        ),
    ]);
    let cache = plant("near_cache", &[]);
    let got = ready_at(Some(&cache), &repo.join("mock"), &repo, None);
    let near = near_words(&got.index, "air saturation heat", 5);
    let find = |w: &str| near.iter().find(|n| n.word == w);
    assert_eq!(
        find("saturated").map(|n| n.synonym),
        Some(false),
        "{near:?}"
    );
    assert_eq!(
        find("warmth").map(|n| (n.synonym, n.rows)),
        Some((true, 1)),
        "{near:?}"
    );
    // THE CONTROL: a word nothing relates to adds nothing.
    assert!(near_words(&got.index, "zebra", 5).is_empty());
    let _ = std::fs::remove_dir_all(&repo);
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn op_words_that_never_name_this_project_are_not_read_and_the_other_roots_are_read_whole() {
    let repo = plant("project", &[(
        "mock/research/a.md",
        "# Note\n\nthe saturation curve of another thing\n",
    )]);
    let words = plant("project_words", &[
        ("a.md", "# a\n\nKaski saturation, said aloud\n"),
        ("b.md", "# b\n\nthe saturation of a different project\n"),
    ]);
    let cache = plant("project_cache", &[]);
    let got = ready_at(Some(&cache), &repo.join("mock"), &repo, Some(&words));
    let q = Query {
        text:   "saturation",
        expand: &[],
        phrase: false,
        limit:  10,
        today:  20_000,
    };
    let mut paths: Vec<String> = got
        .index
        .search(&q)
        .hits
        .into_iter()
        .map(|h| h.path)
        .collect();
    paths.sort();
    // THE CONTROL: the research file names no project and is held all the same.
    assert_eq!(
        paths,
        ["op-responses/a.md", "research/a.md"],
        "{:?}",
        got.notes
    );
    let _ = std::fs::remove_dir_all(&repo);
    let _ = std::fs::remove_dir_all(&words);
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn a_subject_asks_its_name_as_words_then_its_keywords_and_a_question_of_stop_words_is_not_askable()
{
    assert_eq!(
        question_of("air_saturation", &[
            "heat".to_string(),
            "vapour".to_string()
        ]),
        "air saturation heat vapour"
    );
    assert!(askable(&question_of("air_saturation", &[])));
    // THE CONTROL: a name and keywords that are all stop words ask nothing.
    assert!(!askable(&question_of("the_and_of", &["the".to_string()])));
}

const IKIUNI_NOTE: &str = "# A ported note\n\nThe agent writes that the weighing of offers is a curve.\n\nOp: the pheasant ledger is settled and carries no tax.\n\nA closing agent paragraph about moonlit orchards.\n";

fn ask(ready: &Ready, text: &str) -> muisti::repo::RepoReport {
    ready.index.search(&Query {
        text,
        expand: &[],
        phrase: true,
        limit: 5,
        today: 20_700,
    })
}

#[test]
fn a_phrase_op_said_only_in_a_ported_note_comes_back_as_his_with_its_date() {
    let repo = plant("ported", &[(
        "mock/research/202606231730_ikiuni_port.md",
        IKIUNI_NOTE,
    )]);
    let cache = plant("ported_cache", &[]);
    let got = ready_at(Some(&cache), &repo.join("mock"), &repo, None);
    let r = ask(&got, "pheasant ledger");
    assert_eq!(r.hits.len(), 1, "{:?}", r.hits);
    assert_eq!(r.hits[0].class, DocClass::Human);
    assert!(r.hits[0].day.is_some(), "{:?}", r.hits[0]);
    assert!(r.hits[0].age_days.is_some());
    assert!(
        found_by(&r.hits[0]).contains("op's words"),
        "{:?}",
        r.hits[0]
    );
}

#[test]
fn a_phrase_only_an_agent_wrote_in_the_same_ported_note_is_not_op_s() {
    let repo = plant("ported_ctl", &[(
        "mock/research/202606231730_ikiuni_port.md",
        IKIUNI_NOTE,
    )]);
    let cache = plant("ported_ctl_cache", &[]);
    let got = ready_at(Some(&cache), &repo.join("mock"), &repo, None);
    let r = ask(&got, "moonlit orchards");
    assert_eq!(r.hits.len(), 1, "{:?}", r.hits);
    assert_eq!(r.hits[0].class, DocClass::Research);
    assert_eq!(found_by(&r.hits[0]), "");
}

#[test]
fn a_question_in_words_a_research_file_never_uses_finds_it_through_its_keywords() {
    let repo = plant("kw", &[
        (
            "mock/research/granary.md",
            "# Granary stewardship\n\nThe granary keeps wheat dry. The granary door is barred at night.\n",
        ),
        (
            "mock/research/tides.md",
            "# Ocean tides\n\nThe tides pull on the shore twice a day.\n",
        ),
        (
            "mock/registry/tenet/t.toml",
            "[[tenet]]\nid = \"a\"\nkeywords = [\"harvest\", \"granary\", \"storehouse\"]\n",
        ),
    ]);
    let cache = plant("kw_cache", &[]);
    let mut got = ready_at(Some(&cache), &repo.join("mock"), &repo, None);
    let r = ask(&got, "harvest");
    let paths: Vec<&str> = r
        .hits
        .iter()
        .filter(|h| h.path.starts_with("research/"))
        .map(|h| h.path.as_str())
        .collect();
    assert_eq!(paths, ["research/granary.md"], "{:?}", r.hits);
    let granary = r
        .hits
        .iter()
        .find(|h| h.path == "research/granary.md")
        .unwrap();
    assert!(
        found_by(granary).contains("tied to: harvest"),
        "{granary:?}"
    );
    // THE CONTROL: the file the vocabulary ties nothing to is not found.
    assert!(!paths.contains(&"research/tides.md"));
    // Incremental: a second refresh analyses no document again.
    let again = got
        .index
        .refresh(&roots(&repo.join("mock"), None), Some(&repo))
        .unwrap();
    assert_eq!(again.analysed, 0, "{again:?}");
}

#[test]
fn a_persona_note_named_after_the_owner_is_not_op_s_words() {
    let repo = plant("persona", &[(
        "mock/research/202606231730_panel_orgrinrt.md",
        "# A panel file\n\nOrgrinrt: the walrus ledger is settled by the persona.\n\nOp: the pheasant ledger is settled by op.\n",
    )]);
    let cache = plant("persona_cache", &[]);
    let got = ready_at(Some(&cache), &repo.join("mock"), &repo, None);
    let persona = ask(&got, "walrus ledger");
    assert_eq!(
        persona.hits[0].class,
        DocClass::Research,
        "{:?}",
        persona.hits
    );
    assert!(!persona.hits[0].quoted);
    // THE CONTROL: an `Op:` line in the same file is lifted.
    assert_eq!(ask(&got, "pheasant ledger").hits[0].class, DocClass::Human);
}
