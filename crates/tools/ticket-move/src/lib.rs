//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `mock ticket-move <ticket> <state>`: a move appended to a ticket's file.
//!
//! Silent on success. What it refuses, and why, is the `tickets` crate's rules; this reads the command
//! line, hands the request over, and with `--for` rewrites the placeholder markers that cite the
//! ticket to cite its successor. What a marker is, and which files are read for one, is
//! `mock/lints/fixme_markers.rs`, which the lint that refuses a marker citing a closed ticket and
//! `owed` read by too, so the three cannot disagree.

use std::path::Path;

use mockspace::tool::{ArgSpec, Purpose, Tool, ToolContext, ToolReport};

#[path = "../../../lints/fixme_markers.rs"]
pub mod fixme_markers;
#[path = "../../../lints/tree_walk.rs"]
pub mod tree_walk;

/// The live files that hold a marker citing `from`, each with the text it has once the marker cites
/// `to`. Nothing is written; this only reads.
fn recited(repo: &Path, from: &str, to: &str) -> Vec<(String, String)> {
    fixme_markers::sources(repo)
        .into_iter()
        .filter(|s| s.scope == fixme_markers::Scope::Live)
        .filter_map(|s| fixme_markers::recite(&s.text, s.kind, from, to).map(|text| (s.rel, text)))
        .collect()
}

/// The live markers that cite `ticket`, each as its file and line, in the files `recited` reads.
fn citing(repo: &Path, ticket: &str) -> Vec<(String, usize)> {
    fixme_markers::sources(repo)
        .into_iter()
        .filter(|s| s.scope == fixme_markers::Scope::Live)
        .flat_map(|s| {
            fixme_markers::scan(&s.text, s.kind)
                .into_iter()
                .filter(|m| m.tag.as_deref().is_some_and(|t| t.trim() == ticket))
                .map(|m| (s.rel.clone(), m.line))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The refusal for a move that closes a ticket and leaves markers citing it, or nothing where the move
/// does not close the ticket, hands the markers on with `--for`, closes a question with an answer, or
/// leaves no marker behind. A marker citing a closed ticket that nobody answered is an error of the lint
/// `every-fixme-names-a-ticket` on every branch that merges the move, so it is refused here, before the
/// move is written, rather than found red on the trunk.
fn closing_leaves_markers(
    repo: &Path,
    mock_dir: &Path,
    request: &tickets::write::MoveRequest,
) -> Option<String> {
    let answered = !request.answer.trim().is_empty() || !request.picks.is_empty();
    if request.to.is_empty() || request.successor.is_some() || answered {
        return None;
    }
    let store = tickets::load(&tickets::registry_dir(mock_dir)).ok()?;
    if !store.states.get(&request.to).is_some_and(|s| s.closed) {
        return None;
    }
    let found = citing(repo, &request.ticket);
    if found.is_empty() {
        return None;
    }
    let shown: Vec<String> = found
        .iter()
        .take(5)
        .map(|(rel, line)| format!("{rel}:{line}"))
        .collect();
    let more = found.len().saturating_sub(shown.len());
    Some(format!(
        "`{}` is cited by {} live marker(s) ({}{}), and closing it with no successor would make the lint \
         `every-fixme-names-a-ticket` refuse each of them on every branch that merges the move. Close it \
         with `--for <open ticket>` to hand them over, with `--answer` if it is a question and the work its \
         answer sets is owed, or resolve the markers first. Nothing is written.",
        request.ticket,
        found.len(),
        shown.join(", "),
        if more > 0 { format!(" and {more} more") } else { String::new() }
    ))
}

/// The maker.
pub struct TicketMove;

impl Tool for TicketMove {
    fn name(&self) -> &'static str {
        "ticket-move"
    }

    fn description(&self) -> &'static str {
        "appends a move to a ticket, which is its new state and holder, or records its links"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Make {
            writes: fixme_markers::WRITES,
            roots:  &[],
        }
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[
            ArgSpec {
                name:        "ticket",
                required:    false,
                description: "the ticket's id; none with --repoint-superseded or --close-gone",
            },
            ArgSpec {
                name:        "state",
                required:    false,
                description: "the state to move it to; none records the links given and appends no move",
            },
        ]
    }

    fn value_flags(&self) -> &[&'static str] {
        tickets::args::MOVE_FLAGS
    }

    fn help(&self) -> &'static str {
        "--close-gone, with no ticket, moves every open ticket all of whose names the tree lost (a crate or a path it held once and holds no more) to its kind's closing state, the one closed state of the kind that does not satisfy, with a why naming what was deleted and the commit that deleted it: one with an open child, an open ticket that needs it, a kind a question's answer closes, or a holder is left to the list `tickets check` prints. --for names the open ticket that takes over the live markers citing each ticket closed; without it a ticket a live marker cites stays open and is named. The sweep repeats until a pass closes none, writes the registry and leaves the commit to the caller. --repoint-superseded, with no ticket, writes every ticket naming a superseded tenet that has exactly one heir to name the heir in `related` and `tenets`, deduplicated, and leaves a ticket about the demotion (the row in its title, or the tag demoted-row) as it is. --kind, --related, --branch, --session, --needs, --part-of, --tags, --set, --priority and --choice (one or many, how many picks a question takes) record those fields of the ticket's row, alone or with a move, and --span and --core append a member row of the planning layer: a value replaces what the field held, --tenets, --subjects, --needs and --tags take a comma separated list, and an empty value clears the field. --add-tags and --remove-tags edit the tags the ticket holds instead, taking off and then adding the ones named and keeping the rest, and are refused beside --tags, which replaces the whole list. A --needs or a --part-of that names no ticket, the ticket itself or a cycle is refused. --for names the ticket that takes over the placeholder markers citing this one, and belongs to a move: every live marker that cites this ticket is rewritten to cite it, in the files the lint that refuses a marker citing a closed ticket reads, so a ticket the markers cite can be closed in one act; the closed rounds, the research notes, the generated documents and the vendored code are left as they are. A successor that is no ticket, is this ticket, or is closed is refused, and then nothing is written. A move to a closed state while a live marker still cites the ticket is refused too, naming up to five of the markers, unless it carries --for, or an --answer or --picks: a marker citing a ticket closed with nobody's answer is an error of that lint on every branch that merges the move, while one citing a question that was answered is the work the answer sets, which the lint reports and does not refuse. A tag is a free string that marks the ticket, such as provisional-stamp. --kind changes the kind of a standing ticket, which is how a task becomes a job: it is judged as the kind it becomes, in the state it stands in, so a job in a held state needs its brief and branch in the same call. A move is judged against the ticket as the links leave it, so a job gets its branch in the call that starts it. --holder, --release, --why, --answer and --picks belong to a move and are refused without a state. --answer gives the words a question is answered with and --picks the options chosen, separated by a vertical bar as the options are; a move to a state that satisfies a kind listing `satisfied_by` carries one of them, and a move to any other state carries neither. Nothing is written when a rule refuses. The holder carries forward from the last move unless --holder names another or --release lets it go. --why says why, --by and --at give the author and the time the move was made (the environment's channel name and now by default), which is how a move written later keeps the time it happened."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        if ctx.args.contains(&"--repoint-superseded") {
            return repoint_superseded(ctx);
        }
        if ctx.args.contains(&"--close-gone") {
            return close_gone(ctx);
        }
        let request = match tickets::args::move_(ctx.args) {
            Ok(r) => r,
            Err(e) => return ToolReport::inconclusive(e),
        };
        if let Err(e) = tickets::refs::Known::of(ctx.registry).refuse(&request.links) {
            return ToolReport::inconclusive(e);
        }
        if let Some(refusal) = closing_leaves_markers(ctx.repo_root, ctx.mock_dir, &request) {
            return ToolReport::inconclusive(refusal);
        }
        // read before the move is written, so that a move the rules refuse leaves every file as it was
        let rewrites = request
            .successor
            .as_deref()
            .map(|to| recited(ctx.repo_root, &request.ticket, to))
            .unwrap_or_default();
        if let Err(e) = tickets::write::move_ticket(&tickets::registry_dir(ctx.mock_dir), &request)
        {
            return ToolReport::inconclusive(e);
        }
        for (rel, text) in &rewrites {
            if let Err(e) = std::fs::write(ctx.repo_root.join(rel), text) {
                return ToolReport::inconclusive(format!(
                    "the move of `{}` is written and `{rel}` could not be rewritten: {e}",
                    request.ticket
                ));
            }
        }
        ToolReport::reported("", 1)
    }
}

/// `--repoint-superseded`: the tickets naming a superseded tenet that has one heir are written to name it.
/// Silent on success; a ticket the rules refuse is named and the others are still written.
fn repoint_superseded(ctx: &ToolContext<'_>) -> ToolReport {
    let flag = |name: &str| {
        ctx.args
            .iter()
            .position(|a| *a == name)
            .and_then(|i| ctx.args.get(i + 1))
    };
    let by = tickets::args::by(flag("--by"));
    let at = tickets::args::at(flag("--at"));
    match tickets::repoint::repoint_superseded(
        &tickets::registry_dir(ctx.mock_dir),
        ctx.registry,
        &by,
        &at,
    ) {
        Ok(written) => ToolReport::reported("", written.len()),
        Err((written, refused)) => {
            ToolReport::inconclusive(format!(
                "{} ticket(s) written; refused:\n{refused}",
                written.len()
            ))
        },
    }
}

/// `--close-gone`: the open tickets whose only subject the tree lost are moved to their kind's closing
/// state. With `--for`, the live markers citing each are handed to that ticket; without it a ticket a
/// live marker cites is left open and named. It reports the open tickets it examined, and apart from that how
/// many it closed.
fn close_gone(ctx: &ToolContext<'_>) -> ToolReport {
    let flag = |name: &str| {
        ctx.args
            .iter()
            .position(|a| *a == name)
            .and_then(|i| ctx.args.get(i + 1))
    };
    let by = tickets::args::by(flag("--by"));
    let at = tickets::args::at(flag("--at"));
    let successor = flag("--for").copied();
    let cited = |id: &str| !citing(ctx.repo_root, id).is_empty();
    let git = tickets::landing::Git::at(ctx.repo_root);
    let swept = tickets::close_gone::close_gone(
        &tickets::registry_dir(ctx.mock_dir),
        &git,
        &tickets::close_gone::Sweep {
            by: &by,
            at: &at,
            successor,
            cited: &cited,
        },
    );
    let (done, refused) = match swept {
        Ok(done) => (done, None),
        Err((done, refused)) => (done, Some(refused)),
    };
    if let Some(to) = successor {
        for id in &done.closed {
            for (rel, text) in recited(ctx.repo_root, id, to) {
                if let Err(e) = std::fs::write(ctx.repo_root.join(&rel), text) {
                    return ToolReport::inconclusive(format!(
                        "`{id}` is closed and `{rel}` could not be rewritten: {e}"
                    ));
                }
            }
        }
    }
    if let Some(refused) = refused {
        return ToolReport::inconclusive(format!(
            "{} ticket(s) closed; refused:\n{refused}",
            done.closed.len()
        ));
    }
    let mut out = format!("closed {}\n", done.closed.len());
    for id in &done.left {
        out.push_str(&format!(
            "`{id}` is cited by a live marker and stays open: --for names the ticket that takes it\n"
        ));
    }
    ToolReport::reported(out, done.examined)
}

mockspace::lint_pack! {
    tools: [TicketMove],
}

#[cfg(test)]
mod tests {
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
        put(&s, "mock/research/n/NOTES.md", "// §(old_work): quoted\n");
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

    #[test]
    fn the_records_the_generated_documents_and_vendored_code_are_left_alone() {
        let s = Scratch::new();
        pair(&s);
        let quoted = "// §(old_work): quoted\n";
        for rel in [
            "mock/design_rounds/202610010000/x.md",
            "mock/research/n/NOTES.md",
            "docs/GEN.md",
            "mock/vendor/v/src/lib.rs",
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
            "mock/design_rounds/202610010000/x.md",
            "mock/research/n/NOTES.md",
            "docs/GEN.md",
            "mock/vendor/v/src/lib.rs",
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
}

#[cfg(test)]
mod tests_repoint;

#[cfg(test)]
mod tests_close_gone;
