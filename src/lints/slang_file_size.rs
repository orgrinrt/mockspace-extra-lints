//! Lint: a Slang source stays under the line limit.
//!
//! The Rust `file-size` lint reads `src/` of each crate, so a shader of any length passes it. This
//! one counts the lines of every `.slang` file that are neither blank nor only a comment, the same
//! count the Rust lint takes, and reports a file over the limit once, at its first line.
//!
//! Configuration under `[lints.slang-file-size]`: `max_lines`, default 500.

use std::collections::HashMap;

use mockspace_lint_rules::{Lint, LintError, RepoContext, RepoLint, Severity};

use crate::slang::{SlangSource, sources};

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "slang-file-size";

const MAX_KEY: &str = "max_lines";

/// The limit when none is configured.
const DEFAULT_MAX: usize = 500;

/// The configured lint.
pub struct SlangFileSize {
    max_lines: usize,
}

impl Default for SlangFileSize {
    fn default() -> Self {
        Self { max_lines: DEFAULT_MAX }
    }
}

impl Lint for SlangFileSize {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::ADVISORY
    }

    fn config_keys(&self) -> &[&str] {
        &[MAX_KEY]
    }

    fn configure(&mut self, params: &HashMap<String, String>) {
        if let Some(n) = params.get(MAX_KEY).and_then(|v| v.parse().ok()) {
            self.max_lines = n;
        }
    }
}

impl RepoLint for SlangFileSize {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        let files = sources(ctx.repo_root, ctx.mock_dir, ctx.all_crates);
        let limit = self.max_lines.to_string();
        let mut parts: Vec<&[u8]> = vec![limit.as_bytes()];
        for f in &files {
            parts.push(f.rel.as_bytes());
            parts.push(f.text.as_bytes());
        }
        ctx.remembered(NAME, &parts, || self.judge(&files))
    }
}

impl SlangFileSize {
    fn judge(&self, files: &[SlangSource]) -> Vec<LintError> {
        files
            .iter()
            .filter(|f| f.file.code_lines > self.max_lines)
            .map(|f| {
                let mut e = LintError::error(
                    f.crate_name.clone(),
                    1,
                    NAME,
                    format!(
                        "{} has {} lines of code (limit: {}); split it along its concerns into modules",
                        f.rel, f.file.code_lines, self.max_lines
                    ),
                );
                e.severity = self.default_severity();
                e.path = Some(f.rel.clone());
                e
            })
            .collect()
    }
}

#[cfg(test)]
mod tests;
