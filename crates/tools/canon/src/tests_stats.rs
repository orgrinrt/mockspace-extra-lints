//! `canon stats` over a planted registry: each table, and that a wrong question is refused.

use std::path::PathBuf;

use crate::stats::table;

fn planted() -> (PathBuf, corpus::repo::Ready) {
    let repo = std::env::temp_dir().join(format!("kaski_canon_stats_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&repo);
    let reg = repo.join("mock/registry/tenet");
    std::fs::create_dir_all(&reg).unwrap();
    std::fs::write(
        reg.join("t.toml"),
        "[[tenet]]\nid = \"a\"\nsubject = \"air\"\nwhat = \"heat rises\"\ncites = [\"tenet::hub\"]\n\n[[tenet]]\nid = \"hub\"\nsubject = \"air\"\nwhat = \"heat spreads\"\n\n[[tenet]]\nid = \"loner\"\nwhat = \"apart\"\n",
    )
    .unwrap();
    let cache = repo.join("cache");
    let ready = corpus::repo::ready_at(Some(&cache), &repo.join("mock"), &repo, None, &[]);
    (repo, ready)
}

#[test]
fn each_table_answers_from_the_index_and_a_wrong_question_is_refused() {
    let (repo, ready) = planted();
    let i = &ready.index;
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
