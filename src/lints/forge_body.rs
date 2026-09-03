//! Lint: a pull-request or merge-request body, and arbitrary forbidden content.
//!
//! A forge body never passes through git, so no git hook can inspect one. This
//! and [`super::message_attribution`] are the only layers that can, and they are
//! reached from the agent hook before `gh` is invoked.
//!
//! Two jobs, both entirely configured:
//!
//! The first is shape: required sections, a minimum length, and whether process
//! narrative is permitted. A body that reads "final state after review iterations"
//! tells a reader nothing about what changed, and the reader six months from now
//! is the one the body exists for.
//!
//! The second is forbidden patterns: an arbitrary list a project supplies, each with an
//! optional reason shown when it matches, and each scoped to the surfaces it
//! applies to. This is the general facility: internal hostnames, ticket URLs
//! that mean nothing publicly, vocabulary a project has retired. Nothing is
//! forbidden by default, because what counts as leakage is entirely
//! project-specific.

use std::collections::HashMap;

use mockspace_lint_rules::{
    Lint,
    LintError,
    MessageContext,
    MessageDomain,
    MessageLint,
    Severity,
};

const LINT_NAME: &str = "forge-body";

#[derive(Default)]
pub struct ForgeBody {
    /// Headings the body must contain, matched case-insensitively as substrings.
    required_sections: Vec<String>,
    /// Minimum authored length in characters. Zero disables the check.
    min_length:        usize,
    /// Patterns forbidden anywhere in the body, as `pattern` or
    /// `pattern=reason`, matched case-insensitively as substrings.
    forbidden:         Vec<(String, Option<String>)>,
}

impl Lint for ForgeBody {
    fn name(&self) -> &'static str {
        LINT_NAME
    }

    fn description(&self) -> &'static str {
        "a forge body carries the sections a project requires and none of its forbidden content"
    }

    fn source_only(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }

    fn finding_kinds(&self) -> &[&str] {
        &["missing-section", "too-short", "forbidden-pattern"]
    }

    fn config_keys(&self) -> &[&str] {
        &["required_sections", "min_length", "forbidden"]
    }

    fn configure(&mut self, params: &HashMap<String, String>) {
        if let Some(v) = params.get("required_sections") {
            self.required_sections = split_list(v);
        }
        if let Some(v) = params.get("min_length") {
            if let Ok(n) = v.trim().parse::<usize>() {
                self.min_length = n;
            }
        }
        if let Some(v) = params.get("forbidden") {
            self.forbidden = split_list(v)
                .into_iter()
                .map(|entry| {
                    match entry.split_once('=') {
                        Some((p, reason)) => {
                            (p.trim().to_string(), Some(reason.trim().to_string()))
                        },
                        None => (entry, None),
                    }
                })
                .collect();
        }
    }
}

impl MessageLint for ForgeBody {
    fn domains(&self) -> &[MessageDomain] {
        // Shape applies to a PR or MR body. A comment is not expected to carry
        // sections, so holding one to the same requirements would be nonsense.
        &[MessageDomain::PullRequestBody]
    }

    fn check_message(&self, ctx: &MessageContext) -> Vec<LintError> {
        let mut out = Vec::new();
        let lower = ctx.message.to_ascii_lowercase();

        for section in &self.required_sections {
            if !lower.contains(&section.to_ascii_lowercase()) {
                out.push(finding(
                    ctx,
                    "missing-section",
                    &format!("the body should contain a `{section}` section"),
                ));
            }
        }

        if self.min_length > 0 {
            let len = ctx.message.trim().chars().count();
            if len < self.min_length {
                out.push(finding(
                    ctx,
                    "too-short",
                    &format!(
                        "the body is {len} characters, under the configured minimum of {}",
                        self.min_length
                    ),
                ));
            }
        }

        for (pattern, reason) in &self.forbidden {
            if lower.contains(&pattern.to_ascii_lowercase()) {
                out.push(finding(
                    ctx,
                    "forbidden-pattern",
                    &match reason {
                        Some(r) => format!("`{pattern}` is not permitted here: {r}"),
                        None => format!("`{pattern}` is not permitted here"),
                    },
                ));
            }
        }

        out
    }
}

fn split_list(v: &str) -> Vec<String> {
    v.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn finding(ctx: &MessageContext, kind: &'static str, message: &str) -> LintError {
    LintError::with_finding_kind(
        ctx.origin.to_string(),
        1,
        LINT_NAME,
        message.to_string(),
        Severity::HARD_ERROR,
        kind,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockspace_lint_rules::AgentMode;

    fn check(l: &ForgeBody, domain: MessageDomain, msg: &str) -> Vec<String> {
        let ctx = MessageContext {
            domain,
            mode: AgentMode::Assistant,
            message: msg,
            origin: "pr-body",
            repo_root: std::path::Path::new("/tmp"),
            invocation: None,
        };
        l.check_message(&ctx)
            .into_iter()
            .map(|e| e.finding_kind.unwrap_or("none").to_string())
            .collect()
    }

    fn with(pairs: &[(&str, &str)]) -> ForgeBody {
        let mut l = ForgeBody::default();
        let mut p = HashMap::new();
        for (k, v) in pairs {
            p.insert((*k).to_string(), (*v).to_string());
        }
        l.configure(&p);
        l
    }

    #[test]
    fn an_unconfigured_lint_imposes_nothing() {
        let l = ForgeBody::default();
        assert!(check(&l, MessageDomain::PullRequestBody, "").is_empty());
        assert!(check(&l, MessageDomain::PullRequestBody, "anything at all").is_empty());
    }

    #[test]
    fn required_sections_are_reported_individually() {
        let l = with(&[("required_sections", "## Summary,## Test plan")]);
        assert_eq!(
            check(&l, MessageDomain::PullRequestBody, "## Summary\nx"),
            vec!["missing-section"]
        );
        assert!(
            check(&l, MessageDomain::PullRequestBody, "## Summary\nx\n\n## Test plan\ny").is_empty()
        );
    }

    #[test]
    fn section_matching_ignores_case() {
        let l = with(&[("required_sections", "## Summary")]);
        assert!(check(&l, MessageDomain::PullRequestBody, "## SUMMARY\nx").is_empty());
    }

    #[test]
    fn the_minimum_length_counts_characters_of_trimmed_text() {
        let l = with(&[("min_length", "20")]);
        assert_eq!(
            check(&l, MessageDomain::PullRequestBody, "   short   "),
            vec!["too-short"]
        );
        assert!(
            check(&l, MessageDomain::PullRequestBody, "a body long enough to pass the check")
                .is_empty()
        );
    }

    #[test]
    fn a_forbidden_pattern_can_carry_its_reason() {
        // The reason is the point: a bare "not permitted" leaves the author
        // guessing why, and they will work around it rather than fix it.
        let l = with(&[("forbidden", "staging.internal=internal hosts do not belong in a public record")]);
        let ctx = MessageContext {
            domain:     MessageDomain::PullRequestBody,
            mode:       AgentMode::Assistant,
            message:    "see https://staging.internal/x",
            origin:     "pr-body",
            repo_root:  std::path::Path::new("/tmp"),
            invocation: None,
        };
        let errs = l.check_message(&ctx);
        assert_eq!(errs.len(), 1);
        assert!(
            errs[0].message.contains("internal hosts do not belong"),
            "the configured reason should be shown: {}",
            errs[0].message
        );
    }

    #[test]
    fn a_forbidden_pattern_without_a_reason_still_works() {
        let l = with(&[("forbidden", "wip")]);
        assert_eq!(
            check(&l, MessageDomain::PullRequestBody, "WIP do not merge"),
            vec!["forbidden-pattern"]
        );
    }

    #[test]
    fn several_forbidden_patterns_each_report() {
        let l = with(&[("forbidden", "wip,do not merge")]);
        assert_eq!(
            check(&l, MessageDomain::PullRequestBody, "wip: do not merge"),
            vec!["forbidden-pattern", "forbidden-pattern"]
        );
    }

    #[test]
    fn the_shape_rules_do_not_apply_to_a_comment() {
        // A review comment is not expected to carry a Summary section, so the
        // domain restriction is load-bearing rather than decoration.
        let l = with(&[("required_sections", "## Summary")]);
        assert_eq!(l.domains(), &[MessageDomain::PullRequestBody]);
        // and the runner is what enforces it, so check the declaration itself
        assert!(!l.domains().contains(&MessageDomain::ReviewComment));
    }

    #[test]
    fn every_finding_kind_the_lint_emits_is_declared() {
        let l = with(&[
            ("required_sections", "## Summary"),
            ("min_length", "100"),
            ("forbidden", "wip"),
        ]);
        let declared = l.finding_kinds();
        for kind in check(&l, MessageDomain::PullRequestBody, "wip") {
            assert!(declared.contains(&kind.as_str()), "`{kind}` is not declared");
        }
    }
}
