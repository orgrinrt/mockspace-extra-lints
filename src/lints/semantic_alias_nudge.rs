//! Lint: advisory warn on container-shaped primitives at public API boundaries.
//!
//! A width-shaped primitive at a public position says how many bits the value
//! occupies and nothing about what it is, so an index that is a `QWord` reads
//! better as whatever the domain calls it. That is the whole of what this
//! advises.
//!
//! # What it does not cover, and why the list is shorter than it looks
//!
//! **A primitive that already names the domain answer is not nudged.** `Bool` is
//! the case that matters: the workspace's own no-bare-primitives rule names it
//! as the replacement for a host `bool`, so a predicate returning `Bool` has
//! done exactly what it was asked, and there is no alias that reads better than
//! a truth value called a truth value. Nudging it means two rules in one pack
//! pulling opposite ways on one position, with this one arriving at a default
//! severity the consumer never chose.
//!
//! `Cap` is out for the same reason, being a bound rather than a width, and
//! `USize` and `ISize` are out because a platform-sized index is already as
//! specific as the position usually gets.
//!
//! Default severity: ADVISORY (warn everywhere, blocks nothing).

use mockspace_lint_rules::{CrateLint, Lint, LintContext, LintError, Severity};
use tree_sitter::Node;

use crate::util::{for_each_fn, is_public, txt};
use crate::util::line_lint_allowed;

/// Primitives whose name states a width rather than a meaning.
///
/// `Bool`, `Cap`, `USize` and `ISize` were here and are deliberately not, per
/// the module doc: each already names what the value is, so an alias over one
/// is a rename rather than a reading.
const WIDTH_SHAPED_PRIMITIVES: &[&str] = &[
    "UFixed", "IFixed", "FastFloat", "StrictFloat",
    "Byte", "Word", "DWord", "QWord", "Nibble", "Bit",
];

pub struct SemanticAliasNudge;

impl Lint for SemanticAliasNudge {
    fn name(&self) -> &'static str { "semantic-alias-nudge" }
    fn default_severity(&self) -> Severity { Severity::ADVISORY }
}

impl CrateLint for SemanticAliasNudge {
    fn check(&self, ctx: &LintContext) -> Vec<LintError> {
        if ctx.should_skip_proc_macro_source_lint() { return Vec::new(); }
        let mut out = Vec::new();
        for_each_fn(ctx.tree.root_node(), |node| {
            if !is_public(node, ctx.source) { return; }
            check_fn(node, ctx, &mut out);
        });
        out
    }
}

fn check_fn(node: Node, ctx: &LintContext, out: &mut Vec<LintError>) {
    let line = node.start_position().row + 1;
    let src_line = ctx.source.lines().nth(node.start_position().row).unwrap_or("");
    if line_lint_allowed(src_line, "semantic-alias-nudge") { return; }

    let mut sig = String::new();
    if let Some(params) = node.child_by_field_name("parameters") {
        sig.push_str(txt(params, ctx.source));
    }
    if let Some(ret) = node.child_by_field_name("return_type") {
        sig.push(' ');
        sig.push_str(txt(ret, ctx.source));
    }

    for prim in WIDTH_SHAPED_PRIMITIVES {
        if contains_token(&sig, prim) {
            let name = node.child_by_field_name("name")
                .map(|n| txt(n, ctx.source))
                .unwrap_or("<unknown>");
            out.push(LintError::warning(
                ctx.crate_name.to_string(),
                line,
                "semantic-alias-nudge",
                format!(
                    "`{name}` exposes `{prim}` in its signature, which states a width rather than \
                     what the value is. An alias naming the domain reads better at a public position."
                ),
            ));
            return;
        }
    }
}

fn contains_token(hay: &str, tok: &str) -> bool {
    let bytes = hay.as_bytes();
    let needle = tok.as_bytes();
    let mut i = 0;
    while i + needle.len() <= bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
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

fn is_ident(b: u8) -> bool { b.is_ascii_alphanumeric() || b == b'_' }
