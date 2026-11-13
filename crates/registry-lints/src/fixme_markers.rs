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

/// Directories under the root that are never read: vendored code, build output, the generated
/// documents and the generated agent files.
const NEVER: [&str; 7] = [
    "mock/vendor/",
    "mock/target/",
    "mock/test-history/",
    "docs/",
    ".claude/",
    ".github/",
    "target/",
];

/// The scope of a path relative to the repository root, or nothing where it is not read.
pub fn scope_of(rel: &str) -> Option<Scope> {
    if NEVER.iter().any(|p| rel.starts_with(p)) || rel.split('/').any(|s| s == "node_modules") {
        None
    } else if rel.starts_with("mock/design_rounds/") || rel.starts_with("mock/research/") {
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
const LARGEST: u64 = 4 << 20;

/// Every file of the repository at `repo` that can hold a marker, read, sorted by path.
pub fn sources(repo: &std::path::Path) -> Vec<Source> {
    let files = crate::tree_walk::walk_files(
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
        let Some(scope) = scope_of(&rel) else {
            continue;
        };
        if std::fs::metadata(&path).is_ok_and(|m| m.len() > LARGEST) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Some(kind) = kind_of(&rel, text.lines().next().unwrap_or("")) else {
            continue;
        };
        out.push(Source {
            rel,
            scope,
            kind,
            text,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The word is written `§` here, so this file holds no marker of its own.
    fn t(s: &str) -> String {
        s.replace('§', WORD)
    }

    fn one(text: &str, kind: Kind) -> Marker {
        let all = scan(&t(text), kind);
        assert_eq!(all.len(), 1, "{all:?}");
        all.into_iter().next().unwrap()
    }

    #[test]
    fn a_tagged_marker_gives_its_tag_and_its_reason() {
        let m = one("fn a() {} // §(stand-in): a first take\n", Kind::Slashes);
        assert_eq!(m.tag.as_deref(), Some("stand-in"));
        assert!(m.colon);
        assert_eq!(m.reason, "a first take");
        assert_eq!(m.line, 1);
    }

    #[test]
    fn a_mention_without_a_colon_in_prose_is_not_a_marker() {
        let text = "Marked with a § in the source, and §(canon silent) elsewhere.\n";
        assert!(scan(&t(text), Kind::Prose).is_empty());
        assert!(scan(&t("// the § in `Species::draw`\n"), Kind::Slashes).is_empty());
    }

    #[test]
    fn the_word_inside_quotation_marks_or_another_word_is_not_a_marker() {
        assert!(scan(&t("let s = \"§(x): y\";\n"), Kind::Slashes).is_empty());
        assert!(scan(&t("// §s: left over\n"), Kind::Slashes).is_empty());
        assert!(scan(&t("// NOT§: left over\n"), Kind::Slashes).is_empty());
    }

    #[test]
    fn a_comment_opening_with_the_word_and_no_colon_is_a_marker_that_lost_it() {
        let m = one("// § fix this later\n", Kind::Slashes);
        assert!(!m.colon);
        assert_eq!(m.tag, None);
        let m = one("# §(op) later\n", Kind::Hash);
        assert!(!m.colon);
        assert_eq!(m.tag.as_deref(), Some("op"));
    }

    #[test]
    fn in_toml_only_a_comment_is_read() {
        assert!(scan(&t("text = \"see §(x): y\"\n"), Kind::Toml).is_empty());
        assert_eq!(scan(&t("# §(x): y\nkey = 1\n"), Kind::Toml).len(), 1);
        assert_eq!(scan(&t("key = 1 # §: y\n"), Kind::Toml).len(), 1);
        assert!(scan(&t("key = \"a # §: y\"\n"), Kind::Toml).is_empty());
    }

    #[test]
    fn a_backticked_marker_in_a_design_is_read_with_its_tag() {
        let m = one("`§(canon silent)`: a band about the light\n", Kind::Prose);
        assert_eq!(m.tag.as_deref(), Some("canon silent"));
        assert_eq!(m.reason, "a band about the light");
    }

    #[test]
    fn the_reason_carries_on_through_the_comment_and_stops_at_its_end() {
        let text =
            "// §(op): the first line\n// and the second\n//\n// not part of it\nlet x = 1;\n";
        assert_eq!(
            one(text, Kind::Slashes).reason,
            "the first line and the second"
        );
        let text = "/// §: a doc\n/// comment\nfn f() {}\n";
        assert_eq!(one(text, Kind::Slashes).reason, "a doc comment");
    }

    #[test]
    fn the_reason_of_a_prose_marker_is_its_paragraph() {
        let text = "§(canon silent): how a roof pitches,\nand where it ends.\n\nNext paragraph.\n";
        assert_eq!(
            one(text, Kind::Prose).reason,
            "how a roof pitches, and where it ends."
        );
        let text = "<!-- §: a note\nacross two lines -->\ntext\n";
        assert_eq!(one(text, Kind::Prose).reason, "a note across two lines");
    }

    #[test]
    fn two_markers_on_one_line_each_keep_their_own_words() {
        let all = scan(&t("a; §(x): first part; §(y): second part\n"), Kind::Prose);
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].reason, "first part;");
        assert_eq!(all[1].reason, "second part");
    }

    #[test]
    fn rewriting_replaces_the_head_and_leaves_the_rest_byte_for_byte() {
        let text = t("a // §(stand-in): keep  this  spacing\n// § no colon\n");
        let all = scan(&text, Kind::Slashes);
        assert_eq!(all.len(), 2);
        let pairs: Vec<(&Marker, &str)> = all.iter().map(|m| (m, "the_ticket")).collect();
        let out = rewrite(&text, &pairs);
        assert_eq!(
            out,
            t("a // §(the_ticket): keep  this  spacing\n// §(the_ticket): no colon\n")
        );
        // and what it wrote is read back as markers citing the ticket
        assert!(
            scan(&out, Kind::Slashes)
                .iter()
                .all(|m| m.tag.as_deref() == Some("the_ticket"))
        );
        // a backticked head in a design keeps its backticks
        let text = t("`§(canon silent)`: x\n");
        let all = scan(&text, Kind::Prose);
        let out = rewrite(&text, &[(&all[0], "the_ticket")]);
        assert_eq!(out, t("`§(the_ticket)`: x\n"));
    }

    #[test]
    fn reciting_hands_the_markers_of_one_ticket_to_another_and_leaves_the_rest() {
        let text = t(
            "// §(old): first\nlet a = 1; // §(old_two): another ticket\n// §(old): second\n// §(other): a third\n",
        );
        let out = recite(&text, Kind::Slashes, "old", "new").unwrap();
        assert_eq!(
            out,
            t(
                "// §(new): first\nlet a = 1; // §(old_two): another ticket\n// §(new): second\n// §(other): a third\n"
            )
        );
    }

    #[test]
    fn reciting_a_text_with_no_marker_citing_the_ticket_gives_nothing() {
        // the control for the test above: the id in prose, in a string, in a longer id and in no marker at all
        assert_eq!(
            recite("the ticket old is closed\n", Kind::Prose, "old", "new"),
            None
        );
        assert_eq!(
            recite(&t("let s = \"§(old): x\";\n"), Kind::Slashes, "old", "new"),
            None
        );
        assert_eq!(
            recite(
                &t("// §(older): x\n// §(gold): y\n"),
                Kind::Slashes,
                "old",
                "new"
            ),
            None
        );
        assert_eq!(
            recite(&t("// § no ticket\n"), Kind::Slashes, "old", "new"),
            None
        );
        assert_eq!(recite("fn a() {}\n", Kind::Slashes, "old", "new"), None);
    }

    #[test]
    fn reciting_reads_the_tag_as_the_lint_does_trimmed_and_whole() {
        let out = recite(
            &t("// §( old ): spaced\n// §(older): longer\n"),
            Kind::Slashes,
            "old",
            "new",
        )
        .unwrap();
        assert_eq!(out, t("// §(new): spaced\n// §(older): longer\n"));
    }

    #[test]
    fn reciting_works_in_every_kind_of_file_and_adds_the_colon_a_marker_lost() {
        let cases = [
            (
                Kind::Toml,
                "# §(old): in a config\nkey = 1\n",
                "# §(new): in a config\nkey = 1\n",
            ),
            (
                Kind::Hash,
                "# §(old): in a script\n",
                "# §(new): in a script\n",
            ),
            (
                Kind::Prose,
                "`§(old)`: in a design\n",
                "`§(new)`: in a design\n",
            ),
            (
                Kind::Prose,
                "<!-- §(old): in a note -->\n",
                "<!-- §(new): in a note -->\n",
            ),
            (
                Kind::Slashes,
                "// §(old) lost its colon\n",
                "// §(new): lost its colon\n",
            ),
        ];
        for (kind, from, to) in cases {
            assert_eq!(recite(&t(from), kind, "old", "new"), Some(t(to)), "{from}");
        }
    }

    #[test]
    fn what_reciting_wrote_is_read_back_as_markers_of_the_successor_alone() {
        let text = t("// §(old): a\n// §(old): b\n");
        let out = recite(&text, Kind::Slashes, "old", "new").unwrap();
        let all = scan(&out, Kind::Slashes);
        assert_eq!(all.len(), 2);
        assert!(all.iter().all(|m| m.tag.as_deref() == Some("new")));
        assert_eq!(recite(&out, Kind::Slashes, "old", "new"), None);
    }

    #[test]
    fn files_are_in_scope_by_where_they_stand() {
        assert_eq!(scope_of("mock/crates/a/src/lib.rs"), Some(Scope::Live));
        assert_eq!(scope_of("content/kaski/x.toml"), Some(Scope::Live));
        assert_eq!(scope_of("mock/research/n/NOTES.md"), Some(Scope::Record));
        assert_eq!(
            scope_of("mock/design_rounds/2026/x.md"),
            Some(Scope::Record)
        );
        assert_eq!(scope_of("docs/X.md"), None);
        assert_eq!(scope_of("mock/vendor/a/src/lib.rs"), None);
        assert_eq!(scope_of("mock/crates/a/node_modules/x.js"), None);
    }

    #[test]
    fn kinds_follow_the_extension_and_a_shebang() {
        assert_eq!(kind_of("a/b.rs", ""), Some(Kind::Slashes));
        assert_eq!(kind_of("a/b.toml", ""), Some(Kind::Toml));
        assert_eq!(kind_of("a/DESIGN.md.tmpl", ""), Some(Kind::Prose));
        assert_eq!(
            kind_of("a/page/build", "#!/usr/bin/env nutshell"),
            Some(Kind::Hash)
        );
        assert_eq!(kind_of("a/b.png", ""), None);
        assert_eq!(kind_of("a/build", "text"), None);
    }
}
