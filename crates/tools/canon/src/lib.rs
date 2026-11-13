//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `mock canon <verb>`: what the canon holds by subject, word and relation, and who names a tenet.
//!
//! Read from the same rows `mock query` resolves against, which the engine hands a tool, and from the
//! tree the citations live in. The verbs and what each answers are in the workflow's section on the
//! canon.

use corpus::repo::Near;
use mockspace::tool::{
    ArgSpec,
    ConfigKey,
    NotALint,
    Outcome,
    Purpose,
    Tool,
    ToolContext,
    ToolParams,
    ToolReport,
    positional_args,
};
use mockspace::{LintError, RegistryView};

use crate::scope::Scope;

// which files are read, the tiers and how a retired word is matched are owned by the lints directory,
// where `a-retired-word-stays-out` reads the same files, so the verb and the gate cannot disagree
use registry_lints::canon_tree;
use registry_lints::layout::{self, CONTENT_DIRS_KEY, ContentDirs, Layout};
mod dropped;
mod names;
mod near;
mod reference;
mod retired;
use registry_lints::retired_words;
mod rows;
mod scope;
mod stats;
mod tree;
mod verbs;

pub use names::named;
pub use rows::{Tenet, carries, heirs, joined_to, subjects, tenets};
pub use tree::{Line, Tier, mentions, tier_of};

/// The check, holding the content directories `[tools.canon]` names, which are read as the content tier.
#[derive(Default)]
pub struct Canon {
    content: ContentDirs,
}

/// The verbs, in the order the help lists them.
pub const VERBS: [&str; 11] = [
    "subjects",
    "subject",
    "show",
    "find",
    "related",
    "citers",
    "uncited",
    "forbids",
    "contradicts",
    "check",
    "stats",
];

impl Tool for Canon {
    fn name(&self) -> &'static str {
        "canon"
    }

    fn description(&self) -> &'static str {
        "answers what the canon holds by subject, word and relation, and who names a tenet"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Check(NotALint::TakesAQuestion)
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[
            ArgSpec {
                name:        "verb",
                required:    true,
                description: "`subjects`, `subject`, `show`, `find`, `related`, `citers`, `uncited`, `forbids`, `contradicts` or `check`",
            },
            ArgSpec {
                name:        "term",
                required:    false,
                description: "the subject, the word, the row (a bare id, or `namespace::id`) or the tenet the verb asks about; `check` takes one or more tenets, names each provisional one as provisional and still exits 0, and exits 1 on one that is no tenet, is superseded, or is pending or demoted and so binds nothing; `contradicts` takes any number of paths, read in one pass, and with none reads the whole repository; `forbids` with none lists the replaced tenets that name no word they rule out; `show`, `find`, `related` and `citers` read every namespace of the registry, and `--in <namespace>[,<namespace>]` narrows them, `canon` standing for the namespaces `canon_paths` reaches",
            },
        ]
    }

    fn config_keys(&self) -> &[ConfigKey] {
        &[CONTENT_DIRS_KEY]
    }

    fn configure(&mut self, params: &ToolParams) {
        self.content.configure_tool(params);
    }

    fn value_flags(&self) -> &[&'static str] {
        &[scope::FLAG]
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let words = positional_args(self, ctx.args);
        if words.first() == Some(&"stats") {
            let ready = corpus::repo::ready(ctx.mock_dir, ctx.repo_root, None);
            let Some(which) = words.get(1) else {
                return ToolReport::inconclusive(format!(
                    "stats wants a table: {}",
                    stats::TABLES.join(", ")
                ));
            };
            return match stats::table(&ready.index, which, words.get(2).copied()) {
                Ok(text) => ToolReport::reported(text, 1),
                Err(e) => ToolReport::inconclusive(e),
            };
        }
        self.run_with(ctx, &|question| {
            let ready = corpus::repo::ready(ctx.mock_dir, ctx.repo_root, None);
            corpus::repo::near_words(&ready.index, question, NEAR_PER_WORD)
        })
    }
}

/// How many forms and synonyms of each word of a question `find` follows.
const NEAR_PER_WORD: usize = 5;

impl Canon {
    /// [`Tool::run`] with the near words given by a function, so a test needs no index. Asked only
    /// by `find`, and after the plain answer is in hand.
    fn run_with(&self, ctx: &ToolContext<'_>, near_of: &dyn Fn(&str) -> Vec<Near>) -> ToolReport {
        let words = positional_args(self, ctx.args);
        let Some(verb) = words.first() else {
            return ToolReport::inconclusive(format!("canon wants a verb: {}", VERBS.join(", ")));
        };
        let scope = match scope_of(ctx, verb) {
            Ok(scope) => scope,
            Err(e) => return ToolReport::inconclusive(e),
        };
        if *verb == "check" {
            return check_report(&words[1 ..], ctx.registry);
        }
        let term = words.get(1).copied();
        let more = words.get(2 ..).unwrap_or_default();
        let layout = Layout::of(ctx.mock_dir, ctx.repo_root, &self.content);
        match verbs::answer(ctx.registry, ctx.repo_root, &layout, verb, term, more, &scope) {
            Ok((mut text, examined)) => {
                if *verb == "find"
                    && let Some(question) = term
                {
                    let near = near_of(question);
                    text.push_str(&near::section(&rows::tenets(ctx.registry), &near, &text));
                }
                ToolReport::reported(text, examined)
            },
            Err(e) => ToolReport::inconclusive(e),
        }
    }
}

/// The scope `--in` asks for, which only the verbs that read rows of any namespace take.
fn scope_of(ctx: &ToolContext<'_>, verb: &str) -> Result<Scope, String> {
    let spec = scope::flag_value(ctx.args)?;
    if spec.is_some() && !scope::SCOPED.contains(&verb) {
        return Err(format!(
            "`{}` narrows {} and no other verb; `{verb}` reads the tenets and the subjects",
            scope::FLAG,
            scope::SCOPED_NAMES
        ));
    }
    Scope::parse(spec, ctx.registry, ctx.repo_root)
}

/// `check <id>...`: silent when every tenet named exists, still binds and is ratified; one line naming each
/// provisional one, which is no finding and exits 0, since a tenet that matters now is provisional until
/// the maintainer goes through it and nothing would pass otherwise; one blocking finding for each that is no tenet, is
/// superseded, or is pending or demoted. A brief names the rows its job derives from, and a job built on a row that
/// is gone, replaced, pending or demoted is built on nothing, so the coordinator asks before it dispatches.
fn check_report(ids: &[&str], registry: &RegistryView) -> ToolReport {
    if ids.is_empty() {
        return ToolReport::inconclusive("`canon check` wants one or more tenet ids");
    }
    let all = rows::tenets(registry);
    let mut notes = String::new();
    for line in verbs::unratified(&all, ids) {
        notes.push_str(&line);
        notes.push('\n');
    }
    let findings: Vec<LintError> = verbs::check(&all, ids)
        .into_iter()
        .map(|m| LintError::error("canon".to_owned(), 0, "canon-check", m))
        .collect();
    if findings.is_empty() {
        ToolReport::reported(notes, all.len())
    } else {
        ToolReport {
            outcome: Outcome::Findings(findings),
            output:  notes,
        }
    }
}

mockspace::lint_pack! {
    tools: [Canon::default()],
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod near_tests;

#[cfg(test)]
mod stats_tests;

#[cfg(test)]
mod retired_tests;

#[cfg(test)]
mod dropped_tests;

#[cfg(test)]
mod check_tests;

#[cfg(test)]
mod closing_tests;

#[cfg(test)]
mod reference_tests;
#[cfg(test)]
mod relations_tests;

#[cfg(test)]
mod heirs_tests;

#[cfg(test)]
mod standing_tests;
