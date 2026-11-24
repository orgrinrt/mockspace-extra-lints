//! The repository's commits in the one search: a class of hit with its date, the window over what
//! changed, files and commits alike, and newest first, each composing with the other filters.

use super::tests::{Planted, no_rows, output};
use super::*;

/// Commit what is planted under `when` (an ISO date, or the clock where `None`) with `message`.
fn commit(p: &Planted, when: Option<&str>, message: &str) {
    let run = |args: &[&str]| {
        let mut c = std::process::Command::new("git");
        c.args(args)
            .current_dir(&p.repo)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@example.invalid")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@example.invalid");
        if let Some(w) = when {
            c.env("GIT_AUTHOR_DATE", w).env("GIT_COMMITTER_DATE", w);
        }
        assert!(c.status().unwrap().success(), "git {args:?}");
    };
    run(&["add", "-A"]);
    run(&["commit", "-q", "-m", message]);
}

/// Notes written in March, code in April, the notes changed in May.
fn history(name: &str) -> Planted {
    let p = Planted::new(name);
    p.file(
        "mock/research/quokka.md",
        "# Quokka\n\nThe quokka sleeps.\n",
    );
    commit(
        &p,
        Some("2026-03-01T12:00:00Z"),
        "docs: note the quokka and its sleep",
    );
    p.file("src/lib.rs", "//! Digging.\npub fn dig() {}\n");
    commit(
        &p,
        Some("2026-04-01T12:00:00Z"),
        "feat: dig like the quokka does not",
    );
    p.file(
        "mock/research/quokka.md",
        "# Quokka\n\nThe quokka sleeps all day.\n",
    );
    commit(
        &p,
        Some("2026-05-01T12:00:00Z"),
        "fix: the quokka sleeps all day",
    );
    p.refresh();
    p
}

#[test]
fn a_commit_is_a_class_of_hit_with_its_date_and_filters_like_any_class() {
    let p = history("commit-class");
    let all = p.ask(&no_rows(), &["quokka"]);
    let out = output(&all);
    assert!(out.contains("history  .git/"), "{out}");
    assert!(out.contains("2026-05-01"), "{out}");
    let only = p.ask(&no_rows(), &["quokka", "--class", "history"]);
    let out = output(&only);
    assert!(!out.contains("mock/research/quokka.md:"), "{out}");
    assert_eq!(out.matches("history  .git/").count(), 3, "{out}");
    // a commit stays under a prefix where it changed a file under it
    let under = p.ask(&no_rows(), &[
        "quokka", "--class", "history", "--under", "src",
    ]);
    let out = output(&under);
    assert_eq!(out.matches("history  .git/").count(), 1, "{out}");
    assert!(out.contains("touched src/lib.rs"), "{out}");
}

#[test]
fn the_window_keeps_what_changed_in_it_files_and_commits_alike() {
    let p = history("window");
    let may = p.ask(&no_rows(), &[
        "quokka",
        "--since",
        "2026-05-01",
        "--until",
        "2026-05-31",
    ]);
    let out = output(&may);
    assert!(out.contains("mock/research/quokka.md:"), "{out}");
    let note = out
        .lines()
        .find(|l| l.contains("mock/research/quokka.md:"))
        .unwrap();
    assert!(
        note.contains("2026-03-01") && note.contains("changed 2026-05-01"),
        "first-added stays the day, the last change beside it: {out}"
    );
    assert_eq!(out.matches("history  .git/").count(), 1, "{out}");
    assert!(!out.contains("src/lib.rs:"), "{out}");
    // THE CONTROL: April's window holds the code and its commit, and not the note, which changed
    // before it and after it.
    let april = p.ask(&no_rows(), &[
        "quokka",
        "dig",
        "--since",
        "2026-04-01",
        "--until",
        "2026-04-30",
    ]);
    let out = output(&april);
    assert!(out.contains("src/lib.rs:"), "{out}");
    assert!(!out.contains("mock/research/quokka.md:"), "{out}");
}

#[test]
fn a_window_with_no_words_lists_what_changed_newest_first() {
    let p = history("listing");
    p.file("src/new.rs", "pub fn new() {}\n");
    commit(&p, None, "feat: a wombat today");
    p.refresh();
    let lately = p.ask(&no_rows(), &["--days", "2"]);
    let out = output(&lately);
    assert!(out.contains("feat: a wombat today"), "{out}");
    assert!(out.contains("src/new.rs:"), "{out}");
    assert!(!out.contains("mock/research/quokka.md:"), "{out}");
    // newest first over a wider window: today's commit before May's
    let since = p.ask(&no_rows(), &["--since", "2026-03-01", "--class", "history"]);
    let out = output(&since);
    let today = out.find("feat: a wombat today").expect(out);
    let may = out.find("fix: the quokka sleeps all day").expect(out);
    assert!(today < may, "{out}");
    // THE CONTROL: with no words and no window there is nothing to list.
    let none = p.ask(&no_rows(), &["--class", "history"]);
    assert!(matches!(
        none.outcome,
        mockspace::tool::Outcome::Inconclusive { .. }
    ));
}

#[test]
fn newest_orders_a_question_by_the_last_change() {
    let p = history("newest");
    let by_score = p.ask(&no_rows(), &[
        "note", "quokka", "sleep", "--class", "history",
    ]);
    let newest = p.ask(&no_rows(), &[
        "note", "quokka", "sleep", "--class", "history", "--newest",
    ]);
    let first = |out: &str| {
        out.lines()
            .find(|l| l.contains("history  .git/"))
            .unwrap()
            .to_string()
    };
    assert!(
        first(output(&newest)).contains("2026-05-01"),
        "{}",
        newest.output
    );
    // THE CONTROL: by the score, March's commit, which says all three words, comes first.
    assert!(
        first(output(&by_score)).contains("2026-03-01"),
        "{}",
        by_score.output
    );
}

#[test]
fn days_and_since_both_opening_the_window_are_refused() {
    let err = Spec::parse(&["x", "--days", "3", "--since", "2026-01-01"]).unwrap_err();
    assert!(err.contains("--days") && err.contains("--since"), "{err}");
    let s = Spec::parse(&["--days", "3", "--newest"]).unwrap();
    assert_eq!(s.since, Some(corpus::day::today() - 3));
    assert!(s.newest && s.windowed());
    // THE CONTROL: a count that is no number is refused by name.
    assert!(
        Spec::parse(&["--days", "soon"])
            .unwrap_err()
            .contains("--days")
    );
}

#[test]
fn the_window_opens_and_closes_once() {
    for args in [
        &["x", "--since", "2026-01-01", "--since", "2026-02-01"][..],
        &["x", "--days", "3", "--days", "4"][..],
        &["x", "--since", "2026-01-01", "--days", "4"][..],
    ] {
        assert!(
            Spec::parse(args).unwrap_err().contains("opens twice"),
            "{args:?}"
        );
    }
    assert!(
        Spec::parse(&["x", "--until", "2026-01-01", "--until", "2026-02-01"])
            .unwrap_err()
            .contains("closes twice")
    );
    // THE CONTROL: one opening and one closing are taken.
    assert!(Spec::parse(&["x", "--since", "2026-01-01", "--until", "2026-02-01"]).is_ok());
}

/// The index as a search of `args` opens it, with no refresh of the test's own, and how many
/// background refreshes it started.
fn opened(p: &Planted, args: &[&str]) -> (corpus::repo::Opened, usize) {
    let crates = std::collections::BTreeSet::new();
    let dirs: Vec<std::path::PathBuf> = Vec::new();
    let reg = no_rows();
    let ctx = ToolContext {
        mock_dir: &p.mock(),
        repo_root: &p.repo,
        all_crates: &crates,
        src_dirs: &dirs,
        args,
        stdin: None,
        registry: &reg,
    };
    let spec = Spec::parse(args).unwrap();
    let started = std::cell::Cell::new(0);
    let o = opened_for(&spec, &p.index, &ctx, None, &|_: &std::path::Path| {
        started.set(started.get() + 1);
    });
    (o, started.get())
}

#[test]
fn a_question_brings_a_behind_index_current_and_never_builds_a_missing_one() {
    // missing: nothing built in the question's path, the background refresh started, and said
    let p = Planted::new("missing-index");
    p.file(
        "mock/research/quokka.md",
        "# Quokka\n\nThe quokka sleeps.\n",
    );
    let (o, started) = opened(&p, &["quokka"]);
    assert!(o.index.is_none() && started == 1, "{:?}", o.notes);
    assert!(!p.index.exists(), "the question built no index");
    assert!(
        o.notes.iter().any(|n| n.contains("not built")),
        "{:?}",
        o.notes
    );

    // behind: brought current in the question's path, the new file answered, nothing started
    let p = history("behind-index");
    p.file("mock/research/wombat.md", "# Wombat\n\nThe wombat digs.\n");
    commit(&p, None, "docs: the wombat");
    let (o, started) = opened(&p, &["wombat"]);
    assert_eq!(started, 0, "{:?}", o.notes);
    assert!(o.behind.is_empty(), "{:?}", o.behind);
    assert!(
        o.notes[0].contains("brought current first"),
        "{:?}",
        o.notes
    );
    let out = p.ask(&no_rows(), &["wombat"]);
    assert!(
        output(&out).contains("mock/research/wombat.md:"),
        "{}",
        out.output
    );

    // THE CONTROL: `--status` refreshes nothing, so the same change reads as behind there
    p.file("mock/research/koala.md", "# Koala\n");
    let (o, _) = opened(&p, &["--status"]);
    assert!(!o.behind.is_empty(), "status read only: {:?}", o.notes);
    assert!(
        o.notes.iter().all(|n| !n.contains("brought current")),
        "{:?}",
        o.notes
    );
}
