//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The rules a schema cannot say over the planning rows. The schema gate holds each field to its type
//! and each reference to a row that exists; these hold that the facts of a chain agree.

use std::collections::BTreeSet;

use ticket_model::Finding;

use crate::{Plan, Row};

/// The planning namespaces that are facts about one subject, appended and naming in `over` the row
/// they replace, each with the field that names the subject. Fixed here and said so: this is the
/// planning vocabulary's own structure, not content a pack declares.
pub const FACTS: [(&str, &str); 3] =
    [("member", "ticket"), ("window", "ticket"), ("milestone_scope", "ticket")];

fn finding(row: &Row, message: String) -> Finding {
    Finding {
        row: row.address(),
        message,
    }
}

/// Holds a row's `over` to a row of the same namespace about the same subject, and the chain to no
/// cycle.
pub fn check_over(plan: &Plan) -> Vec<Finding> {
    let mut out = Vec::new();
    for (ns, subject) in FACTS {
        for r in plan.rows_of(ns) {
            let over = r.one("over");
            if over.is_empty() {
                continue;
            }
            match plan.row(ns, over) {
                None => {
                    out.push(finding(
                        r,
                        format!("`over` names `{over}`, which is no `{ns}` row"),
                    ))
                },
                Some(p) if p.one(subject) != r.one(subject) => {
                    out.push(finding(
                        r,
                        format!(
                            "`over` names `{over}`, a row of `{}` and this row is about `{}`",
                            p.one(subject),
                            r.one(subject)
                        ),
                    ))
                },
                Some(_) => {},
            }
            let mut seen = BTreeSet::from([r.id.as_str()]);
            let mut cur = r;
            while let Some(next) = plan.row(ns, cur.one("over")) {
                if !seen.insert(next.id.as_str()) {
                    out.push(finding(
                        r,
                        "`over` runs round to a row already passed".to_string(),
                    ));
                    break;
                }
                cur = next;
            }
        }
    }
    out
}

/// Names every chain with more than one head: two writers over one predecessor.
pub fn check_forks(plan: &Plan) -> Vec<Finding> {
    let mut out = Vec::new();
    for (ns, subject) in FACTS {
        let subjects: BTreeSet<&str> = plan.rows_of(ns).map(|r| r.one(subject)).collect();
        for s in subjects {
            let heads = plan.heads(ns, subject, s);
            if heads.len() > 1 {
                let ids: Vec<&str> = heads.iter().map(|r| r.id.as_str()).collect();
                out.push(finding(
                    heads[0],
                    format!(
                        "`{s}` has {} current `{ns}` rows, {}: two writers over one predecessor",
                        heads.len(),
                        ids.join(", ")
                    ),
                ));
            }
        }
    }
    out
}

/// Holds `at` and `by` of a fact to both present or both absent, the absent pair being legacy.
pub fn check_stamp(plan: &Plan) -> Vec<Finding> {
    FACTS
        .iter()
        .flat_map(|(ns, _)| plan.rows_of(ns))
        .filter(|r| r.one("at").is_empty() != r.one("by").is_empty())
        .map(|r| finding(r, "`at` and `by` come together or not at all".to_string()))
        .collect()
}

/// Holds an epoch (`planned` true) to a `within` and an `owner` and no `after` or `until`, and a
/// stretch (`planned` false) to neither `within` nor `owner`.
pub fn check_spans(plan: &Plan) -> Vec<Finding> {
    let mut out = Vec::new();
    for r in plan.rows_of("plan_span") {
        let has = |f: &str| !r.one(f).is_empty();
        match r.one("planned") {
            "true" => {
                if !has("within") {
                    out.push(finding(
                        r,
                        "an epoch names the stretch it runs in, in `within`".to_string(),
                    ));
                } else if plan
                    .row("plan_span", r.one("within"))
                    .is_none_or(|s| s.one("planned") != "false")
                {
                    out.push(finding(
                        r,
                        format!("`within` names `{}`, which is no stretch", r.one("within")),
                    ));
                }
                if !has("owner") {
                    out.push(finding(
                        r,
                        "an epoch names its coordinator, in `owner`".to_string(),
                    ));
                }
                if has("after") || has("until") {
                    out.push(finding(
                        r,
                        "an epoch has no `after` or `until`; its stretch has".to_string(),
                    ));
                }
            },
            _ => {
                if has("within") || has("owner") {
                    out.push(finding(
                        r,
                        "a stretch has no `within` or `owner`; only an epoch has".to_string(),
                    ));
                }
            },
        }
    }
    out
}

/// Every finding the committed rows break.
pub fn check_plan(plan: &Plan) -> Vec<Finding> {
    let mut out = check_over(plan);
    out.extend(check_forks(plan));
    out.extend(check_stamp(plan));
    out.extend(check_spans(plan));
    out
}
