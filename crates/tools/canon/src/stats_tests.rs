//! `canon stats` over a planted registry: each table, and that a wrong question is refused.

use std::path::PathBuf;

use crate::stats::table;

fn planted() -> (PathBuf, corpus::repo::Index) {
    let repo = std::env::temp_dir().join(format!("kaski_canon_stats_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&repo);
    let reg = repo.join("mock/registry/tenet");
    std::fs::create_dir_all(&reg).unwrap();
    std::fs::write(
        reg.join("t.toml"),
        "[[tenet]]\nid = \"a\"\nsubject = \"air\"\nwhat = \"heat rises\"\ncites = [\"tenet::hub\"]\n\n[[tenet]]\nid = \"hub\"\nsubject = \"air\"\nwhat = \"heat spreads\"\n\n[[tenet]]\nid = \"loner\"\nwhat = \"apart\"\n",
    )
    .unwrap();
    let ok = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&repo)
        .status();
    assert!(ok.is_ok_and(|s| s.success()), "git init");
    let index = repo.join("index").join("search.sqlite");
    corpus::repo::refresh_at(&index, &repo.join("mock"), &repo, None).unwrap();
    let opened = corpus::repo::open_at(&index, &repo.join("mock"), &repo, None, &|_| {});
    (repo, opened.index.unwrap())
}

#[test]
fn each_table_answers_from_the_index_and_a_wrong_question_is_refused() {
    let (repo, index) = planted();
    let i = &index;
    assert!(table(i, "cited", None).unwrap().contains("tenet::hub"));
    let orphans = table(i, "orphans", None).unwrap();
    assert!(
        orphans.contains("tenet::loner") && !orphans.contains("tenet::hub"),
        "{orphans}"
    );
    assert!(table(i, "terms", Some("air")).unwrap().contains("heat"));
    assert!(
        table(i, "files", Some("tenet::hub"))
            .unwrap()
            .contains("registry/tenet/t.toml")
    );
    // THE CONTROL: a table that needs a word without one, an unknown table, and a row nothing cites.
    assert!(table(i, "terms", None).unwrap_err().contains("a subject"));
    assert!(table(i, "bogus", None).unwrap_err().contains("no table"));
    assert!(
        table(i, "files", Some("tenet::loner"))
            .unwrap()
            .contains("nothing")
    );
    let _ = std::fs::remove_dir_all(&repo);
}
