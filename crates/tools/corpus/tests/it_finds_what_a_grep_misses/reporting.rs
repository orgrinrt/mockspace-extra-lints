use super::*;

// --- what the corpus report says it holds ---------------------------------------

#[test]
fn words_are_counted_rather_than_guessed_from_bytes() {
    // `mock corpus` printed `bytes / 6000` as thousands of words, which is a
    // guess at bytes per word and was 30% high: this corpus runs at 7.8 bytes
    // per word, so 5.0 million words were reported as 6.5 million. The count is
    // cheap and exact, and a report about what is there should not estimate it.
    let d = Planted::new("word-count");
    d.file("panel/01.md", "one two three four five\n")
        .file("panel/02.md", "  six   seven\t eight  \n\n nine \n");

    assert_eq!(Corpus::load(d.mock()).words(), 9);
}

#[test]
fn punctuation_does_not_inflate_the_word_count() {
    // The control. Splitting on whitespace alone counts `a, b` as two and
    // `(a)` as one, which is what a reader means by a word; splitting on
    // every non-alphanumeric would count differently and the number would
    // stop matching what `wc -w` says about the same tree.
    let d = Planted::new("word-count-punct");
    d.file(
        "panel/01.md",
        "# a heading, with punctuation.\n\n> quoted (aside)\n",
    );

    assert_eq!(Corpus::load(d.mock()).words(), 8);
}

#[test]
fn a_corpus_missing_a_root_offers_a_sentence_naming_it() {
    // Three of the five tools that load a corpus never mentioned a missing root,
    // and each of them reported a document count that silently excluded two
    // whole trees. They forgot because every one of them had to write the
    // sentence itself, so the sentence lives here now and a tool prints it.
    let d = Planted::new("caveat-missing");
    d.file("panel/01.md", "text\n");

    let caveat = Corpus::load(d.mock()).caveat().expect("a root is absent");
    assert!(caveat.contains("design_rounds"), "{caveat}");
}

#[test]
fn a_corpus_with_every_root_present_offers_nothing() {
    // The control. A caveat that always says something is a line every reader
    // learns to skip, which is how the real one gets missed.
    let d = Planted::new("caveat-clean");
    d.file("panel/01.md", "text\n");
    for root in ["design_rounds", "words"] {
        std::fs::create_dir_all(d.mock().join(root)).unwrap();
        std::fs::write(d.mock().join(root).join("01.md"), "text\n").unwrap();
    }

    assert!(
        Corpus::load_with(d.mock(), Some(&d.mock().join("words")))
            .caveat()
            .is_none(),
        "nothing was missed, so nothing is reported as missed"
    );
}

#[test]
fn a_long_document_does_not_win_on_length_alone() {
    // The defect a real question exposed. A slot asking about getting a secret
    // wrong a few times was handed a display-pipeline document, because the
    // words are ordinary and a hundred-thousand-word file contains most
    // ordinary words. Capping the contributing terms did not fix it and
    // flooring the coverage did not either, because the coverage was genuinely
    // high: a long document does carry more of any question, and that is length
    // rather than relevance.
    // The long document mentions each word once, in passing, spread across a
    // lot of unrelated text. That is the shape of the real case: a
    // display-pipeline document is not about entering a secret wrongly and says
    // none of those words more than once, and it won anyway because there was
    // enough of it.
    //
    // **What this does not claim** is that a short document beats one saying the
    // words four hundred times. Nothing sane does: a file repeating a subject
    // that often is about it, and an earlier version of this test asserted
    // otherwise and was wrong rather than unlucky.
    let d = Planted::new("length");
    let mut sprawl = String::from("## The display pipeline\n\n");
    for i in 0 .. 300 {
        sprawl.push_str(&format!(
            "paragraph {i} about buffers, scanout, vblank, planes and compositing\n"
        ));
    }
    sprawl.push_str("a secret is held somewhere, and it goes wrong a few times\n");
    for i in 0 .. 300 {
        sprawl.push_str(&format!(
            "paragraph {i} about colour, gamma and dithering\n"
        ));
    }

    d.file("panel/01_sprawl.md", &sprawl)
        .file(
            "panel/02_the-answer.md",
            "## The secret entered wrong\n\nA few wrong tries and it waits.\n",
        )
        .file("panel/03_filler.md", "unrelated text about fonts\n");

    let hits = search(&d, "secret wrong few", &[]);
    assert_eq!(
        hits.first().map(String::as_str),
        Some("research/panel/02_the-answer.md"),
        "the short document about the subject must beat the long one that merely contains \
         its words: {hits:?}"
    );
}

#[test]
fn but_a_long_document_that_is_genuinely_the_answer_still_wins() {
    // The other half, and the reason the discount is partial. Dividing length
    // out entirely would bury the long consolidations, which in this corpus are
    // very often where the answer is.
    let d = Planted::new("longanswer");
    let mut consolidation = String::from("## The mediation shape and its two surfaces\n\n");
    for i in 0 .. 300 {
        consolidation.push_str(&format!(
            "paragraph {i}: the broker mediates both store surfaces and nothing else \
             reaches either one\n"
        ));
    }
    d.file("panel/01_CONSOLIDATION.md", &consolidation)
        .file("panel/02_passing.md", "the broker is mentioned once here\n");

    let hits = search(&d, "broker mediates store surfaces", &[]);
    assert_eq!(
        hits.first().map(String::as_str),
        Some("research/panel/01_CONSOLIDATION.md")
    );
}
