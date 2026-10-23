//! Lint: every `#[ignore]` on a test is a catalogue entry, and says which row tracks it.
//!
//! A test is ignored for one honest reason in a project that writes tests ahead of the code: it
//! states a contract the code does not meet yet, and it stays red on purpose until the work lands.
//! That is a catalogue test, and the ignore is what keeps the suite green around it. The same
//! attribute is also how a probe, a diagnostic and a test that was given up on stay out of the
//! suite, and from the outside the two look alike, so a project that means its ignores to be
//! catalogue entries has no way to tell a tracked gap from a forgotten one.
//!
//! The reason therefore has one shape:
//!
//! ```text
//! #[ignore = "catalogue: <the gap>; tracked #<id>"]
//! ```
//!
//! More than one id may follow `tracked`, each with its `#` and separated by commas. A bare
//! `#[ignore]` is refused, and so is a reason of any other shape. What makes the id more than a
//! number to satisfy the lint is that it has to resolve: to a row of a ledger, or to a row of the
//! project's registry. A ledger row is a list item or a table row whose first cell is the id, and
//! the project names its ledger files with `ledgers`, a comma-separated list of globs from the
//! repository root, under `[lints.catalogue-ignore]`. The registry needs no naming, since the
//! engine hands it over, and a row resolves by its slug.
//!
//! A project that names no ledger and has no registry row for an id is refused every id, which is
//! right: it has said ignores must be tracked and given nowhere to track them.
//!
//! The attribute is found in the parse, so one inside a string or a comment is not an ignore. One in
//! the tokens of a macro call (`proptest! { #[ignore] fn t() {} }`) is, since the macro hands it to
//! the test harness as written.
//!
//! A conditional ignore, `#[cfg_attr(unix, ignore)]`, is not read: it keeps a test out of one
//! configuration for a reason about that configuration, which is not a gap to track.
//!
//! The record directories (`design_rounds` and `research` under the mock directory) are not read,
//! and `exclude`, a comma-separated list of globs from the repository root, names more paths to
//! leave alone, such as a directory of fixture sources.

use std::collections::{BTreeSet, HashMap};
use std::fs;

use mockspace_lint_rules::{Lint, LintError, RepoContext, RepoLint, Severity};
use tree_sitter::{Node, Parser};

use crate::repo_files::{matching, rust_files};

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "catalogue-ignore";

/// The key naming the ledger files, as globs from the repository root.
const LEDGERS_KEY: &str = "ledgers";

/// What a reason starts with.
const PREFIX: &str = "catalogue: ";

/// What sits between the gap and the ids.
const TRACKED: &str = "; tracked ";

const BARE: &str = "bare";
const MALFORMED: &str = "malformed";
const UNRESOLVED: &str = "unresolved";

/// The key naming the paths no ignore is read in, as globs from the repository root.
const EXCLUDE_KEY: &str = "exclude";

/// The configured lint. With no ledger named, only the registry can resolve an id.
#[derive(Default)]
pub struct CatalogueIgnore {
    ledgers: Vec<String>,
    exclude: Vec<String>,
}

/// A comma-separated config value as its non-empty items.
fn list(value: &str) -> Vec<String> {
    value.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect()
}

impl Lint for CatalogueIgnore {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }

    fn finding_kinds(&self) -> &[&str] {
        &[BARE, MALFORMED, UNRESOLVED]
    }

    fn config_keys(&self) -> &[&str] {
        &[LEDGERS_KEY, EXCLUDE_KEY]
    }

    fn configure(&mut self, params: &HashMap<String, String>) {
        if let Some(v) = params.get(LEDGERS_KEY) {
            self.ledgers = list(v);
        }
        if let Some(v) = params.get(EXCLUDE_KEY) {
            self.exclude = list(v);
        }
    }
}

impl RepoLint for CatalogueIgnore {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        let files = rust_files(ctx.repo_root, ctx.mock_dir, ctx.all_crates, &self.exclude);
        let mut parser = Parser::new();
        if parser.set_language(&tree_sitter_rust::LANGUAGE.into()).is_err() {
            return Vec::new();
        }
        let mut found = Vec::new();
        for file in &files {
            if !file.text.contains("ignore") {
                continue;
            }
            for ignore in ignores(&mut parser, &file.text) {
                found.push((file, ignore));
            }
        }
        if found.is_empty() {
            return Vec::new();
        }
        let known = known_ids(ctx, &self.ledgers);
        let mut out = Vec::new();
        for (file, ignore) in found {
            let (kind, message) = match &ignore.reason {
                Reason::Bare => (
                    BARE,
                    format!("`#[ignore]` carries no reason; write `#[ignore = \"{PREFIX}<the gap>{TRACKED}#<id>\"]`"),
                ),
                Reason::Opaque => (
                    MALFORMED,
                    "the reason of `#[ignore]` is not a string literal, so it cannot be read".to_string(),
                ),
                Reason::Text(text) => match ids(text) {
                    Err(why) => (
                        MALFORMED,
                        format!("the reason \"{text}\" is not `{PREFIX}<the gap>{TRACKED}#<id>`: {why}"),
                    ),
                    Ok(ids) => {
                        let lost: Vec<_> = ids.iter().filter(|i| !known.contains(*i)).collect();
                        if lost.is_empty() {
                            continue;
                        }
                        (
                            UNRESOLVED,
                            format!(
                                "{} to no row of the ledger or the registry; track the gap in a row that exists",
                                lost.iter().map(|i| format!("`#{i}` resolves")).collect::<Vec<_>>().join(", ")
                            ),
                        )
                    }
                },
            };
            let mut e = LintError::error(file.crate_name.clone(), ignore.line, NAME, message);
            e.finding_kind = Some(kind);
            e.path = Some(file.rel.clone());
            out.push(e);
        }
        out
    }
}

/// How an `#[ignore]` was written.
enum Reason {
    /// `#[ignore]`.
    Bare,
    /// `#[ignore = "..."]`, with the string read.
    Text(String),
    /// Anything else after the name.
    Opaque,
}

struct Ignore {
    line: usize,
    reason: Reason,
}

/// Every `#[ignore]` attribute in the source, in order.
fn ignores(parser: &mut Parser, source: &str) -> Vec<Ignore> {
    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect(source, tree.root_node(), &mut out);
    out
}

fn collect(source: &str, node: Node, out: &mut Vec<Ignore>) {
    let mut cursor = node.walk();
    let children: Vec<Node> = node.children(&mut cursor).collect();
    for (i, child) in children.iter().enumerate() {
        match child.kind() {
            "attribute_item" => {
                if let Some(reason) = ignore_reason(source, *child) {
                    out.push(Ignore { line: child.start_position().row + 1, reason });
                }
            }
            // Inside a macro call the attribute is loose tokens: a `#`, then a bracketed tree.
            "#" if node.kind() == "token_tree" => {
                if let Some(reason) = children.get(i + 1).and_then(|t| token_reason(source, *t)) {
                    out.push(Ignore { line: child.start_position().row + 1, reason });
                }
            }
            _ => collect(source, *child, out),
        }
    }
}

/// The reason of the bracketed token tree that follows a `#`, when it is `[ignore ...]`.
fn token_reason(source: &str, tree: Node) -> Option<Reason> {
    if tree.kind() != "token_tree" || !source.get(tree.byte_range())?.starts_with('[') {
        return None;
    }
    let mut cursor = tree.walk();
    let inner: Vec<Node> = tree.children(&mut cursor).filter(|c| !matches!(c.kind(), "[" | "]")).collect();
    let name = inner.first()?;
    if name.kind() != "identifier" || source.get(name.byte_range()) != Some("ignore") {
        return None;
    }
    Some(match inner.as_slice() {
        [_] => Reason::Bare,
        [_, eq, value] if eq.kind() == "=" => match source.get(value.byte_range()).and_then(literal) {
            Some(text) => Reason::Text(text),
            None => Reason::Opaque,
        },
        _ => Reason::Opaque,
    })
}

/// The reason of an attribute item, when the attribute is `ignore`.
fn ignore_reason(source: &str, item: Node) -> Option<Reason> {
    let mut cursor = item.walk();
    let attribute = item.children(&mut cursor).find(|c| c.kind() == "attribute")?;
    let name = attribute.named_child(0)?;
    if name.kind() != "identifier" || source.get(name.byte_range()) != Some("ignore") {
        return None;
    }
    if attribute.child_by_field_name("arguments").is_some() {
        return Some(Reason::Opaque);
    }
    let Some(value) = attribute.child_by_field_name("value") else {
        return Some(Reason::Bare);
    };
    Some(match source.get(value.byte_range()).and_then(literal) {
        Some(text) => Reason::Text(text),
        None => Reason::Opaque,
    })
}

/// The value of a string literal, raw or not. `None` for anything that is not one.
fn literal(text: &str) -> Option<String> {
    if let Some(rest) = text.strip_prefix('r') {
        let hashes = rest.bytes().take_while(|&b| b == b'#').count();
        let inner = rest[hashes..].strip_prefix('"')?;
        let end = format!("\"{}", "#".repeat(hashes));
        return inner.strip_suffix(end.as_str()).map(str::to_string);
    }
    text.strip_prefix('"')?.strip_suffix('"').map(unescape)
}

/// A string literal's body with its escapes read, a backslash at the end of a line taking the
/// whitespace of the next with it.
fn unescape(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\n' | '\r') => {
                while chars.next_if(|c| c.is_whitespace()).is_some() {}
            }
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('0') => out.push('\0'),
            Some(c @ ('"' | '\'' | '\\')) => out.push(c),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

/// The ids a reason tracks, without their `#`, or what is wrong with its shape.
fn ids(reason: &str) -> Result<Vec<String>, &'static str> {
    // A literal that runs over several lines keeps its newlines and the indent after them, and the
    // reason reads the same with each run of whitespace as one space.
    let reason = reason.split_whitespace().collect::<Vec<_>>().join(" ");
    let rest = reason.strip_prefix(PREFIX.trim_end()).ok_or("it does not begin with `catalogue:`")?;
    let rest = rest.strip_prefix(' ').ok_or("`catalogue:` is followed by a space and the gap")?;
    let (gap, tracked) = rest.rsplit_once(TRACKED).ok_or("it has no `; tracked #<id>` after the gap")?;
    if gap.trim().is_empty() {
        return Err("the gap is empty");
    }
    let mut out = Vec::new();
    for part in tracked.split(',') {
        let id = part.trim().strip_prefix('#').ok_or("what follows `tracked` is not a list of `#<id>`")?;
        if id.is_empty() || !id.chars().all(id_char) {
            return Err("an id is made of letters, digits, `_`, `-` and `.`, and sits right after its `#`");
        }
        out.push(id.to_string());
    }
    Ok(out)
}

/// Whether a character can sit inside an id.
fn id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')
}

/// Every id a reason may name: the registry's rows by slug and the rows of each ledger file.
fn known_ids(ctx: &RepoContext, ledgers: &[String]) -> BTreeSet<String> {
    let mut known = BTreeSet::new();
    for ns in ctx.registry.namespaces() {
        for q in ctx.registry.rows_in(ns) {
            known.insert(q.rsplit("::").next().unwrap_or(q).to_string());
        }
    }
    for rel in matching(ctx.repo_root, ledgers) {
        let Ok(text) = fs::read_to_string(ctx.repo_root.join(rel)) else {
            continue;
        };
        known.extend(text.lines().filter_map(row_id));
    }
    known
}

/// What follows a marker when the marker is followed by whitespace.
fn after_marker(rest: Option<&str>) -> Option<&str> {
    rest.filter(|r| r.starts_with(char::is_whitespace))
}

/// The id a ledger line opens with, when the line is a list item, a numbered item, a table row or
/// a heading. The id may carry a `#` of its own, as in `- #C1-188`. A heading reads as a row only
/// when its first word has a digit, so `## Map` is a title and `## C1-188` is a row.
fn row_id(line: &str) -> Option<String> {
    let line = line.trim_start();
    let digits = line.chars().take_while(char::is_ascii_digit).count();
    let hashes = line.chars().take_while(|&c| c == '#').count();
    let (cell, heading) = if let Some(rest) = line.strip_prefix('|') {
        (rest, false)
    } else if let Some(rest) = after_marker(line.strip_prefix(['-', '*', '+'])) {
        (rest, false)
    } else if digits > 0 && after_marker(line[digits..].strip_prefix(['.', ')'])).is_some() {
        (&line[digits + 1..], false)
    } else if (1..=6).contains(&hashes) && after_marker(Some(&line[hashes..])).is_some() {
        (&line[hashes..], true)
    } else {
        return None;
    };
    let cell = cell.trim_start().trim_start_matches('`');
    let cell = cell.strip_prefix('#').unwrap_or(cell);
    let id: String = cell.chars().take_while(|&c| id_char(c)).collect();
    // The run above ends at the first character that is not an id's, so what it took is the whole
    // word; the dashes and dots it ends on are the sentence's punctuation, not the id's.
    let id = id.trim_end_matches(['-', '.']);
    let titled = heading && !id.chars().any(|c| c.is_ascii_digit());
    (!id.is_empty() && !titled).then(|| id.to_string())
}

#[cfg(test)]
mod tests;
