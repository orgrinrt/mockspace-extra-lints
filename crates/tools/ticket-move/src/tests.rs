use std::collections::BTreeSet;

use mockspace::RegistryView;
use mockspace::tool::{Outcome, Tool, ToolContext, ToolReport};
use tickets::testkit::Scratch;
use tickets::{load, registry_dir};

use super::TicketMove;

fn run(s: &Scratch, args: &[&str]) -> ToolReport {
    let crates = BTreeSet::new();
    let registry = RegistryView::default();
    TicketMove.run(&ToolContext {
        mock_dir: &s.mock,
        repo_root: &s.mock,
        all_crates: &crates,
        src_dirs: &[],
        args,
        stdin: None,
        registry: &registry,
    })
}

fn state(s: &Scratch, id: &str) -> String {
    load(&registry_dir(&s.mock))
        .unwrap()
        .state_of(id)
        .unwrap()
        .id
        .clone()
}

#[test]
fn a_move_that_validates_is_appended_and_nothing_is_printed() {
    let s = Scratch::new();
    s.add("Plant", &[]).unwrap();
    let r = run(&s, &[
        "plant",
        "ready",
        "--by",
        "tester",
        "--at",
        "2026-10-23T11:00:00Z",
    ]);
    assert!(matches!(r.outcome, Outcome::Clean {
        examined: 1,
    }));
    assert!(r.output.is_empty());
    assert_eq!(state(&s, "plant"), "ready");
}

#[test]
fn a_state_the_kind_does_not_declare_is_refused_and_the_state_stays() {
    let s = Scratch::new();
    s.add("Plant", &[]).unwrap();
    let r = run(&s, &[
        "plant",
        "limbo",
        "--by",
        "tester",
        "--at",
        "2026-10-23T11:00:00Z",
    ]);
    assert!(matches!(r.outcome, Outcome::Inconclusive { .. }));
    assert_eq!(state(&s, "plant"), "backlog");
}

#[test]
fn a_move_that_leaves_a_relation_dangling_is_refused() {
    let s = Scratch::new();
    let a = s.add("Chicken", &[]).unwrap();
    s.add("Egg", &[("needs", &[&a])]).unwrap();
    let r = run(&s, &[
        "chicken",
        "dropped",
        "--by",
        "tester",
        "--at",
        "2026-10-23T11:00:00Z",
    ]);
    assert!(matches!(r.outcome, Outcome::Inconclusive { .. }));
    assert_eq!(state(&s, "chicken"), "backlog");
}

#[test]
fn a_move_of_a_ticket_that_does_not_exist_is_refused() {
    let s = Scratch::new();
    let r = run(&s, &[
        "ghost",
        "ready",
        "--by",
        "tester",
        "--at",
        "2026-10-23T11:00:00Z",
    ]);
    assert!(matches!(r.outcome, Outcome::Inconclusive { .. }));
}

#[test]
fn links_alone_are_recorded_and_the_state_stays() {
    let s = Scratch::new();
    s.add("Plant", &[]).unwrap();
    let r = run(&s, &[
        "plant",
        "--session",
        "session_p",
        "--branch",
        "work/plant",
        "--by",
        "tester",
        "--at",
        "2026-10-23T11:00:00Z",
    ]);
    assert!(matches!(r.outcome, Outcome::Clean {
        examined: 1,
    }));
    assert!(r.output.is_empty());
    assert_eq!(state(&s, "plant"), "backlog");
    let store = load(&registry_dir(&s.mock)).unwrap();
    assert_eq!(store.tickets["plant"].one("session"), "session_p");
    assert_eq!(store.tickets["plant"].one("branch"), "work/plant");
    assert_eq!(store.moves_of("plant").count(), 1);
}

/// The marker word is written `§` here, so this file holds no marker of its own.
fn t(s: &str) -> String {
    s.replace('§', "FIXME")
}

fn put(s: &Scratch, rel: &str, text: &str) {
    let path = s.mock.join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, t(text)).unwrap();
}

fn read(s: &Scratch, rel: &str) -> String {
    std::fs::read_to_string(s.mock.join(rel)).unwrap()
}

/// Two open tickets, `old_work` and `new_work`.
fn pair(s: &Scratch) {
    s.add("Old work", &[]).unwrap();
    s.add("New work", &[]).unwrap();
}

const AT: [&str; 4] = ["--by", "tester", "--at", "2026-10-23T11:00:00Z"];

fn close_for(s: &Scratch, successor: &str) -> ToolReport {
    let mut args = vec!["old_work", "done", "--for", successor];
    args.extend(AT);
    run(s, &args)
}

#[test]
fn closing_for_a_successor_hands_the_markers_over_and_moves_the_ticket() {
    let s = Scratch::new();
    pair(&s);
    put(
        &s,
        "src/a.rs",
        "// §(old_work): first\nlet x = 1; // §(old_work_two): not this one\n// §(other): nor this\n",
    );
    put(
        &s,
        "content/c.toml",
        "# §(old_work): in a config\nkey = 1\n",
    );
    put(&s, "crate/DESIGN.md.tmpl", "`§(old_work)`: in a design\n");
    put(
        &s,
        "src/untouched.rs",
        "// §(other): elsewhere\nfn f() {}\n",
    );
    let r = close_for(&s, "new_work");
    assert!(
        matches!(r.outcome, Outcome::Clean { .. }),
        "{:?}",
        r.outcome
    );
    assert!(r.output.is_empty(), "silent on success: {}", r.output);
    assert_eq!(state(&s, "old_work"), "done");
    assert_eq!(
        read(&s, "src/a.rs"),
        t(
            "// §(new_work): first\nlet x = 1; // §(old_work_two): not this one\n// §(other): nor this\n"
        )
    );
    assert_eq!(
        read(&s, "content/c.toml"),
        t("# §(new_work): in a config\nkey = 1\n")
    );
    assert_eq!(
        read(&s, "crate/DESIGN.md.tmpl"),
        t("`§(new_work)`: in a design\n")
    );
    assert_eq!(
        read(&s, "src/untouched.rs"),
        t("// §(other): elsewhere\nfn f() {}\n")
    );
}

fn close_alone(s: &Scratch, ticket: &str) -> ToolReport {
    let mut args = vec![ticket, "done"];
    args.extend(AT);
    run(s, &args)
}

#[test]
fn closing_with_markers_left_citing_it_and_no_successor_is_refused_and_writes_nothing() {
    let s = Scratch::new();
    pair(&s);
    put(
        &s,
        "src/a.rs",
        "// §(old_work): first\nlet x = 1; // §(old_work): second\n",
    );
    put(&s, "src/b.rs", "// §(old_work): third\n");
    let r = close_alone(&s, "old_work");
    let Outcome::Inconclusive {
        reason,
    } = r.outcome
    else {
        panic!("the close was not refused: {:?}", r.outcome);
    };
    for needle in ["src/a.rs:1", "src/a.rs:2", "src/b.rs:1", "--for", "--answer", "3 live"] {
        assert!(reason.contains(needle), "{needle}: {reason}");
    }
    assert_eq!(state(&s, "old_work"), "backlog");
    assert_eq!(
        read(&s, "src/a.rs"),
        t("// §(old_work): first\nlet x = 1; // §(old_work): second\n")
    );
}

#[test]
fn the_refusal_names_five_markers_and_counts_the_rest() {
    let s = Scratch::new();
    pair(&s);
    for i in 0 .. 7 {
        put(&s, &format!("src/f{i}.rs"), "// §(old_work): x\n");
    }
    let r = close_alone(&s, "old_work");
    let Outcome::Inconclusive {
        reason,
    } = r.outcome
    else {
        panic!("the close was not refused: {:?}", r.outcome);
    };
    assert!(reason.contains("7 live"), "{reason}");
    assert!(reason.contains("and 2 more"), "{reason}");
}

#[test]
fn what_the_refusal_does_not_cover_still_goes_through() {
    // the controls for the refusal: a move that does not close the ticket, a close that no marker
    // cites, a close that hands the markers over, and a marker only a record or a generated
    // document holds
    let s = Scratch::new();
    pair(&s);
    put(&s, "src/a.rs", "// §(old_work): first\n");
    let mut ready = vec!["old_work", "ready"];
    ready.extend(AT);
    assert!(matches!(run(&s, &ready).outcome, Outcome::Clean { .. }));
    assert_eq!(state(&s, "old_work"), "ready");
    assert_eq!(read(&s, "src/a.rs"), t("// §(old_work): first\n"));
    let mut other = vec!["new_work", "done"];
    other.extend(AT);
    assert!(matches!(run(&s, &other).outcome, Outcome::Clean { .. }));
    assert_eq!(state(&s, "new_work"), "done");
    let s = Scratch::new();
    pair(&s);
    put(&s, "research/n/NOTES.md", "// §(old_work): quoted\n");
    put(&s, "docs/GEN.md", "// §(old_work): quoted\n");
    assert!(matches!(
        close_alone(&s, "old_work").outcome,
        Outcome::Clean { .. }
    ));
    assert_eq!(state(&s, "old_work"), "done");
}

#[test]
fn a_question_closed_with_an_answer_leaves_its_markers_as_the_work_owed() {
    let s = Scratch::new();
    let options = ["Any roof is fine", "Only a tiled roof"];
    let id = s
        .add_as("Which roof", "question", None, "", &[
            ("options", &options),
            ("related", &["subject::engine"]),
        ])
        .unwrap();
    s.mv(&id, "waiting", Some("op"), "2026-10-23T10:30:00Z")
        .unwrap();
    put(&s, "src/a.rs", "// §(which_roof): after the answer\n");
    let mut args = vec![id.as_str(), "done", "--answer", "Any roof is fine."];
    args.extend(AT);
    let r = run(&s, &args);
    assert!(
        matches!(r.outcome, Outcome::Clean { .. }),
        "{:?}",
        r.outcome
    );
    assert_eq!(state(&s, &id), "done");
    assert_eq!(
        read(&s, "src/a.rs"),
        t("// §(which_roof): after the answer\n")
    );
    // the control: the same markers on a task closed with no answer are the refusal above
    pair(&s);
    put(&s, "src/b.rs", "// §(old_work): x\n");
    assert!(matches!(
        close_alone(&s, "old_work").outcome,
        Outcome::Inconclusive { .. }
    ));
}

/// The scratch's mock directory is its root, so the records sit at `design_rounds/` and `research/`.
#[test]
fn the_records_the_generated_documents_and_vendored_code_are_left_alone() {
    let s = Scratch::new();
    pair(&s);
    let quoted = "// §(old_work): quoted\n";
    for rel in [
        "design_rounds/202610010000/x.md",
        "research/n/NOTES.md",
        "docs/GEN.md",
        "vendor/v/src/lib.rs",
    ] {
        put(&s, rel, quoted);
    }
    put(&s, "src/a.rs", quoted);
    assert!(matches!(
        close_for(&s, "new_work").outcome,
        Outcome::Clean { .. }
    ));
    assert_eq!(read(&s, "src/a.rs"), t("// §(new_work): quoted\n"));
    for rel in [
        "design_rounds/202610010000/x.md",
        "research/n/NOTES.md",
        "docs/GEN.md",
        "vendor/v/src/lib.rs",
    ] {
        assert_eq!(read(&s, rel), t(quoted), "{rel}");
    }
}

#[test]
fn a_successor_the_registry_refuses_writes_no_marker_and_no_move() {
    let s = Scratch::new();
    pair(&s);
    s.add("Shut work", &[]).unwrap();
    let mut shut = vec!["shut_work", "done"];
    shut.extend(AT);
    assert!(matches!(run(&s, &shut).outcome, Outcome::Clean { .. }));
    put(&s, "src/a.rs", "// §(old_work): first\n");
    for successor in ["no_such_ticket", "old_work", "shut_work"] {
        let r = close_for(&s, successor);
        assert!(
            matches!(r.outcome, Outcome::Inconclusive { .. }),
            "{successor}"
        );
        assert_eq!(
            read(&s, "src/a.rs"),
            t("// §(old_work): first\n"),
            "{successor}"
        );
        assert_eq!(state(&s, "old_work"), "backlog", "{successor}");
    }
}

#[test]
fn a_move_the_registry_refuses_for_its_own_reason_writes_no_marker() {
    // the control: a good successor and a state the kind does not declare
    let s = Scratch::new();
    pair(&s);
    put(&s, "src/a.rs", "// §(old_work): first\n");
    let mut args = vec!["old_work", "limbo", "--for", "new_work"];
    args.extend(AT);
    assert!(matches!(
        run(&s, &args).outcome,
        Outcome::Inconclusive { .. }
    ));
    assert_eq!(read(&s, "src/a.rs"), t("// §(old_work): first\n"));
}

#[test]
fn a_link_that_a_rule_refuses_is_refused_and_nothing_is_written() {
    let s = Scratch::new();
    s.add("Plant", &[]).unwrap();
    let r = run(&s, &[
        "plant",
        "--needs",
        "ghost",
        "--by",
        "tester",
        "--at",
        "2026-10-23T11:00:00Z",
    ]);
    assert!(matches!(r.outcome, Outcome::Inconclusive { .. }));
    let store = load(&registry_dir(&s.mock)).unwrap();
    assert!(store.tickets["plant"].many("needs").is_empty());
}
