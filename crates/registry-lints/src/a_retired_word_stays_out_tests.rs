use mockspace::reading::Reading;

use super::*;
use crate::canon_rows::testkit::{check_in, git, scratch, view, write};

/// What [`check`] finds over `d`, read through a reading of its own.
fn checked(d: &Path, reg: &RegistryView, config: &str, change: &Change) -> Vec<LintError> {
    check(&Reading::new(d).view(), d, reg, config, change, &layout())
}

fn reg() -> RegistryView {
    view(&[
        ("tenet::there_is_no_zorblax", &[("rules_out", "zorblax")]),
        ("tenet::the_zorblax_grows", &[
            ("rules_out", "grows"),
            ("superseded_by", "there_is_no_zorblax"),
        ]),
        ("tenet::the_ground_is_one", &[("what", "One ground.")]),
    ])
}

fn config(exempt: &[&str]) -> String {
    let list: Vec<String> = exempt.iter().map(|u| format!("\"{u}\"")).collect();
    format!("[lints.{NAME}]\nexempt_crates = [{}]\n", list.join(", "))
}

fn repo() -> std::path::PathBuf {
    let d = scratch("retired-word");
    write(
        &d,
        "mock/crates/kaski-a/DESIGN.md.tmpl",
        "a zorblax stands here\n",
    );
    write(
        &d,
        "mock/crates/kaski-a/src/lib.rs",
        "pub struct Quillfern;\n",
    );
    write(
        &d,
        "mock/crates/kaski-b/src/lib.rs",
        "pub struct ZorblaxCell;\n",
    );
    write(&d, "mock/research/x/NOTES.md", "the zorblax was a window\n");
    d
}

#[test]
fn a_unit_not_on_the_list_is_held_from_its_first_line() {
    let d = repo();
    let found = checked(&d, &reg(), &config(&[]), &Change::default());
    let units: Vec<&str> = found.iter().map(|e| e.crate_name.as_str()).collect();
    assert_eq!(units, ["kaski-a", "kaski-b"], "{found:?}");
    assert!(found[0].message.contains("there_is_no_zorblax"));
}

#[test]
fn a_unit_on_the_list_is_let_through_and_the_others_still_fail() {
    let d = repo();
    let found = checked(&d, &reg(), &config(&["kaski-a"]), &Change::default());
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].crate_name, "kaski-b");
}

#[test]
fn a_unit_on_the_list_with_nothing_left_is_a_finding_so_the_list_only_shrinks() {
    let d = repo();
    let found = checked(
        &d,
        &reg(),
        &config(&["kaski-a", "kaski-b", "kaski-c"]),
        &Change::default(),
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.contains("`kaski-c` is listed"));
}

#[test]
fn the_control_a_tree_that_conforms_has_no_finding_and_a_superseded_tenet_binds_nothing() {
    let d = scratch("retired-word-clean");
    write(
        &d,
        "mock/crates/kaski-a/DESIGN.md.tmpl",
        "the quillfern stands, and it grows\n",
    );
    assert!(checked(&d, &reg(), &config(&[]), &Change::default()).is_empty());
    let none = view(&[("tenet::the_ground_is_one", &[("what", "One ground.")])]);
    assert!(checked(&repo(), &none, &config(&[]), &Change::default()).is_empty());
}


#[test]
fn a_files_key_moves_with_the_rules_so_a_tenet_ruling_out_another_word_judges_every_file_again() {
    // Mutation: the key leaves the rules out, so a finding saved under one set of words replays under another.
    let d = repo();
    let keys = |reg: &RegistryView| -> Vec<String> {
        scanned_files(&Reading::new(&d).view(), &d, &retiring(reg), &layout())
            .into_iter()
            .map(|f| f.key)
            .collect()
    };
    let wider = view(&[("tenet::there_is_no_quillfern", &[("rules_out", "quillfern")])]);
    let (before, again, after) = (keys(&reg()), keys(&reg()), keys(&wider));
    assert_eq!(before.len(), 3, "{before:?}");
    assert_eq!(before, again, "the same rules over the same files keep their keys");
    assert!(before.iter().all(|k| !after.contains(k)), "{before:?} {after:?}");
}

#[test]
fn the_lint_judging_each_file_through_its_own_view_finds_what_the_tree_holds() {
    let d = repo();
    write(&d, "mockspace.toml", &config(&[]));
    let units: Vec<String> = check_in(&lint(), &d, &d.join("mock"), &reg())
        .into_iter()
        .filter(|e| !e.message.contains("cannot tell which lines"))
        .map(|e| format!("{}:{}", e.crate_name, e.line))
        .collect();
    assert_eq!(units, ["kaski-a:1", "kaski-b:1"]);
}


const LIB: &str = "mock/crates/kaski-a/src/lib.rs";

/// A repository whose `dev` holds `kaski-a` with two lines naming `zorblax` and one that does not, with
/// `mockspace.toml` listing `exempt`, on a branch `work` cut from it.
fn on_work(exempt: &[&str]) -> std::path::PathBuf {
    let d = scratch("retired-word-change");
    git(&d, &["init", "-q", "-b", "dev"]);
    write(
        &d,
        LIB,
        "pub struct ZorblaxCell;\npub fn zorblax_a() {}\npub fn calm() {}\n",
    );
    write(&d, "mockspace.toml", &config(exempt));
    git(&d, &["add", "mock", "mockspace.toml"]);
    git(&d, &["commit", "-q", "-m", "base"]);
    git(&d, &["switch", "-q", "-c", "work"]);
    d
}

/// What the lint finds over a work tree, the change read from git.
fn judged(d: &Path) -> Vec<LintError> {
    let change = change_of(&Reading::new(d).view(), d).expect("a change");
    let config = std::fs::read_to_string(d.join("mockspace.toml")).expect("a config");
    checked(d, &reg(), &config, &change)
}

#[test]
fn a_listed_unit_with_no_change_is_let_through_for_the_lines_it_holds() {
    let d = on_work(&["kaski-a"]);
    let found = judged(&d);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_line_a_change_adds_to_a_listed_unit_is_refused_and_the_lines_it_held_still_stand() {
    // Mutation: the touched lines are not consulted, which is the gate that never fired.
    let d = on_work(&["kaski-a"]);
    write(
        &d,
        LIB,
        "pub struct ZorblaxCell;\npub fn zorblax_a() {}\npub fn calm() {}\npub fn zorblax_b() {}\n",
    );
    let found = judged(&d);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        (found[0].crate_name.as_str(), found[0].line),
        ("kaski-a", 4)
    );
    assert!(
        found[0].message.contains("adds or touches"),
        "{}",
        found[0].message
    );
    assert!(found[0].message.contains("there_is_no_zorblax"));
}

#[test]
fn a_line_a_change_touches_in_a_listed_unit_is_refused_though_it_named_the_word_before() {
    let d = on_work(&["kaski-a"]);
    write(
        &d,
        LIB,
        "pub struct ZorblaxCell(u32);\npub fn zorblax_a() {}\npub fn calm() {}\n",
    );
    let found = judged(&d);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, 1);
}

#[test]
fn a_listed_unit_can_shrink_so_a_change_that_clears_a_line_is_let_through() {
    let d = on_work(&["kaski-a"]);
    write(
        &d,
        LIB,
        "pub struct Cell;\npub fn zorblax_a() {}\npub fn calm() {}\n",
    );
    let found = judged(&d);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_listed_unit_whose_changed_lines_name_nothing_is_let_through() {
    let d = on_work(&["kaski-a"]);
    write(
        &d,
        LIB,
        "pub struct ZorblaxCell;\npub fn zorblax_a() {}\npub fn calm() {}\npub fn quiet() {}\n",
    );
    let found = judged(&d);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_new_file_in_a_listed_unit_is_refused_whole() {
    let d = on_work(&["kaski-a"]);
    write(
        &d,
        "mock/crates/kaski-a/src/more.rs",
        "pub fn calm() {}\npub fn zorblax_c() {}\n",
    );
    let found = judged(&d);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, 2);
}

#[test]
fn a_name_added_to_the_list_is_refused_and_one_taken_off_is_not() {
    // Mutation: the list at the base is not read, so every name on it is new or none is.
    let d = on_work(&["kaski-a", "kaski-b"]);
    write(&d, "mockspace.toml", &config(&["kaski-a"]));
    let found = judged(&d);
    assert!(
        found.iter().all(|e| !e.message.contains("is added to")),
        "{found:?}"
    );
    write(
        &d,
        "mockspace.toml",
        &config(&["kaski-a", "kaski-b", "kaski-c"]),
    );
    let found = judged(&d);
    let added: Vec<&str> = found
        .iter()
        .filter(|e| e.message.contains("is added to"))
        .map(|e| e.crate_name.as_str())
        .collect();
    assert_eq!(added, ["kaski-c"], "{found:?}");
}

#[test]
fn a_unit_off_the_list_is_held_from_its_first_line_with_or_without_a_change() {
    let d = on_work(&[]);
    let found = judged(&d);
    let lines: Vec<usize> = found.iter().map(|e| e.line).collect();
    assert_eq!(lines, [1, 2], "{found:?}");
}

#[test]
fn a_change_that_cannot_be_read_is_a_finding_and_the_unlisted_units_are_still_held() {
    // Mutation: an unreadable change is read as none, which lets every listed unit through unwatched.
    let d = repo();
    write(&d, "mockspace.toml", &config(&[]));
    let found = check_in(&lint(), &d, &d.join("mock"), &reg());
    assert!(
        found
            .iter()
            .any(|e| e.message.contains("cannot tell which lines")),
        "{found:?}"
    );
    assert!(found.iter().any(|e| e.crate_name == "kaski-b"), "{found:?}");
}

#[test]
fn the_default_blocks_every_gate() {
    assert_eq!(
        lint().default_severity(),
        Severity::HARD_ERROR
    );
}

/// The layout the fixtures are written in: the mock tree under `mock`, the content under `content`.
fn layout() -> crate::layout::Layout {
    crate::layout::Layout::new("mock", crate::layout::ContentDirs::new(&["content"]))
}

fn lint() -> ARetiredWordStaysOut {
    ARetiredWordStaysOut {
        content: crate::layout::ContentDirs::new(&["content"]),
    }
}
