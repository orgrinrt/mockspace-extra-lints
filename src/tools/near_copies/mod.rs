//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `mock near-copies`: types in different crates that are close without being the same, and types
//! that share a name over different declarations.
//!
//! A helper that needs a type and writes its own beside one another crate already owns leaves a
//! near-copy: the same members under a renamed field, one variant more. `single-source` gates the
//! exact copy, a name over one canonical declaration in two crates, and by design leaves both of
//! these alone, since a gate cannot tell a reinvention from two concepts that share a word. This is
//! the question asked instead, as a report in two lists, each ranked by closeness, the share of members that line up.
//!
//! # A tool and not a lint
//!
//! Where the line between a near-copy and an unrelated type falls has no settled value, and a gate
//! on one would invent it. So nothing here blocks: it reports text and no finding, it exits 0 whenever it can answer, and
//! `--min` is a knob on what is listed and not a threshold on what fails.
//!
//! # Where it lives
//!
//! In the pack, because every mockspace repository can ask it and a tool several repositories want is
//! one implementation here. It reads a crate with `declared_types`, which is `single-source`'s own
//! reading, so the files counted, the test code left out and the canonical form of a declaration are
//! the lint's and not a second reading of them.
//!
//! # What it reads by default
//!
//! The crates the branch touched against `origin/dev` (`--against`), compared with every other
//! crate. That is more than the touched crates and the ones they name a type from, deliberately:
//! the reinvention that matters is the one the author never named, since an author who named the
//! owner would have used it. `--named` narrows it to the crates a touched crate names a type from.
//! Crate names given as arguments are compared with each other alone.

mod compare;
mod render;
mod scope;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_git;

use std::collections::BTreeSet;

use compare::{Options, compare};
use mockspace_lint_rules::tool::{ArgSpec, NotALint, Purpose, Tool, ToolContext, ToolReport};

/// How many of each list are printed unless `--limit` says otherwise.
const DEFAULT_LIMIT: usize = 30;

/// A near-copy is listed from this closeness unless `--min` says otherwise. A guess to tune.
const DEFAULT_MIN_PERCENT: usize = 60;

/// The trunk a branch is read against unless `--against` says otherwise.
const DEFAULT_AGAINST: &str = "origin/dev";

pub struct NearCopies;

/// What the command line said.
#[derive(Debug, PartialEq, Eq)]
struct Args {
    crates:   Vec<String>,
    against:  String,
    min:      usize,
    limit:    usize,
    named:    bool,
    pub_only: bool,
}

fn parse(args: &[&str]) -> Result<Args, String> {
    let mut out = Args {
        crates:   Vec::new(),
        against:  DEFAULT_AGAINST.to_string(),
        min:      DEFAULT_MIN_PERCENT,
        limit:    DEFAULT_LIMIT,
        named:    false,
        pub_only: false,
    };
    let mut it = args.iter().copied();
    while let Some(arg) = it.next() {
        match arg {
            "--against" => out.against = it.next().ok_or("`--against` wants a ref")?.to_string(),
            "--min" => out.min = number("--min", it.next())?.min(100),
            "--limit" => out.limit = number("--limit", it.next())?,
            "--named" => out.named = true,
            "--pub-only" => out.pub_only = true,
            flag if flag.starts_with("--") => {
                return Err(format!("`{flag}` is no flag of near-copies"));
            },
            name => out.crates.push(name.to_string()),
        }
    }
    Ok(out)
}

fn number(flag: &str, value: Option<&str>) -> Result<usize, String> {
    value
        .ok_or_else(|| format!("`{flag}` wants a number"))?
        .parse()
        .map_err(|_| format!("`{flag}` wants a whole number"))
}

impl Tool for NearCopies {
    fn name(&self) -> &'static str {
        "near-copies"
    }

    fn description(&self) -> &'static str {
        "types in different crates that are close but not the same, and same-named types of different shape"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Check(NotALint::NoFailingCase)
    }

    fn args(&self) -> &[ArgSpec] {
        &[ArgSpec {
            name:        "crate",
            required:    false,
            description: "crates to compare with each other; none reads what the branch touched",
        }]
    }

    fn value_flags(&self) -> &[&'static str] {
        &["--against", "--min", "--limit"]
    }

    fn help(&self) -> &'static str {
        "Two lists, each ranked by closeness, the share of members that line up, closest first.\n\n\
         near-copies: types of one keyword under different names whose members largely agree. A member \
         agrees when it is the same name over the same type, or half when only the type is the same \
         (a renamed field). Listed from `--min` percent (default 60) and at least two members lined \
         up.\n\n\
         same name, different shape: one name over two declarations, which `single-source` leaves \
         alone. Every such pair is listed, with its closeness, so an unrelated pair sits at the bottom \
         and a drifted copy near the top.\n\n\
         With no crate arguments, the crates the branch touched against `--against` (default \
         origin/dev) are compared with every other crate; `--named` compares them only with the crates \
         whose types they name. With crate arguments, those crates are compared with each other. \
         `--pub-only` reads only types that are `pub` without restriction. `--limit` caps each list \
         (default 30). Nothing here gates and the exit is 0 whenever it can answer; a crate it cannot read, a bad flag or a git failure is inconclusive, never clean."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let args = match parse(ctx.args) {
            Ok(args) => args,
            Err(why) => return ToolReport::inconclusive(why),
        };
        let all = scope::sources(ctx.repo_root, ctx.mock_dir, ctx.all_crates);
        let types = scope::declarations(&all);
        let (scoped, vs): (BTreeSet<String>, Option<BTreeSet<String>>) = if args.crates.is_empty() {
            let touched = match scope::touched(ctx.repo_root, ctx.all_crates, &args.against) {
                Ok(touched) => touched,
                Err(why) => return ToolReport::inconclusive(why),
            };
            let vs = args
                .named
                .then(|| scope::named_from(&types, &all, &touched));
            (touched, vs)
        } else {
            let named: BTreeSet<String> = args.crates.iter().cloned().collect();
            let unknown: Vec<&String> = named.iter().filter(|c| !all.contains_key(*c)).collect();
            if !unknown.is_empty() {
                return ToolReport::inconclusive(format!("no source read for crate {unknown:?}"));
            }
            // One crate alone has no pair to read, so it is compared with every other crate.
            let vs = (named.len() > 1).then(|| named.clone());
            (named, vs)
        };
        let reads = |t: &mockspace_lint_rules::DeclaredType| {
            vs.as_ref()
                .is_none_or(|set| set.contains(&t.crate_name) || scoped.contains(&t.crate_name))
        };
        let population: Vec<_> = types
            .iter()
            .filter(|t| reads(t) && (!args.pub_only || t.fully_pub()))
            .cloned()
            .collect();
        let options = Options {
            min_percent: args.min,
            pub_only:    args.pub_only,
        };
        let found = compare(&population, &|t| scoped.contains(&t.crate_name), &options);
        let examined = population
            .iter()
            .filter(|t| scoped.contains(&t.crate_name))
            .count();
        ToolReport::reported(render::report(&found, args.limit), examined)
    }
}
