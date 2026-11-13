//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The derived values over the real registry's vocabulary: the states, the kinds and the role rows
//! the registry ships, with the few tickets, moves and planning rows each case needs written beside
//! them. A negative control stands beside every probe that could pass for the wrong reason.

use std::path::{Path, PathBuf};

use mockspace_extra_planning::derive::{Class, Facts, milestone_report, span_report};
use mockspace_extra_planning::roadmap::Roadmap;
use mockspace_extra_planning::time::{date_of, day_of};
use mockspace_extra_planning::{NAMESPACES, Plan};
use ticket_model::Store;

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            walk(&p, out);
        } else if p.extension().is_some_and(|x| x == "toml") {
            out.push(p);
        }
    }
}

fn shipped_texts() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../registry");
    let mut files = Vec::new();
    walk(&root, &mut files);
    files.sort();
    files
        .into_iter()
        .map(|p| {
            (
                p.display().to_string(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect()
}

fn borrowed(texts: &[(String, String)]) -> Vec<(&str, &str)> {
    texts
        .iter()
        .map(|(p, s)| (p.as_str(), s.as_str()))
        .collect()
}

/// The real vocabulary (states, kinds, roles) plus `extra`, read as one registry.
fn world(extra: &str) -> (Plan, Store) {
    let mut texts: Vec<(String, String)> = shipped_texts()
        .into_iter()
        .filter(|(p, _)| {
            p.ends_with("states.toml") || p.ends_with("kinds.toml") || p.ends_with("roles.toml")
        })
        .collect();
    texts.push(("extra.toml".into(), extra.to_string()));
    let files = borrowed(&texts);
    (
        Plan::from_texts(&NAMESPACES, files.iter().copied()).unwrap(),
        Store::from_texts(files.iter().copied()).unwrap(),
    )
}

fn ticket(id: &str, kind: &str, moves: &[(&str, &str)], extra: &str) -> String {
    let mut s =
        format!("[[ticket]]\nid = \"{id}\"\nkind = \"{kind}\"\ntitle = \"{id}\"\n{extra}\n");
    for (i, (to, at)) in moves.iter().enumerate() {
        let holder = if matches!(*to, "doing" | "waiting" | "review" | "held") {
            "coord"
        } else {
            ""
        };
        s += &format!(
            "[[ticket_move]]\nid = \"{id}_m{i}\"\nticket = \"{id}\"\nto = \"{to}\"\nby = \"coord\"\nat = \"{at}\"\nholder = \"{holder}\"\n"
        );
    }
    s
}

fn day(text: &str) -> i64 {
    day_of(text).unwrap()
}

const TODAY: &str = "2026-10-10";

#[test]
fn days_round_trip_and_a_text_that_is_not_a_day_is_none() {
    for t in ["1970-01-01", "2026-11-02", "2024-02-29", "1999-12-31"] {
        assert_eq!(date_of(day_of(t).unwrap()), t);
    }
    assert_eq!(day_of("2026-11-02T10:15:00Z"), day_of("2026-11-02"));
    assert_eq!(day_of("1970-01-02"), Some(1));
    for bad in [
        "",
        "2026-13-01",
        "2026-02-30",
        "2025-02-29",
        "2026/11/02",
        "2026-11-02T10:15:00",
        "soon",
    ] {
        assert_eq!(day_of(bad), None, "{bad}");
    }
}

#[test]
fn a_started_and_closed_ticket_has_actual_days_and_a_slip_against_its_first_window() {
    let extra = [
        ticket("a", "job", &[("backlog", "2026-10-01T09:00:00Z"), ("doing", "2026-10-02T09:00:00Z"), ("done", "2026-10-05T09:00:00Z")], ""),
        "[[window]]\nid = \"w1\"\nticket = \"a\"\nplanned_start = \"2026-10-01\"\nplanned_end = \"2026-10-04\"\n\n[[window]]\nid = \"w2\"\nticket = \"a\"\nplanned_start = \"2026-10-01\"\nplanned_end = \"2026-10-09\"\nover = \"w1\"\n".to_string(),
    ]
    .join("\n");
    let (plan, store) = world(&extra);
    let facts = Facts::of(&plan, &store, day(TODAY), 3);
    let a = facts.get("a").unwrap();
    assert_eq!(a.class, Class::Done);
    assert_eq!(
        (a.start, a.end),
        (Some(day("2026-10-02")), Some(day("2026-10-05")))
    );
    // counted against the first window (ends the 4th), not the later one that ends the 9th
    assert_eq!(a.slip, Some(1));
    assert_eq!(
        a.window,
        Some((Some(day("2026-10-01")), Some(day("2026-10-04"))))
    );
}

#[test]
fn a_ticket_ended_before_its_planned_end_slips_negatively_and_one_with_no_window_has_no_slip() {
    let extra = [
        ticket(
            "early",
            "job",
            &[("doing", "2026-10-02T09:00:00Z"), ("done", "2026-10-03T09:00:00Z")],
            "",
        ),
        "[[window]]\nid = \"w\"\nticket = \"early\"\nplanned_end = \"2026-10-06\"\n".to_string(),
        ticket(
            "bare",
            "job",
            &[("doing", "2026-10-02T09:00:00Z"), ("done", "2026-10-03T09:00:00Z")],
            "",
        ),
    ]
    .join("\n");
    let (plan, store) = world(&extra);
    let facts = Facts::of(&plan, &store, day(TODAY), 3);
    assert_eq!(facts.get("early").unwrap().slip, Some(-3));
    assert_eq!(facts.get("bare").unwrap().slip, None);
}

#[test]
fn an_unknown_start_stays_unknown_and_a_dropped_ticket_is_not_late() {
    let extra = [
        ticket(
            "never",
            "task",
            &[("backlog", "2026-10-01T09:00:00Z"), ("dropped", "2026-10-04T09:00:00Z")],
            "",
        ),
        "[[window]]\nid = \"w\"\nticket = \"never\"\nplanned_end = \"2026-10-02\"\n".to_string(),
        ticket("fresh", "task", &[], ""),
    ]
    .join("\n");
    let (plan, store) = world(&extra);
    let facts = Facts::of(&plan, &store, day(TODAY), 3);
    let never = facts.get("never").unwrap();
    assert_eq!(never.class, Class::Dropped);
    assert_eq!(
        never.start, None,
        "no move into a starting state, so the start is not invented"
    );
    assert_eq!(never.end, Some(day("2026-10-04")));
    assert_eq!(never.slip, None);
    // a ticket that never moved stands in its kind's initial state and has no days at all
    let fresh = facts.get("fresh").unwrap();
    assert_eq!(
        (fresh.class, fresh.start, fresh.end, fresh.last_move),
        (Class::Open, None, None, None)
    );
}

#[test]
fn an_open_ticket_past_its_planned_end_is_late_by_the_days_since_and_one_before_it_is_not() {
    let extra = [
        ticket("late", "job", &[("doing", "2026-10-02T09:00:00Z")], ""),
        "[[window]]\nid = \"wl\"\nticket = \"late\"\nplanned_end = \"2026-10-07\"\n".to_string(),
        ticket("ontime", "job", &[("doing", "2026-10-02T09:00:00Z")], ""),
        "[[window]]\nid = \"wo\"\nticket = \"ontime\"\nplanned_end = \"2026-10-20\"\n".to_string(),
    ]
    .join("\n");
    let (plan, store) = world(&extra);
    let facts = Facts::of(&plan, &store, day(TODAY), 30);
    assert_eq!(facts.get("late").unwrap().slip, Some(3));
    assert_eq!(facts.get("ontime").unwrap().slip, None);
}

#[test]
fn stalled_is_an_active_ticket_whose_newest_move_is_older_than_the_threshold() {
    let extra = ticket("slow", "job", &[("doing", "2026-10-01T09:00:00Z")], "");
    let (plan, store) = world(&extra);
    assert!(
        Facts::of(&plan, &store, day(TODAY), 3)
            .get("slow")
            .unwrap()
            .stalled
    );
    assert!(
        !Facts::of(&plan, &store, day(TODAY), 20)
            .get("slow")
            .unwrap()
            .stalled,
        "control: a long threshold"
    );
    let waiting = ticket("waits", "job", &[("waiting", "2026-10-01T09:00:00Z")], "");
    let (plan, store) = world(&waiting);
    let f = Facts::of(&plan, &store, day(TODAY), 3);
    assert_eq!(f.get("waits").unwrap().class, Class::Stopped);
    assert!(
        !f.get("waits").unwrap().stalled,
        "control: a stopped ticket waits, it does not stall"
    );
}

#[test]
fn a_work_ticket_is_blocked_by_a_need_no_state_satisfies_and_a_question_is_never_work() {
    let extra = [
        ticket("base", "job", &[("doing", "2026-10-01T09:00:00Z")], ""),
        ticket("after", "job", &[], "needs = [\"base\"]"),
        ticket("finished", "job", &[("done", "2026-10-01T09:00:00Z")], ""),
        ticket("free", "job", &[], "needs = [\"finished\"]"),
        ticket("asks", "question", &[], "needs = [\"base\"]"),
    ]
    .join("\n");
    let (plan, store) = world(&extra);
    let f = Facts::of(&plan, &store, day(TODAY), 3);
    assert!(f.get("after").unwrap().blocked);
    assert!(!f.get("free").unwrap().blocked, "control: its need is done");
    assert!(
        !f.get("asks").unwrap().blocked && !f.get("asks").unwrap().work,
        "a question is not work"
    );
}

const SPANS: &str = r#"
[[plan_span]]
id = "stretch"
title = "S"
what = "w"
planned = false

[[plan_span]]
id = "epoch"
title = "E"
what = "w"
planned = true
within = "stretch"
owner = "coord"
planned_start = "2026-10-01"
planned_end = "2026-10-20"

[[milestone]]
id = "m1"
title = "M1"
goal = "g"
planned_on = "2026-10-30"
"#;

fn member(id: &str, ticket: &str, span: &str, core: bool) -> String {
    format!("[[member]]\nid = \"{id}\"\nticket = \"{ticket}\"\nspan = \"{span}\"\ncore = {core}\n")
}

#[test]
fn a_span_counts_its_core_work_by_class_and_reports_blocked_stalled_and_complications() {
    let extra = [
        SPANS.to_string(),
        ticket("d", "job", &[("done", "2026-10-03T09:00:00Z")], ""),
        ticket("p", "job", &[("doing", "2026-10-01T09:00:00Z")], ""),
        ticket("o", "job", &[], "needs = [\"p\"]"),
        ticket("side", "job", &[], ""),
        ticket("cx", "complication", &[], ""),
        member("m_d", "d", "epoch", true),
        member("m_p", "p", "epoch", true),
        member("m_o", "o", "epoch", true),
        member("m_s", "side", "epoch", false),
        member("m_c", "cx", "epoch", true),
    ]
    .join("\n");
    let (plan, store) = world(&extra);
    let facts = Facts::of(&plan, &store, day(TODAY), 3);
    let r = span_report(&plan, &facts, "epoch").unwrap();
    assert_eq!(r.members.len(), 5);
    assert_eq!(r.core.len(), 4);
    // core work: d done, p active, o open; the non-core side ticket and the complication are not counted
    assert_eq!((r.counts.done, r.counts.active, r.counts.open), (1, 1, 1));
    assert_eq!(r.progress(), Some((1, 3)));
    assert_eq!(r.blocked, vec!["o".to_string()]);
    assert_eq!(r.stalled, vec!["p".to_string()]);
    assert_eq!(r.complications, vec!["cx".to_string()]);
    assert_eq!(r.start, Some(day("2026-10-01")));
    assert_eq!(
        (r.planned_start, r.planned_end),
        (Some(day("2026-10-01")), Some(day("2026-10-20")))
    );
    assert!(span_report(&plan, &facts, "nope").is_none());
}

#[test]
fn a_stretch_holds_the_tickets_of_the_epochs_within_it_and_an_epoch_does_not_hold_the_stretchs() {
    let extra = [
        SPANS.to_string(),
        ticket("in_epoch", "job", &[], ""),
        ticket("in_stretch", "job", &[], ""),
        member("a", "in_epoch", "epoch", true),
        member("b", "in_stretch", "stretch", true),
    ]
    .join("\n");
    let (plan, store) = world(&extra);
    let facts = Facts::of(&plan, &store, day(TODAY), 3);
    let s = span_report(&plan, &facts, "stretch").unwrap();
    assert_eq!(s.members.len(), 2);
    let e = span_report(&plan, &facts, "epoch").unwrap();
    assert_eq!(e.members, vec!["in_epoch".to_string()]);
}

#[test]
fn a_milestone_reports_its_scope_and_late_scope_is_unfinished_scope_of_a_reached_one() {
    let extra = [
        SPANS.to_string(),
        ticket("fin", "job", &[("done", "2026-10-03T09:00:00Z")], ""),
        ticket("left", "job", &[("doing", "2026-10-03T09:00:00Z")], ""),
        member("a", "fin", "epoch", true),
        member("b", "left", "epoch", true),
        "[[milestone_scope]]\nid = \"s1\"\nticket = \"fin\"\nmilestone = \"m1\"\n\n[[milestone_scope]]\nid = \"s2\"\nticket = \"left\"\nmilestone = \"m1\"\n\n[[milestone_scope]]\nid = \"s3\"\nticket = \"left\"\nover = \"s2\"\n".to_string(),
    ]
    .join("\n");
    let (plan, store) = world(&extra);
    let facts = Facts::of(&plan, &store, day(TODAY), 3);
    let m = milestone_report(&plan, &facts, "m1").unwrap();
    // `left` was taken out of scope by the newest row of its chain
    assert_eq!(m.scope, vec!["fin".to_string()]);
    assert_eq!(m.progress(), Some((1, 1)));
    assert!(!m.reached);
    assert_eq!(m.planned_on, Some(day("2026-10-30")));
    assert!(
        span_report(&plan, &facts, "epoch")
            .unwrap()
            .late_scope
            .is_empty()
    );

    // control: with `left` back in scope and the milestone merged, it is late
    let merged = extra.replace(
        "[[milestone_scope]]\nid = \"s3\"\nticket = \"left\"\nover = \"s2\"\n",
        "[[milestone_merge]]\nid = \"mm\"\nmilestone = \"m1\"\nrepo = \"kaski\"\nmerge = \"abc\"\nat = \"2026-10-09T10:00:00Z\"\nby = \"coord\"\n",
    );
    let (plan, store) = world(&merged);
    let facts = Facts::of(&plan, &store, day(TODAY), 3);
    let m = milestone_report(&plan, &facts, "m1").unwrap();
    assert!(m.reached);
    assert_eq!(m.merged_on, Some(day("2026-10-09")));
    assert_eq!(
        span_report(&plan, &facts, "epoch").unwrap().late_scope,
        vec!["left".to_string()]
    );
}

#[test]
fn the_roadmap_puts_epochs_in_lanes_by_owner_draft_milestones_as_goalposts_and_filters_to_one_epoch()
 {
    let extra = [
        SPANS.to_string(),
        "[[plan_span]]\nid = \"epoch2\"\ntitle = \"E2\"\nwhat = \"w\"\nplanned = true\nwithin = \"stretch\"\nowner = \"render-coord\"\n".to_string(),
        ticket("t1", "job", &[("doing", "2026-10-02T09:00:00Z")], ""),
        ticket("t2", "job", &[], ""),
        member("a", "t1", "epoch", true),
        member("b", "t2", "epoch2", true),
        "[[window]]\nid = \"w\"\nticket = \"t1\"\nplanned_start = \"2026-10-01\"\nplanned_end = \"2026-10-05\"\n".to_string(),
    ]
    .join("\n");
    let (plan, store) = world(&extra);
    let facts = Facts::of(&plan, &store, day(TODAY), 3);
    let all = Roadmap::of(&plan, &facts, None, None);
    let owners: Vec<&str> = all.lanes.iter().map(|l| l.owner.as_str()).collect();
    assert_eq!(owners, ["coord", "render-coord"]);
    assert_eq!(all.bars.len(), 2);
    let goal = all.marks.iter().find(|m| m.id == "m1").unwrap();
    assert!(
        !goal.reached && goal.day == Some(day("2026-10-30")),
        "a draft is a goalpost on its planned day"
    );
    assert_eq!(all.last, day("2026-10-30"));
    let bar = all.bars.iter().find(|b| b.ticket == "t1").unwrap();
    assert_eq!(
        bar.planned,
        (Some(day("2026-10-01")), Some(day("2026-10-05")))
    );
    assert_eq!(
        bar.actual,
        (Some(day("2026-10-02")), None),
        "open ended: the end is unknown"
    );

    let one = Roadmap::of(&plan, &facts, Some("epoch2"), None);
    assert_eq!(one.lanes.len(), 1);
    assert_eq!(
        one.bars
            .iter()
            .map(|b| b.ticket.as_str())
            .collect::<Vec<_>>(),
        ["t2"]
    );
    // the stretch's own name filters to all of its epochs
    assert_eq!(
        Roadmap::of(&plan, &facts, Some("stretch"), None).bars.len(),
        2
    );

    let keep: std::collections::BTreeSet<String> = ["t1".to_string()].into();
    assert_eq!(Roadmap::of(&plan, &facts, None, Some(&keep)).bars.len(), 1);
}

/// Every combination of closed, held, needs met and satisfies with the class it is, written out and not
/// derived from the rule, so the sixteen are each seen. A reader that holds the flags and not a state asks
/// `from_flags`, and a state asks `of`, which is the same answer.
#[test]
fn the_class_of_the_four_flags_is_one_function_for_all_sixteen() {
    const TABLE: [(bool, bool, bool, bool, Class); 16] = [
        (false, false, false, false, Class::Open),
        (false, false, false, true, Class::Open),
        (false, false, true, false, Class::Open),
        (false, false, true, true, Class::Open),
        (false, true, false, false, Class::Stopped),
        (false, true, false, true, Class::Stopped),
        (false, true, true, false, Class::Active),
        (false, true, true, true, Class::Active),
        (true, false, false, false, Class::Dropped),
        (true, false, false, true, Class::Done),
        (true, false, true, false, Class::Dropped),
        (true, false, true, true, Class::Done),
        (true, true, false, false, Class::Dropped),
        (true, true, false, true, Class::Done),
        (true, true, true, false, Class::Dropped),
        (true, true, true, true, Class::Done),
    ];
    for (closed, held, met, satisfies, class) in TABLE {
        assert_eq!(
            Class::from_flags(closed, held, met, satisfies),
            class,
            "closed {closed}, held {held}, met {met}, satisfies {satisfies}"
        );
        let state = ticket_model::State {
            id: "s".into(),
            column: 0,
            needs_holder: held,
            needs_met: met,
            closed,
            satisfies,
            fields: Default::default(),
        };
        assert_eq!(Class::of(&state), class);
    }
}
