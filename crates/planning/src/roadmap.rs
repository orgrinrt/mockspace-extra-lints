//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The timeline of the whole project, as days: the stretches and the milestones, the epochs in
//! lanes by owner, and a bar for each ticket with its planned and its actual days.

use std::collections::{BTreeMap, BTreeSet};

use crate::Plan;
use crate::derive::{Class, Facts, span_report};
use crate::time::day_of;

/// A start and an end, either of which may be unknown.
pub type Days = (Option<i64>, Option<i64>);

/// A milestone on the timeline: a fact where it has merged, a goalpost where it has not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mark {
    pub id:      String,
    pub title:   String,
    pub goal:    String,
    pub reached: bool,
    /// The day it merged, or the day planned for it; nothing for a goalpost with no date.
    pub day:     Option<i64>,
}

/// A stretch or an epoch on the timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interval {
    pub id:      String,
    pub title:   String,
    pub planned: Days,
    pub actual:  Days,
}

/// The epochs one coordinator owns, which run beside those of the other lanes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lane {
    pub owner:  String,
    pub epochs: Vec<Interval>,
}

/// One ticket's bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bar {
    pub ticket:  String,
    /// The span it is in now, empty where it is in none.
    pub span:    String,
    pub class:   Class,
    pub planned: Days,
    pub actual:  Days,
    pub slip:    Option<i64>,
}

/// The timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Roadmap {
    /// The first and last day anything on it, or today, falls on.
    pub first:     i64,
    pub last:      i64,
    pub marks:     Vec<Mark>,
    pub stretches: Vec<Interval>,
    pub lanes:     Vec<Lane>,
    pub bars:      Vec<Bar>,
}

impl Roadmap {
    /// The timeline of `plan`. With `epoch`, only that span and the tickets in it; with `keep`, only
    /// the tickets whose ids it holds.
    pub fn of(
        plan: &Plan,
        facts: &Facts,
        epoch: Option<&str>,
        keep: Option<&BTreeSet<String>>,
    ) -> Roadmap {
        let marks: Vec<Mark> = plan
            .rows_of("milestone")
            .map(|m| {
                let merged = plan
                    .rows_of("milestone_merge")
                    .filter(|r| r.one("milestone") == m.id)
                    .filter_map(|r| day_of(r.one("at")))
                    .min();
                Mark {
                    id:      m.id.clone(),
                    title:   m.one("title").to_string(),
                    goal:    m.one("goal").to_string(),
                    reached: plan.reached(&m.id),
                    day:     merged.or_else(|| day_of(m.one("planned_on"))),
                }
            })
            .collect();
        let day_of_mark = |id: &str| marks.iter().find(|m| m.id == id).and_then(|m| m.day);
        let interval = |id: &str, planned: Days| -> Interval {
            let report = span_report(plan, facts, id);
            Interval {
                id: id.to_string(),
                title: report.as_ref().map(|r| r.title.clone()).unwrap_or_default(),
                planned,
                actual: report.map_or((None, None), |r| (r.start, r.end)),
            }
        };
        let mut stretches = Vec::new();
        let mut lanes: BTreeMap<String, Vec<Interval>> = BTreeMap::new();
        for s in plan.rows_of("plan_span") {
            if s.one("planned") == "true" {
                if epoch.is_none_or(|e| e == s.id || e == s.one("within")) {
                    lanes
                        .entry(s.one("owner").to_string())
                        .or_default()
                        .push(interval(
                            &s.id,
                            (day_of(s.one("planned_start")), day_of(s.one("planned_end"))),
                        ));
                }
            } else if epoch.is_none() || epoch == Some(s.id.as_str()) {
                stretches.push(interval(
                    &s.id,
                    (day_of_mark(s.one("after")), day_of_mark(s.one("until"))),
                ));
            }
        }
        let now = plan.members_now();
        let in_scope = |ticket: &str| -> bool {
            keep.is_none_or(|k| k.contains(ticket))
                && epoch.is_none_or(|e| {
                    now.get(ticket).is_some_and(|(span, _)| {
                        span == e
                            || plan
                                .row("plan_span", span)
                                .is_some_and(|s| s.one("within") == e)
                    })
                })
        };
        let bars: Vec<Bar> = facts
            .iter()
            .filter(|(id, f)| f.work && in_scope(id))
            .map(|(id, f)| {
                Bar {
                    ticket:  id.clone(),
                    span:    now.get(id).map(|(s, _)| s.clone()).unwrap_or_default(),
                    class:   f.class,
                    planned: f.window.unwrap_or((None, None)),
                    actual:  (f.start, f.end),
                    slip:    f.slip,
                }
            })
            .collect();
        let days: Vec<i64> = marks
            .iter()
            .filter_map(|m| m.day)
            .chain(
                stretches
                    .iter()
                    .flat_map(|i| [i.planned, i.actual])
                    .flat_map(|d| [d.0, d.1])
                    .flatten(),
            )
            .chain(
                lanes
                    .values()
                    .flatten()
                    .flat_map(|i| [i.planned, i.actual])
                    .flat_map(|d| [d.0, d.1])
                    .flatten(),
            )
            .chain(
                bars.iter()
                    .flat_map(|b| [b.planned, b.actual])
                    .flat_map(|d| [d.0, d.1])
                    .flatten(),
            )
            .chain(std::iter::once(facts.today))
            .collect();
        Roadmap {
            first: days.iter().copied().min().unwrap_or(facts.today),
            last: days.iter().copied().max().unwrap_or(facts.today),
            marks,
            stretches,
            lanes: lanes
                .into_iter()
                .map(|(owner, epochs)| {
                    Lane {
                        owner,
                        epochs,
                    }
                })
                .collect(),
            bars,
        }
    }
}
