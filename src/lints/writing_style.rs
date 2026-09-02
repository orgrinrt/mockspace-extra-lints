//! Writing-style heuristics for public-facing docs.
//!
//! Source: `.shared/writing-style-fragment.md` in clause-dev. Threshold-based,
//! not per-occurrence strict: single uses pass, runaway usage blocks.
//!
//! Scope:
//! - `{mock}/PRINCIPLES.md.tmpl`, `{mock}/DESIGN.md.tmpl`, `{mock}/WORKFLOW.md.tmpl`
//! - `{repo_root}/README.md`
//! - `{mock}/crates/**/*.md.tmpl`
//! - `{mock}/agent/**/*.md.tmpl`
//! - Rust `///` / `//!` / `//` comments under `{mock}/crates/**/*.rs`
//!
//! Out of scope (discipline, not gated): `mock/design_rounds/**`.

use std::path::{Path, PathBuf};

use mockspace_lint_rules::{Lint, LintContext, LintError, Severity, WorkspaceLint};

const HYPE_WORDS: &[&str] = &[
    "blazing", "seamless", "powerful", "amazing", "incredible",
    "game-changing", "best-in-class",
];

const CORPORATE_JARGON: &[&str] = &[
    "leverage", "utilize", "utilise", "synergy", "holistic", "paradigm",
];

const FILLER_PHRASES: &[&str] = &[
    "it should be noted that",
    "essentially",
    "basically",
    "at the end of the day",
    "for all intents and purposes",
];

const GREETING_OPENERS: &[&str] = &[
    "Sure!",
    "Happy to help!",
    "Let me explain",
];

/// One em-dash per ~10 lines of prose is the threshold.
const EM_DASH_PER_LINES: usize = 10;

pub struct WritingStyle;

impl Lint for WritingStyle {
    fn name(&self) -> &'static str { "writing-style" }
    fn source_only(&self) -> bool { false }
    fn default_severity(&self) -> Severity { Severity::PUSH_GATE }
}

impl WorkspaceLint for WritingStyle {
    fn check_all(&self, crates: &[(&str, &LintContext)]) -> Vec<LintError> {
        let workspace_root = match crates.first() {
            Some((_, ctx)) => ctx.workspace_root,
            None => return Vec::new(),
        };

        let mut out = Vec::new();

        // Top-level public docs.
        for name in &["PRINCIPLES.md.tmpl", "DESIGN.md.tmpl", "WORKFLOW.md.tmpl"] {
            let path = workspace_root.join(name);
            check_file(&path, name, "writing-style", &mut out);
        }

        // Per-crate public docs.
        for (crate_name, ctx) in crates {
            for doc in &["README.md.tmpl", "DESIGN.md.tmpl", "BACKLOG.md.tmpl"] {
                let path = workspace_root.join("crates").join(crate_name).join(doc);
                if path.exists() {
                    check_file(&path, crate_name, "writing-style", &mut out);
                }
            }
            // Rust doc comments in lib.rs.
            check_rust_doc_comments(ctx.source, crate_name, &mut out);
        }

        // Agent rules + skills.
        let agent_dir = workspace_root.join("agent");
        if agent_dir.is_dir() {
            walk_md_tmpl(&agent_dir, "agent", &mut out);
        }

        out
    }
}

fn check_file(path: &Path, crate_name: &str, _lint: &str, out: &mut Vec<LintError>) {
    if is_self_exempt(path) { return; }
    let content = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(_) => return,
    };
    check_text(&content, crate_name, out);
}

/// The writing-style rule template itself legitimately quotes banned words
/// and em-dashes as examples. Skip it to avoid self-tripping.
fn is_self_exempt(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name.contains("writing-style") || name.contains("writing_style")
}

fn walk_md_tmpl(dir: &Path, crate_name: &str, out: &mut Vec<LintError>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_md_tmpl(&path, crate_name, out);
        } else if path.extension().map(|e| e == "tmpl").unwrap_or(false) {
            if path.file_name().map(|n| n.to_string_lossy().ends_with(".md.tmpl")).unwrap_or(false) {
                check_file(&path, crate_name, "writing-style", out);
            }
        }
    }
}

fn check_text(content: &str, crate_name: &str, out: &mut Vec<LintError>) {
    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len().max(1);

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
            format!("em-dash density {em_dash_count} in {total_lines} lines exceeds threshold (1 per {EM_DASH_PER_LINES}); replace most with periods, commas, or parens"),
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
                "inline emphasis `{marker}`. No bold, italic or underline in prose a human reads; \
                 say it in the words. Backticks on an identifier or a path stay"
            ),
            Severity::PUSH_GATE,
        ));
    }

    // 2. Hype words, corporate jargon, filler phrases, greeting openers.
    let lower = prose.to_lowercase();
    for (word, category) in HYPE_WORDS.iter().map(|w| (*w, "hype"))
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
                format!("`{word}` ({category}) used {count}x; see .shared/writing-style-fragment.md"),
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
                format!("greeting opener `{opener}`. State the first fact instead"),
                Severity::PUSH_GATE,
            ));
        }
    }

    // 3. Exclamation marks in prose (not in inline code or fenced blocks).
    let excl_count = count_exclamations_in_prose(content);
    if excl_count > 1 {
        out.push(LintError::with_severity(
            crate_name.to_string(),
            1,
            "writing-style",
            format!("{excl_count} exclamation marks in prose; drop them"),
            Severity::PUSH_GATE,
        ));
    }

    // 4. Leading-list smell: first 3-4 sections should be prose, not flat bullets.
    if opens_with_flat_bullet_list(&lines) {
        out.push(LintError::with_severity(
            crate_name.to_string(),
            1,
            "writing-style",
            "opens with a flat bulleted list (no hierarchy) in the first 3-4 sections; frame with prose first".into(),
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
            format!("{label_colon_count} `- <label>: <short description>` bullets; use a glossary table or prose"),
            Severity::PUSH_GATE,
        ));
    }
}

/// Every inline-emphasis run in a body, as a line number and the marker used.
///
/// One entry per line rather than per run, because a line carrying three bold
/// spans is one sentence to rewrite and three findings would say so three times.
///
/// What counts: a `**` or `__` pair for bold, and a `*` or `_` pair for italic.
/// The single-character forms are the awkward ones, so both are required to sit
/// against a word: `*this*` is emphasis, and `a * b`, `snake_case`, `_unused`
/// and a bare `---` rule are not. A markdown list's leading `*` never pairs and
/// so never fires.
///
/// Runs over prose that has already had code fences and inline code spans
/// stripped, so a doubled star inside backticks does not reach here.
fn emphasis_runs(prose: &str) -> Vec<(usize, &'static str)> {
    let mut out = Vec::new();
    for (index, line) in prose.lines().enumerate() {
        let marker = if paired(line, "**") {
            Some("**")
        } else if paired(line, "__") {
            Some("__")
        } else if single_paired(line, '*') {
            Some("*")
        } else if single_paired(line, '_') {
            Some("_")
        } else {
            None
        };
        if let Some(marker) = marker {
            out.push((index + 1, marker));
        }
    }
    out
}

/// Whether a line carries at least one opening and closing run of `marker`.
fn paired(line: &str, marker: &str) -> bool {
    line.matches(marker).count() >= 2
}

/// Whether a line carries a single-character emphasis pair.
///
/// Both ends have to touch a word for it to be emphasis rather than arithmetic,
/// an identifier or a horizontal rule. The doubled form is excluded first, so a
/// `**bold**` line is reported once as bold rather than twice.
fn single_paired(line: &str, marker: char) -> bool {
    let doubled: String = [marker, marker].iter().collect();
    if line.contains(&doubled) {
        return false;
    }
    let bytes: Vec<char> = line.chars().collect();
    let mut opened = false;
    for (i, c) in bytes.iter().enumerate() {
        if *c != marker {
            continue;
        }
        let before = i.checked_sub(1).and_then(|p| bytes.get(p)).copied();
        let after = bytes.get(i + 1).copied();
        let opens = after.is_some_and(|c| c.is_alphanumeric())
            && before.is_none_or(|c| !c.is_alphanumeric());
        let closes = before.is_some_and(|c| c.is_alphanumeric())
            && after.is_none_or(|c| !c.is_alphanumeric());
        if opened && closes {
            return true;
        }
        if opens {
            opened = true;
        }
    }
    false
}

fn line_of_first_match(lines: &[&str], needle: &str) -> usize {
    let lower_needle = needle.to_lowercase();
    for (i, line) in lines.iter().enumerate() {
        if line.to_lowercase().contains(&lower_needle) {
            return i + 1;
        }
    }
    1
}

/// Strip fenced code blocks and inline code spans from a markdown body.
/// Used for prose-level checks so code examples and quoted tokens don't
/// trip the heuristics.
fn strip_code_spans(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut in_fence = false;
    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            out.push('\n');
            continue;
        }
        if in_fence {
            out.push('\n');
            continue;
        }
        let mut in_code = false;
        for ch in line.chars() {
            match ch {
                '`' => in_code = !in_code,
                _ if in_code => out.push(' '),
                c => out.push(c),
            }
        }
        out.push('\n');
    }
    out
}

fn count_exclamations_in_prose(content: &str) -> usize {
    let mut in_code_fence = false;
    let mut count = 0;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code_fence = !in_code_fence;
            continue;
        }
        if in_code_fence { continue; }
        // Skip inline code `...` spans roughly.
        let mut in_code = false;
        for ch in line.chars() {
            match ch {
                '`' => in_code = !in_code,
                '!' if !in_code => count += 1,
                _ => {}
            }
        }
    }
    count
}

fn opens_with_flat_bullet_list(lines: &[&str]) -> bool {
    // Find the first 3 top-level sections (## or lower).
    let mut section_count = 0;
    let mut scan_from = 0;
    for (i, line) in lines.iter().enumerate() {
        if line.starts_with("# ") || line.starts_with("## ") {
            section_count += 1;
            if section_count == 1 {
                scan_from = i + 1;
            }
            if section_count > 3 { break; }
        }
    }
    // Within those top sections, look at the first non-blank non-heading content.
    // If it's a flat bullet list (no sub-bullets, all `- something`), flag it.
    let mut bullets_at_top = 0;
    let mut first_content_seen = false;
    for line in &lines[scan_from..lines.len().min(scan_from + 60)] {
        let t = line.trim();
        if t.is_empty() { continue; }
        if t.starts_with("#") {
            // New section; stop scanning the previous section's body.
            if first_content_seen { break; }
            continue;
        }
        if t.starts_with("- ") || t.starts_with("* ") {
            bullets_at_top += 1;
            first_content_seen = true;
        } else if t.starts_with("|") {
            // Tables are allowed if multi-col and multi-row. Don't flag.
            return false;
        } else {
            let _ = first_content_seen;
            break;
        }
    }
    bullets_at_top >= 4
}

fn count_label_colon_bullets(lines: &[&str]) -> usize {
    let mut n = 0;
    for line in lines {
        let t = line.trim_start();
        let rest = match t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")) {
            Some(r) => r,
            None => continue,
        };
        // Pattern: `<label>: <one short line>` where the colon is in the first
        // half of the content and there are no nested bullets.
        if let Some(colon_pos) = rest.find(':') {
            let before = &rest[..colon_pos];
            let after = rest[colon_pos + 1..].trim();
            let is_short_label = before.len() < 40 && !before.contains(' ').then_some(true).unwrap_or(false);
            let short_after = after.len() < 80 && !after.is_empty();
            if is_short_label && short_after {
                n += 1;
            }
        }
    }
    n
}

fn check_rust_doc_comments(source: &str, crate_name: &str, out: &mut Vec<LintError>) {
    // Extract doc comments (`///` and `//!`) as a concatenated corpus, then
    // apply check_text. Line numbers in errors point at the first match.
    let mut corpus = String::new();
    for line in source.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("///").or_else(|| t.strip_prefix("//!")) {
            corpus.push_str(rest);
            corpus.push('\n');
        }
    }
    if corpus.trim().is_empty() { return; }
    check_text(&corpus, crate_name, out);
}

#[allow(dead_code)]
fn _keep_path_alive(_p: PathBuf) {}

#[cfg(test)]
mod emphasis_tests {
    use super::{emphasis_runs, strip_code_spans};

    fn markers(prose: &str) -> Vec<&'static str> {
        emphasis_runs(prose).into_iter().map(|(_, m)| m).collect()
    }

    #[test]
    fn bold_is_caught_in_both_spellings() {
        assert_eq!(markers("a **loud** claim"), vec!["**"]);
        assert_eq!(markers("a __loud__ claim"), vec!["__"]);
    }

    #[test]
    fn italic_is_caught_in_both_spellings() {
        assert_eq!(markers("a *quiet* claim"), vec!["*"]);
        assert_eq!(markers("a _quiet_ claim"), vec!["_"]);
    }

    #[test]
    fn a_line_with_several_runs_reports_once() {
        assert_eq!(markers("**one** and **two** and **three**"), vec!["**"]);
    }

    #[test]
    fn the_line_number_is_the_line_it_sits_on() {
        let prose = "clean\nclean\na **loud** claim\nclean";
        assert_eq!(emphasis_runs(prose), vec![(3, "**")]);
    }

    #[test]
    fn every_line_carrying_emphasis_is_reported() {
        let prose = "**one**\nclean\n*two*";
        assert_eq!(emphasis_runs(prose), vec![(1, "**"), (3, "*")]);
    }

    #[test]
    fn bold_is_reported_as_bold_rather_than_twice_as_italic() {
        // The doubled form contains the single one, so a naive check would
        // report `**bold**` under both markers.
        assert_eq!(markers("**bold**"), vec!["**"]);
    }

    // The negatives. Each of these is a construction the ban does not reach, and
    // each would fire under a plainer `contains('*')` check.

    #[test]
    fn plain_prose_is_left_alone() {
        assert!(markers("a claim carrying its weight in the words").is_empty());
    }

    #[test]
    fn multiplication_is_not_emphasis() {
        assert!(markers("the area is a * b, in whole pixels").is_empty());
        assert!(markers("width * height * 4").is_empty());
    }

    #[test]
    fn a_snake_case_identifier_is_not_emphasis() {
        assert!(markers("takes a from_raw and a to_raw").is_empty());
        assert!(markers("one_two_three_four").is_empty());
    }

    #[test]
    fn a_leading_underscore_is_not_emphasis() {
        assert!(markers("_unused and _opaque are both fine").is_empty());
    }

    #[test]
    fn a_markdown_list_bullet_is_not_emphasis() {
        assert!(markers("* one item").is_empty());
        assert!(markers("* one\n* two\n* three").is_empty());
    }

    #[test]
    fn a_horizontal_rule_is_not_emphasis() {
        assert!(markers("---").is_empty());
        assert!(markers("***").is_empty());
    }

    #[test]
    fn a_pointer_type_is_not_emphasis() {
        assert!(markers("takes a *mut Proxy and returns a *const Interface").is_empty());
    }

    #[test]
    fn a_doc_comment_prefix_does_not_confuse_it() {
        assert_eq!(markers("/// A **loud** claim."), vec!["**"]);
        assert!(markers("/// A plain claim.").is_empty());
    }

    #[test]
    fn emphasis_inside_a_code_span_does_not_reach_the_check() {
        // The pipeline strips code spans before this runs, so the guarantee is
        // about the pair rather than about `emphasis_runs` alone.
        let stripped = strip_code_spans("the literal `**` is not emphasis");
        assert!(markers(&stripped).is_empty());
    }

    #[test]
    fn emphasis_inside_a_fenced_block_does_not_reach_the_check() {
        let stripped = strip_code_spans("prose\n```\nlet x = **y;\n```\nmore prose");
        assert!(markers(&stripped).is_empty());
    }

    #[test]
    fn an_unclosed_marker_is_not_emphasis() {
        assert!(markers("a *start with no end").is_empty());
        assert!(markers("a **start with no end").is_empty());
    }

    #[test]
    fn an_empty_line_is_not_emphasis() {
        assert!(markers("").is_empty());
        assert!(markers("\n\n\n").is_empty());
    }
}
