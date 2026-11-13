use super::*;

#[test]
fn an_unasked_question_is_not_an_absence() {
    // The defect, and it is the pair that has to stay apart. `already-said
    // the` reported 631 of 639 rows carrying it and then nothing in 3995
    // documents, about a word in every one of them, because the corpus half
    // expands into words and `the` expands into none.
    let unasked = research_line("the", 4000, 0, 0, 0, 0);
    assert!(unasked.contains("was not asked"), "{unasked}");
    assert!(
        !unasked.contains("Nothing in the"),
        "an unasked question must not read as a searched one: {unasked}"
    );

    let absent = research_line("quaternion", 4000, 1, 0, 4000, 0);
    assert!(absent.contains("Nothing in the 4000"), "{absent}");
    assert!(
        !absent.contains("was not asked"),
        "a real absence is the answer that licenses asking a person: {absent}"
    );
}

#[test]
fn the_count_is_what_carries_the_term_rather_than_what_is_listed() {
    // This read "in at least 3 of 3970 documents" for every term the corpus
    // holds, because 3 is the limit the search was given, so it said the
    // same thing about `boot` at 1310 files as about a word in four.
    let line = research_line("boot", 4014, 1, 1310, 4014, 3);
    assert!(line.contains("1310 of 4014"), "{line}");
    assert!(
        line.contains("nearest 3"),
        "and says three are listed: {line}"
    );
    assert!(
        !line.contains("at least"),
        "the hedge was the limit wearing a measurement: {line}"
    );
}

#[test]
fn an_unreadable_corpus_claims_nothing_either_way() {
    // The control on the controls. A corpus of zero documents is not a
    // corpus that answered, and the other three sentences all assert
    // something about what the research holds.
    let line = research_line("anything", 0, 1, 0, 0, 0);
    assert!(line.contains("could not be read"), "{line}");
    assert!(!line.contains("Nothing in the"), "{line}");
    assert!(!line.contains("was not asked"), "{line}");
}

#[test]
fn a_form_of_the_word_reaches_the_row_that_uses_another_form() {
    // `already-said conveyed` returned `0 of 640 rows`, and this tool's help
    // says an empty result licenses asking a person. `unearth conveyed`
    // named five rows at the same moment, one of them a ruling whose `says`
    // is entirely about conveyance. A substring test cannot get from
    // `conveyed` to `conveyance`; the stem the corpus half already used can.
    let fields = [(
        "says",
        "Conveyance is the prebuilt image and the builder is the main case",
    )];
    assert_eq!(hits_in_row("row", &fields, "conveyed"), vec![
        "says".to_string()
    ]);
}

#[test]
fn a_word_inside_a_longer_word_is_not_a_hit() {
    // The control, and it is the half of the boundary that holds. A stem
    // starts at a word boundary, so `key` does not reach `monkey` or
    // `donkey`, which is what a bare substring test did and what made 29 of
    // the 73 rows it returned for `key` unrelated to keys.
    let fields = [("says", "a monkey and a donkey and some turkey")];
    assert_eq!(
        hits_in_row("row", &fields, "key"),
        Vec::<String>::new(),
        "none of these begins with `key`"
    );
}

#[test]
fn a_stem_still_runs_forward_as_a_prefix() {
    // The other half, and it is deliberate rather than a residue: the stem
    // matches at a boundary and then runs forward, which is the whole reason
    // `conveyed` reaches `conveyance` in the test above. So `key` does reach
    // `keywords`, and both halves of this tool now agree about that where
    // they used to disagree in both directions.
    let fields = [("keywords", "keywords, keyring, keymap")];
    assert_eq!(hits_in_row("row", &fields, "key"), vec![
        "keywords".to_string()
    ]);
}

#[test]
fn a_term_in_a_field_names_that_field() {
    let fields = [
        ("what", "The image is CONVEYED to a third party"),
        ("because", "the licence says so"),
    ];
    assert_eq!(hits_in_row("some_row", &fields, "convey"), vec![
        "what".to_string()
    ]);
}

#[test]
fn the_slug_counts_as_a_field_with_its_underscores_read_as_spaces() {
    // `slot::the_passphrase_is_gone` is a row about a passphrase being gone,
    // and somebody asking in those words has to find it. Without the
    // substitution the slug matches only if they type the underscores.
    let fields = [("use", "unrelated prose")];
    assert_eq!(
        hits_in_row("the_passphrase_is_gone", &fields, "passphrase is gone"),
        vec!["id".to_string()]
    );
}

#[test]
fn a_citation_field_is_not_a_mention() {
    // The exclusion this tool has always had, now pinned. A `provenance`
    // entry is `root::path::anchor` and names the file a row came from, so a
    // hit there says a document was cited rather than that a subject was
    // discussed. The same term in a prose field on the same row must still
    // land, or the test passes for the wrong reason.
    let fields = [
        (
            "provenance",
            "panel::202608_the_licensing_panel::#conveyance",
        ),
        ("cites", "licensing_is_settled"),
        ("related", "tenet::licensing_is_related"),
        ("what", "conveyance obliges corresponding source"),
    ];
    assert_eq!(
        hits_in_row("row", &fields, "licensing"),
        Vec::<String>::new(),
        "the term is only in the citation fields, so nothing discussed it"
    );
    assert_eq!(hits_in_row("row", &fields, "conveyance"), vec![
        "what".to_string()
    ]);
}

#[test]
fn the_declared_id_field_does_not_double_the_slug() {
    // The slug is already reported as `id`, so a row also declaring one
    // would report the same hit twice under the same name.
    let fields = [("id", "the_passphrase_is_gone"), ("use", "prose")];
    assert_eq!(
        hits_in_row("the_passphrase_is_gone", &fields, "passphrase"),
        vec!["id".to_string()]
    );
}

#[test]
fn a_row_carrying_the_term_nowhere_reports_nothing() {
    // The control. Without it every test above is satisfied by a function
    // that returns every field it is given, and this tool's empty answer is
    // exactly what licenses interrupting a person with a question.
    let fields = [("what", "the compositor consumes actions"), ("because", "it must")];
    assert_eq!(
        hits_in_row("a_slot_about_input", &fields, "licensing"),
        Vec::<String>::new()
    );
}

#[test]
fn every_field_carrying_it_is_named_rather_than_the_first() {
    // A reader deciding whether the row really covers the subject wants to
    // know it appears in the answer as well as in the keywords, and a
    // function stopping at the first hit cannot say so.
    //
    // The order here is the order this slice is written in, which is not
    // what a person sees: `run` hands over a row, and a row is a map, so the
    // fields arrive by name. `a_hit_names_the_row_and_every_field_of_it`
    // pins the order that actually reaches a terminal.
    let fields = [
        ("what", "conveyance obliges source"),
        ("keywords", "convey, conveyance, third party"),
    ];
    assert_eq!(hits_in_row("row", &fields, "convey"), vec![
        "what".to_string(),
        "keywords".to_string()
    ]);
}

// -----------------------------------------------------------------------
// `run` itself. Everything above tests the two functions underneath it, and
// nothing exercised what a person invokes: what the argument becomes, which
// answer an empty registry gets, and whether the research half is wired to
// the sentence that reports it. Every field of `ToolContext` is public and
// `RegistryView::new` is too, so this was never a matter of not being able
// to.
// -----------------------------------------------------------------------
