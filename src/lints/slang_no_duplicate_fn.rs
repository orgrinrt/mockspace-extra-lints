//! Lint: a shader function is defined once.
//!
//! The Rust `no-duplicate-fn` lint reads Rust only, so a function copied from one shader module into
//! another goes unseen. This one reads the functions of every shipped `.slang` file (a file in a
//! crate's `shaders/` directory) and refuses two kinds of copy:
//!
//! - the same name with the same parameter types and return type, in one crate or in two;
//! - nothing else: a different signature under a shared name is an overload, and a similar name is
//!   the near-copy tool's to report, not a gate's.
//!
//! Entry points are not counted, since every pass has a `main` of its own, and a file that is not
//! shipped (a test probe) is not read. A function the project means to keep in two places is let
//! through by a `lint:allow(slang-no-duplicate-fn)` on its line.

use std::collections::BTreeMap;

use mockspace_lint_rules::{Lint, LintError, RepoContext, RepoLint, Severity};

use crate::slang::{ItemKind, SlangSource, sources};

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "slang-no-duplicate-fn";

/// The configured lint.
#[derive(Default)]
pub struct SlangNoDuplicateFn;

impl Lint for SlangNoDuplicateFn {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

impl RepoLint for SlangNoDuplicateFn {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        let files = sources(ctx.view(), ctx.repo_root, ctx.mock_dir, ctx.all_crates());
        judge(&files)
    }
}

/// One function, where it sits.
struct Site<'a> {
    file: &'a SlangSource,
    line: usize,
}

fn judge(files: &[SlangSource]) -> Vec<LintError> {
    let mut by_signature: BTreeMap<(String, String), Vec<Site>> = BTreeMap::new();
    for file in files.iter().filter(|f| f.shipped()) {
        for item in file.file.items.iter().filter(|i| i.kind == ItemKind::Func && i.stage().is_none()) {
            let signature = format!("({}) {}", item.params.join(", "), item.ret);
            by_signature
                .entry((item.name.clone(), signature))
                .or_default()
                .push(Site { file, line: item.line });
        }
    }
    let mut out = Vec::new();
    for ((name, signature), sites) in by_signature {
        let Some((first, rest)) = sites.split_first() else {
            continue;
        };
        for dup in rest.iter().filter(|s| !allowed(s)) {
            let mut e = LintError::error(
                dup.file.crate_name.clone(),
                dup.line,
                NAME,
                format!(
                    "function `{name}{signature}` is also defined in `{}` line {}; keep one and import it",
                    first.file.rel, first.line
                ),
            );
            e.path = Some(dup.file.rel.clone());
            out.push(e);
        }
    }
    out
}

/// Whether the line of the site carries `lint:allow(slang-no-duplicate-fn)`.
fn allowed(site: &Site) -> bool {
    site.file
        .text
        .lines()
        .nth(site.line.saturating_sub(1))
        .is_some_and(|l| l.contains("lint:allow(") && l.contains(NAME))
}

#[cfg(test)]
mod tests;
