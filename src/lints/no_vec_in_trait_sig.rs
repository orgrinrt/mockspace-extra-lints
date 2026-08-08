//! Lint: no `Vec<T>` (or equivalent heap container) in trait method
//! signatures. Traits are contracts; callers provide a sink/iterator,
//! implementers don't return an owned heap container.

use mockspace_lint_rules::{CrateLint, Lint, LintContext, LintError, Severity};
use tree_sitter::Node;

use crate::util::{err, for_each_trait, txt};
use crate::util::line_lint_allowed;

const FORBIDDEN_IN_TRAIT: &[&str] = &["Vec<", "HashMap<", "BTreeMap<", "HashSet<", "BTreeSet<", "VecDeque<", "String"];

pub struct NoVecInTraitSig;

impl Lint for NoVecInTraitSig {
    fn name(&self) -> &'static str { "no-vec-in-trait-sig" }
    fn default_severity(&self) -> Severity { Severity::HARD_ERROR }
}

impl CrateLint for NoVecInTraitSig {
    fn check(&self, ctx: &LintContext) -> Vec<LintError> {
        if ctx.should_skip_proc_macro_source_lint() { return Vec::new(); }
        let mut out = Vec::new();
        for_each_trait(ctx.tree.root_node(), |node| {
            check_trait(node, ctx, &mut out);
        });
        out
    }
}

fn check_trait(node: Node, ctx: &LintContext, out: &mut Vec<LintError>) {
    let body = match node.child_by_field_name("body") {
        Some(b) => b,
        None => return,
    };

    let mut cursor = body.walk();
    for item in body.children(&mut cursor) {
        if item.kind() != "function_item" && item.kind() != "function_signature_item" {
            continue;
        }
        let line = item.start_position().row + 1;
        let src_line = ctx.source.lines().nth(item.start_position().row).unwrap_or("");
        if line_lint_allowed(src_line, "no-vec-in-trait-sig") { continue; }

        let name = item.child_by_field_name("name")
            .map(|n| txt(n, ctx.source))
            .unwrap_or("<unknown>");

        let mut sig = String::new();
        if let Some(params) = item.child_by_field_name("parameters") {
            sig.push_str(txt(params, ctx.source));
        }
        if let Some(ret) = item.child_by_field_name("return_type") {
            sig.push(' ');
            sig.push_str(txt(ret, ctx.source));
        }

        for forbidden in FORBIDDEN_IN_TRAIT {
            if contains_type_named(&sig, forbidden) {
                out.push(err(
                    ctx,
                    line,
                    "no-vec-in-trait-sig",
                    format!("trait method `{name}` signature contains `{forbidden}`; use &[T] / impl Iterator / &mut impl Collector<T> / &mut impl Sink"),
                ));
                break;
            }
        }
    }
}

/// Whether `sig` names the type `needle`, rather than merely containing its
/// letters.
///
/// Every entry but one in the forbidden list ends in `<`, which anchors it. The
/// exception is `String`, and unanchored it matches any type whose name merely
/// begins with those letters: `StringInterner`, `StringBuilder`, `Stringly`.
/// A wrapper around an interner is not a heap string, and reporting it as one
/// trains readers to reach for a `lint:allow` on a lint that was right about
/// everything else.
///
/// So a match counts only when the character following it cannot continue a
/// Rust identifier.
fn contains_type_named(sig: &str, needle: &str) -> bool {
    // An entry ending in `<` is already anchored by that bracket, and what
    // follows it is the generic argument, which of course continues an
    // identifier. Applying the boundary rule there would reject every real
    // match.
    let anchored = !needle
        .chars()
        .next_back()
        .is_some_and(|c| c.is_alphanumeric() || c == '_');
    if anchored {
        return sig.contains(needle);
    }

    let mut from = 0;
    while let Some(at) = sig[from ..].find(needle) {
        let start = from + at;
        let end = start + needle.len();
        let next_continues_ident = sig[end ..]
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_');
        if !next_continues_ident {
            return true;
        }
        from = end;
    }
    false
}
