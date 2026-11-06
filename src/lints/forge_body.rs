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

use mockspace_lint_rules::{Lint, LintError, MessageContext, MessageDomain, MessageLint, Severity};

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

    /// The body is in the command when an agent hook is the caller, so this
    /// lint cannot do its job without the invocation. `body_source` says what it
    /// does with it.
    fn invocation_wanted(&self) -> bool {
        true
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

        // The forbidden scan reads everything, and the two shape checks read
        // only what was authored. That asymmetry is deliberate and it is not
        // the same defect wearing two faces.
        //
        // A shape check over the whole text answers about the command: a
        // configured minimum is satisfied by a long command carrying an empty
        // body, and a required section is found among the command's own words.
        // Both fail permissively, so narrowing them is the fix.
        //
        // A forbidden pattern is the other way round. Narrowing that scan to
        // the extracted body loses every shape the extractor does not reach and
        // loses the title, which publishes exactly as the body does, so a
        // pattern moved from one to the other stops being seen.
        //
        // The price is stated rather than minimised: on the hook path this text
        // is the whole serialised tool input, so the pattern is also caught in
        // a file path and in the agent's own description of the call, neither
        // of which publishes anything. That is a refusal somebody can read and
        // answer in one edit, and the other direction publishes the thing.
        let everything = ctx.message.to_ascii_lowercase();
        for (pattern, reason) in &self.forbidden {
            if everything.contains(&pattern.to_ascii_lowercase()) {
                out.push(finding(ctx, "forbidden-pattern", &match reason {
                    Some(r) => format!("`{pattern}` not permitted: {r}"),
                    None => format!("`{pattern}` not permitted"),
                }));
            }
        }

        let Some(body) = body_source(ctx) else {
            return out;
        };
        let lower = body.to_ascii_lowercase();

        for section in &self.required_sections {
            if !lower.contains(&section.to_ascii_lowercase()) {
                out.push(finding(
                    ctx,
                    "missing-section",
                    &format!("body lacks a `{section}` section"),
                ));
            }
        }

        if self.min_length > 0 {
            let len = body.trim().chars().count();
            if len < self.min_length {
                out.push(finding(
                    ctx,
                    "too-short",
                    &format!(
                        "body is {len} characters, under the minimum of {}",
                        self.min_length
                    ),
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

/// The text that is the body, which is not always `ctx.message`.
///
/// The same two callers as the commit lint has, and the same trap. Without an
/// invocation the text is the body, which is how a forge tool's own hook would
/// call this. With one, the text is the serialised tool input and the body is on
/// the command line, so measuring the given text measures the command: a
/// configured minimum would then be satisfied by a long command carrying an
/// empty body, and a required section would be found in the command's own words.
///
/// `--body-file` names a path this cannot open, so it declines rather than
/// measuring the path's length, which is the shape that reads as a real check
/// and is not one.
fn body_source(ctx: &MessageContext) -> Option<String> {
    let Some(invocation) = ctx.invocation.as_ref() else {
        return Some(ctx.message.to_string());
    };
    super::authored_message::body_on_the_command_line(invocation.command?)
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
mod tests;
