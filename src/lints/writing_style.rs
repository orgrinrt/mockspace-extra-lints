//! Writing-style heuristics for public-facing docs.
//!
//! Threshold-based,
//! not per-occurrence strict: single uses pass, runaway usage blocks.
//!
//! Scope, under the workspace root (the mock dir):
//! - `PRINCIPLES.md.tmpl`, `DESIGN.md.tmpl`, `WORKFLOW.md.tmpl`
//! - `crates/<crate>/{README,DESIGN,BACKLOG}.md.tmpl`
//! - `agent/**/*.md.tmpl`
//! - Rust `///`, `//!` and `//` comments in every source file of each crate
//!
//! Design rounds are not read.

mod scan;

use std::path::Path;

use mockspace_lint_rules::{Lint, LintContext, LintError, Severity, View, WorkspaceLint};

use scan::{
    count_exclamations_in_prose,
    count_label_colon_bullets,
    emphasis_runs,
    line_of_first_match,
    opens_with_flat_bullet_list,
    strip_code_spans,
};

const HYPE_WORDS: &[&str] = &[
    "blazing",
    "seamless",
    "powerful",
    "amazing",
    "incredible",
    "game-changing",
    "best-in-class",
];

const CORPORATE_JARGON: &[&str] =
    &["leverage", "utilize", "utilise", "synergy", "holistic", "paradigm"];

const FILLER_PHRASES: &[&str] = &[
    "it should be noted that",
    "essentially",
    "basically",
    "at the end of the day",
    "for all intents and purposes",
];

const GREETING_OPENERS: &[&str] = &["Sure!", "Happy to help!", "Let me explain"];

/// One em-dash per ~10 lines of prose is the threshold.
const EM_DASH_PER_LINES: usize = 10;

pub struct WritingStyle;

impl Lint for WritingStyle {
    fn name(&self) -> &'static str {
        "writing-style"
    }

    fn source_only(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::PUSH_GATE
    }
}

impl WorkspaceLint for WritingStyle {
    fn check_all(&self, crates: &[(&str, &LintContext)]) -> Vec<LintError> {
        let (workspace_root, view) = match crates.first() {
            Some((_, ctx)) => (ctx.workspace_root(), ctx.view()),
            None => return Vec::new(),
        };

        let mut out = Vec::new();

        // Top-level public docs. The label is the file itself, so the rendered
        // location is already the path and there is nothing to hang off it.
        for name in &["PRINCIPLES.md.tmpl", "DESIGN.md.tmpl", "WORKFLOW.md.tmpl"] {
            let path = workspace_root.join(name);
            check_file(view, &path, name, None, &mut out);
        }

        // Per-crate public docs.
        for (crate_name, ctx) in crates {
            for doc in &["README.md.tmpl", "DESIGN.md.tmpl", "BACKLOG.md.tmpl"] {
                let path = workspace_root.join("crates").join(crate_name).join(doc);
                if view.exists(&path) {
                    check_file(view, &path, crate_name, Some(doc), &mut out);
                }
            }
            // Rust comments, every module file rather than the crate root.
            // `all_sources` is empty on an older engine, and the crate root is
            // what that engine gives, so the fallback is what the scope used to
            // be rather than nothing.
            let sources: Vec<(String, &str)> = if ctx.all_sources().is_empty() {
                vec![("src/lib.rs".to_string(), ctx.source())]
            } else {
                ctx.all_sources()
                    .iter()
                    .map(|f| (f.rel_path.display().to_string(), f.text))
                    .collect()
            };
            for (rel_path, source) in sources {
                check_rust_comments(&rel_path, source, crate_name, &mut out);
            }
        }

        // Agent rules + skills.
        let agent_dir = workspace_root.join("agent");
        if view.is_dir(&agent_dir) {
            walk_md_tmpl(view, &agent_dir, workspace_root, &mut out);
        }

        out
    }
}

/// Check one document, labelling every finding with the file it came from.
///
/// `rel_label` is the path the renderer hangs off `crate_name`, so a finding in
/// a crate's own readme reads `<crate>/README.md.tmpl:12` rather than naming a
/// line in a crate root that has no such line. `None` where `crate_name` is
/// already the file, which is the workspace-level case.
fn check_file(
    view: &View<'_>,
    path: &Path,
    crate_name: &str,
    rel_label: Option<&str>,
    out: &mut Vec<LintError>,
) {
    if is_self_exempt(path) {
        return;
    }
    let Some(content) = view.text(path) else {
        return;
    };
    let mut found = Vec::new();
    check_text(content, crate_name, Body::Document, &mut found);
    for mut e in found {
        e.path = rel_label.map(str::to_string);
        out.push(e);
    }
}

/// The writing-style rule template itself legitimately quotes banned words
/// and em-dashes as examples. Skip it to avoid self-tripping.
fn is_self_exempt(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.contains("writing-style") || name.contains("writing_style")
}

/// Walk the agent templates, labelling each finding with its path relative to
/// the mock workspace root.
///
/// The label goes in the crate-name position rather than in `path`, because
/// these sit under no crate, and a bare `agent:5` names a directory instead of
/// a file anybody can open.
fn walk_md_tmpl(view: &View<'_>, dir: &Path, workspace_root: &Path, out: &mut Vec<LintError>) {
    for path in view.walk(dir, |_, _| false) {
        if path.extension().is_some_and(|e| e == "tmpl")
            && path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with(".md.tmpl"))
        {
            let label = path
                .strip_prefix(workspace_root)
                .unwrap_or(&path)
                .display()
                .to_string();
            check_file(view, &path, &label, None, out);
        }
    }
}

/// What kind of body `check_text` was handed.
///
/// The two differ in more than register. A document has sections, a first
/// screen and a table of contents, so the structural checks below have
/// something to be about. A file's comments have none of that: they are a
/// corpus assembled out of lines that were never adjacent, interleaved with
/// code that happens to be commented out, and the structural checks read that
/// as prose and are wrong every time.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum Body {
    /// A markdown file, checked whole.
    Document,
    /// The comments of one source file, with `lines` the length of the source
    /// rather than of the corpus, so a density is measured against the file a
    /// reader opens.
    Comments {
        lines: usize,
    },
}

fn check_text(content: &str, crate_name: &str, body: Body, out: &mut Vec<LintError>) {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = match body {
        Body::Document => lines.len().max(1),
        Body::Comments {
            lines,
        } => lines.max(1),
    };

    // Strip code fences and inline code spans for prose-level checks.
    // Raw content is retained for structural checks (leading lists, tables).
    let prose = strip_code_spans(content);

    // 1. Em-dash density.
    let em_dash_count = prose.matches('—').count();
    let threshold = total_lines / EM_DASH_PER_LINES;
    if em_dash_count > threshold && em_dash_count > 1 {
        out.push(LintError::with_severity(
            crate_name.to_string(),
            1,
            "writing-style",
            format!("em-dash density {em_dash_count} in {total_lines} lines, over 1 per {EM_DASH_PER_LINES}; use periods, commas or parens"),
            Severity::PUSH_GATE,
        ));
    }

    // 1b. Inline emphasis, which is banned outright rather than rationed.
    //
    // A sentence carrying its weight in the words needs no typeface to say
    // which half matters, and emphasis turns into decoration the moment there
    // is more than a little of it, at which point a reader stops seeing any of
    // it. No threshold for that reason: one is a violation.
    //
    // Agent instructions under `.claude/` are exempt and never reach here,
    // because this lint's file set is the published surface plus rust comments.
    for (number, marker) in emphasis_runs(&prose) {
        out.push(LintError::with_severity(
            crate_name.to_string(),
            number,
            "writing-style",
            format!(
                "inline emphasis `{marker}`; say it in the words (backticks on identifiers and paths stay)"
            ),
            Severity::PUSH_GATE,
        ));
    }

    // 2. Hype words, corporate jargon, filler phrases, greeting openers.
    let lower = prose.to_lowercase();
    for (word, category) in HYPE_WORDS
        .iter()
        .map(|w| (*w, "hype"))
        .chain(CORPORATE_JARGON.iter().map(|w| (*w, "jargon")))
        .chain(FILLER_PHRASES.iter().map(|w| (*w, "filler")))
    {
        let count = lower.matches(word).count();
        if count >= 2 {
            let line = line_of_first_match(&lines, word);
            out.push(LintError::with_severity(
                crate_name.to_string(),
                line,
                "writing-style",
                format!(
                    "`{word}` ({category}) used {count}x"
                ),
                Severity::PUSH_GATE,
            ));
        }
    }
    for opener in GREETING_OPENERS {
        if prose.contains(opener) {
            let line = line_of_first_match(&lines, opener);
            out.push(LintError::with_severity(
                crate_name.to_string(),
                line,
                "writing-style",
                format!("greeting opener `{opener}`; state the first fact"),
                Severity::PUSH_GATE,
            ));
        }
    }

    // The three below are about a document's shape, so they run on a document
    // and on nothing else. Each was measured firing on ordinary rust: two
    // commented-out macro calls read as exclamations in prose, a module doc
    // opening on bullets reads as a readme opening on a feature dump, and a
    // rustdoc argument list reads as a glossary that should have been a table.
    // None of those is what the rule is about.
    if body != Body::Document {
        return;
    }

    // 3. Exclamation marks in prose (not in inline code or fenced blocks).
    let excl_count = count_exclamations_in_prose(content);
    if excl_count > 1 {
        out.push(LintError::with_severity(
            crate_name.to_string(),
            1,
            "writing-style",
            format!("{excl_count} exclamation marks; drop them"),
            Severity::PUSH_GATE,
        ));
    }

    // 4. Leading-list smell: first 3-4 sections should be prose, not flat bullets.
    if opens_with_flat_bullet_list(&lines) {
        out.push(LintError::with_severity(
            crate_name.to_string(),
            1,
            "writing-style",
            "flat bulleted list in the first 3-4 sections; open with prose".into(),
            Severity::PUSH_GATE,
        ));
    }

    // 5. `- <label>: <short>` label-colon cheat-sheet pattern (forbidden everywhere).
    let label_colon_count = count_label_colon_bullets(&lines);
    if label_colon_count > 3 {
        out.push(LintError::with_severity(
            crate_name.to_string(),
            1,
            "writing-style",
            format!("{label_colon_count} `- <label>: <text>` bullets; use a glossary table or prose"),
            Severity::PUSH_GATE,
        ));
    }
}

/// Every comment line in one source file, as a corpus plus the source line each
/// corpus line came from.
///
/// `///` and `//!` are tried before the bare `//`, since that is a prefix of
/// both. Plain comments are in scope because the style rules bind what a
/// maintainer reads exactly as they bind what renders.
fn comment_corpus(source: &str) -> (String, Vec<usize>) {
    let mut corpus = String::new();
    let mut map = Vec::new();
    for (index, line) in source.lines().enumerate() {
        let t = line.trim_start();
        let rest = t
            .strip_prefix("///")
            .or_else(|| t.strip_prefix("//!"))
            .or_else(|| t.strip_prefix("//"));
        if let Some(rest) = rest {
            corpus.push_str(rest);
            corpus.push('\n');
            map.push(index + 1);
        }
    }
    (corpus, map)
}

/// Check one source file's comments, mapping every finding back to the line it
/// sits on and labelling it with the file.
///
/// `check_text` counts over the corpus, which is a different document, so a
/// number it returns names nothing a reader can open until it goes through the
/// map. The source's own length goes in with it, so a density is measured
/// against the file rather than against however many of its lines happened to
/// be comments.
fn check_rust_comments(rel_path: &str, source: &str, crate_name: &str, out: &mut Vec<LintError>) {
    let (corpus, map) = comment_corpus(source);
    check_comment_corpus(rel_path, &corpus, &map, source.lines().count(), crate_name, out);
}

/// Check a comment corpus read from a source of any language, `map` giving the source line each
/// corpus line came from and `source_lines` the length of the source it was read from.
pub(crate) fn check_comment_corpus(
    rel_path: &str,
    corpus: &str,
    map: &[usize],
    source_lines: usize,
    crate_name: &str,
    out: &mut Vec<LintError>,
) {
    if corpus.trim().is_empty() {
        return;
    }
    let mut found = Vec::new();
    check_text(
        corpus,
        crate_name,
        Body::Comments {
            lines: source_lines,
        },
        &mut found,
    );
    for mut e in found {
        e.line = map.get(e.line.saturating_sub(1)).copied().unwrap_or(e.line);
        e.path = Some(rel_path.to_string());
        out.push(e);
    }
}

#[cfg(test)]
mod emphasis_tests;
#[cfg(test)]
mod corpus_tests;
#[cfg(test)]
mod body_tests;
