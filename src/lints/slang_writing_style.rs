//! Lint: the comments of a shader are held to the writing style of the comments of Rust.
//!
//! `writing-style` reads the `///`, `//!` and `//` comments of Rust. A shader's comments are prose a
//! maintainer reads as much, so they go through the same checks (em-dash density, inline emphasis,
//! hype, jargon, filler, greeting openers), over the comments the Slang reader found, so a `//` inside
//! a string is not a comment and a block comment is read line by line.

use mockspace_lint_rules::{Lint, LintError, RepoContext, RepoLint, Severity};

use super::writing_style::check_comment_corpus;
use crate::slang::{SlangSource, sources};

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "slang-writing-style";

/// The configured lint.
#[derive(Default)]
pub struct SlangWritingStyle;

impl Lint for SlangWritingStyle {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::PUSH_GATE
    }
}

impl RepoLint for SlangWritingStyle {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        let files = sources(ctx.repo_root, ctx.mock_dir, ctx.all_crates);
        let mut parts: Vec<&[u8]> = Vec::new();
        for f in &files {
            parts.push(f.rel.as_bytes());
            parts.push(f.text.as_bytes());
        }
        ctx.remembered(NAME, &parts, || files.iter().flat_map(judge).collect())
    }
}

/// The findings for the comments of one source.
fn judge(source: &SlangSource) -> Vec<LintError> {
    let mut corpus = String::new();
    let mut map = Vec::new();
    for c in &source.file.comments {
        corpus.push_str(&c.text);
        corpus.push('\n');
        map.push(c.line);
    }
    let mut found = Vec::new();
    check_comment_corpus(
        &source.rel,
        &corpus,
        &map,
        source.text.lines().count(),
        &source.crate_name,
        &mut found,
    );
    for e in &mut found {
        e.lint_name = NAME;
    }
    found
}

#[cfg(test)]
mod tests;
