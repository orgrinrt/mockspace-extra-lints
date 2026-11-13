//! What the corpus must do over kaski's own shape: research notes in a directory per round of research,
//! the round transcripts, and op's captured words, which sit outside the repository.

use std::path::Path;

use mockspace_extra_corpus::corpus::{citation_path, op_words_dir};
use mockspace_extra_corpus::doc::Kind;
use mockspace_extra_corpus::{Corpus, Index, Query, rank};

use super::Planted;

#[test]
fn each_of_kaskis_documents_is_the_kind_it_is() {
    let k = |p: &str| Kind::of(Path::new(p));
    assert_eq!(
        k("op-responses/202610040900_op-on-kaski-knowledge.md"),
        Kind::Op
    );
    assert_eq!(
        k("research/202610031300_time-history-minds-and-cultures/NOTES.md"),
        Kind::Notes
    );
    assert_eq!(
        k("research/202610040605_canon-shape/ids-before.txt"),
        Kind::Notes
    );
    assert_eq!(
        k("design_rounds/202610040805/202610040800_topic.op-on-the-canon-shape-questions.md"),
        Kind::Round
    );
    assert_eq!(
        k("design_rounds/202610040815_topic.asking-the-canon-and-the-research.md"),
        Kind::Round
    );
    assert_eq!(k("research/sketches/fire.rs"), Kind::Sketch);
    assert_eq!(
        k("research/202610031650_animations/drive.cjs"),
        Kind::Sketch
    );
    // a note named for op is still a note: what earns op's weight is where it was captured, never a name
    assert_eq!(
        k("research/202610040605_canon-shape/op-answers.md"),
        Kind::Notes
    );
}

#[test]
fn op_s_captured_words_are_searched_and_weigh_as_his() {
    let d = Planted::new("kaski-op");
    d.file(
        "202610031300_minds/NOTES.md",
        "# Minds\n\nA mind weighs its options.\n",
    );
    let op = d.mock().join("outside").join("op-responses");
    std::fs::create_dir_all(&op).unwrap();
    std::fs::write(
        op.join("202610040900_op-on-research.md"),
        "# who directs research\n\n> the realm sets what is studied, a scholar chooses how\n",
    )
    .unwrap();

    let corpus = Corpus::load_with(d.mock(), Some(&op));
    let q = Query::new("who directs research scholar");
    let found = rank(&corpus, &Index::build(&corpus), &q, 5);
    let hit = found
        .hits
        .iter()
        .find(|h| h.path.starts_with("op-responses/"))
        .unwrap_or_else(|| {
            panic!(
                "op's capture not among {:?}",
                found.hits.iter().map(|h| &h.path).collect::<Vec<_>>()
            )
        });
    assert_eq!(hit.kind, Kind::Op);
    assert!(
        hit.excerpt.contains("who directs research") || hit.excerpt.contains("realm sets"),
        "{}",
        hit.excerpt
    );
    assert!(
        corpus
            .caveat()
            .is_none_or(|c| !c.contains("op's captured words"))
    );
}

#[test]
fn a_corpus_without_op_s_words_says_so() {
    let d = Planted::new("kaski-no-op");
    d.file(
        "202610031300_minds/NOTES.md",
        "# Minds\n\nA mind weighs its options.\n",
    );
    let corpus = Corpus::load_with(d.mock(), None);
    let caveat = corpus
        .caveat()
        .expect("a caveat when op's words were not read");
    assert!(caveat.contains("op's captured words"), "{caveat}");
}

#[test]
fn op_s_words_are_found_above_the_repository_or_beside_it() {
    let d = Planted::new("kaski-find-op");
    // op's own layout: the repository is a clone inside the workspace that holds `.data`
    let ws = d.mock().join("clause-dev");
    let inside = ws.join("kaski");
    std::fs::create_dir_all(ws.join(".data/op-responses")).unwrap();
    std::fs::create_dir_all(&inside).unwrap();
    assert_eq!(
        op_words_dir(&inside, None),
        Some(ws.join(".data/op-responses"))
    );

    // a helper's layout: the workspace is cloned beside the repository
    let beside = d.mock().join("kaski");
    std::fs::create_dir_all(&beside).unwrap();
    assert_eq!(
        op_words_dir(&beside, None),
        Some(ws.join(".data/op-responses"))
    );

    // a named directory wins, and a name naming nothing finds nothing rather than falling back
    let named = d.mock().join("elsewhere");
    std::fs::create_dir_all(&named).unwrap();
    assert_eq!(
        op_words_dir(&beside, Some(named.to_str().unwrap())),
        Some(named.clone())
    );
    assert_eq!(op_words_dir(&beside, Some("/no/such/dir")), None);
}

#[test]
fn a_kaski_citation_names_its_note() {
    let exists = |p: &str| p == "research/202610031300_time-history-minds-and-cultures/NOTES.md";
    assert_eq!(
        citation_path(
            "research::202610031300_time-history-minds-and-cultures::NOTES::#op-on-where-knowledge-lives",
            exists
        ),
        Some("research/202610031300_time-history-minds-and-cultures/NOTES.md".to_string())
    );
}

#[test]
fn a_sketch_is_read_and_an_image_or_a_table_is_not() {
    let d = Planted::new("kaski-ext");
    d.file(
        "sketches/fire.rs",
        "// the flame climbs the heat lattice\nfn main() {}\n",
    );
    d.file("202610031650_animations/frames.csv", "a,b\n1,2\n");
    d.file("202610031650_animations/sheet.png", "not an image");
    let corpus = Corpus::load_with(d.mock(), None);
    let kinds: Vec<(String, Kind)> = corpus
        .docs
        .iter()
        .map(|doc| (doc.path.to_string_lossy().to_string(), doc.kind))
        .collect();
    assert_eq!(kinds, vec![(
        "research/sketches/fire.rs".to_string(),
        Kind::Sketch
    )]);
}

#[test]
fn a_capture_about_another_project_is_held_out_and_counted() {
    let d = Planted::new("kaski-scope");
    d.file("202610031300_minds/NOTES.md", "# Minds\n");
    let op = d.mock().join("words");
    std::fs::create_dir_all(&op).unwrap();
    std::fs::write(
        op.join("a.md"),
        "# on kaski\n\nthe realm directs research\n",
    )
    .unwrap();
    std::fs::write(
        op.join("b.md"),
        "# on arvo\n\nthe realm of a type directs research\n",
    )
    .unwrap();

    let corpus = Corpus::load_about(d.mock(), Some(&op), "kaski");
    let ops: Vec<String> = corpus
        .docs
        .iter()
        .filter(|doc| doc.kind == Kind::Op)
        .map(|doc| doc.path.to_string_lossy().to_string())
        .collect();
    assert_eq!(ops, vec!["op-responses/a.md".to_string()]);
    assert_eq!(corpus.elsewhere, 1);
    let caveat = corpus.caveat().expect("held-out captures are said");
    assert!(
        caveat.contains("1 of op's captures name no `kaski`"),
        "{caveat}"
    );

    // THE CONTROL. Unscoped, both are read and nothing is held out.
    let all = Corpus::load_with(d.mock(), Some(&op));
    assert_eq!(
        all.docs.iter().filter(|doc| doc.kind == Kind::Op).count(),
        2
    );
    assert_eq!(all.elsewhere, 0);
}
