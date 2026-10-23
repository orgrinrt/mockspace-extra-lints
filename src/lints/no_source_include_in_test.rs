//! Lint: a test never `include_str!`s or `include_bytes!`s a source file.
//!
//! A test that reads the text of a `.rs` file and searches it for a word is a check on the
//! source's wording, and it passes or fails on spelling. Whatever it is checking is a lint's job:
//! a lint is run on every commit with a severity the project chooses, reports where the finding
//! is, and reads the parse instead of the text. The test is the same check written where nothing
//! runs it as one, and it is also how a catalogue test pins a convention it has no lint for yet.
//!
//! What is refused is a `include_str!` or `include_bytes!` (spelled bare, or as `std::` or
//! `core::`) whose path ends in `.rs`, when it sits inside a test. A test is:
//!
//! - a function under `#[test]`, `#[rstest]`, `#[test_case(..)]`, `#[quickcheck]`, or a macro
//!   of the same last name such as `#[tokio::test]`;
//! - an item under `#[cfg(..)]` that needs `test`, as `cfg(test)` and `cfg(all(test, unix))` do
//!   and `cfg(not(test))`, `cfg(any(test, unix))` and `cfg(feature = "test")` do not, or anything
//!   in a file or module opened with `#![cfg(..)]` of the same kind;
//! - anything in a file that is test code by where it sits: under a directory called `tests`, or
//!   named `tests.rs` or `*_tests.rs`;
//! - anything in a file a test module declares: `#[cfg(test)] mod support;` makes `support.rs`
//!   (or `support/mod.rs`) test code, and so every module it declares in turn.
//!
//! A data fixture is included freely, a `.ron`, a `.toml`, a `.json`, a `.slang`. So is source that
//! is not in a test, since a binary may carry its own source to print. A source file that really is
//! test data is let through by `allow` under `[lints.no-source-include-in-test]`, a comma-separated
//! list of globs, each matched against the path from the repository root of what is included, and
//! against the literal as it is written.
//!
//! The path is the last string literal in the macro's arguments, so `concat!(env!("DIR"),
//! "/src/lib.rs")` is caught by its last piece, `concat!("a.rs", ".bak")` is not, and a path built
//! from nothing but a constant is not seen. The macro is found in the parse, so one inside a comment
//! or a string is not an include. The record directories are not read.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use mockspace_lint_rules::{Lint, LintError, RepoContext, RepoLint, Severity};
use tree_sitter::{Node, Parser, Tree};

use crate::repo_files::{glob, rust_files, RustFile};
use crate::string_literal::literal;

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "no-source-include-in-test";

/// The key naming the source files a test may include, as globs.
const ALLOW_KEY: &str = "allow";

/// The macros that read a file into the program.
const INCLUDES: &[&str] = &["include_str", "include_bytes"];

/// The path prefixes the std spellings of those macros carry.
const STD_PATHS: &[&str] = &["std", "core"];

/// The extension of a file that is source.
const SOURCE: &str = ".rs";

/// The last segment of an attribute that makes the function under it a test.
const TEST_ATTRIBUTES: &[&str] = &["test", "rstest", "test_case", "quickcheck"];

/// The configured lint.
#[derive(Default)]
pub struct NoSourceIncludeInTest {
    allow: Vec<String>,
}

impl Lint for NoSourceIncludeInTest {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }

    fn config_keys(&self) -> &[&str] {
        &[ALLOW_KEY]
    }

    fn configure(&mut self, params: &HashMap<String, String>) {
        if let Some(v) = params.get(ALLOW_KEY) {
            self.allow = v.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect();
        }
    }
}

impl RepoLint for NoSourceIncludeInTest {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        let mut parser = Parser::new();
        if parser.set_language(&tree_sitter_rust::LANGUAGE.into()).is_err() {
            return Vec::new();
        }
        let parsed: Vec<(RustFile, Tree)> = rust_files(ctx.repo_root, ctx.mock_dir, ctx.all_crates, &[])
            .into_iter()
            .filter_map(|f| parser.parse(&f.text, None).map(|t| (f, t)))
            .collect();
        let by_path: BTreeMap<&str, usize> =
            parsed.iter().enumerate().map(|(i, (f, _))| (f.rel.as_str(), i)).collect();
        // A file a test module declares is test code, and so is every file that file declares, so
        // the set grows until a pass adds nothing.
        let mut test_files: BTreeSet<usize> = BTreeSet::new();
        let found = loop {
            let found: Vec<Found> = parsed
                .iter()
                .enumerate()
                .map(|(i, (f, tree))| {
                    let mut found = Found::default();
                    let in_test = is_test_file(&f.rel) || test_files.contains(&i);
                    find(&f.text, tree.root_node(), in_test, &mut found);
                    found
                })
                .collect();
            let before = test_files.len();
            for ((f, _), found) in parsed.iter().zip(&found) {
                for name in &found.modules {
                    test_files.extend(module_files(&f.rel, name).iter().filter_map(|p| by_path.get(p.as_str())));
                }
            }
            if test_files.len() == before {
                break found;
            }
        };
        let mut out = Vec::new();
        for ((file, _), found) in parsed.iter().zip(found) {
            for hit in found.hits.into_iter().filter(|h| !self.allowed(&file.rel, &h.path)) {
                let mut e = LintError::error(
                    file.crate_name.clone(),
                    hit.line,
                    NAME,
                    format!(
                        "`{}!` reads the source file `{}` inside a test; a check on source text belongs in a lint, which runs on every commit and reads the parse, and a data fixture may be included instead",
                        hit.mac, hit.path
                    ),
                );
                e.path = Some(file.rel.clone());
                out.push(e);
            }
        }
        out
    }
}

impl NoSourceIncludeInTest {
    /// Whether `allow` lets the file included from `includer` through.
    fn allowed(&self, includer: &str, included: &str) -> bool {
        let resolved = resolve(includer, included);
        self.allow.iter().any(|g| glob(g, included) || resolved.as_deref().is_some_and(|r| glob(g, r)))
    }
}

/// The path of `included`, written in the file `includer`, from the repository root. `None` for an
/// absolute path or one that climbs out of the root.
fn resolve(includer: &str, included: &str) -> Option<String> {
    if included.starts_with('/') {
        return None;
    }
    let mut parts: Vec<&str> = includer.split('/').collect();
    parts.pop();
    for segment in included.split('/') {
        match segment {
            ".." => {
                parts.pop()?;
            }
            "." | "" => {}
            s => parts.push(s),
        }
    }
    Some(parts.join("/"))
}

/// An include of a source file.
struct Hit {
    line: usize,
    mac: &'static str,
    path: String,
}

/// What a file holds that matters: its includes of source in a test, and the modules a test
/// declares.
#[derive(Default)]
struct Found {
    hits: Vec<Hit>,
    modules: Vec<String>,
}

/// Whether a path is test code by where it sits.
fn is_test_file(rel: &str) -> bool {
    let mut parts: Vec<&str> = rel.split('/').collect();
    let file = parts.pop().unwrap_or("");
    let stem = file.strip_suffix(SOURCE).unwrap_or(file);
    parts.contains(&"tests") || stem == "tests" || stem.ends_with("_tests")
}

/// The paths the module `name`, declared in the file `declaring`, can be in.
fn module_files(declaring: &str, name: &str) -> [String; 2] {
    let (dir, file) = declaring.rsplit_once('/').unwrap_or(("", declaring));
    let stem = file.strip_suffix(SOURCE).unwrap_or(file);
    let base = match (stem, dir) {
        ("lib" | "main" | "mod", _) | (_, "") => dir.to_string(),
        _ => format!("{dir}/{stem}"),
    };
    let join = |tail: String| if base.is_empty() { tail } else { format!("{base}/{tail}") };
    [join(format!("{name}.rs")), join(format!("{name}/mod.rs"))]
}

/// Collect what `node` holds, `in_test` saying whether it already is in a test.
///
/// Attributes are siblings that come before the item they belong to, so they are gathered as the
/// children are walked and applied to the next item. An inner attribute is a sibling too, and
/// applies to the node that holds it.
fn find(source: &str, node: Node, in_test: bool, out: &mut Found) {
    let mut cursor = node.walk();
    let children: Vec<Node> = node.children(&mut cursor).collect();
    let in_test = in_test
        || children.iter().any(|c| c.kind() == "inner_attribute_item" && is_test_attribute(source, *c));
    let mut attributes: Vec<Node> = Vec::new();
    for (i, child) in children.iter().enumerate() {
        match child.kind() {
            "attribute_item" => {
                attributes.push(*child);
                continue;
            }
            "line_comment" | "block_comment" => continue,
            _ => {}
        }
        let here = in_test || attributes.iter().any(|a| is_test_attribute(source, *a));
        attributes.clear();
        if here {
            if let Some(hit) = include_at(source, &children, i) {
                out.hits.push(hit);
            }
            if child.kind() == "mod_item" && child.child_by_field_name("body").is_none() {
                let name = child.child_by_field_name("name").and_then(|n| source.get(n.byte_range()));
                out.modules.extend(name.map(str::to_string));
            }
        }
        find(source, *child, here, out);
    }
}

/// The macro name at the end of a callee, when the callee is a bare name or a std path.
fn include_name(source: &str, callee: Node) -> Option<&'static str> {
    let text = source.get(callee.byte_range())?;
    let name = match callee.kind() {
        "identifier" => text,
        "scoped_identifier" => {
            let (path, name) = text.trim_start_matches("::").rsplit_once("::")?;
            STD_PATHS.contains(&path).then_some(name)?
        }
        _ => return None,
    };
    INCLUDES.iter().copied().find(|m| *m == name)
}

/// The include whose callee is the child at `at`, when it is one of a source file.
fn include_at(source: &str, children: &[Node], at: usize) -> Option<Hit> {
    let name = children[at];
    let mac = include_name(source, name)?;
    let bang = children.get(at + 1)?;
    let args = children.get(at + 2)?;
    if bang.kind() != "!" || args.kind() != "token_tree" {
        return None;
    }
    let path = literals(source, *args).pop().filter(|l| l.ends_with(SOURCE))?;
    Some(Hit { line: name.start_position().row + 1, mac, path })
}

/// The string literals anywhere in a token tree, in order.
fn literals(source: &str, node: Node) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "string_literal" || child.kind() == "raw_string_literal" {
            out.extend(source.get(child.byte_range()).and_then(literal));
        } else {
            out.extend(literals(source, child));
        }
    }
    out
}

/// Whether an attribute item makes what it belongs to a test.
fn is_test_attribute(source: &str, item: Node) -> bool {
    let mut cursor = item.walk();
    let Some(attribute) = item.children(&mut cursor).find(|c| c.kind() == "attribute") else {
        return false;
    };
    let Some(text) = source.get(attribute.byte_range()) else {
        return false;
    };
    let text: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let (path, args) = text.split_at(text.find('(').unwrap_or(text.len()));
    if path == "cfg" {
        return args.strip_prefix('(').and_then(|a| a.strip_suffix(')')).is_some_and(requires_test);
    }
    path.rsplit("::").next().is_some_and(|last| TEST_ATTRIBUTES.contains(&last))
}

/// Whether a `cfg` predicate, without whitespace, can only be true when `test` is.
///
/// `test` does, `all(..)` does when one of its terms does, `any(..)` when every term does, and
/// `not(..)` and every other predicate never does.
fn requires_test(predicate: &str) -> bool {
    let predicate = predicate.trim();
    let end = predicate.find(|c: char| !(c.is_alphanumeric() || c == '_')).unwrap_or(predicate.len());
    let (name, rest) = predicate.split_at(end);
    let Some(inner) = rest.strip_prefix('(').and_then(|r| r.strip_suffix(')')) else {
        return name == "test" && rest.is_empty();
    };
    let terms = split_terms(inner);
    match name {
        "all" => terms.iter().any(|t| requires_test(t)),
        "any" => !terms.is_empty() && terms.iter().all(|t| requires_test(t)),
        _ => false,
    }
}

/// The comma-separated terms of a predicate's arguments, leaving alone a comma inside parentheses
/// or a string.
fn split_terms(args: &str) -> Vec<&str> {
    let (mut depth, mut quoted, mut start) = (0usize, false, 0usize);
    let mut terms = Vec::new();
    for (i, c) in args.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth = depth.saturating_sub(1),
            ',' if !quoted && depth == 0 => {
                terms.push(&args[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < args.len() {
        terms.push(&args[start..]);
    }
    terms
}

#[cfg(test)]
mod tests;
