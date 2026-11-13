use super::*;
use crate::canon_rows::testkit::{git, scratch, view, write};
use mockspace::reading::Reading;

/// The marker word is written `§` here, so this file holds no marker of its own.
fn t(s: &str) -> String {
    s.replace('§', "FIXME")
}

fn reg() -> RegistryView {
    view(&[
        ("ticket::a_flat_roof", &[("title", "A flat roof")]),
        ("ticket::a_shut_roof", &[("title", "A shut roof")]),
        ("ticket::a_roof_op_answered", &[(
            "title",
            "A roof op answered",
        )]),
        ("ticket_move::m4", &[
            ("ticket", "a_roof_op_answered"),
            ("to", "doing"),
            ("at", "2026-11-01T00:00:00Z"),
        ]),
        ("ticket_move::m5", &[
            ("ticket", "a_roof_op_answered"),
            ("to", "done"),
            ("at", "2026-11-03T09:39:09Z"),
            ("answer", "Any roof is fine."),
        ]),
        ("ticket_state::doing", &[("closed", "false")]),
        ("ticket_state::done", &[("closed", "true")]),
        ("ticket_move::m1", &[
            ("ticket", "a_flat_roof"),
            ("to", "doing"),
            ("at", "2026-11-01T00:00:00Z"),
        ]),
        ("ticket_move::m2", &[
            ("ticket", "a_shut_roof"),
            ("to", "doing"),
            ("at", "2026-11-01T00:00:00Z"),
        ]),
        ("ticket_move::m3", &[
            ("ticket", "a_shut_roof"),
            ("to", "done"),
            ("at", "2026-11-02T00:00:00Z"),
        ]),
    ])
}

fn found(files: &[(&str, &str)]) -> Vec<LintError> {
    let d = scratch("every-fixme");
    for (rel, body) in files {
        write(&d, rel, &t(body));
    }
    check(&Reading::new(&d).view(), &d, &reg(), &[], &layout())
}

const SRC: &str = "mock/crates/kaski-a/src/lib.rs";

#[test]
fn a_bare_marker_is_refused_with_its_file_and_line() {
    let f = found(&[(SRC, "fn a() {}\n// §: later\n")]);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].line, 2);
    assert_eq!(f[0].path.as_deref(), Some(SRC));
    assert_eq!(f[0].crate_name, "kaski-a");
    assert!(f[0].message.contains("cites no ticket"), "{}", f[0].message);
    assert!(f[0].message.contains("cargo mock owed"));
}

#[test]
fn a_bare_marker_in_a_shader_is_refused_and_one_citing_a_ticket_is_not() {
    const SHADER: &str = "mock/crates/kaski-a/shaders/a.slang";
    let f = found(&[(SHADER, "module a;\n// §: later\n")]);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].line, 2);
    assert_eq!(f[0].path.as_deref(), Some(SHADER));
    assert!(f[0].message.contains("cites no ticket"), "{}", f[0].message);
    let ok = found(&[(SHADER, "module a;\n// §(a_flat_roof): later\n")]);
    assert!(ok.is_empty(), "{ok:?}");
}

#[test]
fn a_marker_citing_a_ticket_that_is_not_in_the_registry_is_refused() {
    let f = found(&[(SRC, "// §(no_such_ticket): later\n")]);
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(
        f[0].message
            .contains("`no_such_ticket`, which is no ticket"),
        "{}",
        f[0].message
    );
}

#[test]
fn a_tag_is_not_a_ticket_however_it_is_spelled() {
    let f = found(&[(
        SRC,
        "// §(canon silent): a\n// §(stand-in): b\n// §(op): c\n",
    )]);
    // `op` is shaped like an id and is no ticket in the registry; the other two are not shaped like one
    assert_eq!(f.len(), 3, "{f:?}");
    assert!(f[0].message.contains("a tag and not a ticket id"));
    assert!(f[1].message.contains("a tag and not a ticket id"));
    assert!(f[2].message.contains("no ticket in the registry"));
}

#[test]
fn a_marker_citing_a_ticket_passes_in_every_kind_of_file() {
    let f = found(&[
        (SRC, "// §(a_flat_roof): the roof is flat\n"),
        (
            "content/kaski/a.toml",
            "# §(a_flat_roof): a first take\nkey = 1\n",
        ),
        (
            "mock/crates/kaski-a/DESIGN.md.tmpl",
            "`§(a_flat_roof)`: a roof\n",
        ),
        ("mock/WORKFLOW.md.tmpl", "<!-- §(a_flat_roof): a note -->\n"),
    ]);
    assert!(f.is_empty(), "{f:?}");
}

fn waiver(subject: &str) -> Vec<ratchets::Row> {
    ratchets::parse(&format!(
        "[[row]]\nlint = \"{NAME}\"\nsubject = \"{subject}\"\nticket = \"a_flat_roof\"\n"
    ))
}

fn found_waived(files: &[(&str, &str)], rows: &[ratchets::Row]) -> Vec<LintError> {
    let d = scratch("every-fixme-waived");
    for (rel, body) in files {
        write(&d, rel, &t(body));
    }
    check(&Reading::new(&d).view(), &d, &reg(), rows, &layout())
}

#[test]
fn a_marker_citing_a_closed_ticket_is_refused() {
    let f = found(&[(SRC, "// §(a_shut_roof): the roof is shut\n")]);
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(
        f[0].message.contains("a ticket that is closed"),
        "{}",
        f[0].message
    );
    assert_eq!(f[0].line, 1);
}

#[test]
fn a_marker_citing_an_answered_question_is_owed_work_and_not_an_error() {
    let f = found(&[(SRC, "// §(a_roof_op_answered): the roof is open\n")]);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].severity, Severity::ADVISORY, "{f:?}");
    assert!(
        f[0].message.contains("answered on 2026-11-03"),
        "{}",
        f[0].message
    );
    assert!(f[0].message.contains("cargo mock owed"), "{}", f[0].message);
    assert_eq!(f[0].line, 1);
    assert_eq!(f[0].path.as_deref(), Some(SRC));
    // the control: the same marker citing a ticket closed with no answer is still an error
    let shut = found(&[(SRC, "// §(a_shut_roof): the roof is shut\n")]);
    assert_eq!(shut.len(), 1, "{shut:?}");
    assert_ne!(shut[0].severity, Severity::ADVISORY, "{shut:?}");
}

#[test]
fn a_row_holds_a_marker_citing_an_answered_question_and_goes_stale_with_it() {
    let rows = ratchets::parse(&format!(
        "[[row]]\nlint = \"{NAME}\"\nsubject = \"{SRC}::a_roof_op_answered\"\nticket = \"a_flat_roof\"\n"
    ));
    let held = found_waived(&[(SRC, "// §(a_roof_op_answered): open\n")], &rows);
    assert!(
        held.is_empty(),
        "a held marker is neither a warning nor a stale row: {held:?}"
    );
    // the control: the marker gone, the row is stale
    let stale = found_waived(&[(SRC, "// §(a_flat_roof): flat\n")], &rows);
    assert_eq!(stale.len(), 1, "{stale:?}");
    assert!(
        stale[0].message.contains("remove the row"),
        "{}",
        stale[0].message
    );
}

#[test]
fn a_marker_citing_an_open_ticket_beside_it_passes() {
    let f = found(&[(SRC, "// §(a_flat_roof): flat\n// §(a_shut_roof): shut\n")]);
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].line, 2);
}

#[test]
fn a_row_for_the_file_and_the_closed_ticket_allows_that_marker_only() {
    let rows = waiver(&format!("{SRC}::a_shut_roof"));
    let allowed = found_waived(&[(SRC, "// §(a_shut_roof): shut\n")], &rows);
    assert!(allowed.is_empty(), "{allowed:?}");
    // the control: the same row allows nothing in another file, and nothing citing another closed ticket
    let elsewhere = found_waived(
        &[
            (SRC, "// §(a_shut_roof): shut\n"),
            ("mock/crates/kaski-a/src/b.rs", "// §(a_shut_roof): shut\n"),
        ],
        &rows,
    );
    assert_eq!(elsewhere.len(), 1, "{elsewhere:?}");
    assert_eq!(
        elsewhere[0].path.as_deref(),
        Some("mock/crates/kaski-a/src/b.rs")
    );
}

#[test]
fn a_row_whose_file_no_longer_cites_the_closed_ticket_is_refused() {
    let rows = waiver(&format!("{SRC}::a_shut_roof"));
    let stale = found_waived(&[(SRC, "// §(a_flat_roof): flat\n")], &rows);
    assert_eq!(stale.len(), 1, "{stale:?}");
    assert!(
        stale[0].message.contains("remove the row"),
        "{}",
        stale[0].message
    );
    assert_eq!(stale[0].path.as_deref(), Some("mock/ratchets.toml"));
}

#[test]
fn a_marker_with_a_ticket_and_no_colon_is_refused() {
    let f = found(&[(SRC, "// §(a_flat_roof) the roof is flat\n")]);
    assert_eq!(f.len(), 1, "{f:?}");
    assert!(f[0].message.contains("no colon"), "{}", f[0].message);
}

#[test]
fn what_is_not_a_marker_or_is_not_read_is_left_alone() {
    let f = found(&[
        // prose that names the marker, a string, and the word inside another word
        (
            "mock/WORKFLOW.md.tmpl",
            "A line marked with a § in the source is a placeholder.\n",
        ),
        (SRC, "let s = \"§(x): y\";\nlet n = NOT§;\n"),
        // todo is not a marker
        ("mock/crates/kaski-a/src/other.rs", "// TODO: later\n"),
        // the records, the vendored code and the generated documents are not read
        (
            "mock/design_rounds/2026/x.md",
            "§(canon silent): a quotation\n",
        ),
        (
            "mock/design_rounds/202610010000_changelist.doc.lock.md",
            "§: a quotation\n",
        ),
        ("mock/research/n/NOTES.md", "§(op): a quotation\n"),
        ("mock/research/n/p.patch", "+// §: a quotation\n"),
        ("mock/vendor/v/src/lib.rs", "// §: vendored\n"),
        ("docs/DESIGN.md", "§: generated\n"),
        ("mock/target/x/lib.rs", "// §: built\n"),
    ]);
    assert!(f.is_empty(), "{f:?}");
}

#[test]
fn the_default_blocks_every_gate() {
    assert_eq!(
        EveryFixmeNamesATicket::default().default_severity(),
        Severity::HARD_ERROR
    );
}

const SIDE: &str = "mock/crates/kaski-a/src/side.rs";
const MAIN: &str = "mock/crates/kaski-a/src/main_side.rs";
const ADDED: &str = "mock/crates/kaski-a/src/added.rs";

/// A repository whose `main` and `side` each added a file holding a marker that cites no ticket,
/// with the merge of `side` into `main` begun and not committed, so `MERGE_HEAD` exists.
fn merging() -> std::path::PathBuf {
    let d = scratch("every-fixme-merge");
    git(&d, &["init", "-q", "-b", "main"]);
    write(&d, "mock/crates/kaski-a/src/base.rs", "fn a() {}\n");
    git(&d, &["add", "mock"]);
    git(&d, &["commit", "-q", "-m", "base"]);
    git(&d, &["switch", "-q", "-c", "side"]);
    write(&d, SIDE, &t("// §: from the side\n"));
    git(&d, &["add", "mock"]);
    git(&d, &["commit", "-q", "-m", "side"]);
    git(&d, &["switch", "-q", "main"]);
    write(&d, MAIN, &t("// §: from main\n"));
    git(&d, &["add", "mock"]);
    git(&d, &["commit", "-q", "-m", "main"]);
    git(&d, &["merge", "-q", "--no-commit", "--no-ff", "side"]);
    d
}

#[test]
fn a_merge_does_not_judge_the_markers_either_parent_carried() {
    let d = merging();
    let f = check(&Reading::new(&d).view(), &d, &reg(), &[], &layout());
    assert!(f.is_empty(), "{f:?}");
}

#[test]
fn a_merge_still_judges_the_marker_it_adds() {
    let d = merging();
    write(&d, ADDED, &t("// §: the merge's own\n"));
    let f = check(&Reading::new(&d).view(), &d, &reg(), &[], &layout());
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].path.as_deref(), Some(ADDED));
}

#[test]
fn a_carried_marker_whose_words_the_merge_changed_is_the_merges_own() {
    let d = merging();
    write(&d, SIDE, &t("// §: reworded in the resolution\n"));
    let f = check(&Reading::new(&d).view(), &d, &reg(), &[], &layout());
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].path.as_deref(), Some(SIDE));
}

#[test]
fn a_second_marker_beside_a_carried_one_is_still_judged() {
    // the control: a marker in the same file as a carried one, with other words, is judged
    let d = merging();
    write(
        &d,
        MAIN,
        &t("// §: from main\n// §: a second, added by the merge\n"),
    );
    let f = check(&Reading::new(&d).view(), &d, &reg(), &[], &layout());
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(f[0].line, 2);
}

#[test]
fn the_same_markers_are_refused_once_the_merge_is_committed() {
    // the control for the three above: it is the merge in progress that excuses them
    let d = merging();
    git(&d, &["commit", "-q", "-m", "merge"]);
    let f = check(&Reading::new(&d).view(), &d, &reg(), &[], &layout());
    assert_eq!(f.len(), 2, "{f:?}");
}

/// The layout the fixtures are written in: the mock tree under `mock`, the content under `content`.
fn layout() -> crate::layout::Layout {
    crate::layout::Layout::new("mock", crate::layout::ContentDirs::new(&["content"]))
}
