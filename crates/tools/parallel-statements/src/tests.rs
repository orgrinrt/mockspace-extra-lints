use super::*;

fn row(id: &str, topic: &str, words: &[&str]) -> (String, String, BTreeSet<String>) {
    (
        id.to_string(),
        topic.to_string(),
        words.iter().map(|w| (*w).to_string()).collect(),
    )
}

#[test]
fn a_ruling_and_a_proposal_about_one_topic_are_compared() {
    // The defect. The comparison sat inside the namespace loop, so these two
    // never met, and the real pair it could not see is a ratified ruling
    // against a proposal at 82% where nothing in its reach exceeded 61%.
    let rows = vec![
        row("ruling::x", "identity", &[
            "authority",
            "extensions",
            "native",
            "custom",
        ]),
        row("proposal::y", "identity", &[
            "authority",
            "extensions",
            "native",
            "build",
        ]),
    ];

    let pairs = same_group_pairs(&rows);
    assert_eq!(pairs.len(), 1, "one pair, across the namespace boundary");
    assert_eq!(pairs[0].2, 75, "three of the smaller four");
}

#[test]
fn rows_about_different_topics_are_not_compared() {
    // The control, and it is the reason the boundary was there at all:
    // comparing across topics returns the corpus's house vocabulary. Topic
    // is what does that work, not the namespace.
    let rows = vec![
        row("ruling::x", "identity", &[
            "authority",
            "extensions",
            "native",
            "custom",
        ]),
        row("proposal::y", "licensing", &[
            "authority",
            "extensions",
            "native",
            "custom",
        ]),
    ];

    assert!(
        same_group_pairs(&rows).is_empty(),
        "identical words, different subjects, and no pair"
    );
}

#[test]
fn a_short_statement_clears_the_threshold_on_two_shared_words() {
    // The reason every hit prints its denominator. At the pool floor, two
    // words in common is 50% and clears a threshold of 40, so a pair can be
    // reported on the strength of two ordinary words. The measure is not
    // wrong; a reader told `50%` and not told `2 of 4` cannot see it.
    let rows = vec![
        row("slot::a", "t", &["getting", "onto", "paper", "document"]),
        row("slot::b", "t", &["getting", "onto", "screen", "pixels"]),
    ];
    let p = same_group_pairs(&rows);
    assert_eq!(p[0].2, 50);
    assert!(p[0].2 >= THRESHOLD, "and it is reported");

    // The control, and the first version of it did not build the case its
    // own comment described: it made two identical twenty-word sets, called
    // them a dilution, and asserted 100%. It could not have failed.
    //
    // Two long statements sharing the same two words is 2 of 20 and is not
    // reported. **Short against long would not have shown this either**,
    // because the share is of the smaller statement, so four words sharing
    // two with a twenty-word row is still 50% and still reported. Both
    // sides have to be long for the denominator to bite, which is the whole
    // reason the denominator is worth printing.
    let a: Vec<String> = (0 .. 20).map(|i| format!("alpha{i}")).collect();
    let b: Vec<String> = (0 .. 20).map(|i| format!("beta{i}")).collect();
    let mut a: Vec<&str> = a.iter().map(String::as_str).collect();
    let mut b: Vec<&str> = b.iter().map(String::as_str).collect();
    a[0] = "getting";
    a[1] = "onto";
    b[0] = "getting";
    b[1] = "onto";
    let rows = vec![row("slot::a", "t", &a), row("slot::b", "t", &b)];
    let share = same_group_pairs(&rows)[0].2;
    assert_eq!(share, 10, "two shared of twenty");
    assert!(share < THRESHOLD, "and it is not reported");
}

#[test]
fn the_pool_floor_is_below_where_the_measure_gets_coarse() {
    // Stated as a relation rather than as two numbers, so the gap cannot
    // close silently if either moves. The band between them is unmeasured
    // and the report counts what falls in it rather than filtering it.
    assert!(POOL_FLOOR < SOFT_DENOMINATOR);
    assert!(
        POOL_FLOOR >= 2,
        "a denominator of one makes every share 0 or 100"
    );
}

#[test]
fn distinctive_drops_the_house_vocabulary_and_the_short_words() {
    let w = distinctive("The Kaski player SHOULD never grant the same raw access");
    // Case is folded, and `kaski`, `same` and `never` are on the house list
    // for the reason the list states: they occur in many statements here and
    // say nothing about which statement it is. `raw` goes for being three
    // letters.
    for gone in ["the", "never", "kaski", "same", "raw", "a"] {
        assert!(!w.contains(gone), "{gone} should not be distinctive");
    }
    for kept in ["should", "grant", "access", "player"] {
        assert!(w.contains(kept), "{kept} should be");
    }
}

#[test]
fn distinctive_counts_a_repeated_word_once() {
    // It is a set, and the share is over set sizes, so a statement repeating
    // one word does not inflate its own denominator.
    assert_eq!(distinctive("grant grant grant access").len(), 2);
}

#[test]
fn the_share_is_of_the_smaller_statement() {
    // What the number means, pinned. A long row restating a short one in
    // full is 100%, because every distinctive word of the short one is
    // present, and reporting it against the longer would hide exactly that.
    let rows = vec![
        row("ruling::short", "t", &["alpha", "beta", "gamma", "delta"]),
        row("proposal::long", "t", &[
            "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta",
        ]),
    ];

    assert_eq!(same_group_pairs(&rows)[0].2, 100);
}

// -----------------------------------------------------------------------
// `run` itself. Everything above tests the two functions underneath it, and
// nothing tested the thing a person invokes: which field a statement is
// read from, what the report says it did not compare, and what the argument
// narrows. Every field of `ToolContext` is public and `RegistryView::new`
// is too, so this was never a matter of not being able to.
// -----------------------------------------------------------------------

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mockspace::{RegistryView, RowFields};

/// A registry from rows given as `("ns::slug", &[("field", "value")])`.
///
/// No reverse edges, which this tool never asks for. A tool that did would
/// get a view answering empty for every row, indistinguishable from a
/// project whose rows reference nothing, and would need them passed.
fn registry(rows: &[(&str, &[(&str, &str)])]) -> RegistryView {
    let mut map: BTreeMap<String, RowFields> = BTreeMap::new();
    for (q, fields) in rows {
        let mut f = RowFields::new();
        for (k, v) in *fields {
            f.insert((*k).to_string(), (*v).to_string());
        }
        map.insert((*q).to_string(), f);
    }
    RegistryView::new(map, BTreeMap::new())
}

fn run(reg: &RegistryView, args: &[&str]) -> ToolReport {
    let crates = BTreeSet::new();
    let dirs: Vec<PathBuf> = Vec::new();
    ParallelStatements.run(&ToolContext {
        mock_dir: Path::new("/mock"),
        repo_root: Path::new("/"),
        all_crates: &crates,
        src_dirs: &dirs,
        args,
        stdin: None,
        registry: reg,
    })
}

/// Four distinctive words, which is the floor to enter the pool.
const FOUR: &str = "alpha beta gamma delta";

#[test]
fn a_statement_is_read_from_says_before_does() {
    // The field list is ordered and the order is load-bearing: a row
    // carrying both must be compared on what it says rather than on what it
    // does. If the order flipped, these two would share nothing and no pair
    // would form at all.
    let reg = registry(&[
        ("ruling::a", &[
            ("subject", "t"),
            ("says", FOUR),
            ("does", "kappa lambda sigma omega"),
        ]),
        ("proposal::b", &[("subject", "t"), ("says", FOUR)]),
    ]);
    let out = run(&reg, &[]).output;
    assert!(
        out.contains("100%"),
        "read from `says`, so identical: {out}"
    );
    assert!(
        out.contains("alpha"),
        "and the shared words are the ones said"
    );
    assert!(
        !out.contains("kappa"),
        "`does` was not what it was compared on"
    );
}

#[test]
fn a_row_that_says_it_only_in_does_or_in_use_is_still_compared() {
    // The control for the test above, and the reason the list has four
    // entries rather than two. A `mechanism` states itself in `does`, the
    // field op ordered added, and a `slot` and a `role` state it in `use`.
    // Reading only `says` and `what` put 262 real rows in the silent bucket.
    for field in ["does", "use", "what"] {
        let reg = registry(&[
            ("mechanism::a", &[("subject", "t"), (field, FOUR)]),
            ("slot::b", &[("subject", "t"), ("use", FOUR)]),
        ]);
        let out = run(&reg, &[]).output;
        assert!(out.contains("100%"), "`{field}` is a statement: {out}");
        assert!(
            out.contains("0 rows state nothing at all"),
            "and neither row is silent: {out}"
        );
    }
}

#[test]
fn the_report_accounts_for_every_row_it_did_not_compare() {
    // The law: a row carrying a statement is either in the pool or in one of
    // the named buckets. Three ways out, and the third had no counter and no
    // line, so a row dropped for being under the floor was reported nowhere
    // while the paragraph above it called the other two "the corner".
    let reg = registry(&[
        ("ruling::a", &[("subject", "t"), ("says", FOUR)]),
        ("proposal::b", &[("subject", "t"), ("says", FOUR)]),
        ("ruling::untopiced", &[("says", FOUR)]),
        ("ruling::silent", &[("subject", "t"), ("keywords", "x")]),
        ("ruling::short", &[
            ("subject", "t"),
            ("says", "alpha beta gamma"),
        ]),
    ]);
    let out = run(&reg, &[]).output;
    assert!(
        out.contains("1 rows state something and carry no subject"),
        "{out}"
    );
    assert!(out.contains("1 rows state nothing at all"), "{out}");
    assert!(
        out.contains("1 say something in fewer than 4"),
        "the third way out is named and counted: {out}"
    );
}

#[test]
fn the_third_bucket_reports_zero_rather_than_going_quiet() {
    // The control. A count printed only when it is non-zero cannot be read
    // as an accounting, because its absence and a zero look the same.
    let reg = registry(&[
        ("ruling::a", &[("subject", "t"), ("says", FOUR)]),
        ("proposal::b", &[("subject", "t"), ("says", FOUR)]),
    ]);
    let out = run(&reg, &[]).output;
    assert!(out.contains("0 say something in fewer than 4"), "{out}");
}

#[test]
fn naming_a_subject_without_a_pair_in_it_is_inconclusive() {
    let rows: &[(&str, &[(&str, &str)])] = &[
        ("ruling::a", &[("subject", "t"), ("says", FOUR)]),
        ("proposal::b", &[("subject", "t"), ("says", FOUR)]),
        ("tenet::c", &[("subject", "u"), ("what", FOUR)]),
    ];
    let reg = registry(rows);

    // Across every namespace, which is the default and the fix that made
    // kamu's copy see a ratified ruling restating the proposal it came from.
    assert!(run(&reg, &[]).output.contains("100%"));

    // Narrowed to a subject holding one row, there is no pair at all and the
    // report says that rather than reporting zero hits.
    let one = run(&reg, &["u"]);
    assert!(
        matches!(one.outcome, mockspace::tool::Outcome::Inconclusive { .. }),
        "one row cannot pair with itself"
    );
}

#[test]
fn nothing_to_compare_is_inconclusive_rather_than_clean() {
    // A clean verdict over an empty population is the vacuous pass this
    // whole review has been removing. Two rows that share no topic are not
    // evidence that the canon does not repeat itself.
    let reg = registry(&[
        ("ruling::a", &[("subject", "one"), ("says", FOUR)]),
        ("proposal::b", &[("subject", "two"), ("says", FOUR)]),
    ]);
    assert!(matches!(
        run(&reg, &[]).outcome,
        mockspace::tool::Outcome::Inconclusive { .. }
    ));
}

// -----------------------------------------------------------------------
// kaski's shape: tenets grouped by `subject`, superseded tenets no longer
// binding, and a pair split across two subjects by a first binning.
// -----------------------------------------------------------------------

/// One rule, written twice in different words and order.
const RULE_A: &str = "A record helps a culture reach understanding along the advancement tracks \
                      and quickens teaching skills to individuals.";
const RULE_B: &str = "Teaching individuals skills is quickened by a record, which helps the \
                      culture's understanding move along advancement tracks.";
const OTHER: &str = "Weather fronts carry moisture across the planet's oceans toward mountains.";

#[test]
fn two_tenets_of_one_subject_stating_one_rule_are_found() {
    let reg = registry(&[
        ("tenet::records_aid_understanding", &[
            ("subject", "knowledge"),
            ("what", RULE_A),
        ]),
        ("tenet::teaching_is_quickened_by_records", &[
            ("subject", "knowledge"),
            ("what", RULE_B),
        ]),
        ("tenet::fronts_carry_moisture", &[
            ("subject", "knowledge"),
            ("what", OTHER),
        ]),
    ]);
    let out = run(&reg, &[]).output;
    assert!(
        out.contains("tenet::records_aid_understanding\ntenet::teaching_is_quickened_by_records\n"),
        "{out}"
    );
    assert!(!out.contains("fronts_carry_moisture\n"), "{out}");
}

#[test]
fn a_pair_split_across_subjects_is_found_only_across() {
    // The subjects are a first binning, so one rule written into two of them is
    // exactly the pair a per-subject comparison cannot see.
    let reg = registry(&[
        ("tenet::records_aid_understanding", &[
            ("subject", "knowledge"),
            ("what", RULE_A),
        ]),
        ("tenet::teaching_is_quickened_by_records", &[
            ("subject", "culture_and_faith"),
            ("what", RULE_B),
        ]),
        ("tenet::fronts_carry_moisture", &[
            ("subject", "weather"),
            ("what", OTHER),
        ]),
    ]);
    let within = run(&reg, &[]);
    assert!(
        !within.output.contains("records_aid_understanding\n"),
        "{}",
        within.output
    );
    let across = run(&reg, &["across"]).output;
    assert!(
        across.contains(
            "tenet::records_aid_understanding\ntenet::teaching_is_quickened_by_records\n"
        ),
        "{across}"
    );
}

#[test]
fn a_superseded_tenet_is_not_compared_with_what_replaced_it() {
    // A replacement keeps the intent of what it supersedes and much of its
    // wording, by design. Reported, every supersession is a false pair.
    let reg = registry(&[
        ("tenet::records_aid_understanding", &[
            ("subject", "knowledge"),
            ("what", RULE_A),
        ]),
        ("tenet::records_aid_understanding_once", &[
            ("subject", "knowledge"),
            ("what", RULE_B),
            ("superseded_by", "records_aid_understanding"),
        ]),
        ("tenet::fronts_carry_moisture", &[
            ("subject", "knowledge"),
            ("what", OTHER),
        ]),
    ]);
    let out = run(&reg, &[]).output;
    assert!(!out.contains("records_aid_understanding_once\n"), "{out}");
    assert!(out.contains("1 superseded"), "{out}");
}

#[test]
fn a_subject_named_narrows_the_pool_to_it() {
    let reg = registry(&[
        ("tenet::records_aid_understanding", &[
            ("subject", "knowledge"),
            ("what", RULE_A),
        ]),
        ("tenet::teaching_is_quickened_by_records", &[
            ("subject", "knowledge"),
            ("what", RULE_B),
        ]),
        ("tenet::fronts_carry_moisture", &[
            ("subject", "weather"),
            ("what", OTHER),
        ]),
        ("tenet::fronts_move_moisture", &[
            ("subject", "weather"),
            ("what", OTHER),
        ]),
    ]);
    let out = run(&reg, &["weather"]).output;
    assert!(
        out.contains("tenet::fronts_carry_moisture\ntenet::fronts_move_moisture\n"),
        "{out}"
    );
    assert!(!out.contains("records_aid_understanding\n"), "{out}");
}
