//! Lint: no bare `String` or non-static `&str` anywhere in source.
//!
//! Use `hilavitkutin_str::Str` (interned handle) or a `&'static str`
//! (compile-time literal). Bare `String` is heap-allocated and
//! forbidden by the no_alloc rule; non-static `&str` leaks unowned
//! borrowed state across API boundaries.
//!
//! Scans every non-comment, non-string line; any appearance in any
//! position is drift. Beyond the foreign-trait signature below,
//! line-local `lint:allow(no-bare-string)` is the only escape hatch.
//!
//! A signature the crate writes only because a trait of another crate fixed
//! it is not read: the signature of a method in an `impl <Trait> for <Type>`
//! whose trait the crate does not declare, from the `fn` to its `{` or `;`,
//! and an associated constant's declaration up to its `=`.
//! The trait's owner is where that signature is judged. The method's body
//! is still read, and so is every inherent impl and every impl of a trait
//! the crate declares. `crate::foreign_trait_impls` says how a trait counts
//! as declared and what that misjudges.
//!
//! Where it applies is the crate's own nature, read by `crate::nature`, never a list.

use mockspace_lint_rules::{CrateLint, Lint, LintContext, LintError, Severity};

use crate::foreign_trait_impls::scanned_sources;
use crate::util::{
    categories,
    crate_introduces_category,
    err_in_file,
    line_lint_allowed,
    names_type,
};

pub struct NoBareString;

impl Lint for NoBareString {
    /// Walks `all_sources` itself, so the dispatcher must hand it the crate
    /// once rather than once per file. Left at the default it would report
    /// every finding once per file in the crate.
    fn per_file(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "no-bare-string"
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

impl CrateLint for NoBareString {
    fn check(&self, ctx: &LintContext) -> Vec<LintError> {
        // Only a crate built on the stack's vocabulary can be held to it.
        if !crate::nature::is_of_the_stack(ctx) {
            return Vec::new();
        }
        if ctx.should_skip_proc_macro_source_lint() {
            return Vec::new();
        }
        let mut out = Vec::new();

        if crate_introduces_category(ctx, categories::STRING) {
            return Vec::new();
        }
        // A signature a foreign trait fixed is blanked, so it is judged at the trait.
        for file in scanned_sources(ctx) {
            let rel_path = &file.rel_path;
            for (idx, (raw_line, kept)) in file.text.lines().zip(file.scanned.lines()).enumerate() {
                let trimmed = raw_line.trim_start();
                if trimmed.starts_with("//") {
                    continue;
                }
                if line_lint_allowed(raw_line, "no-bare-string") {
                    continue;
                }

                let scan = strip_strings_and_chars(kept);
                let scan = strip_line_comment(&scan);

                if names_type(&scan, "String") {
                    out.push(err_in_file(
                        ctx,
                        rel_path,
                        idx + 1,
                        "no-bare-string",
                        "bare `String`; use `hilavitkutin_str::Str`".to_string(),
                    ));
                    continue;
                }
                if contains_non_static_str_ref(&scan) {
                    out.push(err_in_file(
                        ctx,
                        rel_path,
                        idx + 1,
                        "no-bare-string",
                        "non-static `&str`; use `&'static str` or `hilavitkutin_str::Str`".to_string(),
                    ));
                }
            }
        }

        out
    }
}

fn contains_non_static_str_ref(hay: &str) -> bool {
    let bytes = hay.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            let mut j = i + 1;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            // Optionally `mut`.
            if j + 3 <= bytes.len() && &bytes[j .. j + 3] == b"mut" {
                let k = j + 3;
                if k < bytes.len() && !is_ident(bytes[k]) {
                    j = k;
                    while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                        j += 1;
                    }
                }
            }
            let lifetime_start = j;
            if j < bytes.len() && bytes[j] == b'\'' {
                j += 1;
                while j < bytes.len() && is_ident(bytes[j]) {
                    j += 1;
                }
                while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
            }
            if j + 3 <= bytes.len() && &bytes[j .. j + 3] == b"str" {
                let after = j + 3;
                let after_ok = after >= bytes.len() || !is_ident(bytes[after]);
                if after_ok {
                    let lifetime = core::str::from_utf8(&bytes[lifetime_start .. j])
                        .unwrap_or("")
                        .trim();
                    if lifetime != "'static" {
                        return true;
                    }
                }
            }
        }
        i += 1;
    }
    false
}

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn strip_strings_and_chars(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'"' {
            out.push('"');
            i += 1;
            while i < bytes.len() {
                let c = bytes[i];
                if c == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                if c == b'"' {
                    out.push('"');
                    i += 1;
                    break;
                }
                i += 1;
            }
        } else if b == b'\'' {
            // Don't consume the tick here. Char literals are recognised
            // above, lifetimes start with a tick that's NOT a char-literal
            // opener (no closing tick on the line). Treat identical to
            // the numeric lint.
            out.push('\'');
            i += 1;
            let start = i;
            while i < bytes.len() {
                let c = bytes[i];
                if c == b'\\' && i + 1 < bytes.len() {
                    i += 2;
                    continue;
                }
                if c == b'\'' && i != start {
                    out.push('\'');
                    i += 1;
                    break;
                }
                if !is_ident(c) && i == start + 1 {
                    // Short single-tick followed by non-ident: lifetime.
                    // Restore cursor; emit nothing more.
                    break;
                }
                out.push(c as char);
                i += 1;
            }
        } else {
            out.push(b as char);
            i += 1;
        }
    }
    out
}

fn strip_line_comment(line: &str) -> String {
    if let Some(idx) = line.find("//") {
        line[.. idx].to_string()
    } else {
        line.to_string()
    }
}
