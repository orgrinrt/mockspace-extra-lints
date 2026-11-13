//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What the planning rows and the tickets together say, derived and never stored: the class of a
//! ticket, its actual days, whether it is blocked or stalled, its slip, and a span or a milestone as
//! the sitrep needs it.

use std::collections::{BTreeMap, BTreeSet};

use kaski_tickets::{State, Store};

use crate::time::day_of;
use crate::{Plan, Row};

/// What a ticket's state means to progress, read from the state's own flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Class {
    /// Not begun, or held by nobody.
    Open,
    /// Held with its needs met: work is going on.
    Active,
    /// Held with its needs not met: it waits.
    Stopped,
    /// Closed and satisfied.
    Done,
    /// Closed and not satisfied: left out of the work.
    Dropped,
}

impl Class {
    /// Every class, in the order a sitrep groups them.
    pub const ALL: [Class; 5] =
        [Class::Active, Class::Stopped, Class::Open, Class::Done, Class::Dropped];

    /// The class a state puts a ticket in.
    pub const fn of(state: &State) -> Class {
        Class::from_flags(
            state.closed,
            state.needs_holder,
            state.needs_met,
            state.satisfies,
        )
    }

    /// The class the four flags of a state stand for: the one rule, for a caller that holds the flags
    /// without a `State`.
    pub const fn from_flags(closed: bool, held: bool, met: bool, satisfies: bool) -> Class {
        if closed {
            if satisfies { Class::Done } else { Class::Dropped }
        } else if held {
            if met { Class::Active } else { Class::Stopped }
        } else {
            Class::Open
        }
    }

    /// The word a view shows.
    pub const fn label(self) -> &'static str {
        match self {
            Class::Open => "open",
            Class::Active => "active",
            Class::Stopped => "stopped",
            Class::Done => "done",
            Class::Dropped => "dropped",
        }
    }

    /// Whether the ticket is finished with.
    pub const fn closed(self) -> bool {
        matches!(self, Class::Done | Class::Dropped)
    }
}

/// How many tickets stand in each class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Counts {
    pub open:    usize,
    pub active:  usize,
    pub stopped: usize,
    pub done:    usize,
    pub dropped: usize,
}

impl Counts {
    fn add(&mut self, class: Class) {
        match class {
            Class::Open => self.open += 1,
            Class::Active => self.active += 1,
            Class::Stopped => self.stopped += 1,
            Class::Done => self.done += 1,
            Class::Dropped => self.dropped += 1,
        }
    }

    /// The tickets that count toward progress: every class but the dropped.
    pub const fn counted(&self) -> usize {
        self.open + self.active + self.stopped + self.done
    }

    /// Done over counted, or nothing where none counts.
    pub const fn progress(&self) -> Option<(usize, usize)> {
        if self.counted() == 0 { None } else { Some((self.done, self.counted())) }
    }
}

/// What is derived of one ticket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketFacts {
    pub class:        Class,
    /// Whether the kind's role row says it is work that counts toward progress.
    pub work:         bool,
    /// Whether the kind's role row says it is a complication.
    pub complication: bool,
    /// The day of the first move into a state that starts work; nothing where none is known.
    pub start:        Option<i64>,
    /// The day of the move that closed it; nothing while it is open.
    pub end:          Option<i64>,
    /// The day of its newest move.
    pub last_move:    Option<i64>,
    /// Work, not closed, needing a ticket no state of theirs satisfies yet.
    pub blocked:      bool,
    /// Active, and its newest move is further back than the threshold.
    pub stalled:      bool,
    /// The planned start and end of the first window of its chain.
    pub window:       Option<(Option<i64>, Option<i64>)>,
    /// Days past the first planned end it ended on, or is still open on; negative when it ended early.
    pub slip:         Option<i64>,
}

/// The derived facts of every ticket of a store, for one plan, one day and one stall threshold.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Facts {
    pub today:      i64,
    pub stall_days: i64,
    tickets:        BTreeMap<String, TicketFacts>,
}

fn flag(r: &Row, field: &str) -> bool {
    r.one(field) == "true"
}

/// The first window of a ticket's chain: the row naming nothing in `over`, the oldest of them
/// where a fork left several.
fn first_window<'a>(plan: &'a Plan, ticket: &str) -> Option<&'a Row> {
    plan.rows_of("window")
        .filter(|r| r.one("ticket") == ticket && r.one("over").is_empty())
        .min_by(|a, b| (a.one("at"), &a.id).cmp(&(b.one("at"), &b.id)))
}

impl Facts {
    /// Derives the facts of every ticket of `store`.
    pub fn of(plan: &Plan, store: &Store, today: i64, stall_days: i64) -> Facts {
        let starts: BTreeSet<&str> = plan
            .rows_of("plan_state_role")
            .filter(|r| flag(r, "starts"))
            .map(|r| r.one("ticket_state"))
            .collect();
        let roles: BTreeMap<&str, (bool, bool)> = plan
            .rows_of("plan_kind_role")
            .map(|r| {
                (
                    r.one("ticket_kind"),
                    (flag(r, "work"), flag(r, "complication")),
                )
            })
            .collect();
        let mut tickets = BTreeMap::new();
        for t in store.tickets.values() {
            let moves: Vec<_> = store.moves_of(&t.id).collect();
            let state_id = moves
                .last()
                .map(|m| m.to.clone())
                .or_else(|| store.kinds.get(t.one("kind")).map(|k| k.initial.clone()));
            let class = state_id
                .as_deref()
                .and_then(|s| store.states.get(s))
                .map_or(Class::Open, Class::of);
            let (work, complication) = roles.get(t.one("kind")).copied().unwrap_or_default();
            let first_start = moves.iter().position(|m| starts.contains(m.to.as_str()));
            let start = first_start.and_then(|i| day_of(&moves[i].at));
            let end = if class.closed() {
                moves
                    .iter()
                    .skip(first_start.unwrap_or(0))
                    .find(|m| store.states.get(&m.to).is_some_and(|s| s.closed))
                    .and_then(|m| day_of(&m.at))
            } else {
                None
            };
            let last_move = moves.last().and_then(|m| day_of(&m.at));
            let blocked = work
                && !class.closed()
                && t.many("needs")
                    .iter()
                    .filter(|n| store.tickets.contains_key(n.as_str()))
                    .any(|n| !store.state_of(n).is_some_and(|s| s.satisfies));
            let stalled =
                class == Class::Active && last_move.is_some_and(|d| today - d > stall_days);
            let window = first_window(plan, &t.id)
                .map(|w| (day_of(w.one("planned_start")), day_of(w.one("planned_end"))));
            let slip = window
                .and_then(|(_, planned_end)| planned_end)
                .and_then(|pe| {
                    match class {
                        Class::Dropped => None,
                        Class::Done => end.map(|e| e - pe),
                        _ => Some(today - pe).filter(|late| *late > 0),
                    }
                });
            tickets.insert(t.id.clone(), TicketFacts {
                class,
                work,
                complication,
                start,
                end,
                last_move,
                blocked,
                stalled,
                window,
                slip,
            });
        }
        Facts {
            today,
            stall_days,
            tickets,
        }
    }

    pub fn get(&self, ticket: &str) -> Option<&TicketFacts> {
        self.tickets.get(ticket)
    }

    /// Every ticket with its facts, in id order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &TicketFacts)> {
        self.tickets.iter()
    }

    /// The counts of these tickets by class, those `keep` passes.
    pub fn counts<'a>(
        &self,
        ids: impl IntoIterator<Item = &'a String>,
        keep: impl Fn(&TicketFacts) -> bool,
    ) -> Counts {
        let mut c = Counts::default();
        for f in ids
            .into_iter()
            .filter_map(|i| self.get(i))
            .filter(|f| keep(f))
        {
            c.add(f.class);
        }
        c
    }
}

/// The milestone each ticket is targeted at now, by the newest head of its scope chain; a ticket
/// whose newest scope row names none is not in the map.
pub fn scope_now(plan: &Plan) -> BTreeMap<String, String> {
    let tickets: BTreeSet<&str> = plan
        .rows_of("milestone_scope")
        .map(|r| r.one("ticket"))
        .collect();
    let mut now = BTreeMap::new();
    for t in tickets {
        let mut heads = plan.heads("milestone_scope", "ticket", t);
        heads.sort_by(|a, b| (a.one("at"), &a.id).cmp(&(b.one("at"), &b.id)));
        if let Some(h) = heads.last().filter(|h| !h.one("milestone").is_empty()) {
            now.insert(t.to_string(), h.one("milestone").to_string());
        }
    }
    now
}

/// A span as the sitrep needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanReport {
    pub id:            String,
    pub title:         String,
    pub what:          String,
    /// True for an epoch, false for a stretch.
    pub planned:       bool,
    pub owner:         String,
    pub within:        String,
    /// Every ticket in the span now, a stretch's including those of the epochs within it.
    pub members:       Vec<String>,
    /// The members that are core work of the span.
    pub core:          Vec<String>,
    /// The counts by class of the core work.
    pub counts:        Counts,
    pub blocked:       Vec<String>,
    pub stalled:       Vec<String>,
    pub complications: Vec<String>,
    /// Members that are scope of a milestone already reached and are not done.
    pub late_scope:    Vec<String>,
    pub start:         Option<i64>,
    pub end:           Option<i64>,
    pub planned_start: Option<i64>,
    pub planned_end:   Option<i64>,
}

impl SpanReport {
    /// Done over counted among the core work, nothing where none counts.
    pub const fn progress(&self) -> Option<(usize, usize)> {
        self.counts.progress()
    }
}

/// The report of the span `id`, or nothing where there is none.
pub fn span_report(plan: &Plan, facts: &Facts, id: &str) -> Option<SpanReport> {
    let span = plan.row("plan_span", id)?;
    let planned = flag(span, "planned");
    let here: BTreeSet<&str> = std::iter::once(id)
        .chain(
            plan.rows_of("plan_span")
                .filter(|r| !planned && r.one("within") == id)
                .map(|r| r.id.as_str()),
        )
        .collect();
    let now = plan.members_now();
    let mut members = Vec::new();
    let mut core = Vec::new();
    for (ticket, (span, is_core)) in &now {
        if here.contains(span.as_str()) && facts.get(ticket).is_some() {
            members.push(ticket.clone());
            if *is_core {
                core.push(ticket.clone());
            }
        }
    }
    let scope = scope_now(plan);
    let listed = |keep: &dyn Fn(&TicketFacts) -> bool| -> Vec<String> {
        members
            .iter()
            .filter(|m| facts.get(m).is_some_and(keep))
            .cloned()
            .collect()
    };
    let late_scope = members
        .iter()
        .filter(|m| {
            scope.get(*m).is_some_and(|ms| plan.reached(ms))
                && facts
                    .get(m)
                    .is_some_and(|f| f.class != Class::Done && f.class != Class::Dropped)
        })
        .cloned()
        .collect();
    let days = |pick: fn(&TicketFacts) -> Option<i64>| -> Vec<i64> {
        members
            .iter()
            .filter_map(|m| facts.get(m).and_then(pick))
            .collect()
    };
    Some(SpanReport {
        id: id.to_string(),
        title: span.one("title").to_string(),
        what: span.one("what").to_string(),
        planned,
        owner: span.one("owner").to_string(),
        within: span.one("within").to_string(),
        counts: facts.counts(&core, |f| f.work),
        blocked: listed(&|f| f.blocked),
        stalled: listed(&|f| f.stalled),
        complications: listed(&|f| f.complication),
        late_scope,
        start: days(|f| f.start).into_iter().min(),
        end: days(|f| f.end).into_iter().max(),
        planned_start: day_of(span.one("planned_start")),
        planned_end: day_of(span.one("planned_end")),
        members,
        core,
    })
}

/// A milestone as the sitrep needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MilestoneReport {
    pub id:         String,
    pub title:      String,
    pub goal:       String,
    pub reached:    bool,
    /// The day of its first merge row.
    pub merged_on:  Option<i64>,
    pub planned_on: Option<i64>,
    /// The tickets targeted at it now.
    pub scope:      Vec<String>,
    /// The counts by class of the scope's work.
    pub counts:     Counts,
    pub blocked:    Vec<String>,
    pub stalled:    Vec<String>,
}

impl MilestoneReport {
    pub const fn progress(&self) -> Option<(usize, usize)> {
        self.counts.progress()
    }
}

/// The report of the milestone `id`, or nothing where there is none.
pub fn milestone_report(plan: &Plan, facts: &Facts, id: &str) -> Option<MilestoneReport> {
    let m = plan.row("milestone", id)?;
    let scope: Vec<String> = scope_now(plan)
        .into_iter()
        .filter(|(t, ms)| ms == id && facts.get(t).is_some())
        .map(|(t, _)| t)
        .collect();
    let listed = |keep: fn(&TicketFacts) -> bool| -> Vec<String> {
        scope
            .iter()
            .filter(|s| facts.get(s).is_some_and(keep))
            .cloned()
            .collect()
    };
    Some(MilestoneReport {
        id: id.to_string(),
        title: m.one("title").to_string(),
        goal: m.one("goal").to_string(),
        reached: plan.reached(id),
        merged_on: plan
            .rows_of("milestone_merge")
            .filter(|r| r.one("milestone") == id)
            .filter_map(|r| day_of(r.one("at")))
            .min(),
        planned_on: day_of(m.one("planned_on")),
        counts: facts.counts(&scope, |f| f.work),
        blocked: listed(|f| f.blocked),
        stalled: listed(|f| f.stalled),
        scope,
    })
}
