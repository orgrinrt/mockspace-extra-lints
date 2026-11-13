//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The rows a planning edit appends. A fact is never edited: a new row names in `over` the head of
//! the chain it continues, and a chain that is a fork already is refused, since there is then no one
//! row to continue. Every row is checked against the plan with it added before it is handed out.

use kaski_tickets::Fields;

use crate::plan::Plan;
use crate::time::day_of;
use crate::{Row, rules};

/// A value a new row holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Text(String),
    Flag(bool),
}

/// Where a new row sits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Home {
    /// In the item file of this ticket, beside its moves.
    Ticket(String),
    /// In a planning file of its own, named by its stem.
    Plan(&'static str),
}

/// A row to append.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewRow {
    pub ns:     &'static str,
    pub id:     String,
    pub home:   Home,
    pub fields: Vec<(String, Value)>,
}

/// Who wrote a fact, when, and the tag that keeps two writes of one second apart.
#[derive(Debug, Clone, Copy)]
pub struct Stamp<'a> {
    pub at:  &'a str,
    pub by:  &'a str,
    pub tag: &'a str,
}

impl Stamp<'_> {
    fn id(&self, prefix: &str) -> String {
        let digits: String = self
            .at
            .chars()
            .filter(char::is_ascii_digit)
            .take(14)
            .collect();
        format!("{prefix}{digits}_{}_{}", self.by, self.tag)
    }
}

fn slug(text: &str, what: &str) -> Result<(), String> {
    if !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
    {
        Ok(())
    } else {
        Err(format!(
            "{what} `{text}` is not a slug of lowercase letters, digits and underscores"
        ))
    }
}

fn day(text: &str, what: &str) -> Result<(), String> {
    if text.is_empty() || day_of(text).is_some() {
        Ok(())
    } else {
        Err(format!("{what} `{text}` is not a date YYYY-MM-DD"))
    }
}

fn exists(plan: &Plan, ns: &str, id: &str) -> Result<(), String> {
    if plan.row(ns, id).is_some() {
        Ok(())
    } else {
        Err(format!("there is no `{ns}` row `{id}`"))
    }
}

fn fields_of(fields: &[(String, Value)]) -> Fields {
    fields
        .iter()
        .map(|(k, v)| {
            let text = match v {
                Value::Text(t) => t.clone(),
                Value::Flag(f) => f.to_string(),
            };
            (k.clone(), vec![text])
        })
        .collect()
}

/// The row, refused where the plan with it added breaks a rule the plan did not break before.
fn checked(plan: &Plan, row: NewRow) -> Result<NewRow, String> {
    if plan.row(row.ns, &row.id).is_some() {
        return Err(format!("`{}::{}` is held already", row.ns, row.id));
    }
    let before = rules::check_plan(plan);
    let mut with = plan.clone();
    with.rows.push(Row {
        ns:     row.ns.to_string(),
        id:     row.id.clone(),
        fields: fields_of(&row.fields),
    });
    match rules::check_plan(&with)
        .into_iter()
        .find(|f| !before.contains(f))
    {
        None => Ok(row),
        Some(f) => Err(format!("{}: {}", f.row, f.message)),
    }
}

/// The row that continues the chain of `subject` in `ns`.
fn continued(
    plan: &Plan,
    ns: &'static str,
    subject_field: &str,
    subject: &str,
    prefix: &str,
    stamp: Stamp,
    mut fields: Vec<(String, Value)>,
) -> Result<NewRow, String> {
    let heads = plan.heads(ns, subject_field, subject);
    if heads.len() > 1 {
        return Err(format!(
            "the `{ns}` rows of `{subject}` are a fork already, {} rows over one predecessor, so there is no one row to continue",
            heads.len()
        ));
    }
    let mut all = vec![(subject_field.to_string(), Value::Text(subject.to_string()))];
    all.append(&mut fields);
    if let Some(h) = heads.first() {
        all.push(("over".into(), Value::Text(h.id.clone())));
    }
    all.push(("at".into(), Value::Text(stamp.at.to_string())));
    all.push(("by".into(), Value::Text(stamp.by.to_string())));
    checked(plan, NewRow {
        ns,
        id: stamp.id(prefix),
        home: Home::Ticket(subject.to_string()),
        fields: all,
    })
}

/// Puts a ticket in a span, or takes it out with `None`, as core work or not.
pub fn member_fact(
    plan: &Plan,
    ticket: &str,
    span: Option<&str>,
    core: bool,
    stamp: Stamp,
) -> Result<NewRow, String> {
    let mut fields = Vec::new();
    if let Some(s) = span {
        exists(plan, "plan_span", s)?;
        fields.push(("span".to_string(), Value::Text(s.to_string())));
    }
    fields.push(("core".to_string(), Value::Flag(core)));
    continued(plan, "member", "ticket", ticket, "pm", stamp, fields)
}

/// Sets a ticket's planned bar. A day left empty is unplanned.
pub fn window_fact(
    plan: &Plan,
    ticket: &str,
    start: &str,
    end: &str,
    stamp: Stamp,
) -> Result<NewRow, String> {
    day(start, "the planned start")?;
    day(end, "the planned end")?;
    if let (Some(s), Some(e)) = (day_of(start), day_of(end))
        && e < s
    {
        return Err(format!(
            "the planned end {end} is before the planned start {start}"
        ));
    }
    let fields = [("planned_start", start), ("planned_end", end)]
        .into_iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| (k.to_string(), Value::Text(v.to_string())))
        .collect();
    continued(plan, "window", "ticket", ticket, "w", stamp, fields)
}

/// Targets a ticket at a milestone, or takes it out of scope with `None`.
pub fn scope_fact(
    plan: &Plan,
    ticket: &str,
    milestone: Option<&str>,
    stamp: Stamp,
) -> Result<NewRow, String> {
    let mut fields = Vec::new();
    if let Some(m) = milestone {
        exists(plan, "milestone", m)?;
        fields.push(("milestone".to_string(), Value::Text(m.to_string())));
    }
    continued(
        plan,
        "milestone_scope",
        "ticket",
        ticket,
        "sc",
        stamp,
        fields,
    )
}

/// A new epoch inside a stretch, owned by a coordinator, with planned days where given.
pub fn epoch_fact(
    plan: &Plan,
    id: &str,
    title: &str,
    what: &str,
    within: &str,
    owner: &str,
    start: &str,
    end: &str,
) -> Result<NewRow, String> {
    slug(id, "the epoch id")?;
    if title.is_empty() || what.is_empty() || owner.is_empty() {
        return Err("an epoch needs a title, a sentence on what it is and an owner".into());
    }
    exists(plan, "plan_span", within)?;
    day(start, "the planned start")?;
    day(end, "the planned end")?;
    if let (Some(s), Some(e)) = (day_of(start), day_of(end))
        && e < s
    {
        return Err(format!(
            "the planned end {end} is before the planned start {start}"
        ));
    }
    let mut fields: Vec<(String, Value)> = vec![
        ("title".into(), Value::Text(title.into())),
        ("what".into(), Value::Text(what.into())),
        ("planned".into(), Value::Flag(true)),
        ("within".into(), Value::Text(within.into())),
        ("owner".into(), Value::Text(owner.into())),
    ];
    for (k, v) in [("planned_start", start), ("planned_end", end)] {
        if !v.is_empty() {
            fields.push((k.into(), Value::Text(v.into())));
        }
    }
    checked(plan, NewRow {
        ns: "plan_span",
        id: id.to_string(),
        home: Home::Plan("spans"),
        fields,
    })
}

/// A new milestone, a goalpost until its merge is recorded, with a goal and a planned day where given.
pub fn milestone_fact(
    plan: &Plan,
    id: &str,
    title: &str,
    goal: &str,
    planned_on: &str,
) -> Result<NewRow, String> {
    slug(id, "the milestone id")?;
    if title.is_empty() {
        return Err("a milestone needs a title".into());
    }
    day(planned_on, "the planned day")?;
    let mut fields: Vec<(String, Value)> = vec![("title".into(), Value::Text(title.into()))];
    for (k, v) in [("goal", goal), ("planned_on", planned_on)] {
        if !v.is_empty() {
            fields.push((k.into(), Value::Text(v.into())));
        }
    }
    checked(plan, NewRow {
        ns: "milestone",
        id: id.to_string(),
        home: Home::Plan("milestones"),
        fields,
    })
}
