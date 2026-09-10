//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What the coverage test files plant their registries with and run the tool
//! through.
//!
//! A directory rather than a file beside the tests, so cargo does not build it
//! as a test target of its own.

use std::collections::BTreeMap;

use mockspace_extra_lints::tools::coverage::{Coverage, Reach, reach};
use mockspace_lint_rules::RegistryView;
use mockspace_lint_rules::tool::{Outcome, Tool, ToolContext};

/// A registry with the rows a test names.
///
/// The reverse edges are passed empty throughout, and deliberately: nothing here
/// reads `referrers`. Every edge this tool walks is a forward one it reads off
/// the row itself, which is what lets it tell an edge from a ruling apart from
/// an edge from a retirement. The engine's reverse index knows a row is
/// referenced and does not know through which field, and the field is the whole
/// of what decides a tier.
pub fn view(rows: &[(&str, &[(&str, &str)])]) -> RegistryView {
    let mut r: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for (q, fields) in rows {
        r.insert(
            (*q).to_string(),
            fields
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        );
    }
    RegistryView::new(r, BTreeMap::new())
}

pub fn run(v: &RegistryView, args: &[&str]) -> (Outcome, String) {
    let crates = Default::default();
    let dirs: Vec<std::path::PathBuf> = Vec::new();
    let ctx = ToolContext {
        mock_dir: std::path::Path::new("."),
        repo_root: std::path::Path::new("."),
        all_crates: &crates,
        src_dirs: &dirs,
        args,
        stdin: None,
        registry: v,
    };
    let rep = Coverage.run(&ctx);
    // An inconclusive verdict carries its reason on the outcome and leaves
    // `output` empty, so a test reading `output` alone cannot tell a refusal
    // from a silent pass.
    let text = match &rep.outcome {
        Outcome::Inconclusive {
            reason,
        } => reason.clone(),
        _ => rep.output.clone(),
    };
    (rep.outcome, text)
}

/// The demand namespace the fixtures use.
///
/// One of the two real spellings rather than an invented one, so the arms read
/// against a shape that exists. The parallel arms in the corpus file plant the
/// other.
pub const NS: &str = "obligation";

/// The one demand row every fixture is about.
pub const DEMAND: (&str, &[(&str, &str)]) = ("obligation::the_thing", &[("what", "a demand")]);

/// A proposal naming the demand row with nothing stamping it.
pub fn unstamped() -> RegistryView {
    view(&[DEMAND, ("proposal::a_claim", &[("obligation", "the_thing")])])
}

/// The tier the fixtures put `the_thing` at.
pub fn tier(v: &RegistryView) -> Reach {
    reach(v, NS)["the_thing"].0
}
