//! Lint: a shader function is defined once.
//!
//! The Rust `no-duplicate-fn` lint reads Rust only, so a function copied from one shader module into
//! another goes unseen. This one reads the functions of every shipped `.slang` file (a file in a
//! crate's `shaders/` directory) and refuses two kinds of copy, each its own finding kind:
//!
//! - `signature`: the same name with the same parameter types and return type, in one crate or in
//!   two. A different signature under a shared name is an overload and passes;
//! - `body`: the same body under another name or signature, once comments and spacing are dropped,
//!   for a body of at least `min_body_tokens` tokens (24 when not set), since a short body such as
//!   `return x;` is the same in many unrelated functions. A function already reported as
//!   `signature` is not compared by its body, so one copy gets one finding.
//!
//! A `min_body_tokens` that is not a whole number is reported as a finding of its own, and the lint
//! reads nothing until it is fixed.
//!
//! A similar body that is not the same is the near-copy tool's to report, not a gate's. Entry points
//! are not counted, since every pass has a `main` of its own, and a file that is not shipped (a test
//! probe) is not read. A function the project means to keep in two places is let through by a
//! `lint:allow(slang-no-duplicate-fn)` on its line.

use std::collections::{BTreeMap, HashMap};

use mockspace_lint_rules::{Lint, LintError, Reach, RepoContext, RepoLint, Severity};

use crate::repo_files::WORKSPACE;
use crate::slang::{Item, ItemKind, SlangSource, sources};

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "slang-no-duplicate-fn";

/// The finding kind of two functions with one name and signature.
pub const SIGNATURE: &str = "signature";

/// The finding kind of one body under two functions.
pub const BODY: &str = "body";

/// The key under `[lints.slang-no-duplicate-fn]` that sets the shortest body compared.
pub const MIN_BODY_TOKENS_KEY: &str = "min_body_tokens";

/// The shortest body compared when the project sets none.
pub const DEFAULT_MIN_BODY_TOKENS: usize = 24;

/// The configured lint.
pub struct SlangNoDuplicateFn {
    min_body_tokens: usize,
    /// A `min_body_tokens` value that did not parse, reported in place of the check.
    bad_min_body_tokens: Option<String>,
}

impl Default for SlangNoDuplicateFn {
    fn default() -> Self {
        Self {
            min_body_tokens: DEFAULT_MIN_BODY_TOKENS,
            bad_min_body_tokens: None,
        }
    }
}

impl Lint for SlangNoDuplicateFn {
    fn name(&self) -> &'static str {
        NAME
    }

    fn reach(&self) -> Reach {
        Reach::Paths(crate::slang::SLANG_READS)
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }

    fn finding_kinds(&self) -> &[&str] {
        &[SIGNATURE, BODY]
    }

    fn config_keys(&self) -> &[&str] {
        &[MIN_BODY_TOKENS_KEY]
    }

    fn configure(&mut self, params: &HashMap<String, String>) {
        if let Some(v) = params.get(MIN_BODY_TOKENS_KEY) {
            match v.trim().parse() {
                Ok(n) => {
                    self.min_body_tokens = n;
                    self.bad_min_body_tokens = None;
                }
                Err(_) => self.bad_min_body_tokens = Some(v.clone()),
            }
        }
    }
}

impl RepoLint for SlangNoDuplicateFn {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        if let Some(v) = &self.bad_min_body_tokens {
            return vec![LintError::error(
                WORKSPACE.to_string(),
                0,
                NAME,
                format!("`{MIN_BODY_TOKENS_KEY}` is `{v}`, which is not a whole number, so the lint reads nothing"),
            )];
        }
        let files = sources(ctx.repo_root, ctx.mock_dir, ctx.all_crates);
        let min = self.min_body_tokens.to_string();
        let mut parts: Vec<&[u8]> = vec![min.as_bytes()];
        for f in &files {
            parts.push(f.rel.as_bytes());
            parts.push(f.text.as_bytes());
        }
        ctx.remembered(NAME, &parts, || judge(&files, self.min_body_tokens))
    }
}

/// One function, where it sits.
struct Site<'a> {
    file: &'a SlangSource,
    item: &'a Item,
}

impl Site<'_> {
    fn signature(&self) -> String {
        format!("({}) {}", self.item.params.join(", "), self.item.ret)
    }
}

fn judge(files: &[SlangSource], min_body_tokens: usize) -> Vec<LintError> {
    let functions: Vec<Site> = files
        .iter()
        .filter(|f| f.shipped())
        .flat_map(|file| {
            file.file
                .items
                .iter()
                .filter(|i| i.kind == ItemKind::Func && i.stage().is_none())
                .map(move |item| Site { file, item })
        })
        .collect();
    let mut by_signature: BTreeMap<(String, String), Vec<usize>> = BTreeMap::new();
    for (i, site) in functions.iter().enumerate() {
        by_signature.entry((site.item.name.clone(), site.signature())).or_default().push(i);
    }
    // a function past the first under its signature is reported as that, and only as that, so it
    // takes no part in the body comparison
    let mut signature_copy = vec![false; functions.len()];
    let mut out = Vec::new();
    for ((name, signature), sites) in &by_signature {
        let Some((&first, rest)) = sites.split_first() else {
            continue;
        };
        let first = &functions[first];
        for &dup in rest {
            signature_copy[dup] = true;
            out.extend(finding(
                &functions[dup],
                SIGNATURE,
                format!(
                    "function `{name}{signature}` is also defined in `{}` line {}; keep one and import it",
                    first.file.rel, first.item.line
                ),
            ));
        }
    }
    let mut by_body: BTreeMap<&str, Vec<&Site>> = BTreeMap::new();
    for (site, _) in functions.iter().zip(&signature_copy).filter(|(_, copy)| !**copy) {
        if site.item.body_tokens >= min_body_tokens {
            by_body.entry(site.item.body.as_str()).or_default().push(site);
        }
    }
    for sites in by_body.values() {
        let Some((first, rest)) = sites.split_first() else {
            continue;
        };
        for dup in rest {
            out.extend(finding(
                dup,
                BODY,
                format!(
                    "the body of `{}{}` is the body of `{}{}` in `{}` line {}; keep one function and call it",
                    dup.item.name,
                    dup.signature(),
                    first.item.name,
                    first.signature(),
                    first.file.rel,
                    first.item.line
                ),
            ));
        }
    }
    out
}

/// The finding at `site`, or none where its line lets it through.
fn finding(site: &Site, kind: &'static str, message: String) -> Option<LintError> {
    if allowed(site) {
        return None;
    }
    let mut e = LintError::error(site.file.crate_name.clone(), site.item.line, NAME, message);
    e.path = Some(site.file.rel.clone());
    e.finding_kind = Some(kind);
    Some(e)
}

/// Whether the line of the site carries `lint:allow(slang-no-duplicate-fn)`.
fn allowed(site: &Site) -> bool {
    site.file
        .text
        .lines()
        .nth(site.item.line.saturating_sub(1))
        .is_some_and(|l| l.contains("lint:allow(") && l.contains(NAME))
}

#[cfg(test)]
mod tests;
