//! What the corpus must do over a project's own shape: research notes in a directory per round of
//! research, the round transcripts, and the maintainer's captured words, which sit outside the
//! repository.

use std::path::Path;

use mockspace_extra_corpus::corpus::citation_path;
use mockspace_extra_corpus::doc::Kind;
use mockspace_extra_corpus::maintainer::maintainer_words_dir;
use mockspace_extra_corpus::{Corpus, Index, Query, rank};

use super::Planted;

#[test]
fn each_document_is_the_kind_it_is() {
    let k = |p: &str| Kind::of(Path::new(p));
    assert_eq!(
        k("maintainer-words/202610040900_on-knowledge.md"),
        Kind::Maintainer
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
        k("design_rounds/202610040805/202610040800_topic.on-the-canon-shape-questions.md"),
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
    // a note named for the maintainer is still a note: what earns their weight is where it was
    // captured, never a name
    assert_eq!(
        k("research/202610040605_canon-shape/maintainer-answers.md"),
        Kind::Notes
    );
}

#[test]
fn the_maintainer_s_captured_words_are_searched_and_weigh_as_theirs() {
    let d = Planted::new("shape-words");
    d.file(
        "202610031300_minds/NOTES.md",
        "# Minds\n\nA mind weighs its options.\n",
    );
    let words = d.mock().join("outside").join("words");
    std::fs::create_dir_all(&words).unwrap();
    std::fs::write(
        words.join("202610040900_on-research.md"),
        "# who directs research\n\n> the realm sets what is studied, a scholar chooses how\n",
    )
    .unwrap();

    let corpus = Corpus::load_with(d.mock(), Some(&words));
    let q = Query::new("who directs research scholar");
    let found = rank(&corpus, &Index::build(&corpus), &q, 5);
    let hit = found
        .hits
        .iter()
        .find(|h| h.path.starts_with("maintainer-words/"))
        .unwrap_or_else(|| {
            panic!(
                "the maintainer's capture not among {:?}",
                found.hits.iter().map(|h| &h.path).collect::<Vec<_>>()
            )
        });
    assert_eq!(hit.kind, Kind::Maintainer);
    assert!(
        hit.excerpt.contains("who directs research") || hit.excerpt.contains("realm sets"),
        "{}",
        hit.excerpt
    );
    assert!(
        corpus
            .caveat()
            .is_none_or(|c| !c.contains("captured words"))
    );
}

#[test]
fn a_corpus_without_the_maintainer_s_words_says_how_to_name_them() {
    let d = Planted::new("shape-no-words");
    d.file(
        "202610031300_minds/NOTES.md",
        "# Minds\n\nA mind weighs its options.\n",
    );
    let corpus = Corpus::load_with(d.mock(), None);
    let caveat = corpus
        .caveat()
        .expect("a caveat when the maintainer's words were not read");
    assert!(
        caveat.contains("captured words were not read: name their directory with `maintainer_words`"),
        "{caveat}"
    );
    assert!(caveat.contains("`MOCKSPACE_MAINTAINER_WORDS`"), "{caveat}");
}

#[test]
fn a_named_directory_that_does_not_exist_is_said_to_be_missing_and_reads_nothing() {
    let d = Planted::new("shape-missing-words");
    d.file("202610031300_minds/NOTES.md", "# Minds\n");
    let named = maintainer_words_dir(d.mock(), Some("no-such-words"), None).unwrap();
    let corpus = Corpus::load_with(d.mock(), Some(&named));
    assert!(!corpus.words_read);
    assert!(corpus.docs.iter().all(|doc| doc.kind != Kind::Maintainer));
    let caveat = corpus.caveat().expect("a caveat for a missing directory");
    assert!(
        caveat.contains("no-such-words`, named for them, is not a directory"),
        "{caveat}"
    );

    // THE CONTROL. The same name, once the directory exists, is read and not reported.
    std::fs::create_dir_all(&named).unwrap();
    std::fs::write(named.join("a.md"), "# a\n\nwords\n").unwrap();
    let corpus = Corpus::load_with(d.mock(), Some(&named));
    assert!(corpus.words_read);
    assert!(corpus.caveat().is_none_or(|c| !c.contains("captured words")));
}

#[test]
fn a_citation_names_its_note() {
    let exists = |p: &str| p == "research/202610031300_time-history-minds-and-cultures/NOTES.md";
    assert_eq!(
        citation_path(
            "research::202610031300_time-history-minds-and-cultures::NOTES::#on-where-knowledge-lives",
            exists
        ),
        Some("research/202610031300_time-history-minds-and-cultures/NOTES.md".to_string())
    );
}

#[test]
fn a_sketch_is_read_and_an_image_or_a_table_is_not() {
    let d = Planted::new("shape-ext");
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
    let d = Planted::new("shape-scope");
    d.file("202610031300_minds/NOTES.md", "# Minds\n");
    let words = d.mock().join("words");
    std::fs::create_dir_all(&words).unwrap();
    std::fs::write(
        words.join("a.md"),
        "# on granary\n\nthe realm directs research\n",
    )
    .unwrap();
    std::fs::write(
        words.join("b.md"),
        "# on orchard\n\nthe realm of a type directs research\n",
    )
    .unwrap();

    let corpus = Corpus::load_about(d.mock(), Some(&words), "granary");
    let held: Vec<String> = corpus
        .docs
        .iter()
        .filter(|doc| doc.kind == Kind::Maintainer)
        .map(|doc| doc.path.to_string_lossy().to_string())
        .collect();
    assert_eq!(held, vec!["maintainer-words/a.md".to_string()]);
    assert_eq!(corpus.elsewhere, 1);
    let caveat = corpus.caveat().expect("held-out captures are said");
    assert!(
        caveat.contains("1 of the maintainer's captures name no `granary`"),
        "{caveat}"
    );

    // THE CONTROL. Unscoped, both are read and nothing is held out.
    let all = Corpus::load_with(d.mock(), Some(&words));
    assert_eq!(
        all.docs
            .iter()
            .filter(|doc| doc.kind == Kind::Maintainer)
            .count(),
        2
    );
    assert_eq!(all.elsewhere, 0);
}
