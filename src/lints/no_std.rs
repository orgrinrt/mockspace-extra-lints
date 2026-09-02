//! Lint: no std. Hard-block `use std::*`, `std::*` path references, and
//! absence of `#![no_std]` at the crate root.
//!
//! Escape via `// lint:allow(no-std)`. Test crates may disable via
//! `[lints.no-std] severity = "off"`.

use mockspace_lint_rules::{CrateLint, Lint, LintContext, LintError, Severity};

use crate::util::err;
use crate::util::line_lint_allowed;

pub struct NoStd;

impl Lint for NoStd {
    fn name(&self) -> &'static str { "no-std" }
    fn default_severity(&self) -> Severity { Severity::HARD_ERROR }
}

impl CrateLint for NoStd {
    fn check(&self, ctx: &LintContext) -> Vec<LintError> {
        let mut out = Vec::new();

        // Root-level #![no_std] must be present (allowing a lint:allow on the
        // first 20 lines to opt out for test / proc-macro crates).
        // FIXME: root-ness is inferred by comparing text, because LintContext
        // carries no discriminator for which file it currently holds. Two
        // byte-identical files would both count as the root, and if the root
        // fails to parse it is dropped from all_sources and the check silently
        // disappears. Both are narrow and both fail safe. The real fix is a
        // rel_path on the context, requested upstream.
        // This half is a claim about the crate root. The dispatcher hands a
        // per-file lint the same context with `source` swapped for each module
        // file, so it must fire only when `source` IS the root; otherwise every
        // module file is reported as a root missing its attribute.
        let is_crate_root = ctx
            .all_sources
            .first()
            .map_or(true, |f| f.text == ctx.source);
        let head = crate_prelude(ctx.source);
        let has_no_std = head.contains("#![no_std]");
        let allowed_at_root = line_lint_allowed(&head, "no-std");
        if is_crate_root && !has_no_std && !allowed_at_root && !ctx.is_proc_macro_crate() {
            out.push(err(
                ctx,
                1,
                "no-std",
                "crate root is missing `#![no_std]`. Every stack crate must be no_std unless explicitly allowed".to_string(),
            ));
        }

        // Flag per-line `std::*` and `use std::`.
        for (idx, line) in ctx.source.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if line_lint_allowed(line, "no-std") {
                continue;
            }
            let has_use_std = line.contains("use std::");
            let has_std_path = line.contains(" std::") || line.starts_with("std::") || line.contains("(std::");
            let has_extern_std = line.contains("extern crate std");
            if has_use_std || has_std_path || has_extern_std {
                out.push(err(
                    ctx,
                    idx + 1,
                    "no-std",
                    "`std::*` reference found; use `core::*` equivalents or the stack's own primitives".to_string(),
                ));
            }
        }

        out
    }
}

/// The part of a crate root that may carry an inner attribute.
///
/// Everything up to and including the last line before the first item, which is
/// where the language allows `#![...]` and nowhere else. A module doc runs as
/// long as it runs, so a fixed line count is a guess about how much prose a
/// crate is allowed to open with, and this reads the shape instead.
///
/// The window used to be thirty lines. A crate whose module doc ran to
/// thirty-one reported its root as missing an attribute that was on line
/// thirty-two, which is the whole reason this function exists.
///
/// Conservative at the boundary: an unrecognised line ends the prelude, so
/// anything this cannot classify falls back to reporting rather than to
/// silence.
fn crate_prelude(source: &str) -> String {
    let mut kept = Vec::new();
    for line in source.lines() {
        let text = line.trim_start();
        // `#!` and not `#[`: an outer attribute opens an item, and an item is
        // exactly what ends the prelude.
        let is_prelude = text.is_empty()
            || text.starts_with("//")
            || text.starts_with("#!")
            || text.starts_with("/*")
            || text.starts_with('*');
        if !is_prelude {
            break;
        }
        kept.push(line);
    }
    kept.join("\n")
}

#[cfg(test)]
mod tests {
    use super::crate_prelude;

    #[test]
    fn an_attribute_past_a_long_module_doc_is_still_found() {
        let doc = "//! prose\n".repeat(80);
        let source = format!("{doc}\n#![no_std]\n\npub mod thing;\n");
        assert!(
            crate_prelude(&source).contains("#![no_std]"),
            "a module doc of any length may precede the attribute"
        );
    }

    #[test]
    fn an_attribute_on_the_very_first_line_is_found() {
        assert!(crate_prelude("#![no_std]\npub mod thing;\n").contains("#![no_std]"));
    }

    #[test]
    fn the_prelude_stops_at_the_first_item_so_a_later_line_cannot_pass_for_one() {
        // The string appears, in a position the language would not accept an
        // inner attribute in. Reading the whole file would find it; reading the
        // prelude does not, which is the difference this function is for.
        let source = "//! prose\npub mod thing;\nconst NOTE: &str = \"#![no_std]\";\n";
        assert!(!crate_prelude(source).contains("#![no_std]"));
    }

    #[test]
    fn a_crate_with_no_attribute_reports_none() {
        let source = "//! prose\n//! more\n\npub mod thing;\n";
        assert!(!crate_prelude(source).contains("#![no_std]"));
    }

    #[test]
    fn a_block_comment_header_does_not_end_the_prelude() {
        let source = "/* a banner\n * over several lines\n */\n#![no_std]\npub mod thing;\n";
        assert!(crate_prelude(source).contains("#![no_std]"));
    }

    #[test]
    fn an_outer_attribute_on_the_first_item_ends_the_prelude() {
        // `#[derive(...)]` opens an item rather than the crate, so anything
        // after it is not a place an inner attribute may sit.
        let source = "//! prose\n#[derive(Debug)]\npub struct Thing;\n#![no_std]\n";
        assert!(!crate_prelude(source).contains("#![no_std]"));
    }

    #[test]
    fn an_empty_source_is_a_prelude_with_no_attribute_in_it() {
        assert!(!crate_prelude("").contains("#![no_std]"));
    }
}
