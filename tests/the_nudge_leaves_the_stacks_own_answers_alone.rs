//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What the alias nudge fires on, and what it must not.
//!
//! The lint had no tests of its own while it carried `Bool` in its trigger
//! list, so nothing said that the pack was advising against the very type the
//! no-bare-primitives rule names as the answer for a truth value. A predicate
//! returning `Bool` has done what the stack asks, and a nudge on it is two
//! rules in one pack disagreeing at one position.
//!
//! Both halves are here on purpose. The refusals alone would pass against a
//! lint that fires on nothing at all, so the width-shaped arm is the control
//! that says the lint still works.

use mockspace_extra_lints::lints::semantic_alias_nudge::SemanticAliasNudge;
use mockspace_lint_rules::CrateLint;
use mockspace_lint_rules::testkit::LintFixture;

fn ctx_with(source: &str) -> LintFixture {
    LintFixture::new(source).with_workspace_root(std::path::Path::new("/tmp"))
}

/// The control, and it has to come first: without it every refusal below is
/// satisfied by a lint that never fires.
#[test]
fn a_width_shaped_primitive_at_a_public_position_is_still_nudged() {
    let src = "pub fn record_at(index: QWord) -> Byte { index.into() }\n";
    let hits = SemanticAliasNudge.check(&ctx_with(src).ctx());
    assert_eq!(
        hits.len(),
        1,
        "a public signature naming a width-shaped primitive must still be nudged"
    );
}

/// The regression this file exists for.
#[test]
fn a_predicate_returning_the_stacks_own_truth_value_is_not_nudged() {
    let src = "pub fn denotes(&self) -> Bool { self.inner }\n";
    let hits = SemanticAliasNudge.check(&ctx_with(src).ctx());
    assert!(
        hits.is_empty(),
        "`Bool` is what the no-bare-primitives rule asks for at this position, so nudging away \
         from it advises against the pack's own rule: {hits:?}"
    );
}

/// The three that left the list with `Bool`, each for the same reason: the name
/// already says what the value is, so an alias over one is a rename.
#[test]
fn a_bound_and_the_platform_indices_are_not_nudged() {
    for src in [
        "pub fn capacity(&self) -> Cap { self.cap }\n",
        "pub fn count(&self) -> USize { self.n }\n",
        "pub fn delta(&self) -> ISize { self.d }\n",
    ] {
        let hits = SemanticAliasNudge.check(&ctx_with(src).ctx());
        assert!(
            hits.is_empty(),
            "`{src}` names what the value is and must not be nudged: {hits:?}"
        );
    }
}

/// A private signature is nobody's public reading problem.
#[test]
fn a_private_signature_is_not_nudged_whatever_it_names() {
    let src = "fn record_at(index: QWord) -> Byte { index.into() }\n";
    let hits = SemanticAliasNudge.check(&ctx_with(src).ctx());
    assert!(
        hits.is_empty(),
        "the nudge is about public reading and must not reach a private fn: {hits:?}"
    );
}

/// The advice a consumer reads must not carry another repo's nouns.
#[test]
fn the_message_names_no_other_repos_vocabulary() {
    let src = "pub fn record_at(index: QWord) -> Byte { index.into() }\n";
    let hits = SemanticAliasNudge.check(&ctx_with(src).ctx());
    let msg = format!("{:?}", hits);
    for noun in ["NodeId", "SlotId", "RecordIndex"] {
        assert!(
            !msg.contains(noun),
            "the pack ships to every consumer, so `{noun}` in the advice carries one repo's \
             vocabulary into all of them: {msg}"
        );
    }
}
