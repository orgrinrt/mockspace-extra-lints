use super::*;

// --- ranking behaviour ----------------------------------------------------------

#[test]
fn a_heading_outranks_a_mention_in_passing() {
    let d = Planted::new("heading");
    d.file(
        "panel/01_a.md",
        "## The mediation shape\n\nsomething else entirely\n",
    )
    .file(
        "panel/01_b.md",
        "## Fonts\n\nthe renderer is unrelated to mediation here\n",
    );

    let hits = search(&d, "mediation", &[]);
    assert_eq!(
        hits.first().map(String::as_str),
        Some("research/panel/01_a.md")
    );
}

#[test]
fn op_outranks_a_note_and_a_note_outranks_a_sketch_saying_the_same_thing() {
    let d = Planted::new("kind");
    let body = "the realm directs what its scholars study\n";
    d.file("202610031300_minds/NOTES.md", body)
        .file("sketches/scholars.rs", body);
    let op = d.mock().join("words");
    std::fs::create_dir_all(&op).unwrap();
    std::fs::write(op.join("202610040900_op-on-research.md"), body).unwrap();

    let corpus = Corpus::load_with(d.mock(), Some(&op));
    let hits: Vec<String> = rank(
        &corpus,
        &Index::build(&corpus),
        &Query::new("realm directs scholars"),
        5,
    )
    .hits
    .into_iter()
    .map(|h| h.path)
    .collect();
    assert_eq!(
        hits,
        vec![
            "op-responses/202610040900_op-on-research.md".to_string(),
            "research/202610031300_minds/NOTES.md".to_string(),
            "research/sketches/scholars.rs".to_string(),
        ],
        "op's words are where a settled answer lives, and a sketch the weakest"
    );
}

#[test]
fn covering_more_of_the_question_beats_repeating_one_word_of_it() {
    let d = Planted::new("coverage");
    d.file(
        "panel/01_narrow.md",
        &"sentinel sentinel sentinel sentinel sentinel sentinel sentinel\n".repeat(6),
    )
    .file(
        "panel/02_broad.md",
        "a sentinel guards the vault and the extension data\n",
    );

    let hits = search(&d, "sentinel vault extension data", &[]);
    assert_eq!(
        hits.first().map(String::as_str),
        Some("research/panel/02_broad.md"),
        "one word forty times is not the subject"
    );
}
