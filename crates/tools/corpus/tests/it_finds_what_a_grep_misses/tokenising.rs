use super::*;

// --- a term the tokeniser would split is still one term ------------------------

/// Returns the terms the search gave up on, so a test can assert a term was
/// actually asked rather than merely absent from the hits.
fn dead(dir: &Planted, phrase: &str, keywords: &[&str]) -> Vec<String> {
    let corpus = Corpus::load(dir.mock());
    let mut q = Query::new(phrase);
    for k in keywords {
        q.push((*k).to_string(), Source::Keyword);
    }
    rank(&corpus, &Index::build(&corpus), &q, 20).dead
}

#[test]
fn a_hyphenated_keyword_is_found_rather_than_reported_dead() {
    // The index is keyed on whole tokens and `words` splits on the hyphen, so
    // no key can ever equal `dm-verity`. Routing such a term to the index makes
    // the corpus's own vocabulary unsearchable, and the tool then reports the
    // term dead, which its verdict reads as evidence of a real absence. Every
    // compound name this project actually argues about has this shape.
    let d = Planted::new("hyphen");
    d.file(
        "panel/01_boot.md",
        "the root filesystem is sealed and dm-verity checks every block on read\n",
    );

    assert!(
        !dead(&d, "sealed root", &["dm-verity"]).contains(&"dm-verity".to_string()),
        "a hyphenated keyword the corpus contains was reported dead"
    );
    assert_eq!(search(&d, "sealed root", &["dm-verity"]), vec![
        "research/panel/01_boot.md".to_string()
    ]);
}

#[test]
fn a_dotted_keyword_is_found_rather_than_reported_dead() {
    // Same mechanism, different separator, and this one is how a question about
    // a manifest gets answered with silence.
    let d = Planted::new("dotted");
    d.file(
        "panel/01_manifest.md",
        "every crate declares its licence in cargo.toml\n",
    );

    assert!(!dead(&d, "declared licence", &["cargo.toml"]).contains(&"cargo.toml".to_string()));
    assert_eq!(search(&d, "declared licence", &["cargo.toml"]), vec![
        "research/panel/01_manifest.md".to_string()
    ]);
}

#[test]
fn a_compound_term_the_corpus_lacks_is_still_reported_dead() {
    // The negative control, and without it the two above are satisfied by a fix
    // that simply stops calling anything dead. A compound absent from the corpus
    // must still come back dead, because that is the signal the verdict weighs.
    let d = Planted::new("hyphen-absent");
    d.file(
        "panel/01_boot.md",
        "the root filesystem is sealed and checked on read\n",
    );

    assert!(dead(&d, "sealed root", &["dm-verity"]).contains(&"dm-verity".to_string()));
}

#[test]
fn a_compound_term_does_not_match_inside_a_longer_word() {
    // The boundary the literal scan must keep. `dm-verity` occurs nowhere here;
    // a scan that ignores the leading boundary would find it inside `xdm-verity`
    // and report a hit the corpus does not contain.
    let d = Planted::new("hyphen-boundary");
    d.file(
        "panel/01_boot.md",
        "the vendor ships xdm-verity, which is a different thing\n",
    );

    assert!(dead(&d, "sealed root", &["dm-verity"]).contains(&"dm-verity".to_string()));
}

// --- a question that could not be asked is not an absence -----------------------

#[test]
fn a_question_of_only_common_words_reports_nothing_searched() {
    // Every word here is on the stop list, so the question expands to no terms
    // at all and `rank` returns before reading a single document. It used to
    // report the whole corpus as searched, which is the number a caller weighs
    // to decide whether an empty result means anything, so the tool answered a
    // question it had never asked and called the silence a real absence.
    let d = Planted::new("all-stop-words");
    d.file(
        "panel/01_boot.md",
        "the root filesystem is sealed and checked on every read\n",
    );

    let out = run(&d, "what does it do with all of this");
    assert!(out.hits.is_empty());
    assert_eq!(
        out.searched, 0,
        "no document was consulted, and the caller reads this to know the result is void"
    );
    assert!(
        out.dead.is_empty(),
        "a term that was never searched did not match nothing anywhere; it does not exist"
    );
}

#[test]
fn an_unasked_question_names_the_words_it_dropped() {
    // The dropped words are the whole of what a reader can act on: they say the
    // question was thrown away rather than answered, and which words to replace.
    let d = Planted::new("dropped-named");
    d.file("panel/01_boot.md", "the root filesystem is sealed\n");

    let out = run(&d, "what does it do");
    assert!(out.dropped.contains(&"what".to_string()));
    assert!(out.dropped.contains(&"does".to_string()));
    assert_eq!(
        out.dropped,
        vec!["what", "does", "it", "do"],
        "once each, in the order asked"
    );
}

#[test]
fn a_word_dropped_twice_is_named_once() {
    // `do` occurs twice in this question and naming it twice reads as a defect
    // in the report rather than as a fact about the question. Nothing counts
    // these, so the set is what a reader wants.
    let d = Planted::new("dropped-twice");
    d.file("panel/01_boot.md", "the root filesystem is sealed\n");

    let out = run(&d, "how do we do this");
    assert_eq!(out.dropped, vec!["how", "do", "this"]);
}
