use super::*;

// --- stemming -------------------------------------------------------------------

#[test]
fn a_stem_never_becomes_a_wildcard() {
    // Below four characters a stem matches most of the language, so a short
    // word is matched whole instead. Without this, `is` matched `island`.
    assert_eq!(terms::stem("is"), "is");
    assert_eq!(terms::stem("does"), "does");
    assert_eq!(terms::stem("data"), "data");
    // And a real suffix does strip, longest first.
    assert_eq!(terms::stem("conveyance"), "convey");
    assert_eq!(terms::stem("mediators"), "mediat");
    assert_eq!(terms::stem("mediation"), "mediat");
    assert_eq!(terms::stem("mediator"), "mediat");
}

#[test]
fn the_forms_this_cannot_unify_are_pinned_rather_than_discovered() {
    // The known limit, written down so nobody rediscovers it as a bug. Suffix
    // stripping unifies forms that share a stem after one cut and cannot unify
    // a pair needing two different cuts to meet.
    //
    // A real stemmer would join these and would also join `mediate` with
    // `media`, which in this corpus are two subjects. That trade was made
    // deliberately: the mitigation is that a question expands through the
    // registry's own `keywords`, and a row about configuration carries both
    // spellings because a person wrote them there.
    assert_ne!(terms::stem("configuration"), terms::stem("configurable"));
    assert_ne!(terms::stem("verify"), terms::stem("verification"));
    // What it does hold: everything one cut apart.
    assert_eq!(terms::stem("verification"), terms::stem("verificat"));
    assert_eq!(terms::stem("sentinels"), terms::stem("sentinel"));
}

#[test]
fn the_same_stem_reached_twice_does_not_double_its_weight() {
    // A keyword list repeating the asked word must not outweigh the question.
    let mut q = Query::new("mediator");
    let before = q.terms.len();
    q.push("mediators".to_string(), Source::Keyword);
    q.push("mediation".to_string(), Source::Keyword);
    assert_eq!(q.terms.len(), before, "all three share a stem");
    assert_eq!(
        q.terms[0].source,
        Source::Word,
        "and the stronger provenance survives"
    );
}

// --- what a caller must be able to disbelieve -----------------------------------

#[test]
fn an_empty_corpus_is_reported_rather_than_read_as_no_hits() {
    let d = Planted::new("empty");
    let corpus = Corpus::load(d.mock());
    assert_eq!(corpus.len(), 0);
    let out = rank(&corpus, &Index::build(&corpus), &Query::new("anything"), 20);
    assert!(out.hits.is_empty());
    assert_eq!(
        out.searched, 0,
        "the caller needs this to say the result is void"
    );
}

#[test]
fn a_missing_root_is_named() {
    let d = Planted::new("missing");
    d.file("panel/01.md", "text\n");
    let corpus = Corpus::load(d.mock());
    assert!(
        corpus.missing.iter().any(|m| m == "design_rounds"),
        "an absent root must be reported: {:?}",
        corpus.missing
    );
}

// --- a tool holding a corpus can say what it did not read, in one call ---------

// --- asking for the phrase itself, not for any word of it -----------------------

#[test]
fn a_phrase_only_question_does_not_match_a_document_carrying_one_word_of_it() {
    // Found by hand while checking `already-said` after an extraction:
    // `already-said quaternion-nonsense-zz` reported "the research carries it,
    // in 306 of 4019 documents" over a corpus containing that string zero times.
    // `Query::new` splits the phrase and searches every word, and `zz` alone is
    // in 596 files. The registry half of that tool substring-matches the whole
    // term, so the two halves were answering different questions under one
    // sentence saying the term is carried.
    let d = Planted::new("phrase-only");
    d.file("panel/01_boot.md", "the boot sequence is measured\n")
        .file("panel/02_chain.md", "a chain of trust is verified\n")
        .file(
            "panel/03_both.md",
            "the boot chain is measured end to end\n",
        );

    let corpus = Corpus::load(d.mock());
    let q = Query::phrase_only("boot chain");
    assert_eq!(q.terms.len(), 1, "one term, which is the phrase as typed");

    let hits: Vec<String> = rank(&corpus, &Index::build(&corpus), &q, 20)
        .hits
        .into_iter()
        .map(|h| h.path)
        .collect();
    assert_eq!(hits, vec!["research/panel/03_both.md".to_string()]);
}

#[test]
fn a_phrase_only_question_of_one_word_still_finds_its_other_forms() {
    // The control, and it is what stops the fix being "stop searching". A single
    // word is still stemmed, because `already-said convey` finding
    // `conveyance` is the behaviour its own help promises and the reason the
    // stemmer exists.
    let d = Planted::new("phrase-only-single");
    d.file("panel/01.md", "Conveyance obliges corresponding source.\n");

    let corpus = Corpus::load(d.mock());
    let out = rank(
        &corpus,
        &Index::build(&corpus),
        &Query::phrase_only("convey"),
        20,
    );
    assert_eq!(out.hits.len(), 1, "the stem still reaches the other form");
}

#[test]
fn a_phrase_only_question_of_only_common_words_is_still_unasked() {
    // Caught by running the tool rather than by any test here, which is why this
    // one exists. `push` does not consult the stop list, so the first version of
    // `phrase_only` searched for `the` and reported the research carrying it in
    // 3430 of 4019 documents. That is the unasked-question defect coming back
    // through a second door, three commits after it was closed.
    let d = Planted::new("phrase-only-stop");
    d.file("panel/01.md", "the boot sequence is measured\n");

    let corpus = Corpus::load(d.mock());
    let q = Query::phrase_only("the");
    assert!(q.terms.is_empty(), "nothing searchable was asked");

    let out = rank(&corpus, &Index::build(&corpus), &q, 20);
    assert_eq!(out.searched, 0, "so no document was read");
    assert!(out.dropped.contains(&"the".to_string()));
}
