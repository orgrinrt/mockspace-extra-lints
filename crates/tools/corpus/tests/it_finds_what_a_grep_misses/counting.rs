use super::*;

// --- what was cut and what merely did not fit are different numbers -------------

/// Eight documents that all carry the subject, so the only thing separating one
/// run from another is where the limit falls.
///
/// Named per caller because `Planted::new` clears its directory and the tests
/// run in parallel in one process, so a shared name has one test wiping
/// another's corpus mid-run. That is how this helper first failed.
fn eight_matching(name: &str) -> Planted {
    let d = Planted::new(name);
    for i in 0 .. 8 {
        d.file(
            &format!("panel/{i:02}_verity.md"),
            "dm-verity seals the root filesystem and every block is checked on read\n",
        );
    }
    d
}

fn ranked(dir: &Planted, phrase: &str, limit: usize) -> mockspace_extra_corpus::Search {
    let corpus = Corpus::load(dir.mock());
    rank(&corpus, &Index::build(&corpus), &Query::new(phrase), limit)
}

#[test]
fn the_display_limit_does_not_move_the_confidence_cut_count() {
    // The count was `before - hits.len()` taken after both the cut and the
    // truncation, so every document the limit pushed out was reported as having
    // scored too low to show. On the real corpus that sentence covered 501 of
    // the 543 it named, and a reader deciding whether to rephrase reads it as
    // the search having found nothing worth reading.
    //
    // A document's score does not know what limit it was ranked under, so the
    // number of documents below the cut cannot change when the limit does. That
    // is the whole assertion and it needs no engineered scores.
    let d = eight_matching("cut-vs-limit");

    let wide = ranked(&d, "sealed root filesystem", 100);
    let tight = ranked(&d, "sealed root filesystem", 2);

    assert_eq!(
        tight.trimmed, wide.trimmed,
        "the confidence cut is a property of the scores, not of the limit"
    );
    assert!(
        tight.capped > 0,
        "documents past the limit are counted, and counted separately"
    );
}

#[test]
fn every_scoring_document_is_shown_trimmed_or_capped() {
    // The two counts partition what the search found, so nothing can go missing
    // between them and nothing can be counted twice. Without this the previous
    // test is satisfied by a `capped` that is always zero.
    let d = eight_matching("partition");
    let out = ranked(&d, "sealed root filesystem", 2);

    assert_eq!(
        out.hits.len() + out.trimmed + out.capped,
        8,
        "every document that scored is shown, cut, or held back, and only one of them"
    );
    assert_eq!(out.hits.len(), 2, "the limit was honoured");
}

#[test]
fn a_limit_wider_than_the_corpus_caps_nothing() {
    // The control. A `capped` that reported everything not shown would satisfy
    // the partition above just as well, and would be the same lie wearing the
    // other name.
    let d = eight_matching("wide-limit");
    let out = ranked(&d, "sealed root filesystem", 100);

    assert_eq!(
        out.capped, 0,
        "nothing was held back by a limit nothing reached"
    );
}

#[test]
fn how_many_documents_scored_is_not_how_many_were_shown() {
    // `already-said` printed "in at least 3 of 3970 documents" for every term
    // the corpus carries at all, and the 3 was the limit it had passed in. The
    // real figure was always available and is what a reader is after: `boot` is
    // in 1284 of them, and "at least 3" is what the tool says about a word in
    // every third file.
    let d = eight_matching("scored-not-shown");
    let out = ranked(&d, "sealed root filesystem", 3);

    assert_eq!(out.hits.len(), 3, "three were shown");
    assert_eq!(out.scored(), 8, "and eight were found");
}

#[test]
fn a_corpus_that_carries_nothing_scored_nothing() {
    // The control, and it is the sentence `already-said` rests on: a term the
    // corpus does not hold has to come back at zero rather than at the limit.
    let d = eight_matching("scored-nothing");
    let out = ranked(&d, "quaternion interpolation", 3);

    assert_eq!(out.scored(), 0);
    assert!(out.hits.is_empty());
}

#[test]
fn a_question_carrying_one_real_word_is_asked() {
    // The control. Without it every assertion above is satisfied by a search
    // that stopped searching, and the stop list would be free to grow until it
    // swallowed the corpus.
    let d = Planted::new("one-real-word");
    d.file(
        "panel/01_boot.md",
        "the root filesystem is sealed and checked on every read\n",
    );

    let out = run(&d, "what does it do with the filesystem");
    assert_eq!(out.searched, 1, "the corpus was read");
    assert_eq!(
        out.hits.len(),
        1,
        "the one real word found the one document"
    );
    assert!(
        out.dropped.contains(&"what".to_string()),
        "the dropped words are still reported on a question that was asked"
    );
}

#[test]
fn a_real_question_over_a_silent_corpus_is_dead_rather_than_unasked() {
    // The second control, and the distinction the whole finding rests on. A
    // question the corpus genuinely does not answer must still report the corpus
    // as read and the terms as dead, because that is the state where an absence
    // is real and a person is worth interrupting. Collapsing the two is what
    // made silence indistinguishable from a real answer.
    let d = Planted::new("silent-corpus");
    d.file("panel/01_boot.md", "the root filesystem is sealed\n");

    let out = run(&d, "duckdb");
    assert_eq!(out.searched, 1, "the corpus was read and had nothing");
    assert!(out.hits.is_empty());
    assert!(
        out.dead.contains(&"duckdb".to_string()),
        "a term that was asked and matched nothing is dead"
    );
    assert!(
        out.dropped.is_empty(),
        "nothing was dropped, so nothing suggests rephrasing"
    );
}
