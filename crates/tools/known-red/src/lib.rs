//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `mock known-red-set` and `mock known-red`: the catalogue of the tests red on `dev`.
//!
//! A helper that meets a red on its branch asks `known-red` before it builds `dev` to learn whose
//! the red is. The rows are claims about `dev` at the sha they carry, and are the registry's
//! `known_red` namespace. The design is `mock/WORKFLOW.md.tmpl`, "Known reds".

use mockspace::tool::{ArgSpec, NotALint, Purpose, Tool, ToolContext, ToolReport};

mod store;
#[cfg(test)]
mod tests;

use store::Red;

/// The value of `--name` in `args`, none when the flag is absent or last.
fn flag<'a>(args: &[&'a str], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| *a == name)
        .and_then(|i| args.get(i + 1))
        .copied()
}

/// Adds or clears a row.
pub struct KnownRedSet;

impl Tool for KnownRedSet {
    fn name(&self) -> &'static str {
        "known-red-set"
    }

    fn description(&self) -> &'static str {
        "adds a test red on dev to the catalogue, or clears it"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Make {
            writes: &["mock/registry/known_red/reds.toml"],
            roots:  &[],
        }
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[]
    }

    fn value_flags(&self) -> &[&'static str] {
        &["--crate", "--test", "--sha", "--by", "--why", "--at"]
    }

    fn help(&self) -> &'static str {
        "Add: --crate, --test (the filter as `cargo mock test -p <crate> -- <filter>` takes it), --sha (the dev sha it was seen red at), --by and --why (a ticket id or a line), and --at (now by default). A row of the same crate and test is replaced. Clear: --clear with --crate and --test, refused when there is no such row. Silent on success."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        match set(ctx) {
            Ok(()) => ToolReport::reported("", 1),
            Err(e) => ToolReport::inconclusive(e),
        }
    }
}

fn set(ctx: &ToolContext<'_>) -> Result<(), String> {
    let known = ["--clear", "--crate", "--test", "--sha", "--by", "--why", "--at"];
    let mut i = 0;
    while i < ctx.args.len() {
        let a = ctx.args[i];
        if !known.contains(&a) {
            return Err(format!("known-red-set takes no `{a}`"));
        }
        i += if a == "--clear" { 1 } else { 2 };
    }
    let need = |name: &str| -> Result<String, String> {
        flag(ctx.args, name)
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .ok_or_else(|| format!("known-red-set wants {name}"))
    };
    let krate = need("--crate")?;
    let test = need("--test")?;
    let mut rows = store::load(ctx.mock_dir)?;
    let at = rows.iter().position(|r| r.krate == krate && r.test == test);
    if ctx.args.contains(&"--clear") {
        let Some(at) = at else {
            return Err(format!("no known red `{krate} {test}` to clear"));
        };
        rows.remove(at);
        return store::save(ctx.mock_dir, rows);
    }
    let row = Red {
        id: store::id_of(&krate, &test),
        krate,
        test,
        sha: need("--sha")?,
        by: need("--by")?,
        why: need("--why")?,
        at: flag(ctx.args, "--at").map_or_else(
            || tickets::time::format_utc(tickets::time::now_secs()),
            str::to_string,
        ),
    };
    match at {
        Some(at) => rows[at] = row,
        None => rows.push(row),
    }
    store::save(ctx.mock_dir, rows)
}

/// Asks whether a test is in the catalogue.
pub struct KnownRed;

impl Tool for KnownRed {
    fn name(&self) -> &'static str {
        "known-red"
    }

    fn description(&self) -> &'static str {
        "answers whether a crate's test is a known red on dev, and at what sha"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Check(NotALint::TakesAQuestion)
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[
            ArgSpec {
                name:        "crate",
                required:    true,
                description: "the crate the test is in",
            },
            ArgSpec {
                name:        "test",
                required:    true,
                description: "the test filter as `cargo mock test -p <crate> -- <filter>` takes it",
            },
        ]
    }

    fn help(&self) -> &'static str {
        "With a crate and a filter, prints `known <sha> <why>` when the catalogue holds the test and `unknown` when it does not; the answer is the first word of the output, whatever the exit."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let rows = match store::load(ctx.mock_dir) {
            Ok(r) => r,
            Err(e) => return ToolReport::inconclusive(e),
        };
        let out = match ctx.args {
            [krate, test] => {
                rows.iter()
                    .find(|r| r.krate == *krate && r.test == *test)
                    .map_or_else(
                        || "unknown\n".to_string(),
                        |r| format!("known {} {}\n", r.sha, r.why),
                    )
            },
            _ => return ToolReport::inconclusive("known-red takes a crate and a test filter"),
        };
        ToolReport::reported(out, rows.len())
    }
}

mockspace::lint_pack! {
    tools: [KnownRedSet, KnownRed],
}
