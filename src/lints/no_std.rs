//! Lint: no std. In a crate that declares `#![no_std]`, hard-block `use std::*`, `std::*` path
//! references and `extern crate std`. A crate that declares nothing is not this lint's.
//!
//! Escape via `// lint:allow(no-std)`. Test crates may disable via
//! `[lints.no-std] severity = "off"`.
//!
//! Where it applies is the crate's own nature, read by `crate::nature`, never a list.

use mockspace_lint_rules::{CrateLint, Lint, LintContext, LintError, Severity};

use crate::nature::declares_no_std;
use crate::util::{err, line_lint_allowed};

pub struct NoStd;

impl Lint for NoStd {
    fn name(&self) -> &'static str {
        "no-std"
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

impl CrateLint for NoStd {
    fn check(&self, ctx: &LintContext) -> Vec<LintError> {
        let mut out = Vec::new();
        // A crate that does not declare `#![no_std]` has said nothing this lint could hold it to.
        if !declares_no_std(ctx) {
            return out;
        }

        // Flag per-line `std::*` and `use std::`.
        for (idx, line) in ctx.source().lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if line_lint_allowed(line, "no-std") {
                continue;
            }
            let has_use_std = line.contains("use std::");
            let has_std_path =
                line.contains(" std::") || line.starts_with("std::") || line.contains("(std::");
            let has_extern_std = line.contains("extern crate std");
            if has_use_std || has_std_path || has_extern_std {
                out.push(err(
                    ctx,
                    idx + 1,
                    "no-std",
                    "`std::*` used; use `core::*` or the stack primitives".to_string(),
                ));
            }
        }

        out
    }
}

