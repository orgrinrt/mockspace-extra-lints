//! Lint: no bare `Option<T>` anywhere in source. Use `notko::Maybe<T>`.
//!
//! Scans every non-comment, non-string line. Any appearance of the
//! `Option` type name (word boundary, followed by `<` or used as the
//! standalone type) is drift, regardless of context: pub API, private
//! impl, let bindings, struct fields, match arms with
//! `Option::Some(...)`, etc. Beyond the foreign-trait signature
//! below, the only form that passes is an explicit
//! `lint:allow(no-bare-option)` on the offending line.
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
use crate::util::{categories, crate_introduces_category, err_in_file, line_lint_allowed};

pub struct NoBareOption;

impl Lint for NoBareOption {
    /// Walks `all_sources` itself, so the dispatcher must hand it the crate
    /// once rather than once per file. Left at the default it would report
    /// every finding once per file in the crate.
    fn per_file(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        "no-bare-option"
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

impl CrateLint for NoBareOption {
    fn check(&self, ctx: &LintContext) -> Vec<LintError> {
        // Only a crate built on the stack's vocabulary can be held to it.
        if !crate::nature::is_of_the_stack(ctx) {
            return Vec::new();
        }
        if ctx.should_skip_proc_macro_source_lint() {
            return Vec::new();
        }
        let mut out = Vec::new();

        if crate_introduces_category(ctx, categories::FALLIBILITY) {
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
                if line_lint_allowed(raw_line, "no-bare-option") {
                    continue;
                }

                let scan = strip_strings_and_chars(kept);
                let scan = strip_line_comment(&scan);

                if contains_option_token(&scan) {
                    out.push(err_in_file(
                        ctx,
                        rel_path,
                        idx + 1,
                        "no-bare-option",
                        "bare `Option`; use `notko::Maybe<T>`".to_string(),
                    ));
                }
            }
        }

        out
    }
}

/// True when `Option` appears as a type/path identifier. Excludes
/// `std::option` path segments and `core::option` since those are
/// module references rather than the generic type.
fn contains_option_token(hay: &str) -> bool {
    let bytes = hay.as_bytes();
    let needle = b"Option";
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if &bytes[i .. i + needle.len()] == needle {
            let before_ok = i == 0 || !is_ident(bytes[i - 1]);
            let after_pos = i + needle.len();
            let after_ok = after_pos >= bytes.len() || !is_ident(bytes[after_pos]);
            if before_ok && after_ok {
                return true;
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
