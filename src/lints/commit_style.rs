//! Lint: a commit message matches the project's declared convention.
//!
//! Every knob is configured, and nothing is a default, because a commit
//! convention is exactly the kind of thing no default fits: it is neither
//! universal nor obviously shared between projects. A project declares its style,
//! or declares none and this lint does nothing.
//!
//! Two presets in `presets/` list the knobs: `commit-conventional` for
//! Conventional Commits as written, and `commit-hiisi` for the stricter variant
//! (no parenthesised scope, lowercase subject, no trailing period, under 72).
//! Nothing loads them yet; set the keys under `[lints.commit-style]`.
//!
//! Shape only. Authorship trailers are [`super::message_attribution`]'s job,
//! because they apply to pull-request bodies too and this lint does not.

use std::collections::HashMap;

use mockspace_lint_rules::{Lint, LintError, MessageContext, MessageDomain, MessageLint, Severity};

const LINT_NAME: &str = "commit-style";

/// How the subject's first letter is constrained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum SubjectCase {
    /// Must begin lowercase. The imperative-mood convention's usual companion.
    Lower,
    /// Must begin uppercase.
    Sentence,
    /// Unconstrained.
    #[default]
    Any,
}

impl SubjectCase {
    fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "lower" | "lowercase" => Some(Self::Lower),
            "sentence" | "upper" | "capital" => Some(Self::Sentence),
            "any" | "off" | "" => Some(Self::Any),
            _ => None,
        }
    }
}

/// The configured convention. Every field starts permissive, so an unconfigured
/// lint reports nothing rather than imposing a convention nobody asked for.
pub struct CommitStyle {
    /// Permitted type prefixes. Empty disables type checking entirely.
    types:                  Vec<String>,
    /// What separates the type from the subject.
    separator:              String,
    /// Suffix marking a breaking change, appended to the type.
    breaking_marker:        String,
    /// Whether a type prefix is required at all.
    require_type:           bool,
    /// Maximum subject length. Zero disables the check.
    max_subject:            usize,
    /// How the subject's first letter is constrained.
    subject_case:           SubjectCase,
    /// Whether `type(scope):` is permitted.
    allow_scope:            bool,
    /// Whether a trailing period on the subject is a violation.
    forbid_trailing_period: bool,
    /// Whether the subject must be blank-line separated from the body.
    require_blank_line:     bool,
}

impl Default for CommitStyle {
    fn default() -> Self {
        Self {
            types:                  Vec::new(),
            separator:              ": ".to_string(),
            breaking_marker:        "!".to_string(),
            require_type:           false,
            max_subject:            0,
            subject_case:           SubjectCase::Any,
            allow_scope:            true,
            forbid_trailing_period: false,
            require_blank_line:     false,
        }
    }
}

impl Lint for CommitStyle {
    fn name(&self) -> &'static str {
        LINT_NAME
    }

    /// The subject is in the command when an agent hook is the caller, so this
    /// lint cannot do its job without the invocation. `subject_source` says
    /// what it does with it.
    fn invocation_wanted(&self) -> bool {
        true
    }

    fn description(&self) -> &'static str {
        "a commit subject matches the project's declared convention"
    }

    fn source_only(&self) -> bool {
        false
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }

    fn finding_kinds(&self) -> &[&str] {
        &[
            "missing-type",
            "unknown-type",
            "scope-forbidden",
            "subject-case",
            "subject-length",
            "trailing-period",
            "missing-blank-line",
            "empty-subject",
        ]
    }

    fn config_keys(&self) -> &[&str] {
        &[
            "types",
            "separator",
            "breaking_marker",
            "require_type",
            "max_subject",
            "subject_case",
            "allow_scope",
            "forbid_trailing_period",
            "require_blank_line",
        ]
    }

    fn configure(&mut self, params: &HashMap<String, String>) {
        if let Some(v) = params.get("types") {
            self.types = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        }
        if let Some(v) = params.get("separator") {
            self.separator = v.clone();
        }
        if let Some(v) = params.get("breaking_marker") {
            self.breaking_marker = v.clone();
        }
        if let Some(v) = params.get("require_type") {
            self.require_type = truthy(v);
        }
        if let Some(v) = params.get("max_subject") {
            if let Ok(n) = v.trim().parse::<usize>() {
                self.max_subject = n;
            }
        }
        if let Some(v) = params.get("subject_case") {
            if let Some(c) = SubjectCase::parse(v) {
                self.subject_case = c;
            }
        }
        if let Some(v) = params.get("allow_scope") {
            self.allow_scope = truthy(v);
        }
        if let Some(v) = params.get("forbid_trailing_period") {
            self.forbid_trailing_period = truthy(v);
        }
        if let Some(v) = params.get("require_blank_line") {
            self.require_blank_line = truthy(v);
        }
    }
}

impl MessageLint for CommitStyle {
    fn domains(&self) -> &[MessageDomain] {
        &[MessageDomain::CommitMessage]
    }

    fn check_message(&self, ctx: &MessageContext) -> Vec<LintError> {
        let mut out = Vec::new();
        let Some(text) = subject_source(ctx) else {
            return out;
        };
        let body = authored_body(&text);
        let Some(subject) = body.lines().next() else {
            return out;
        };
        let subject = subject.trim_end();

        // A fixup or merge subject is generated by git rather than authored, so
        // holding it to an authored convention would fail commits the user never
        // wrote the subject for.
        if is_generated_subject(subject) {
            return out;
        }

        if subject.trim().is_empty() {
            out.push(finding(ctx, "empty-subject", "the commit subject is empty"));
            return out;
        }

        let rest = self.check_type(ctx, subject, &mut out);

        if self.max_subject > 0 && subject.chars().count() > self.max_subject {
            out.push(finding(
                ctx,
                "subject-length",
                &format!(
                    "subject is {} characters, over the limit of {}",
                    subject.chars().count(),
                    self.max_subject
                ),
            ));
        }

        if let Some(first) = rest.trim_start().chars().next() {
            match self.subject_case {
                SubjectCase::Lower if first.is_uppercase() => {
                    out.push(finding(
                        ctx,
                        "subject-case",
                        "subject must start lowercase",
                    ));
                },
                SubjectCase::Sentence if first.is_lowercase() => {
                    out.push(finding(
                        ctx,
                        "subject-case",
                        "subject must start uppercase",
                    ));
                },
                _ => {},
            }
        }

        if self.forbid_trailing_period && subject.ends_with('.') && !subject.ends_with("..") {
            out.push(finding(
                ctx,
                "trailing-period",
                "drop the trailing period",
            ));
        }

        if self.require_blank_line {
            let mut lines = body.lines();
            let _ = lines.next();
            if let Some(second) = lines.next() {
                if !second.trim().is_empty() {
                    out.push(finding(
                        ctx,
                        "missing-blank-line",
                        "separate subject and body with a blank line",
                    ));
                }
            }
        }

        out
    }
}

impl CommitStyle {
    /// Check the type prefix, returning the subject text after it.
    fn check_type<'m>(
        &self,
        ctx: &MessageContext,
        subject: &'m str,
        out: &mut Vec<LintError>,
    ) -> &'m str {
        if self.types.is_empty() && !self.require_type {
            return subject;
        }
        let Some(sep_at) = subject.find(&self.separator) else {
            if self.require_type {
                out.push(finding(
                    ctx,
                    "missing-type",
                    &format!(
                        "subject must start with a type then `{}`; types: {}",
                        self.separator,
                        self.types.join(", ")
                    ),
                ));
            }
            return subject;
        };
        let (head, rest) = subject.split_at(sep_at);
        let rest = &rest[self.separator.len() ..];

        // Strip the breaking marker before matching, so `feat!` is the `feat`
        // type rather than an unknown one.
        let mut ty = head;
        if !self.breaking_marker.is_empty() {
            ty = ty.strip_suffix(&self.breaking_marker).unwrap_or(ty);
        }

        // A parenthesised scope, as Conventional Commits permits.
        let scoped = ty
            .split_once('(')
            .map(|(base, tail)| (base, tail.ends_with(')')));
        let base = match scoped {
            Some((base, closed)) => {
                if !self.allow_scope {
                    out.push(finding(
                        ctx,
                        "scope-forbidden",
                        "drop the parenthesised scope",
                    ));
                } else if !closed {
                    out.push(finding(
                        ctx,
                        "unknown-type",
                        "unclosed scope parenthesis",
                    ));
                }
                base
            },
            None => ty,
        };

        if !self.types.is_empty() && !self.types.iter().any(|t| t == base) {
            out.push(finding(
                ctx,
                "unknown-type",
                &format!(
                    "unknown type `{base}`; types: {}",
                    self.types.join(", ")
                ),
            ));
        }
        rest
    }
}

/// The text whose first line is the subject, which is not always `ctx.message`.
///
/// Two callers hand this lint two different things. The `commit-msg` git hook
/// passes the message file, and that is the message. An agent's PreToolUse hook
/// passes the serialised tool input, because it cannot parse a shell command in
/// shell, and the first line of that is JSON. Reading the second as a subject
/// reports a type of `{"command":"git commit -m 'fix` and a length that is the
/// length of the command line, on every commit, however well written.
///
/// The invocation is what tells them apart: it is present only on the hook
/// path. There, the subject comes out of the command or it does not come at
/// all, and declining is right rather than cautious, since a message authored
/// where the command line cannot show it still reaches the `commit-msg` hook
/// with the real file in hand.
fn subject_source(ctx: &MessageContext) -> Option<String> {
    let Some(invocation) = ctx.invocation.as_ref() else {
        // No invocation at all: the `commit-msg` git hook, whose text is the
        // message file. This is the path every arm in this file's test module
        // other than the hook ones goes through.
        return Some(ctx.message.to_string());
    };
    // FIXME: an MCP commit tool is not subject-checked. The gate spells
    // `--command "$COMMAND"` unconditionally and such a tool has no shell
    // command, so what arrives is `Some("")`, and its message is in a named
    // field of the serialised input rather than on any command line. Reading
    // the serialised input as a subject is what this whole change removes, so
    // the two available answers are both wrong and this takes the quieter one.
    // Closing it means either the gate passing the message it found, or this
    // lint parsing the input, and the second wants a JSON dependency in a
    // library every consumer loads. The gate side is the right half.
    let command = invocation.command.filter(|c| !c.trim().is_empty())?;
    super::authored_message::authored_on_the_command_line(command)
}

/// The authored part of a commit message.
///
/// Drops comment lines and everything from git's verbose-diff scissors marker,
/// so a convention violation quoted in the commented help text, or anything in
/// an attached diff, cannot trip the lint.
fn authored_body(message: &str) -> String {
    message
        .split("# ------------------------ >8")
        .next()
        .unwrap_or(message)
        .lines()
        .filter(|l| !l.starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether git generated this subject rather than a person authoring it.
fn is_generated_subject(subject: &str) -> bool {
    const GENERATED: &[&str] = &["merge ", "revert ", "fixup! ", "squash! ", "amend! ", "rebase "];
    let lower = subject.to_ascii_lowercase();
    GENERATED.iter().any(|p| lower.starts_with(p))
}

fn truthy(v: &str) -> bool {
    matches!(
        v.trim().to_ascii_lowercase().as_str(),
        "true" | "yes" | "on" | "1"
    )
}

fn finding(ctx: &MessageContext, kind: &'static str, message: &str) -> LintError {
    // Carrying the finding kind is what lets a project set a per-kind severity,
    // so a convention can block on an unknown type while only warning on length.
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
