//! What the commit-identity tests share: the pack as the engine drives it, and a
//! commit judged by `message-attribution` alone.
//!
//! Every case goes through the pack as the engine drives it: collected,
//! configured from a `LintConfig`, and run by `check_message_with_extra`. A lint
//! called by hand on a struct nobody configured would pass for a pack whose
//! configuration never reached it.

use std::collections::HashMap;
use std::path::Path;

use mockspace_extra_lints as pack;
use mockspace_lint_rules::{
    AgentMode,
    LintConfig,
    LintPack,
    MessageContext,
    MessageDomain,
    check_message_with_extra,
};

pub const AGENT: &str = "Claude <noreply@anthropic.com>";
pub const PERSON: &str = "Jane Smith <jane@example.com>";

/// The pack, configured as the engine configures it, with `params` as the
/// `[lints.message-attribution]` section.
pub fn configured(params: &[(&str, &str)]) -> (LintPack, LintConfig) {
    let mut p = LintPack::default();
    pack::collect(&mut p);
    let mut cfg = LintConfig::empty();
    cfg.params.insert(
        "message-attribution".to_string(),
        params
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect::<HashMap<_, _>>(),
    );
    p.configure_from(&cfg);
    (p, cfg)
}

/// What `message-attribution` alone says about a commit: its finding kinds, and
/// the text of each.
pub fn judged(
    params: &[(&str, &str)],
    mode: AgentMode,
    message: &str,
    author: Option<&str>,
    committer: Option<&str>,
) -> Vec<(String, String)> {
    let (p, cfg) = configured(params);
    let ctx = MessageContext::new(
        MessageDomain::CommitMessage,
        mode,
        message,
        "abc1234 feat: a subject",
        Path::new("/tmp"),
    )
    .with_identity(author, committer);
    check_message_with_extra(&ctx, Some(&cfg), &p.message_lints)
        .into_iter()
        .filter(|f| f.lint_name == "message-attribution")
        .map(|f| (f.finding_kind.unwrap_or("none").to_string(), f.message))
        .collect()
}

/// A clean message, so any finding is about the identity.
pub const CLEAN: &str = "feat: a subject\n\nA plain body.\n";

pub fn kinds(found: &[(String, String)]) -> Vec<&str> {
    found.iter().map(|(k, _)| k.as_str()).collect()
}
