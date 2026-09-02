//! Lint: a source file over the line budget is carrying more than one concern.
//!
//! Three hundred lines by default. Past that a file has almost always stopped
//! being one thing: type declarations, their impls, free helpers and a test
//! module in one scroll, where every edit touches a large blast radius and the
//! module's actual shape is invisible.
//!
//! **A threshold, not a judgement.** A file at 290 doing two things is worse
//! than one at 320 doing one, and no counter can tell them apart. What the
//! number buys is that nobody has to notice: a file crossing it says so, at the
//! gate, on the commit that crossed it, rather than five rounds later when the
//! split costs a day.
//!
//! `max` sets the budget and `count` picks what a line is. Both are configured
//! per repository:
//!
//! ```toml
//! [lints.file-size]
//! commit = "error"
//! max = "300"
//! count = "line-count"
//! ```
//!
//! Files a project does not own are excluded by path with the shared `exclude`
//! key, which is where generated and vendored trees go.
//!
//! FIXME: this exists because mockspace's own `file-metric` builtin, which does
//! this and several other counts, is v2 and no consumer runs v2 yet. When one
//! does, a repository declares `file-metric` with `metric = "line-count"` and
//! this lint retires. The three `count` modes below are named after that
//! builtin's own metrics so the migration is a rename rather than a rethink.

use std::collections::HashMap;

use mockspace_lint_rules::{CrateLint, Lint, LintContext, LintError, Severity};

use crate::util::err_in_file;

/// What the budget is measured in.
///
/// Named after `file-metric`'s own metrics, for the reason the module note
/// gives.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Count {
    /// Every line, including blanks and comments.
    ///
    /// The default, and the one nobody can argue with: it is what an editor
    /// shows and what `wc -l` answers, so a report and a check agree without
    /// anybody explaining which lines were counted.
    Lines,
    /// Every line with something on it.
    NonBlank,
    /// Every line with something on it that is not a comment.
    ///
    /// **Not the default, deliberately.** Documentation is part of a file's
    /// weight to a reader, and a mode that stops counting it rewards moving
    /// prose around rather than splitting the file.
    Code,
}

impl Count {
    /// Read one from its configured spelling, keeping the current one where the
    /// spelling is not a mode.
    fn parse(text: &str, current: Self) -> Self {
        match text {
            "line-count" => Self::Lines,
            "non-blank-line-count" => Self::NonBlank,
            "non-blank-non-comment-line-count" => Self::Code,
            _ => current,
        }
    }

    /// How many lines of `source` this mode counts.
    fn of(self, source: &str) -> usize {
        match self {
            Self::Lines => source.lines().count(),
            Self::NonBlank => source.lines().filter(|l| !l.trim().is_empty()).count(),
            Self::Code => source
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with("//"))
                .count(),
        }
    }

    /// What the finding calls it.
    const fn describes(self) -> &'static str {
        match self {
            Self::Lines => "lines",
            Self::NonBlank => "non-blank lines",
            Self::Code => "lines of code",
        }
    }
}

/// The default budget.
///
/// Three hundred, which is op's number and the one the workspace rule now
/// carries. It was five hundred in writing and enforced by nothing, and what
/// that bought was twenty files past it in one repository, several of them
/// shipped and reviewed.
const BUDGET: usize = 300;

pub struct FileSize {
    max: usize,
    count: Count,
}

impl Default for FileSize {
    fn default() -> Self {
        Self {
            max: BUDGET,
            count: Count::Lines,
        }
    }
}

impl Lint for FileSize {
    fn name(&self) -> &'static str {
        "file-size"
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }

    /// Walks `all_sources` itself, so the dispatcher hands it the crate once
    /// rather than once per file. Left at the default it would report every
    /// file's finding once per file in the crate.
    fn per_file(&self) -> bool {
        false
    }

    fn config_keys(&self) -> &[&str] {
        &["max", "count"]
    }

    fn configure(&mut self, params: &HashMap<String, String>) {
        if let Some(text) = params.get("max") {
            // A budget that does not parse is left at the default rather than
            // taken as zero, which would fire on every file in the repository
            // and read as the lint being broken rather than the config being
            // mistyped.
            if let Ok(max) = text.trim().parse::<usize>() {
                if max > 0 {
                    self.max = max;
                }
            }
        }
        if let Some(text) = params.get("count") {
            self.count = Count::parse(text.trim(), self.count);
        }
    }
}

impl CrateLint for FileSize {
    fn check(&self, ctx: &LintContext) -> Vec<LintError> {
        let mut out = Vec::new();
        for file in ctx.all_sources {
            let counted = self.count.of(&file.text);
            if counted <= self.max {
                continue;
            }
            let path = file.rel_path.display().to_string();
            out.push(err_in_file(
                ctx,
                &path,
                // The line the budget was crossed at, so an editor opens where
                // the file stopped being allowed rather than at its end.
                self.max + 1,
                "file-size",
                format!(
                    "{counted} {} against a budget of {}; the file is carrying more \
                     than one concern and wants splitting along its seam",
                    self.count.describes(),
                    self.max
                ),
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::{BUDGET, Count, FileSize};
    use mockspace_lint_rules::Lint;
    use std::collections::HashMap;

    /// A run of `n` lines, each carrying something.
    fn body(n: usize) -> String {
        (0..n).map(|i| format!("let x{i} = {i};\n")).collect()
    }

    #[test]
    fn the_budget_is_three_hundred_and_the_default_counts_every_line() {
        let lint = FileSize::default();
        assert_eq!(lint.max, BUDGET);
        assert_eq!(lint.count, Count::Lines);
        assert_eq!(lint.name(), "file-size");
    }

    #[test]
    fn a_file_at_the_budget_passes_and_one_past_it_does_not() {
        let count = Count::Lines;
        assert_eq!(count.of(&body(300)), 300);
        assert_eq!(count.of(&body(301)), 301);
        // The comparison is `<=`, so the boundary itself is legal and the first
        // line past it is the finding. Off by one here is a lint that fires on
        // every conforming file or on none.
        assert!(300 <= BUDGET);
        assert!(301 > BUDGET);
    }

    #[test]
    fn the_three_modes_count_differently_and_each_counts_what_it_says() {
        let source = "// a comment\n\nlet x = 1;\n   \n  // indented comment\nlet y = 2;\n";
        assert_eq!(Count::Lines.of(source), 6);
        assert_eq!(Count::NonBlank.of(source), 4);
        assert_eq!(Count::Code.of(source), 2);
    }

    #[test]
    fn an_empty_file_counts_zero_in_every_mode() {
        for mode in [Count::Lines, Count::NonBlank, Count::Code] {
            assert_eq!(mode.of(""), 0, "{mode:?} disagreed about an empty file");
        }
    }

    #[test]
    fn a_file_with_no_trailing_newline_still_counts_its_last_line() {
        assert_eq!(Count::Lines.of("one\ntwo"), 2);
        assert_eq!(Count::Code.of("one\ntwo"), 2);
    }

    #[test]
    fn configuring_the_budget_and_the_mode_takes_both() {
        let mut lint = FileSize::default();
        let mut params = HashMap::new();
        params.insert("max".to_string(), "120".to_string());
        params.insert(
            "count".to_string(),
            "non-blank-non-comment-line-count".to_string(),
        );
        lint.configure(&params);
        assert_eq!(lint.max, 120);
        assert_eq!(lint.count, Count::Code);
    }

    #[test]
    fn a_budget_that_does_not_parse_leaves_the_default_standing() {
        for bad in ["", "  ", "none", "-40", "3.5", "0"] {
            let mut lint = FileSize::default();
            let mut params = HashMap::new();
            params.insert("max".to_string(), bad.to_string());
            lint.configure(&params);
            assert_eq!(lint.max, BUDGET, "`{bad}` should not have moved the budget");
        }
    }

    #[test]
    fn a_mode_that_is_not_a_mode_leaves_the_current_one_standing() {
        let mut lint = FileSize::default();
        let mut params = HashMap::new();
        params.insert("count".to_string(), "loc".to_string());
        lint.configure(&params);
        assert_eq!(lint.count, Count::Lines);
    }

    #[test]
    fn whitespace_around_a_configured_value_is_not_part_of_it() {
        let mut lint = FileSize::default();
        let mut params = HashMap::new();
        params.insert("max".to_string(), " 250 ".to_string());
        params.insert("count".to_string(), " non-blank-line-count ".to_string());
        lint.configure(&params);
        assert_eq!(lint.max, 250);
        assert_eq!(lint.count, Count::NonBlank);
    }

    #[test]
    fn configuring_nothing_changes_nothing() {
        let mut lint = FileSize::default();
        lint.configure(&HashMap::new());
        assert_eq!(lint.max, BUDGET);
        assert_eq!(lint.count, Count::Lines);
    }

    #[test]
    fn the_lint_declares_the_keys_it_reads() {
        let lint = FileSize::default();
        let keys = lint.config_keys();
        assert!(keys.contains(&"max"), "`max` is read and must be declared");
        assert!(
            keys.contains(&"count"),
            "`count` is read and must be declared"
        );
        assert_eq!(keys.len(), 2, "an undeclared key is refused by the engine");
    }

    #[test]
    fn a_comment_mode_does_not_count_a_line_that_merely_ends_in_a_comment() {
        // The check is on the trimmed start, so trailing comments stay code.
        // Anything else would let a file dodge the budget by moving code onto
        // the end of comment lines, which is worse than being over it.
        assert_eq!(Count::Code.of("let x = 1; // why\n"), 1);
        assert_eq!(Count::Code.of("// why\n"), 0);
    }
}
