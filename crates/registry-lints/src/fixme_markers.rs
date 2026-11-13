//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What a `FIXME` marker is, read out of a file's text.
//!
//! This file declares no lint. It is the one reading of a marker, so a gate on markers and the `owed`
//! tool, which includes this file by path as `canon` includes `canon_tree.rs`, cannot disagree about
//! what is one. It uses
//! nothing but `std`, for that reason.
//!
//! A marker is the word, an optional tag in parentheses and a colon, then the reason. The word
//! is read whole (`FIXMEs` is not one) and not inside quotation marks, so a string that names the
//! marker is not one. A marker that has lost its colon is still one where it opens a comment, since a
//! comment that begins with the word is a marker whatever follows it. In a TOML file only a comment is
//! read, since a value there is data. A mention that is none of these (`marked with a FIXME`) is prose
//! about markers and is left alone.

use crate::layout::Layout;

/// One marker in a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Marker {
    /// The line it is on, from 1.
    pub line:   usize,
    /// The byte offset of the word `FIXME` in the text.
    pub start:  usize,
    /// The byte offset just past the word, or past its closing parenthesis where it has a tag.
    pub end:    usize,
    /// What is between the parentheses, where there are any.
    pub tag:    Option<String>,
    /// Whether a colon follows the head, with at most a backtick between.
    pub colon:  bool,
    /// The marker's words: the rest of its line and the lines of the same comment or paragraph that
    /// carry on from it, joined with single spaces.
    pub reason: String,
}

/// The kinds of file a marker is read in, each with its own idea of a comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Rust, Slang, WGSL, JavaScript and TypeScript: `//` and `/* */`.
    Slashes,
    /// TOML: `#`, and only there.
    Toml,
    /// Shell and other `#` languages.
    Hash,
    /// Markdown and the templates: prose, and `<!-- -->`.
    Prose,
}

/// The kind of file a path is, or nothing where its markers are not read.
pub fn kind_of(path: &str, first_line: &str) -> Option<Kind> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let ext = name.rsplit_once('.').map(|(_, e)| e);
    match ext {
        Some("rs" | "slang" | "wgsl" | "js" | "cjs" | "mjs" | "ts" | "css") => Some(Kind::Slashes),
        Some("toml") => Some(Kind::Toml),
        Some("sh" | "yml" | "yaml") => Some(Kind::Hash),
        Some("md" | "tmpl" | "html") => Some(Kind::Prose),
        None if first_line.starts_with("#!") => Some(Kind::Hash),
        _ => None,
    }
}

const WORD: &str = "FIXME";

fn word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Where the `#` comment of a TOML line begins, outside any string.
fn toml_comment_at(line: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (i, c) in line.char_indices() {
        match quote {
            Some(q) => {
                if escaped {
                    escaped = false;
                } else if c == '\\' && q == '"' {
                    escaped = true;
                } else if c == q {
                    quote = None;
                }
            },
            None => {
                if c == '"' || c == '\'' {
                    quote = Some(c);
                } else if c == '#' {
                    return Some(i);
                }
            },
        }
    }
    None
}

/// Whether what precedes a word on its line is only the opening of a comment.
fn opens_comment(before: &str, kind: Kind) -> bool {
    let b = before.trim();
    match kind {
        Kind::Slashes => {
            matches!(
                b,
                "//" | "///" | "//!" | "/*" | "/**" | "/*!" | "*" | "///!"
            )
        },
        Kind::Toml | Kind::Hash => b == "#" || b == "##",
        Kind::Prose => b.is_empty() || b == "<!--" || b == "-" || b == "*",
    }
}

/// The head of a marker found at `at` in `line`: where it ends, its tag, whether a colon follows.
fn head(line: &str, at: usize) -> (usize, Option<String>, bool) {
    let rest = &line[at + WORD.len() ..];
    let mut end = at + WORD.len();
    let mut tag = None;
    if let Some(inner) = rest.strip_prefix('(')
        && let Some(close) = inner
            .find(')')
            .filter(|c| *c <= 80 && !inner[.. *c].contains('('))
    {
        tag = Some(inner[.. close].to_string());
        end += 1 + close + 1;
    }
    let after = &line[end ..];
    let after_tick = after.strip_prefix('`').unwrap_or(after);
    (end, tag, after_tick.starts_with(':'))
}

/// Every marker head on one line, as (offset in line, end, tag, colon).
fn heads_in(line: &str, kind: Kind) -> Vec<(usize, usize, Option<String>, bool)> {
    let comment_from = match kind {
        Kind::Toml => toml_comment_at(line),
        _ => Some(0),
    };
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = line[from ..].find(WORD) {
        let at = from + i;
        from = at + WORD.len();
        let before = &line[.. at];
        let prev = before.chars().next_back();
        let next = line[at + WORD.len() ..].chars().next();
        if prev.is_some_and(word_char) || next.is_some_and(|c| c.is_alphanumeric() || c == '_') {
            continue;
        }
        if !comment_from.is_some_and(|c| at > c) && kind == Kind::Toml {
            continue;
        }
        let (end, tag, colon) = head(line, at);
        let quoted = prev.is_some_and(|c| c == '"' || c == '\'');
        let accepted = (colon && !quoted) || (!colon && opens_comment(before, kind));
        if accepted {
            out.push((at, end, tag, colon));
        }
    }
    out
}

/// The leader a comment line opens with, or nothing where it is not one.
fn leader(line: &str, kind: Kind) -> Option<&'static str> {
    let t = line.trim_start();
    match kind {
        Kind::Slashes => {
            ["///", "//!", "//", "*"]
                .into_iter()
                .find(|l| t.starts_with(l) && !(*l == "*" && t.starts_with("*/")))
        },
        Kind::Toml | Kind::Hash => t.starts_with('#').then_some("#"),
        Kind::Prose => None,
    }
}

fn clean(s: &str) -> String {
    let s = s.trim();
    let s = s.strip_suffix("-->").unwrap_or(s);
    let s = s.strip_suffix("*/").unwrap_or(s);
    s.trim().to_string()
}

/// A line of prose that ends the paragraph a marker is in.
fn ends_paragraph(line: &str) -> bool {
    let t = line.trim_start();
    t.is_empty()
        || t.starts_with(['#', '|', '>'])
        || t.starts_with("- ")
        || t.starts_with("* ")
        || t.starts_with("```")
        || t.starts_with("<!--")
        || t.chars().next().is_some_and(|c| c.is_ascii_digit()) && t.contains(". ")
}

/// Every marker in `text`, a file of `kind`, in the order they stand.
pub fn scan(text: &str, kind: Kind) -> Vec<Marker> {
    let mut lines: Vec<(usize, &str)> = Vec::new();
    let mut off = 0;
    for l in text.split_inclusive('\n') {
        lines.push((off, l.strip_suffix('\n').unwrap_or(l)));
        off += l.len();
    }
    let mut out = Vec::new();
    for (n, (base, line)) in lines.iter().enumerate() {
        let heads = heads_in(line, kind);
        for (k, (at, end, tag, colon)) in heads.iter().enumerate() {
            let stop = heads.get(k + 1).map_or(line.len(), |h| h.0);
            let skip = if *colon {
                line[*end .. stop].find(':').map_or(*end, |c| *end + c + 1)
            } else {
                *end
            };
            let mut words = vec![clean(&line[skip.min(stop) .. stop])];
            if k + 1 == heads.len() {
                words.extend(carry_on(&lines, n, *at, kind));
            }
            let reason = words
                .into_iter()
                .filter(|w| !w.is_empty())
                .collect::<Vec<_>>()
                .join(" ");
            out.push(Marker {
                line: n + 1,
                start: base + at,
                end: base + end,
                tag: tag.clone(),
                colon: *colon,
                reason,
            });
        }
    }
    out
}

/// The lines after line `n` that carry on the marker standing at `at` in it.
fn carry_on(lines: &[(usize, &str)], n: usize, at: usize, kind: Kind) -> Vec<String> {
    let line = lines[n].1;
    let mut out = Vec::new();
    let starts_head = |l: &str| !heads_in(l, kind).is_empty();
    match kind {
        Kind::Prose => {
            let in_comment = line[.. at].contains("<!--") && !line.contains("-->");
            for (_, l) in &lines[n + 1 ..] {
                if in_comment {
                    out.push(clean(l));
                    if l.contains("-->") {
                        break;
                    }
                } else if ends_paragraph(l) || starts_head(l) {
                    break;
                } else {
                    out.push(clean(l));
                }
            }
        },
        _ => {
            let Some(mine) = opens_comment(&line[.. at], kind)
                .then(|| leader(line, kind))
                .flatten()
            else {
                return out;
            };
            for (_, l) in &lines[n + 1 ..] {
                if leader(l, kind) != Some(mine) || starts_head(l) {
                    break;
                }
                let body = l.trim_start().trim_start_matches(mine).trim();
                if body.is_empty() {
                    break;
                }
                out.push(clean(body));
            }
        },
    }
    out
}

/// A text with the head of each marker in `at` replaced by `FIXME(<id>)` and a colon where there was
/// none, the rest of every line untouched. `at` is a list of (marker, id) pairs, in any order.
pub fn rewrite(text: &str, at: &[(&Marker, &str)]) -> String {
    let mut order: Vec<&(&Marker, &str)> = at.iter().collect();
    order.sort_by_key(|(m, _)| m.start);
    let mut out = String::with_capacity(text.len() + at.len() * 24);
    let mut done = 0;
    for (m, id) in order {
        out.push_str(&text[done .. m.start]);
        out.push_str(&format!("{WORD}({id})"));
        if !m.colon {
            out.push(':');
        }
        done = m.end;
    }
    out.push_str(&text[done ..]);
    out
}

/// A text with every marker that cites the ticket `from` made to cite `to`, or nothing where none
/// does. The tag is read as the lint reads it, trimmed and whole, so a marker citing a longer id that
/// begins with `from` is left as it is, and the rest of every line stays as it was.
pub fn recite(text: &str, kind: Kind, from: &str, to: &str) -> Option<String> {
    let markers = scan(text, kind);
    let at: Vec<(&Marker, &str)> = markers
        .iter()
        .filter(|m| m.tag.as_deref().map(str::trim) == Some(from))
        .map(|m| (m, to))
        .collect();
    (!at.is_empty()).then(|| rewrite(text, &at))
}

/// The paths a tool that rewrites markers declares it may write, as a maker declares them: every
/// authored place a live marker is read in. The registry's own files are under `mock/`.
pub const WRITES: &[&str] = &[
    // FIXME(a_maker_tool_s_writes_cannot_name_the_mock_directory): `writes` names `mock` and `content`, since a pattern cannot name the mock directory the config declares or the content directories a tool table names
    "mock/**",
    "content/**",
    "README.md",
    "mockspace.toml",
    "deny.toml",
    "rustfmt.toml",
    "rust-toolchain.toml",
];

/// Which files hold markers that can be asked to cite a ticket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// Authored files that can be rewritten: source, designs, content, the config.
    Live,
    /// The closed design rounds and the research notes: records that quote markers, patches among
    /// them, and a locked changelist cannot be rewritten. Read for a count, never for a ticket.
    Record,
}

/// Directories under the mock directory that are never read: vendored code, build output and the
/// recorded test history.
const NEVER_IN_MOCK: [&str; 3] = ["vendor/", "target/", "test-history/"];

/// Directories under the root that are never read: build output, the generated documents and the
/// generated agent files.
const NEVER: [&str; 4] = ["docs/", ".claude/", ".github/", "target/"];

/// The scope of a path relative to the repository root, laid out as `layout` says, or nothing where
/// it is not read.
pub fn scope_of(rel: &str, layout: &Layout) -> Option<Scope> {
    let in_mock = layout.under_mock(rel);
    let never = NEVER.iter().any(|p| rel.starts_with(p))
        || in_mock.is_some_and(|m| NEVER_IN_MOCK.iter().any(|p| m.starts_with(p)))
        || rel.split('/').any(|s| s == "node_modules");
    if never {
        None
    } else if in_mock.is_some_and(|m| m.starts_with("design_rounds/") || m.starts_with("research/")) {
        Some(Scope::Record)
    } else {
        Some(Scope::Live)
    }
}

/// A file read for markers.
#[derive(Debug, Clone)]
pub struct Source {
    /// Relative to the repository root, with `/`.
    pub rel:   String,
    pub scope: Scope,
    pub kind:  Kind,
    pub text:  String,
}

/// The longest file read, in bytes.
const LARGEST: usize = 4 << 20;

/// Every file of the repository at `repo`, laid out as `layout` says, that can hold a marker, read, sorted
/// by path.
pub fn sources(view: &mockspace::View<'_>, repo: &std::path::Path, layout: &Layout) -> Vec<Source> {
    let files = crate::tree_walk::walk_files(
        view,
        repo,
        |rel, name| {
            matches!(name, ".git" | "target" | "node_modules")
                || (rel.as_os_str().is_empty() && matches!(name, "docs" | ".claude" | ".github"))
        },
        |_| true,
    );
    let mut out = Vec::new();
    for path in files {
        let rel = path
            .strip_prefix(repo)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let Some(scope) = scope_of(&rel, layout) else {
            continue;
        };
        let Some(bytes) = view.bytes(&path) else {
            continue;
        };
        if bytes.len() > LARGEST {
            continue;
        }
        let Ok(text) = std::str::from_utf8(bytes) else {
            continue;
        };
        let Some(kind) = kind_of(&rel, text.lines().next().unwrap_or("")) else {
            continue;
        };
        out.push(Source {
            rel,
            scope,
            kind,
            text: text.to_owned(),
        });
    }
    out
}

#[cfg(test)]
#[path = "fixme_markers_tests.rs"]
mod tests;
