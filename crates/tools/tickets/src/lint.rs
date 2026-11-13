//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `a-ticket-graph-holds`: the committed tickets break none of the rules the makers refuse by.
//!
//! A hand edit or a merge can leave a cycle in `needs`, a ticket in a state its kind does not list, a
//! held ticket with no holder, or a ticket advanced past a need that is not met. The makers refuse
//! each of those at write time; this is the gate for the ways round them.

use kaski_tickets::rules;
use mockspace::{Lint, LintError, RepoContext, RepoLint, Severity};

use crate::registry_dir;

const NAME: &str = "a-ticket-graph-holds";

/// The lint.
pub struct ATicketGraphHolds;

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
        let registry = registry_dir(ctx.mock_dir);
        match (crate::load(&registry), crate::load_plan(&registry)) {
            (Ok(store), Ok(plan)) => {
                rules::check_store(&store)
                    .into_iter()
                    .chain(kaski_planning::rules::check_plan(&plan))
                    .map(|f| {
                        LintError::error(
                            "registry".to_string(),
                            0,
                            NAME,
                            format!("`{}`: {}", f.row, f.message),
                        )
                    })
                    .collect()
            },
            (Err(e), _) | (_, Err(e)) => vec![LintError::error("registry".to_string(), 0, NAME, e)],
        }
    }
}
