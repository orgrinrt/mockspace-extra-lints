//! Shared helpers for stack lints.

use mockspace_lint_rules::{LintContext, LintError, Severity};
use tree_sitter::Node;

/// Primitive-substrate categories that the ecosystem's types group
/// into. Each lint in this pack declares which category (or
/// categories) of substrate it protects via the `Lint::categories`
/// helper; the `[primitive-introductions]` section in a consumer's
/// mockspace.toml lists the categories that each crate introduces.
/// When the two intersect, the lint self-exempts for that crate.
/// The crate is the one bringing the substrate to the table, so
/// whatever it does internally to define it is legitimate.
///
/// Categories (not specific arvo type names) keep the map stable as
/// the stack evolves. Adding a new arvo type (e.g. a new strategy
/// marker, a new arithmetic kind, a new opaque-bit container) just
/// means tagging its introducing crate with the right category. No
/// lint pack change, no recompile. Categories change only when a
/// genuinely new substrate DOMAIN appears (rare; e.g. if a "temporal
/// substrate" layer ever joined, a new lint would carry its own
/// domain).
///
/// The current stable categories:
///
/// | Category      | What it covers                                              | Typical introducers |
/// |---------------|-------------------------------------------------------------|---------------------|
/// | `numeric`     | Numeric + bool wrappers (UFixed, USize, Bool, Bits, …)      | arvo, arvo-bits, arvo-hash |
/// | `fallibility` | Hot/warm/cold fallibility tier (Maybe, Outcome, Just)       | notko |
/// | `string`      | Interned / static string identity (Str)                     | hilavitkutin-str |
///
/// Future direction (task #119): auto-derive the categories a crate
/// introduces from its DESIGN.md.tmpl / source parse, eliminating the
/// manual TOML declaration entirely.
pub mod categories {
    pub const NUMERIC: &str = "numeric";
    pub const FALLIBILITY: &str = "fallibility";
    pub const STRING: &str = "string";
    /// Compile-time static string identity. The introducing crate
    /// (hilavitkutin-str) defines interned string handles that replace
    /// bare `const NAME: &str` / `static NAME: &str` across the stack;
    /// other crates must gate any remaining static-string literals
    /// behind `#[cfg(debug_assertions)]` so release builds drop them.
    pub const STATIC_STRING: &str = "static-string";
}

/// Whether the current crate is declared (via
/// `[primitive-introductions]`) to introduce the substrate `category`.
/// Bare-primitive lints call this once at the top of `check`: if the
/// crate introduces the category the lint enforces, the lint returns
/// an empty violation set for that crate, unconditionally.
///
/// Matching is exact string compare against the crate's list. Adding
/// an unknown category to a crate's list has no effect; no lint
/// watches that category. Adding `"numeric"` or `"fallibility"` to a
/// crate's list is an auditable architectural claim (the crate must
/// actually define the substrate types) rather than a list of
/// forbidden tokens to bypass.
#[must_use]
pub fn crate_introduces_category(ctx: &LintContext, category: &str) -> bool {
    ctx.primitive_introductions
        .get(ctx.crate_name)
        .map(|list| list.iter().any(|c| c == category))
        .unwrap_or(false)
}

/// Whether the current crate introduces ANY of `categories`.
pub fn crate_introduces_any_category(ctx: &LintContext, categories: &[&str]) -> bool {
    categories.iter().any(|c| crate_introduces_category(ctx, c))
}

/// Slice of source for a node.
pub fn txt<'a>(node: Node<'a>, src: &'a str) -> &'a str {
    &src[node.byte_range()]
}

/// Whether a source line carries `lint:allow(<rule_name>)`, including
/// the case where one marker silences multiple lints at once via the
/// comma-separated form `lint:allow(rule_a, rule_b, rule_c)`. Whitespace
/// inside the parens is tolerated. Multiple `lint:allow(...)` markers on
/// the same line each get scanned.
///
/// The historical naive substring check (`line.contains("lint:allow(<rule>)")`)
/// did not handle comma-separated lists. Multi-lint allow comments became
/// useless because the substring never matched. This helper is the canonical
/// allow-detection for every lint in this pack.
pub fn line_lint_allowed(line: &str, rule_name: &str) -> bool {
    let mut search = line;
    let needle = "lint:allow(";
    while let Some(start) = search.find(needle) {
        let after_open = &search[start + needle.len()..];
        if let Some(close) = after_open.find(')') {
            let names = &after_open[..close];
            for name in names.split(',') {
                if name.trim() == rule_name {
                    return true;
                }
            }
            search = &after_open[close..];
        } else {
            break;
        }
    }
    false
}

/// Whether the line a node starts on contains a `lint:allow(<name>)`
/// (including comma-list form). Use [`line_lint_allowed`] for non-node
/// callers.
#[allow(dead_code)]
pub fn is_lint_allowed(node: Node, ctx: &LintContext, rule_name: &str) -> bool {
    let row = node.start_position().row;
    let line = ctx.source.lines().nth(row).unwrap_or("");
    line_lint_allowed(line, rule_name)
}

#[cfg(test)]
mod tests {
    use super::line_lint_allowed;

    #[test]
    fn single_name_matches() {
        assert!(line_lint_allowed("use std::env; // lint:allow(no-std)", "no-std"));
    }

    #[test]
    fn comma_list_matches_first() {
        assert!(line_lint_allowed(
            "use std::env; // lint:allow(no-std, forbidden-imports)",
            "no-std",
        ));
    }

    #[test]
    fn comma_list_matches_second() {
        assert!(line_lint_allowed(
            "use std::env; // lint:allow(forbidden-imports, no-std)",
            "no-std",
        ));
    }

    #[test]
    fn comma_list_matches_third() {
        assert!(line_lint_allowed("x // lint:allow(a, b, c)", "c"));
    }

    #[test]
    fn whitespace_tolerated() {
        assert!(line_lint_allowed("x // lint:allow(  a  ,  b  )", "a"));
        assert!(line_lint_allowed("x // lint:allow(  a  ,  b  )", "b"));
    }

    #[test]
    fn non_match_returns_false() {
        assert!(!line_lint_allowed("x // lint:allow(other)", "no-std"));
        assert!(!line_lint_allowed("x // lint:allow(a, b)", "c"));
        assert!(!line_lint_allowed("no lint marker here", "no-std"));
    }

    #[test]
    fn malformed_unclosed_returns_false() {
        assert!(!line_lint_allowed("x // lint:allow(no-std", "no-std"));
    }

    #[test]
    fn similar_name_does_not_match_partial() {
        assert!(!line_lint_allowed("x // lint:allow(no-std-extra)", "no-std"));
        assert!(!line_lint_allowed("x // lint:allow(extra-no-std)", "no-std"));
    }

    #[test]
    fn multiple_markers_on_line() {
        let line = "x // lint:allow(a) further text lint:allow(b, c)";
        assert!(line_lint_allowed(line, "a"));
        assert!(line_lint_allowed(line, "b"));
        assert!(line_lint_allowed(line, "c"));
        assert!(!line_lint_allowed(line, "d"));
    }

    #[test]
    fn empty_parens_matches_nothing() {
        assert!(!line_lint_allowed("x // lint:allow()", "anything"));
    }
}

/// Build a blocking error with the standard (crate, line, lint, message) shape.
pub fn err(
    ctx: &LintContext,
    line: usize,
    lint_name: &'static str,
    message: String,
) -> LintError {
    LintError::with_severity(
        ctx.crate_name.to_string(),
        line,
        lint_name,
        message,
        Severity::HARD_ERROR,
    )
}

/// Visit every `function_item` / `function_signature_item` in the tree and
/// call `visit` for each one.
pub fn for_each_fn<F: FnMut(Node)>(root: Node, mut visit: F) {
    walk(root, &mut visit);
}

fn walk<F: FnMut(Node)>(node: Node, visit: &mut F) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_item" | "function_signature_item" => visit(child),
            _ => {}
        }
        if child.named_child_count() > 0 {
            walk(child, visit);
        }
    }
}

/// Visit every `struct_item` in the tree.
pub fn for_each_struct<F: FnMut(Node)>(root: Node, mut visit: F) {
    walk_kind(root, "struct_item", &mut visit);
}

/// Visit every `trait_item` in the tree.
pub fn for_each_trait<F: FnMut(Node)>(root: Node, mut visit: F) {
    walk_kind(root, "trait_item", &mut visit);
}

fn walk_kind<F: FnMut(Node)>(node: Node, kind: &str, visit: &mut F) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            visit(child);
        }
        if child.named_child_count() > 0 {
            walk_kind(child, kind, visit);
        }
    }
}

/// Whether a function is marked `pub` / `pub(crate)` / `pub(super)` / `pub(...)`.
pub fn is_public(node: Node, src: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            let text = txt(child, src);
            return text.starts_with("pub");
        }
    }
    false
}
