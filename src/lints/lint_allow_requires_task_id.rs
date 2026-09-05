//! Lint: every `// lint:allow(...)` escape hatch must say why. Format:
//!
//! ```text
//! // lint:allow(<rule>) reason: <why>
//! ```
//!
//! A bare allow is rejected. The reason is what makes the escape auditable: a
//! later reader can tell an irreducible foreign-ABI boundary from somebody
//! silencing a gate they did not want to satisfy, and only one of those is
//! meant to survive.
//!
//! The lint used to demand `tracked: #<id>` alongside, and no longer does. Op's
//! call: no standard was ever settled for what that identifier refers to, so it
//! pointed at nothing and was written to satisfy the lint rather than to be
//! followed. A number nobody can resolve is worse than no number, because it
//! reads as an audit trail.
//!
//! The name is left as it is. Renaming it would silently disable the lint in
//! every consumer's `mockspace.toml` at once, which is the opposite of what
//! this change is for.

use mockspace_lint_rules::{CrateLint, Lint, LintContext, LintError, Severity};

use crate::util::err;

pub struct LintAllowRequiresTaskId;

impl Lint for LintAllowRequiresTaskId {
    fn name(&self) -> &'static str {
        "lint-allow-requires-task-id"
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

impl CrateLint for LintAllowRequiresTaskId {
    fn check(&self, ctx: &LintContext) -> Vec<LintError> {
        let mut out = Vec::new();
        for (idx, line) in ctx.source.lines().enumerate() {
            let Some(pos) = line.find("lint:allow(") else {
                continue;
            };
            if !has_reason(&line[pos ..]) {
                out.push(err(
                    ctx,
                    idx + 1,
                    "lint-allow-requires-task-id",
                    "lint:allow(...) missing `reason: <why>`; an escape says what it is escaping"
                        .to_string(),
                ));
            }
        }
        out
    }
}

/// Whether an allow carries a reason with something in it.
///
/// The word alone does not count. `reason:` followed by nothing is the shape a
/// writer reaches for when the requirement is in the way, and it carries as
/// little as no reason at all.
fn has_reason(s: &str) -> bool {
    let Some(pos) = s.find("reason:") else {
        return false;
    };
    !s[pos + "reason:".len() ..].trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::has_reason;

    #[test]
    fn a_reason_is_accepted() {
        assert!(has_reason(
            "lint:allow(arvo-types-only) reason: libwayland's own signature"
        ));
    }

    #[test]
    fn a_tracked_id_is_still_accepted_where_one_exists() {
        // Nothing requires it any more and nothing rejects it. Existing markers
        // across the workspace carry one and stay valid.
        assert!(has_reason(
            "lint:allow(no-alloc) reason: proc-macro host; tracked: #42"
        ));
    }

    #[test]
    fn a_missing_id_is_now_fine() {
        // The whole of the change: this line used to report.
        assert!(has_reason("lint:allow(no-std) reason: the build script"));
    }

    #[test]
    fn a_bare_allow_is_refused() {
        assert!(!has_reason("lint:allow(arvo-types-only)"));
    }

    #[test]
    fn an_empty_reason_is_refused() {
        assert!(!has_reason("lint:allow(arvo-types-only) reason:"));
        assert!(!has_reason("lint:allow(arvo-types-only) reason:    "));
    }

    #[test]
    fn a_reason_of_only_a_tracked_id_still_counts_as_a_reason() {
        // Weak and permitted. The lint checks that something was written, not
        // that it was written well, and a reviewer is what reads it.
        assert!(has_reason("lint:allow(no-std) reason: tracked: #7"));
    }

    #[test]
    fn the_word_has_to_be_spelled_the_way_the_format_spells_it() {
        assert!(!has_reason("lint:allow(no-std) because: the build script"));
        assert!(!has_reason("lint:allow(no-std) Reason: the build script"));
    }

    #[test]
    fn a_multi_rule_allow_carrying_a_reason_is_accepted() {
        assert!(has_reason(
            "lint:allow(arvo-types-only, no-public-raw-field) reason: the C struct's own layout"
        ));
    }
}
