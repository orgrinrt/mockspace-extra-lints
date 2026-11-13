use super::*;

// --- the failure this exists to correct -----------------------------------------

#[test]
fn a_document_using_other_words_is_found_through_registry_keywords() {
    // The real case. The question says "two surfaces"; the corpus says "broker
    // with two backends" and never says surface. A grep for the question
    // returns nothing, which is what happened and what was reported to op.
    let d = Planted::new("otherwords");
    d.file(
        "panel/04_taxonomy.md",
        "## Data architecture\n\nBoth stores are owned by the broker. \
         A column store for the catalogue, a memory-mapped tree for the index.\n",
    )
    .file(
        "panel/07_unrelated.md",
        "## Fonts\n\nThe renderer picks a face.\n",
    );

    let bare = search(&d, "two surfaces", &[]);
    assert!(
        bare.is_empty(),
        "the plain question must miss it, or this test is not about the defect: {bare:?}"
    );

    let expanded = search(&d, "two surfaces", &["broker", "column store", "index"]);
    assert_eq!(
        expanded.first().map(String::as_str),
        Some("research/panel/04_taxonomy.md"),
        "registry keywords must reach the document the question could not"
    );
}

#[test]
fn a_word_is_found_in_every_form_it_takes() {
    let d = Planted::new("forms");
    d.file("panel/01.md", "The image is conveyed to a third party.\n")
        .file("panel/02.md", "Conveyance obliges corresponding source.\n")
        .file(
            "panel/03.md",
            "Nothing here is about giving anything away.\n",
        );

    let hits = search(&d, "convey", &[]);
    assert_eq!(hits.len(), 2, "both forms, and only those: {hits:?}");
    assert!(hits.iter().any(|h| h.ends_with("01.md")));
    assert!(hits.iter().any(|h| h.ends_with("02.md")));
}

// --- the negative controls ------------------------------------------------------

#[test]
fn a_subject_the_corpus_does_not_carry_returns_nothing() {
    // The control that makes every other result mean something. Without it a
    // search that always returns the whole corpus would pass every test above.
    let d = Planted::new("absent");
    d.file("panel/01.md", "The compositor consumes actions.\n")
        .file("panel/02.md", "The seal is verified at boot.\n");

    let hits = search(&d, "quaternion interpolation", &[]);
    assert!(hits.is_empty(), "must come back empty: {hits:?}");
}

#[test]
fn a_stem_does_not_match_inside_another_word() {
    // `mediat` must not find `intermediate`, and this is the specific reason a
    // real stemmer was refused: conflating two subjects is worse than missing
    // one, because the reader cannot tell it happened.
    let d = Planted::new("boundary");
    d.file(
        "panel/01.md",
        "An intermediate representation is lowered.\n",
    );

    let hits = search(&d, "mediator", &[]);
    assert!(
        hits.is_empty(),
        "prefix must start at a word boundary: {hits:?}"
    );
}

#[test]
fn a_term_in_every_document_ranks_nothing() {
    // Inverse frequency, as a behaviour rather than as arithmetic. A word in
    // every file cannot decide which file to open, and a ranking that lets it
    // is a word count wearing a score.
    let d = Planted::new("ubiquitous");
    for i in 1 ..= 5 {
        d.file(&format!("panel/0{i}.md"), "the canon says something\n");
    }
    d.file("panel/06.md", "the canon says something about duckdb\n");

    let corpus = Corpus::load(d.mock());
    let q = Query::new("canon duckdb");
    let out = rank(&corpus, &Index::build(&corpus), &q, 20);
    assert_eq!(
        out.hits.first().map(|h| h.path.as_str()),
        Some("research/panel/06.md"),
        "the rare term must decide it"
    );
    assert!(
        out.ubiquitous.iter().any(|t| t == "canon"),
        "and the useless term must be reported as useless: {:?}",
        out.ubiquitous
    );
}

#[test]
fn a_term_that_matches_nothing_is_named() {
    // The most useful line in the output: which expansions were dead. A reader
    // cannot tell a real absence from a bad expansion without it.
    let d = Planted::new("dead");
    d.file("panel/01.md", "The broker owns both stores.\n");

    let corpus = Corpus::load(d.mock());
    let mut q = Query::new("broker");
    q.push("fjall".to_string(), Source::Keyword);
    let out = rank(&corpus, &Index::build(&corpus), &q, 20);
    assert!(
        out.dead.iter().any(|t| t == "fjall"),
        "dead terms: {:?}",
        out.dead
    );
    assert!(!out.dead.iter().any(|t| t == "broker"));
}
