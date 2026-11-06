//! Lint: a shader type is declared in one crate.
//!
//! The same struct, interface or enum (the same name over the same members) written into the shaders
//! of two crates is one declaration that has been copied, and the copy drifts the first time one of them
//! changes. This is the Rust `single-source` lint over the types the shaders declare: the first
//! declaration, in path order, is the source, and each later one is reported with where the source is.
//! A pair that differs in any member is not this lint's, it is a name over two shapes and the
//! `near-copies` tool lists it.
//!
//! A declaration the project means to keep in two places is let through by a
//! `lint:allow(slang-single-source)` on its line.

use std::collections::BTreeMap;

use mockspace_lint_rules::{DeclaredType, Lint, LintError, RepoContext, RepoLint, Severity};

use crate::slang::{SlangSource, declared_types, sources};

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "slang-single-source";

/// The configured lint.
#[derive(Default)]
pub struct SlangSingleSource;

impl Lint for SlangSingleSource {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

impl RepoLint for SlangSingleSource {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        let files = sources(ctx.repo_root, ctx.mock_dir, ctx.all_crates);
        let mut parts: Vec<&[u8]> = Vec::new();
        for f in &files {
            parts.push(f.rel.as_bytes());
            parts.push(f.text.as_bytes());
        }
        ctx.remembered(NAME, &parts, || judge(&files))
    }
}

fn judge(files: &[SlangSource]) -> Vec<LintError> {
    let mut by_shape: BTreeMap<String, Vec<DeclaredType>> = BTreeMap::new();
    for t in declared_types(files) {
        by_shape.entry(t.shape.clone()).or_default().push(t);
    }
    let mut out = Vec::new();
    for group in by_shape.values() {
        let Some((first, rest)) = group.split_first() else {
            continue;
        };
        for dup in rest.iter().filter(|d| d.crate_name != first.crate_name) {
            let rel = full_path(files, dup);
            if allowed(files, &rel, dup.line) {
                continue;
            }
            let mut e = LintError::error(
                dup.crate_name.clone(),
                dup.line,
                NAME,
                format!(
                    "`{}` is declared the same in `{}` as in `{}` line {}; keep one and import it",
                    dup.name,
                    dup.crate_name,
                    first.crate_name,
                    first.line
                ),
            );
            e.path = Some(rel);
            out.push(e);
        }
    }
    out
}

/// The path from the repository root of the file a declaration came from.
fn full_path(files: &[SlangSource], t: &DeclaredType) -> String {
    let within = t.path.as_deref().unwrap_or("");
    files
        .iter()
        .find(|f| f.crate_name == t.crate_name && f.rel.ends_with(within))
        .map_or_else(|| within.to_string(), |f| f.rel.clone())
}

fn allowed(files: &[SlangSource], rel: &str, line: usize) -> bool {
    files
        .iter()
        .find(|f| f.rel == rel)
        .and_then(|f| f.text.lines().nth(line.saturating_sub(1)))
        .is_some_and(|l| l.contains("lint:allow(") && l.contains(NAME))
}

#[cfg(test)]
mod tests;
