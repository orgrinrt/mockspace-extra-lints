use super::*;
use crate::canon_rows::testkit::view;

const TEXT: &str = "# a comment\n[[row]]\nlint = \"a\"\nsubject = \"kaski-x\"\nticket = \"t_one\"\n\n[[row]]\nlint = \"b\"\nsubject = \"f.rs::t_two\"\nticket = \"t_three\"\nreason = \"over\"\n";

#[test]
fn rows_are_read_with_their_keys_and_lines() {
    let rows = parse(TEXT);
    assert_eq!(rows.len(), 2);
    assert_eq!(
        (
            rows[0].lint.as_str(),
            rows[0].subject.as_str(),
            rows[0].line
        ),
        ("a", "kaski-x", 2)
    );
    assert_eq!(rows[1].reason, "over");
    assert_eq!(of(&rows, "b").len(), 1);
}

#[test]
fn a_size_row_keeps_its_paths_files_and_lines() {
    let rows = parse(
        "[[row]]\nlint = \"d\"\nsubject = \"kaski-x\"\nticket = \"t\"\npaths = \"src/a.rs,src/b\"\nfiles = 3\nlines = 120\n",
    );
    assert_eq!(rows[0].paths, "src/a.rs,src/b");
    assert_eq!((rows[0].files, rows[0].lines), (3, 120));
    // the control: a row naming none has no size, so nothing reads one into it
    let bare = parse("[[row]]\nlint = \"d\"\n");
    assert_eq!(
        (bare[0].files, bare[0].lines, bare[0].paths.as_str()),
        (0, 0, "")
    );
}

#[test]
fn a_key_before_any_table_and_an_unknown_key_are_left_alone() {
    assert!(parse("lint = \"a\"\n").is_empty());
    assert_eq!(parse("[[row]]\nwhat = \"a\"\n")[0].lint, "");
}

#[test]
fn the_verdict_refuses_a_new_offender_and_a_row_with_nothing_left() {
    assert_eq!(verdict(true, false), Verdict::New);
    assert_eq!(verdict(false, true), Verdict::Stale);
    assert_eq!(verdict(true, true), Verdict::Holds);
    assert_eq!(verdict(false, false), Verdict::Holds);
}

fn reg() -> RegistryView {
    view(&[
        ("ticket::open_one", &[("title", "x")]),
        ("ticket::shut_one", &[("title", "y")]),
        ("ticket_state::doing", &[("closed", "false")]),
        ("ticket_state::done", &[("closed", "true")]),
        ("ticket_move::m1", &[
            ("ticket", "open_one"),
            ("to", "doing"),
            ("at", "2026-11-01T00:00:00Z"),
        ]),
        ("ticket_move::m2", &[
            ("ticket", "shut_one"),
            ("to", "doing"),
            ("at", "2026-11-01T00:00:00Z"),
        ]),
        ("ticket_move::m3", &[
            ("ticket", "shut_one"),
            ("to", "done"),
            ("at", "2026-11-02T00:00:00Z"),
        ]),
    ])
}

#[test]
fn a_ticket_whose_newest_move_is_to_a_closed_state_is_closed() {
    let closed = closed_tickets(&reg());
    assert!(closed.contains("shut_one"));
    assert!(!closed.contains("open_one"));
}

#[test]
fn a_ticket_closed_with_an_answer_or_picks_is_answered_and_one_closed_with_neither_is_not() {
    let r = view(&[
        ("ticket_state::doing", &[("closed", "false")]),
        ("ticket_state::done", &[("closed", "true")]),
        ("ticket_move::w", &[
            ("ticket", "worded"),
            ("to", "done"),
            ("at", "2026-11-02T00:00:00Z"),
            ("answer", "Any roof."),
        ]),
        ("ticket_move::p", &[
            ("ticket", "picked"),
            ("to", "done"),
            ("at", "2026-11-02T01:00:00Z"),
            ("picks", "[\"Tool required\"]"),
        ]),
        ("ticket_move::e", &[
            ("ticket", "empty"),
            ("to", "done"),
            ("at", "2026-11-02T02:00:00Z"),
            ("answer", ""),
            ("picks", "[]"),
        ]),
        ("ticket_move::n", &[
            ("ticket", "bare"),
            ("to", "done"),
            ("at", "2026-11-02T03:00:00Z"),
        ]),
        ("ticket_move::o", &[
            ("ticket", "reopened"),
            ("to", "done"),
            ("at", "2026-11-01T00:00:00Z"),
            ("answer", "yes"),
        ]),
        ("ticket_move::o2", &[
            ("ticket", "reopened"),
            ("to", "doing"),
            ("at", "2026-11-02T00:00:00Z"),
        ]),
    ]);
    let answered = answered_tickets(&r);
    assert_eq!(answered.keys().map(String::as_str).collect::<Vec<_>>(), [
        "picked", "worded"
    ]);
    assert_eq!(answered["worded"], "2026-11-02T00:00:00Z");
    // the control: every one of them is closed but the reopened one, answered or not
    let closed = closed_tickets(&r);
    for t in ["worded", "picked", "empty", "bare"] {
        assert!(closed.contains(t), "{t}");
    }
    assert!(!closed.contains("reopened"));
}

#[test]
fn an_older_move_to_a_closed_state_does_not_close_a_reopened_ticket() {
    let r = view(&[
        ("ticket::t", &[("title", "x")]),
        ("ticket_state::doing", &[("closed", "false")]),
        ("ticket_state::done", &[("closed", "true")]),
        ("ticket_move::a", &[
            ("ticket", "t"),
            ("to", "done"),
            ("at", "2026-11-01T00:00:00Z"),
        ]),
        ("ticket_move::b", &[
            ("ticket", "t"),
            ("to", "doing"),
            ("at", "2026-11-02T00:00:00Z"),
        ]),
    ]);
    assert!(closed_tickets(&r).is_empty());
}

#[test]
fn the_judge_refuses_a_new_unit_and_a_stale_row_and_lets_a_listed_unit_hold() {
    let d = crate::canon_rows::testkit::scratch("judge");
    let found = |units: &[&str]| {
        units
            .iter()
            .map(|u| ((*u).to_owned(), vec![("p".to_owned(), 3, "bad".to_owned())]))
            .collect::<std::collections::BTreeMap<_, _>>()
    };
    crate::canon_rows::testkit::write(
        &d,
        FILE,
        "[[row]]\nlint = \"l\"\nsubject = \"listed\"\nticket = \"t\"\n\n[[row]]\nlint = \"l\"\nsubject = \"gone\"\nticket = \"t\"\n",
    );
    let said: Vec<String> = judge(&mockspace::reading::Reading::new(&d).view(), &d, "l", &found(&["listed", "new"]))
        .into_iter()
        .map(|e| e.message)
        .collect();
    assert_eq!(said.len(), 2);
    assert!(said.iter().any(|m| m.contains("`new` has no row")));
    assert!(
        said.iter()
            .any(|m| m.contains("`gone`") && m.contains("remove the row"))
    );
}

#[test]
fn a_row_keeps_the_count_it_allows_and_a_row_naming_none_allows_any() {
    let rows = parse("[[row]]\nlint = \"l\"\nsubject = \"u\"\nticket = \"t\"\nfindings = 7\n");
    assert_eq!(rows[0].findings, 7);
    // the control: a row with no count reads as none, which allows every finding it ever did
    assert_eq!(parse("[[row]]\nlint = \"l\"\n")[0].findings, 0);
}

/// The messages the judge gives a unit with `n` findings under a row allowing `cap`.
fn counted(cap: usize, n: usize) -> Vec<String> {
    let d = crate::canon_rows::testkit::scratch("counted");
    crate::canon_rows::testkit::write(
        &d,
        FILE,
        &format!("[[row]]\nlint = \"l\"\nsubject = \"u\"\nticket = \"t\"\nfindings = {cap}\n"),
    );
    let found = std::iter::once((
        "u".to_owned(),
        (0 .. n)
            .map(|i| ("p".to_owned(), i + 1, "bad".to_owned()))
            .collect::<Vec<_>>(),
    ))
    .collect::<std::collections::BTreeMap<_, _>>();
    judge(&mockspace::reading::Reading::new(&d).view(), &d, "l", &found)
        .into_iter()
        .map(|e| e.message)
        .collect()
}

#[test]
fn a_counted_row_holds_a_unit_at_its_count() {
    assert!(counted(4, 4).is_empty());
}

#[test]
fn a_counted_row_refuses_a_unit_over_its_count() {
    let said = counted(4, 5);
    assert!(!said.is_empty());
    assert!(
        said.iter()
            .all(|m| m.contains("4 findings") && m.contains("it has 5")),
        "{said:?}"
    );
}

#[test]
fn a_counted_row_above_the_findings_left_says_to_lower_it() {
    let said = counted(7, 4);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].contains("lower") && said[0].contains('4') && said[0].contains('7'),
        "{said:?}"
    );
}

#[test]
fn a_ticket_is_in_the_registry_or_not() {
    assert!(has_ticket(&reg(), "open_one"));
    assert!(!has_ticket(&reg(), "nope"));
}
