//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `a-ticket-graph-holds`: the committed tickets break none of the rules the makers refuse by.
//!
//! A hand edit or a merge can leave a cycle in `needs`, a ticket in a state its kind does not list, a
//! held ticket with no holder, or a ticket advanced past a need that is not met. The makers refuse
//! each of those at write time; this is the gate for the ways round them.
//!
//! The registry's files are read through the view the lint is handed, by [`registry_texts`], which
//! the `tickets` tool reads by too over a view of its own, so the two cannot read a different set of
//! files.

use std::path::Path;

use mockspace::{Lint, LintError, RepoContext, RepoLint, Severity, View};
use planning::{NAMESPACES, Plan};
use ticket_model::{Store, rules};

use crate::tree_walk::with_extension;

const NAME: &str = "a-ticket-graph-holds";

/// The lint, for the pack.
pub fn repo_lint() -> Box<dyn RepoLint> {
    Box::new(ATicketGraphHolds)
}

struct ATicketGraphHolds;

impl Lint for ATicketGraphHolds {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

impl RepoLint for ATicketGraphHolds {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        let registry = ctx.mock_dir.join("registry");
        let say = |message: String| LintError::error("registry".to_string(), 0, NAME, message);
        let texts = match registry_texts(ctx.view(), &registry) {
            Ok(t) => t,
            Err(e) => return vec![say(e)],
        };
        let pairs = || texts.iter().map(|(p, s)| (p.as_str(), s.as_str()));
        match (Store::from_texts(pairs()), Plan::from_texts(&NAMESPACES, pairs())) {
            (Ok(store), Ok(plan)) => rules::check_store(&store)
                .into_iter()
                .chain(planning::rules::check_plan(&plan))
                .map(|f| say(format!("`{}`: {}", f.row, f.message)))
                .collect(),
            (Err(e), _) | (_, Err(e)) => vec![say(e)],
        }
    }
}

/// Every TOML file under `registry`, wherever it sits, with its path and its text, read through
/// `view`.
///
/// # Errors
///
/// When a file the listing holds cannot be read as text.
pub fn registry_texts(view: &View<'_>, registry: &Path) -> Result<Vec<(String, String)>, String> {
    let mut texts = Vec::new();
    for file in with_extension(view, registry, "toml", &[]) {
        let src = view
            .text(&file)
            .ok_or_else(|| format!("cannot read {}", file.display()))?;
        texts.push((file.display().to_string(), src.to_string()));
    }
    Ok(texts)
}

#[cfg(test)]
#[path = "a_ticket_graph_holds_tests.rs"]
mod tests;
