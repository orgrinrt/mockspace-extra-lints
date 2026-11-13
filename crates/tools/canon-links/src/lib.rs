//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `mock canon-links [view]`: how well the canon's rows are joined and found.
//!
//! A report from the registry alone. `totals` counts keywords and relations and names the rows joined
//! to nothing, `rows` is one line a tenet, and `probes` puts the committed probe set to `canon find`
//! and `canon related` as they answer now, so a pass over the metadata can be measured against the
//! one before it.

use mockspace::tool::{ArgSpec, NotALint, Purpose, Tool, ToolContext, ToolReport};

mod measure;
mod probes;
mod render;

pub use measure::{Counts, THIN_KEYWORDS, Totals, counts, totals};
pub use probes::{Outcome, Probe, Via, parse, run};

/// The views, in the order the help lists them.
pub const VIEWS: [&str; 3] = ["totals", "rows", "probes"];

/// Where the probe set sits, under the mock directory.
pub const PROBES: &str = "tools/canon-links/probes.toml";

/// The report.
pub struct CanonLinks;

impl Tool for CanonLinks {
    fn name(&self) -> &'static str {
        "canon-links"
    }

    fn description(&self) -> &'static str {
        "how well the canon's rows are joined and found: keywords, relations and a probe set"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Check(NotALint::NoFailingCase)
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[ArgSpec {
            name:        "view",
            required:    false,
            description: "`totals` (the default), `rows` or `probes`",
        }]
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let all = canon::tenets(ctx.registry);
        let examined = all.len();
        match ctx.args.first().copied().unwrap_or("totals") {
            "totals" => ToolReport::reported(render::totals_view(&all), examined),
            "rows" => ToolReport::reported(render::rows_view(&all), examined),
            "probes" => {
                let path = ctx.mock_dir.join(PROBES);
                let text = match std::fs::read_to_string(&path) {
                    Ok(t) => t,
                    Err(e) => {
                        return ToolReport::inconclusive(format!(
                            "the probe set at {} could not be read: {e}",
                            path.display()
                        ));
                    },
                };
                match parse(&text) {
                    Ok(p) => ToolReport::reported(render::probes_view(&run(&all, &p)), examined),
                    Err(e) => ToolReport::inconclusive(e),
                }
            },
            other => {
                ToolReport::inconclusive(format!(
                    "there is no view `{other}`; the views are {}",
                    VIEWS.join(", ")
                ))
            },
        }
    }
}

mockspace::lint_pack! {
    tools: [CanonLinks],
}

#[cfg(test)]
mod tests;
