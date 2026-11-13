//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The rules of a move, shown by adding and moving tickets in a scratch registry that holds the
//! vocabularies the repository ships: a ticket is refused what the rules refuse, and let through
//! once the thing refused is put right, so a refusal is never a refusal of everything.

use kaski_tickets::rules;

use crate::testkit::Scratch;
use crate::view;

const T0: &str = "2026-10-23T10:00:00Z";
const T1: &str = "2026-10-23T11:00:00Z";
const T2: &str = "2026-10-23T12:00:00Z";

/// A kind the repository does not ship, listing three states, for the rule that a kind lists them.
fn with_small_kind(s: &Scratch) {
    s.append(
        "ticket/small.toml",
        "[[ticket_kind]]\nid = \"small\"\ntitle = \"Small\"\nwhat = \"A kind that allows three states.\"\nstates = [\"backlog\", \"ready\", \"done\"]\ninitial = \"backlog\"\n",
    );
}

#[test]
fn a_ticket_is_added_with_a_first_move_in_its_kinds_initial_state() {
    let s = Scratch::new();
    let id = s
        .add("Wire the lamp", &[
            ("what", &["by the door"]),
            ("repos", &["orgrinrt/kaski", "orgrinrt/clause-dev"]),
        ])
        .unwrap();
    assert_eq!(id, "wire_the_lamp");
    let store = crate::load(&s.root).unwrap();
    let t = &store.tickets[&id];
    assert_eq!(t.one("kind"), "task");
    assert_eq!(t.many("repos"), ["orgrinrt/kaski", "orgrinrt/clause-dev"]);
    let now = store.now_of(&id).unwrap();
    assert_eq!(
        (now.to.as_str(), now.by.as_str(), now.at.as_str()),
        ("backlog", "tester", T0)
    );
    assert!(
        s.read("ticket/items/wire_the_lamp.toml")
            .contains("[[ticket_move]]")
    );
    assert!(rules::check_store(&store).is_empty());
}

#[test]
fn a_title_with_quotes_and_lines_comes_back_as_it_went_in() {
    let s = Scratch::new();
    let title = "Say \"hello\" \\ again";
    let what = "line one\nline two\ttabbed";
    let id = s.add(title, &[("what", &[what])]).unwrap();
    let store = crate::load(&s.root).unwrap();
    assert_eq!(store.tickets[&id].one("title"), title);
    assert_eq!(store.tickets[&id].one("what"), what);
}

#[test]
fn an_id_already_taken_is_refused_and_a_named_one_is_not() {
    let s = Scratch::new();
    s.add("Same title", &[]).unwrap();
    let err = s.add("Same title", &[]).unwrap_err();
    assert!(err.contains("`same_title` is a ticket already"), "{err}");
    s.add_as("Same title", "task", None, "", &[]).unwrap_err();
}

#[test]
fn a_state_the_kind_does_not_list_is_refused() {
    let s = Scratch::new();
    with_small_kind(&s);
    let id = s.add_as("Small thing", "small", None, "", &[]).unwrap();
    let err = s.mv(&id, "doing", Some("someone"), T1).unwrap_err();
    assert!(
        err.contains("a `small` ticket may be in") && err.contains("`doing` is not among them"),
        "{err}"
    );
    s.mv(&id, "ready", None, T1).unwrap();
    s.mv(&id, "done", None, T2).unwrap();
}

#[test]
fn a_state_the_kind_does_not_list_is_refused_at_add_too() {
    let s = Scratch::new();
    with_small_kind(&s);
    let err = s
        .add_as("Small thing", "small", Some("review"), "someone", &[])
        .unwrap_err();
    assert!(err.contains("`review` is not among them"), "{err}");
}

#[test]
fn a_state_that_is_no_state_and_a_kind_that_is_no_kind_are_refused() {
    let s = Scratch::new();
    let id = s.add("Anything", &[]).unwrap();
    assert!(
        s.mv(&id, "frozen", None, T1)
            .unwrap_err()
            .contains("`frozen` is no ticket state")
    );
    assert!(
        s.add_as("Other", "epic", None, "", &[])
            .unwrap_err()
            .contains("`epic` is no ticket kind")
    );
}

#[test]
fn a_held_state_wants_a_holder_and_a_job_wants_its_brief_and_branch() {
    let s = Scratch::new();
    let task = s.add("A task", &[]).unwrap();
    assert!(
        s.mv(&task, "doing", None, T1)
            .unwrap_err()
            .contains("`doing` needs a holder")
    );
    s.mv(&task, "doing", Some("registry-tasks-design"), T1)
        .unwrap();

    let job = s.add_as("A job", "job", None, "", &[]).unwrap();
    let err = s.mv(&job, "doing", Some("helper"), T1).unwrap_err();
    assert!(err.contains("needs its `brief` filled"), "{err}");
    let ready = s
        .add_as("Another job", "job", None, "", &[("brief", &[
            "briefs/job-x.md",
        ])])
        .unwrap();
    let err = s.mv(&ready, "doing", Some("helper"), T1).unwrap_err();
    assert!(err.contains("needs its `branch` filled"), "{err}");
    let whole = s
        .add_as("Whole job", "job", Some("doing"), "helper", &[
            ("brief", &["briefs/job-y.md"]),
            ("branch", &["work/y"]),
        ])
        .unwrap();
    assert_eq!(
        crate::load(&s.root).unwrap().now_of(&whole).unwrap().holder,
        "helper"
    );
}

#[test]
fn a_ticket_is_not_advanced_past_a_need_that_is_not_met() {
    let s = Scratch::new();
    let a = s.add("Foundation", &[]).unwrap();
    let b = s.add("Walls", &[("needs", &[&a])]).unwrap();
    let err = s.mv(&b, "doing", Some("builder"), T1).unwrap_err();
    assert!(err.contains("`foundation` (in `backlog`)"), "{err}");
    // control: the same move goes through once the need is met, and waiting needs nothing met
    s.mv(&b, "waiting", Some("builder"), T1).unwrap();
    s.mv(&a, "done", None, T1).unwrap();
    s.mv(&b, "doing", None, T2).unwrap();
}

#[test]
fn a_dropped_need_stays_unmet() {
    let s = Scratch::new();
    let a = s.add("Foundation", &[]).unwrap();
    let b = s.add("Walls", &[("needs", &[&a])]).unwrap();
    // the dependent has to close first: nothing open may still need a dropped ticket
    assert!(
        s.mv(&a, "dropped", None, T1)
            .unwrap_err()
            .contains("`walls` still need it")
    );
    s.mv(&b, "dropped", None, T1).unwrap();
    s.mv(&a, "dropped", None, T2).unwrap();
}

#[test]
fn a_move_names_a_ticket_that_exists_and_an_add_names_needs_that_exist() {
    let s = Scratch::new();
    assert!(
        s.mv("ghost", "ready", None, T1)
            .unwrap_err()
            .contains("`ghost` is no ticket")
    );
    let err = s.add("Lonely", &[("needs", &["ghost"])]).unwrap_err();
    assert!(
        err.contains("`needs` names `ghost`, which is no ticket"),
        "{err}"
    );
    let err = s.add("Orphan", &[("part_of", &["ghost"])]).unwrap_err();
    assert!(err.contains("`part_of` names `ghost`"), "{err}");
}

#[test]
fn the_holder_carries_forward_until_released_or_changed() {
    let s = Scratch::new();
    let id = s
        .add_as("Carried", "task", Some("doing"), "alpha", &[])
        .unwrap();
    s.mv(&id, "waiting", None, T1).unwrap();
    assert_eq!(
        crate::load(&s.root).unwrap().now_of(&id).unwrap().holder,
        "alpha"
    );
    s.mv(&id, "review", Some("beta"), T2).unwrap();
    assert_eq!(
        crate::load(&s.root).unwrap().now_of(&id).unwrap().holder,
        "beta"
    );
    let req = crate::write::MoveRequest {
        ticket: id.clone(),
        to: "ready".into(),
        release: true,
        by: "tester".into(),
        at: "2026-10-23T13:00:00Z".into(),
        ..Default::default()
    };
    crate::write::move_ticket(&s.root, &req).unwrap();
    assert_eq!(
        crate::load(&s.root).unwrap().now_of(&id).unwrap().holder,
        ""
    );
}

#[test]
fn a_cycle_a_dangling_edge_and_a_state_out_of_kind_are_found_in_written_rows() {
    let s = Scratch::new();
    with_small_kind(&s);
    let a = s.add("Chicken", &[]).unwrap();
    let b = s.add("Egg", &[("needs", &[&a])]).unwrap();
    assert!(
        rules::check_store(&crate::load(&s.root).unwrap()).is_empty(),
        "a clean store has no finding"
    );
    // hand edits the makers would have refused
    let text = s.read("ticket/items/chicken.toml").replace(
        "kind = \"task\"\n",
        "kind = \"task\"\nneeds = [\"egg\", \"ghost\"]\n",
    );
    std::fs::write(s.root.join("ticket/items/chicken.toml"), text).unwrap();
    s.append(
        "ticket/items/egg.toml",
        "\n[[ticket_move]]\nid = \"m20261023130000_hand_0000\"\nticket = \"egg\"\nto = \"frozen\"\nby = \"hand\"\nat = \"2026-10-23T13:00:00Z\"\n",
    );
    let found = rules::check_store(&crate::load(&s.root).unwrap());
    let said: Vec<String> = found
        .iter()
        .map(|f| format!("{}: {}", f.row, f.message))
        .collect();
    assert!(
        said.iter().any(|m| {
            m.contains("`needs` goes round: chicken -> egg -> chicken")
                || m.contains("`needs` goes round: egg -> chicken -> egg")
        }),
        "{said:?}"
    );
    assert!(
        said.iter()
            .any(|m| m.contains("`needs` names `ghost`, which is no ticket")),
        "{said:?}"
    );
    assert!(
        said.iter().any(|m| m.contains("which is no ticket state")),
        "{said:?}"
    );
    let _ = b;
}

#[test]
fn a_held_ticket_without_a_holder_and_a_ticket_ahead_of_its_needs_are_found() {
    let s = Scratch::new();
    let a = s.add("Prior", &[]).unwrap();
    let b = s.add("After", &[("needs", &[&a])]).unwrap();
    s.append(
        &format!("ticket/items/{b}.toml"),
        "\n[[ticket_move]]\nid = \"m20261023140000_hand_0000\"\nticket = \"after\"\nto = \"doing\"\nby = \"hand\"\nat = \"2026-10-23T14:00:00Z\"\n",
    );
    let said: Vec<String> = rules::check_store(&crate::load(&s.root).unwrap())
        .into_iter()
        .map(|f| f.message)
        .collect();
    assert!(
        said.iter()
            .any(|m| m.contains("needs a holder, and has none")),
        "{said:?}"
    );
    assert!(
        said.iter().any(|m| m.contains("while it waits on `prior`")),
        "{said:?}"
    );
}

#[test]
fn listing_is_in_board_order_and_empty_when_nothing_matches() {
    let s = Scratch::new();
    let none = view::list(
        &crate::load(&s.root).unwrap(),
        &kaski_planning::Plan::default(),
        &view::Filter::default(),
    );
    assert!(none.is_empty(), "a registry with no ticket lists none");
    let a = s.add("Backlog item", &[]).unwrap();
    let b = s
        .add_as("Doing item", "task", Some("doing"), "alpha", &[])
        .unwrap();
    let c = s.add("Ranked backlog item", &[("rank", &["1"])]).unwrap();
    let store = crate::load(&s.root).unwrap();
    let ids: Vec<String> = view::list(
        &store,
        &kaski_planning::Plan::default(),
        &view::Filter::default(),
    )
    .into_iter()
    .map(|r| r.id)
    .collect();
    assert_eq!(ids, [c, a, b]);
    let held = view::list(&store, &kaski_planning::Plan::default(), &view::Filter {
        holder: vec!["alpha".into()],
        ..Default::default()
    });
    assert_eq!(held.len(), 1);
    assert_eq!(
        view::line(&held[0], false),
        "doing_item doing alpha Doing item"
    );
    let nothing = view::list(&store, &kaski_planning::Plan::default(), &view::Filter {
        state: vec!["review".into()],
        ..Default::default()
    });
    assert!(nothing.is_empty());
}

#[test]
fn next_ranks_free_work_by_what_it_unblocks() {
    let s = Scratch::new();
    let lone = s.add_as("Lone", "task", Some("ready"), "", &[]).unwrap();
    let key = s.add_as("Key", "task", Some("ready"), "", &[]).unwrap();
    let w = s.add("Walls", &[("needs", &[&key])]).unwrap();
    s.add("Roof", &[("needs", &[&w])]).unwrap();
    let taken = s
        .add_as("Taken", "task", Some("doing"), "alpha", &[])
        .unwrap();
    s.add("Shed", &[("needs", &[&taken])]).unwrap();
    let store = crate::load(&s.root).unwrap();
    let next = view::next(
        &store,
        &kaski_planning::Plan::default(),
        &view::Filter::default(),
        &crate::testkit::Branches::default(),
    );
    let ids: Vec<String> = next.iter().map(|r| r.id.clone()).collect();
    // `key` unblocks walls and, through walls, roof; walls and roof and shed are blocked, taken is held
    assert_eq!(ids, [key.clone(), lone.clone()], "{next:?}");
    assert_eq!(next[0].unblocks, 2);
    assert_eq!(view::line(&next[0], true), "key ready - Key (unblocks 2)");
}

#[test]
fn the_arguments_are_read_into_requests_and_a_wrong_one_is_refused() {
    let r = crate::args::add(&[
        "Build", "a", "wall", "--kind", "job", "--needs", "a,b", "--by", "x", "--at", T0, "--rank",
        "3",
    ])
    .unwrap();
    assert_eq!(
        (r.title.as_str(), r.kind.as_str(), r.by.as_str()),
        ("Build a wall", "job", "x")
    );
    assert_eq!(r.fields["needs"], ["a", "b"]);
    assert_eq!(r.fields["rank"], ["3"]);
    assert!(
        crate::args::add(&["--kind", "job"])
            .unwrap_err()
            .contains("wants a title")
    );
    assert!(
        crate::args::add(&["T", "--ghost"])
            .unwrap_err()
            .contains("`--ghost` is no flag")
    );
    assert!(
        crate::args::add(&["T", "--kind"])
            .unwrap_err()
            .contains("wants a value")
    );
    let m =
        crate::args::move_(&["a_ticket", "done", "--by", "x", "--at", T1, "--release"]).unwrap();
    assert!(m.release && m.holder.is_none());
    assert!(
        crate::args::move_(&["only_one"])
            .unwrap_err()
            .contains("a state to move to, or a link")
    );
    assert!(
        crate::args::move_(&["a", "b", "--release", "--holder", "h"])
            .unwrap_err()
            .contains("opposite")
    );
    let l = crate::args::tickets(&["check"]).unwrap();
    assert_eq!(l.verb, crate::args::Verb::Check);
    let l = crate::args::tickets(&["next", "--holder", "a,b"]).unwrap();
    assert!(l.verb == crate::args::Verb::Next && l.filter.holder == ["a", "b"]);
    assert_eq!(
        crate::args::tickets(&["list"]).unwrap().verb,
        crate::args::Verb::List
    );
    assert!(
        crate::args::tickets(&[])
            .unwrap_err()
            .contains("wants a verb")
    );
    assert!(crate::args::tickets(&["frobnicate"]).unwrap_err().contains(
        "the verbs are `list`, `next`, `check`, `branch`, `landed`, `contract`, `tracks` and `answerable`"
    ));
}

#[test]
fn a_move_without_an_author_or_with_a_bad_time_is_refused() {
    let s = Scratch::new();
    let id = s.add("Timed", &[]).unwrap();
    let mut req = crate::write::MoveRequest {
        ticket: id,
        to: "ready".into(),
        at: T1.into(),
        ..Default::default()
    };
    assert!(
        crate::write::move_ticket(&s.root, &req)
            .unwrap_err()
            .contains("`--by`")
    );
    req.by = "x".into();
    req.at = "yesterday".into();
    assert!(
        crate::write::move_ticket(&s.root, &req)
            .unwrap_err()
            .contains("not a time")
    );
}

fn ask(s: &Scratch, args: &[&str]) -> mockspace::tool::ToolReport {
    use mockspace::tool::ToolContext;
    let crates = std::collections::BTreeSet::new();
    let registry = mockspace::RegistryView::default();
    crate::Tickets
        .answer(&ToolContext {
            mock_dir: &s.mock,
            repo_root: &s.mock,
            all_crates: &crates,
            src_dirs: &[],
            args,
            stdin: None,
            registry: &registry,
        })
        .1
}

#[test]
fn listing_an_empty_set_prints_nothing_and_a_non_empty_one_prints_a_line_each() {
    let s = Scratch::new();
    for verb in ["list", "next"] {
        let r = ask(&s, &[verb]);
        assert!(
            r.output.is_empty(),
            "`{verb}` on no tickets printed {:?}",
            r.output
        );
    }
    s.add("One", &[]).unwrap();
    assert_eq!(ask(&s, &["list"]).output, "one backlog - One\n");
    assert!(ask(&s, &["list", "--state", "done"]).output.is_empty());
}
