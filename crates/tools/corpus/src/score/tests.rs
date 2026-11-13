use super::*;
use crate::corpus::Corpus;

/// A corpus on disk, removed by the caller.
fn planted(name: &str, files: &[(&str, &str)]) -> (std::path::PathBuf, Corpus) {
    let dir = std::env::temp_dir().join(format!("kamu-score-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("research")).unwrap();
    for (f, body) in files {
        std::fs::write(dir.join("research").join(f), body).unwrap();
    }
    let corpus = Corpus::load(&dir);
    (dir, corpus)
}

fn search(corpus: &Corpus, phrase: &str) -> Search {
    let index = Index::build(corpus);
    rank(corpus, &index, &Query::new(phrase), 8)
}

#[test]
fn a_document_carrying_only_the_common_words_is_worth_less_than_its_share_says() {
    // The measured shape, in miniature. `paper` is in one of the four
    // documents and `machine` is in all four, so a document holding
    // `machine` alone carries half the question's words and a fraction of
    // what they are worth, and the report shows both.
    let (dir, corpus) = planted("gap", &[
        ("01.md", "the machine and the paper it prints on"),
        ("02.md", "the machine again"),
        ("03.md", "another machine here"),
        ("04.md", "a machine and nothing else"),
    ]);
    // Three terms, not two: `Query::new` searches the whole phrase as well
    // as each word, which is what makes a third of the question the share a
    // document holding one word carries.
    let found = search(&corpus, "paper machine");
    let common = found
        .hits
        .iter()
        .find(|h| h.path.ends_with("02.md"))
        .expect("the common-words-only document is a hit");
    assert!(
        (common.coverage - 1.0 / 3.0).abs() < 1e-9,
        "one of the question's three terms: {}",
        common.coverage
    );
    assert!(
        common.informative < common.coverage,
        "and less than a third of what they are worth: {} against {}",
        common.informative,
        common.coverage
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_document_carrying_the_whole_question_is_worth_all_of_it() {
    // The negative control, and the one that would fail if `informative`
    // were merely a smaller number rather than a share: a document holding
    // every term must reach one on both, whatever the terms cost.
    // The phrase term as well as both words, so the document really does
    // hold the whole question rather than two thirds of it.
    let (dir, corpus) = planted("whole", &[
        (
            "01.md",
            "a paper machine: the machine and the paper it prints on",
        ),
        ("02.md", "the machine again"),
    ]);
    let found = search(&corpus, "paper machine");
    let whole = found
        .hits
        .iter()
        .find(|h| h.path.ends_with("01.md"))
        .expect("the document holding both terms is a hit");
    assert!((whole.coverage - 1.0).abs() < 1e-9, "{}", whole.coverage);
    assert!(
        (whole.informative - 1.0).abs() < 1e-9,
        "{}",
        whole.informative
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn terms_the_corpus_values_equally_are_weighted_equally() {
    // The other negative control, and the invariant that says the weight is
    // the corpus's own frequency rather than something else: two documents
    // each carrying one of two equally common words must come back
    // indistinguishable on both shares. Neither word is ever adjacent to the
    // other, so the phrase term matches nothing and penalises both alike.
    let (dir, corpus) = planted("flat", &[
        ("01.md", "aardvark and cornice"),
        ("02.md", "buttress and cornice"),
        ("03.md", "aardvark, then buttress, apart"),
    ]);
    let found = search(&corpus, "aardvark buttress");
    let a = found
        .hits
        .iter()
        .find(|h| h.path.ends_with("01.md"))
        .expect("the aardvark document is a hit");
    let b = found
        .hits
        .iter()
        .find(|h| h.path.ends_with("02.md"))
        .expect("the buttress document is a hit");
    assert!(
        (a.coverage - b.coverage).abs() < 1e-9,
        "{} against {}",
        a.coverage,
        b.coverage
    );
    assert!(
        (a.informative - b.informative).abs() < 1e-9,
        "{} against {}",
        a.informative,
        b.informative
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn two_documents_carrying_the_same_share_differ_by_what_that_share_was_worth() {
    // The control the other three do not supply. Each of them passes for any
    // function of coverage alone, a square included, so none of them says
    // the weight is the corpus's own frequency. This one does: the two
    // documents carry one term each, so their shares are identical, and the
    // only thing left to separate them is that `paper` is in one document
    // and `machine` in three.
    let (dir, corpus) = planted("rarity", &[
        ("01.md", "paper alone"),
        ("02.md", "machine alone"),
        ("03.md", "machine here"),
        ("04.md", "machine there"),
    ]);
    let found = search(&corpus, "paper machine");
    let rare = found
        .hits
        .iter()
        .find(|h| h.path.ends_with("01.md"))
        .expect("the rare-term document is a hit");
    let common = found
        .hits
        .iter()
        .find(|h| h.path.ends_with("02.md"))
        .expect("the common-term document is a hit");
    assert!(
        (rare.coverage - common.coverage).abs() < 1e-9,
        "the same share of the question: {} against {}",
        rare.coverage,
        common.coverage
    );
    assert!(
        rare.informative > common.informative,
        "and the rarer share is worth more: {} against {}",
        rare.informative,
        common.informative
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_excerpt_comes_back_in_the_case_it_was_written_in() {
    // The corpus holds one lowercased copy and that is right for searching.
    // A line shown to a person is not being searched, and `SANE` arriving as
    // `sane` is a different word.
    let (dir, corpus) = planted("case", &[(
        "01.md",
        "unrelated first line\nThe SANE backend and the OCR pass, per K72.\n",
    )]);
    let found = search(&corpus, "backend");
    let hit = found.hits.first().expect("hit");
    assert!(
        hit.excerpt.contains("SANE") && hit.excerpt.contains("K72"),
        "{}",
        hit.excerpt
    );
    assert!(
        !hit.excerpt.contains("sane"),
        "the lowercased line is what this replaces: {}",
        hit.excerpt
    );
    assert_eq!(hit.line, 2, "and it is the line the hit is on");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_file_that_has_moved_still_yields_the_lowercased_line() {
    // Wrong case is a worse excerpt. No excerpt is a worse report, and a
    // corpus loaded before a rebase is an ordinary way to reach this.
    let (dir, corpus) = planted("moved", &[("01.md", "The SANE backend, per K72.\n")]);
    std::fs::remove_dir_all(&dir).unwrap();
    let found = search(&corpus, "backend");
    let hit = found.hits.first().expect("hit");
    assert!(hit.excerpt.contains("sane backend"), "{}", hit.excerpt);
}
