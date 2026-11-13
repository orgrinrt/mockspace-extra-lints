//! Lint: a row on a provisional stamp is never read as ratified, and a stamp says what decided it.
//!
//! While the lead designer is away, a call that would stop the work is made provisionally: the
//! coordinators converge on it, or two domain experts agree after a panel stretch, and the stamp
//! is as good as his for now. It is never the final say. He is asked, in a question ticket, to make
//! it formal, and his answer can overturn it. A registry records the stamp on the row it decided,
//! in two fields: `stamped_by`, which of the two routes stamped it, and `stamp`, what was stamped,
//! why, where it is recorded and the question ticket that asks him. The row keeps its own tier: a
//! stamp is beside the ratification ladder, not a rung on it.
//!
//! What it refuses, each kind fixed differently:
//!
//! - `half`: one of the two fields without the other. A stamp nobody owns, or an owner with no
//!   record, cannot be checked against what decided it.
//! - `no-ticket`: a `stamp` naming no ticket, written as the word `ticket` and the id in
//!   backticks. A stamp he is never asked about stays provisional forever.
//! - `on-a-ratified-row`: a stamp on a row at the ratified tier. Ratified and provisional at once
//!   is one of them wrong; when he ratifies it the stamp comes off.
//! - `ratified-still-stamped`: a stamped row that a ratified row names under its ratifying edge,
//!   for a registry whose blessing is a separate row naming what it promotes.
//!
//! A project names its registry's words under `[lints.a-provisional-stamp-is-not-a-ratification]`.
//! Every key has a default, so a registry using the common words needs no table: `namespaces`, the
//! namespaces whose rows may carry a stamp (`ruling, proposal`); `tiered`, the namespace carrying a
//! tier (`ruling`); `tier_field` and `ratified` (`rung`, `ratified`); `ratifies_field`
//! (`ratifies`). A registry with none of those namespaces gets no finding.

use std::collections::HashMap;

use mockspace_lint_rules::{Lint, LintError, RegistryView, RepoContext, RepoLint, Severity};

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "a-provisional-stamp-is-not-a-ratification";

/// The two fields the stamp is written in. Fixed rather than configured: they are the shape the
/// projects that carry a stamp agreed on, and a second spelling would be a second mechanism.
const STAMPED_BY: &str = "stamped_by";
const STAMP: &str = "stamp";

/// The configuration keys.
const NAMESPACES_KEY: &str = "namespaces";
const TIERED_KEY: &str = "tiered";
const TIER_FIELD_KEY: &str = "tier_field";
const RATIFIED_KEY: &str = "ratified";
const RATIFIES_KEY: &str = "ratifies_field";

/// The separator the engine joins a list field with, and a configured list is written with.
const JOIN: &str = ",";

/// The word a stamp names its ticket after.
const TICKET: &str = "ticket";

/// The configured lint.
pub struct AProvisionalStampIsNotARatification {
    namespaces:     Vec<String>,
    tiered:         String,
    tier_field:     String,
    ratified:       String,
    ratifies_field: String,
}

impl Default for AProvisionalStampIsNotARatification {
    fn default() -> Self {
        Self {
            namespaces:     vec!["ruling".to_string(), "proposal".to_string()],
            tiered:         "ruling".to_string(),
            tier_field:     "rung".to_string(),
            ratified:       "ratified".to_string(),
            ratifies_field: "ratifies".to_string(),
        }
    }
}

impl Lint for AProvisionalStampIsNotARatification {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }

    fn config_keys(&self) -> &[&str] {
        &[NAMESPACES_KEY, TIERED_KEY, TIER_FIELD_KEY, RATIFIED_KEY, RATIFIES_KEY]
    }

    fn configure(&mut self, params: &HashMap<String, String>) {
        if let Some(v) = params.get(NAMESPACES_KEY) {
            self.namespaces = split(v).map(str::to_string).collect();
        }
        set(&mut self.tiered, params.get(TIERED_KEY));
        set(&mut self.tier_field, params.get(TIER_FIELD_KEY));
        set(&mut self.ratified, params.get(RATIFIED_KEY));
        set(&mut self.ratifies_field, params.get(RATIFIES_KEY));
    }
}

/// Overwrites a field with a configured value, trimmed, where one was given.
fn set(slot: &mut String, value: Option<&String>) {
    if let Some(v) = value {
        *slot = v.trim().to_string();
    }
}

/// The non-empty items of a joined list.
fn split(v: &str) -> impl Iterator<Item = &str> {
    v.split(JOIN).map(str::trim).filter(|s| !s.is_empty())
}

/// A field's value, with a blank one read as absent.
fn present<'a>(reg: &'a RegistryView, q: &str, field: &str) -> Option<&'a str> {
    reg.field(q, field).filter(|v| !v.trim().is_empty())
}

/// Whether a stamp names its ticket: the word, then an id in backticks.
fn names_a_ticket(stamp: &str) -> bool {
    stamp.match_indices(TICKET).any(|(at, _)| {
        let rest = stamp[at + TICKET.len() ..].trim_start();
        rest.strip_prefix('`')
            .and_then(|r| r.split_once('`'))
            .is_some_and(|(id, _)| !id.trim().is_empty() && !id.contains(char::is_whitespace))
    })
}

fn finding(kind: &'static str, message: String) -> LintError {
    let mut e = LintError::error("registry".to_string(), 0, NAME, message);
    e.finding_kind = Some(kind);
    e
}

impl RepoLint for AProvisionalStampIsNotARatification {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        let reg = ctx.registry();
        let mut out = Vec::new();
        // Each row a ratified row promotes, with the row promoting it.
        let promoted: Vec<(String, &str)> = reg
            .rows_in(&self.tiered)
            .iter()
            .filter(|r| {
                reg.field(r, &self.tier_field).map(str::trim) == Some(self.ratified.as_str())
            })
            .flat_map(|r| {
                reg.field(r, &self.ratifies_field)
                    .into_iter()
                    .flat_map(split)
                    .map(move |t| (t.rsplit("::").next().unwrap_or(t).to_string(), r.as_str()))
            })
            .collect();
        for ns in &self.namespaces {
            for q in reg.rows_in(ns) {
                let by = present(reg, q, STAMPED_BY);
                let stamp = present(reg, q, STAMP);
                if by.is_none() && stamp.is_none() {
                    continue;
                }
                match (by, stamp) {
                    (None, _) => {
                        out.push(finding(
                            "half",
                            format!(
                                "`{q}` carries `{STAMP}` and no `{STAMPED_BY}`. Say which route \
                             stamped it, the coordinators converging or two experts, or take the \
                             stamp off where neither did."
                            ),
                        ))
                    },
                    (_, None) => out.push(finding(
                        "half",
                        format!(
                            "`{q}` carries `{STAMPED_BY}` and no `{STAMP}`. Say what was stamped, \
                             why, where it is recorded and the question ticket that asks him."
                        ),
                    )),
                    (Some(_), Some(s)) if !names_a_ticket(s) => out.push(finding(
                        "no-ticket",
                        format!(
                            "`{q}`'s `{STAMP}` names no question ticket. A stamp stays provisional \
                             until he answers one; name it as {TICKET} `<id>`."
                        ),
                    )),
                    _ => {},
                }
                if *ns == self.tiered
                    && reg.field(q, &self.tier_field).map(str::trim) == Some(self.ratified.as_str())
                {
                    out.push(finding(
                        "on-a-ratified-row",
                        format!(
                            "`{q}` is at `{} = \"{}\"` and carries a provisional stamp. A ratified \
                             row is past its stamp: take the stamp off, or the tier is what is wrong.",
                            self.tier_field, self.ratified
                        ),
                    ));
                }
                let slug = q.rsplit("::").next().unwrap_or(q);
                for (_, by_row) in promoted.iter().filter(|(t, _)| t == slug) {
                    out.push(finding(
                        "ratified-still-stamped",
                        format!(
                            "`{q}` carries a provisional stamp and `{by_row}` ratifies it. The \
                             stamp comes off when he ratifies what it decided."
                        ),
                    ));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests;
