//! Lint: a test never `include_str!`s or `include_bytes!`s a source file.
//!
//! A test that reads the text of a `.rs` file and searches it for a word is a check on the
//! source's wording, and it passes or fails on spelling. Whatever it is checking is a lint's job:
//! a lint is run on every commit with a severity the project chooses, reports where the finding
//! is, and reads the parse instead of the text. The test is the same check written where nothing
//! runs it as one, and it is also how a catalogue test pins a convention it has no lint for yet.
//!
//! What is refused is a `include_str!` or `include_bytes!` whose path ends in `.rs`, when it sits
//! inside a test: in a `#[test]` function (or one under a macro of the same last name, such as
//! `#[tokio::test]`), in a `#[cfg(test)]` item, or in a file that is test code by where it sits,
//! under a directory called `tests`, or named `tests.rs` or `*_tests.rs`. A data fixture is
//! included freely, a `.ron`, a `.toml`, a `.json`, a `.slang`. So is source that is not in a
//! test, since a binary may carry its own source to print.
//!
//! The path is read from the literals in the macro's arguments, so `concat!(env!("DIR"),
//! "/src/lib.rs")` is caught by its last piece, and a path built from nothing but a constant is not
//! seen. The macro is found in the parse, so one inside a comment or a string is not an include.

use std::collections::BTreeSet;

use mockspace_lint_rules::{Lint, LintError, RepoContext, RepoLint, Severity};
use tree_sitter::{Node, Parser};

use crate::repo_files::rust_files;
use crate::string_literal::literal;

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "no-source-include-in-test";

/// The macros that read a file into the program.
const INCLUDES: &[&str] = &["include_str", "include_bytes"];

/// The extension of a file that is source.
const SOURCE: &str = ".rs";

pub struct NoSourceIncludeInTest;

impl Lint for NoSourceIncludeInTest {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

impl RepoLint for NoSourceIncludeInTest {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        let mut parser = Parser::new();
        if parser.set_language(&tree_sitter_rust::LANGUAGE.into()).is_err() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for file in rust_files(ctx.repo_root, ctx.all_crates) {
            if !INCLUDES.iter().any(|m| file.text.contains(m)) {
                continue;
            }
            let Some(tree) = parser.parse(&file.text, None) else {
                continue;
            };
            let mut found = Vec::new();
            find(&file.text, tree.root_node(), is_test_file(&file.rel), &mut found);
            for hit in found {
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

/// An include of a source file.
struct Hit {
    line: usize,
    mac: &'static str,
    path: String,
}

/// Whether a path is test code by where it sits.
fn is_test_file(rel: &str) -> bool {
    let mut parts: Vec<&str> = rel.split('/').collect();
    let file = parts.pop().unwrap_or("");
    let stem = file.strip_suffix(SOURCE).unwrap_or(file);
    parts.contains(&"tests") || stem == "tests" || stem.ends_with("_tests")
}

/// Collect every include of a source file under `node` that sits in a test.
///
/// `in_test` is whether `node` already is one. Attributes are siblings that come before the item
/// they belong to, so they are gathered as the children are walked and applied to the next item.
fn find(source: &str, node: Node, in_test: bool, out: &mut Vec<Hit>) {
    let mut cursor = node.walk();
    let children: Vec<Node> = node.children(&mut cursor).collect();
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
                out.push(hit);
            }
        }
        find(source, *child, here, out);
    }
}

/// The include whose macro name is the child at `at`, when it is one of a source file.
fn include_at(source: &str, children: &[Node], at: usize) -> Option<Hit> {
    let name = children[at];
    if name.kind() != "identifier" {
        return None;
    }
    let mac = *INCLUDES.iter().find(|m| source.get(name.byte_range()) == Some(**m))?;
    let bang = children.get(at + 1)?;
    let args = children.get(at + 2)?;
    if bang.kind() != "!" || args.kind() != "token_tree" {
        return None;
    }
    let path = literals(source, *args).into_iter().rfind(|l| l.ends_with(SOURCE))?;
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

/// Whether an attribute item makes what follows it a test.
fn is_test_attribute(source: &str, item: Node) -> bool {
    let mut cursor = item.walk();
    let Some(attribute) = item.children(&mut cursor).find(|c| c.kind() == "attribute") else {
        return false;
    };
    let Some(text) = source.get(attribute.byte_range()) else {
        return false;
    };
    let text: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    if let Some(args) = text.strip_prefix("cfg(").and_then(|t| t.strip_suffix(')')) {
        let unquoted: String = args.split('"').step_by(2).collect::<Vec<_>>().join(" ");
        let words: BTreeSet<&str> = unquoted.split(|c: char| !c.is_alphanumeric() && c != '_').collect();
        return words.contains("test") && !words.contains("not");
    }
    text == "test" || text.ends_with("::test")
}

#[cfg(test)]
mod tests;
