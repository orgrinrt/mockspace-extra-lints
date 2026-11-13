//! Lint: a provisional stamp is never a ratification, and says what decided it.
//!
//! While the lead designer is away, a call that would stop the work is made provisionally: the
//! coordinators converge on it, two domain experts agree after a panel stretch, or he himself
//! directs the row onto a stamp, his passage cited in the stamp. The row then binds as its
//! standing does. It is never the final say. He is asked, in a question ticket, to make it formal,
//! and his answer can overturn it. A registry records the stamp on the row it decided, in two
//! fields: `stamped_by`, which of the three routes stamped it, and `stamp`, what was stamped, why,
//! where it is recorded and the question ticket that asks him. The row keeps its own tier: a stamp
//! is beside the ratification ladder, not a rung on it.
//!
//! What it refuses, each kind fixed differently:
//!
//! - `half`: one of the two fields without the other. A stamp nobody owns, or an owner with no
//!   record, cannot be checked against what decided it.
//! - `no-ticket`: a `stamp` naming no ticket, written as the word `ticket` and the id in
//!   backticks. A stamp he is never asked about stays provisional forever.
//! - `no-passage`: a stamp by the maintainer citing no passage of his as a backticked
//!   `<path>:<line>`. His direction is the whole of what decided it, so it is cited.
//! - `on-a-ratified-row`: a stamp on a row at the ratified tier. Ratified and provisional at once
//!   is one of them wrong; when he ratifies it the stamp comes off.
//! - `not-stampable`: where `stampable` names the tiers a stamp may sit on, a stamp on a row at any
//!   other.
//! - `on-a-superseded-row`: a stamp on a row another row supersedes, which nothing stands on,
//!   whether the newer row names it or it names its heir.
//! - `ratified-still-stamped`: a stamped row that a ratified row names under its ratifying edge,
//!   for a registry whose blessing is a separate row naming what it promotes.
//!
//! A project names its registry's words under `[lints.a-provisional-stamp-is-not-a-ratification]`.
//! Every key has a default, so a registry using the common words needs no table: `namespaces`, the
//! namespaces whose rows may carry a stamp (`ruling, proposal`); `tiered`, the namespace carrying a
//! tier (`ruling`); `tier_field` and `ratified` (`rung`, `ratified`); `ratifies_field`
//! (`ratifies`); `stampable`, the tiers a stamp may sit on (every tier but the ratified one when
//! unset); `supersedes_field` (`supersedes`), read on the newer row naming what it replaces;
//! `superseded_by_field` (`superseded_by`), read on the replaced row naming what replaces it. A
//! registry with none of those namespaces gets no finding.

use std::collections::HashMap;

use mockspace_lint_rules::{Lint, LintError, RegistryView, RepoContext, RepoLint, Severity};

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "a-provisional-stamp-is-not-a-ratification";

/// The two fields the stamp is written in, the routes and the ticket word, from the crate the canon
/// tool reads stamps through too. Fixed rather than configured: they are the shape the projects that
/// carry a stamp agreed on, and a second spelling would be a second mechanism.
use mockspace_extra_stamp::{MAINTAINER, STAMP, STAMPED_BY, TICKET};

/// The configuration keys.
const NAMESPACES_KEY: &str = "namespaces";
const TIERED_KEY: &str = "tiered";
const TIER_FIELD_KEY: &str = "tier_field";
const RATIFIED_KEY: &str = "ratified";
const RATIFIES_KEY: &str = "ratifies_field";
const STAMPABLE_KEY: &str = "stampable";
const SUPERSEDES_KEY: &str = "supersedes_field";
const SUPERSEDED_BY_KEY: &str = "superseded_by_field";

/// The separator the engine joins a list field with, and a configured list is written with.
const JOIN: &str = ",";

/// The configured lint.
pub struct AProvisionalStampIsNotARatification {
    namespaces:     Vec<String>,
    tiered:         String,
    tier_field:     String,
    ratified:       String,
    ratifies_field: String,
    stampable:      Vec<String>,
    supersedes:     String,
    superseded_by:  String,
}

impl Default for AProvisionalStampIsNotARatification {
    fn default() -> Self {
        Self {
            namespaces:     vec!["ruling".to_string(), "proposal".to_string()],
            tiered:         "ruling".to_string(),
            tier_field:     "rung".to_string(),
            ratified:       "ratified".to_string(),
            ratifies_field: "ratifies".to_string(),
            stampable:      Vec::new(),
            supersedes:     "supersedes".to_string(),
            superseded_by:  "superseded_by".to_string(),
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
        &[
            NAMESPACES_KEY,
            TIERED_KEY,
            TIER_FIELD_KEY,
            RATIFIED_KEY,
            RATIFIES_KEY,
            STAMPABLE_KEY,
            SUPERSEDES_KEY,
            SUPERSEDED_BY_KEY,
        ]
    }

    fn configure(&mut self, params: &HashMap<String, String>) {
        if let Some(v) = params.get(NAMESPACES_KEY) {
            self.namespaces = split(v).map(str::to_string).collect();
        }
        set(&mut self.tiered, params.get(TIERED_KEY));
        set(&mut self.tier_field, params.get(TIER_FIELD_KEY));
        set(&mut self.ratified, params.get(RATIFIED_KEY));
        set(&mut self.ratifies_field, params.get(RATIFIES_KEY));
        if let Some(v) = params.get(STAMPABLE_KEY) {
            self.stampable = split(v).map(str::to_string).collect();
        }
        set(&mut self.supersedes, params.get(SUPERSEDES_KEY));
        set(&mut self.superseded_by, params.get(SUPERSEDED_BY_KEY));
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

/// Whether a stamp cites a passage: a backticked `<path>:<line>`, the path holding a dot.
fn cites_a_passage(stamp: &str) -> bool {
    stamp.split('`').skip(1).step_by(2).any(|span| {
        span.rsplit_once(':').is_some_and(|(path, line)| {
            path.contains('.')
                && !path.contains(char::is_whitespace)
                && !line.is_empty()
                && line.chars().all(|c| c.is_ascii_digit() || c == '-')
                && line.starts_with(|c: char| c.is_ascii_digit())
        })
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
        // Each row another row supersedes, with the row superseding it.
        let superseding: Vec<(String, &str)> = self
            .namespaces
            .iter()
            .flat_map(|ns| reg.rows_in(ns))
            .flat_map(|r| {
                reg.field(r, &self.supersedes)
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
                             stamped it, the coordinators converging, two experts or the \
                             maintainer directing it, or take the stamp off where none did."
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
                    (Some(_), Some(s)) if mockspace_extra_stamp::ticket(s).is_none() => out.push(finding(
                        "no-ticket",
                        format!(
                            "`{q}`'s `{STAMP}` names no question ticket. A stamp stays provisional \
                             until he answers one; name it as {TICKET} `<id>`."
                        ),
                    )),
                    _ => {},
                }
                if by == Some(MAINTAINER) && stamp.is_some_and(|s| !cites_a_passage(s)) {
                    out.push(finding(
                        "no-passage",
                        format!(
                            "`{q}` is stamped by the {MAINTAINER} and its `{STAMP}` cites no \
                             passage of his. Name where he directed it, as a backticked \
                             `<path>:<line>`."
                        ),
                    ));
                }
                let tier = reg.field(q, &self.tier_field).map(str::trim);
                if tier == Some(self.ratified.as_str()) {
                    out.push(finding(
                        "on-a-ratified-row",
                        format!(
                            "`{q}` is at `{} = \"{}\"` and carries a provisional stamp. A ratified \
                             row is past its stamp: take it off, or the tier is what is wrong.",
                            self.tier_field, self.ratified
                        ),
                    ));
                } else if let Some(t) = tier.filter(|t| {
                    !self.stampable.is_empty() && !self.stampable.iter().any(|s| s == t)
                }) {
                    out.push(finding(
                        "not-stampable",
                        format!(
                            "`{q}` is at `{} = \"{t}\"` and carries a provisional stamp. A stamp \
                             sits only on a row at {}: take it off, or move the row.",
                            self.tier_field,
                            self.stampable.join(", ")
                        ),
                    ));
                }
                let slug = q.rsplit("::").next().unwrap_or(q);
                // The rows replacing this one, from either end of the relation, each named once.
                let mut heirs: Vec<&str> =
                    superseding.iter().filter(|(t, _)| t == slug).map(|(_, r)| *r).collect();
                for heir in present(reg, q, &self.superseded_by).into_iter().flat_map(split) {
                    let tail = heir.rsplit("::").next().unwrap_or(heir);
                    if !heirs.iter().any(|h| h.rsplit("::").next().unwrap_or(h) == tail) {
                        heirs.push(heir);
                    }
                }
                for by_row in heirs {
                    out.push(finding(
                        "on-a-superseded-row",
                        format!(
                            "`{q}` carries a provisional stamp and `{by_row}` supersedes it. A \
                             stamp on a row nothing stands on approves nothing: take it off."
                        ),
                    ));
                }
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
