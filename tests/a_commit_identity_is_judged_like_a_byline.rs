//! A commit whose author or committer is an agent is judged by
//! `message-attribution` under the policy that judges an agent `Co-Authored-By`.
//!
//! An agent committing under its own name leaves no trailer and no advert in the
//! message, so a check that reads only the message passes it, and the author and
//! committer are where the claim is made. What names an agent is held to the
//! shared table in `an_agent_identity_is_named_by_what_only_an_agent_carries.rs`,
//! and this file is what the lint does with the answer.
//!
//! Every case here goes through the pack as the engine drives it, by the helpers
//! in `commit_identity`.

mod commit_identity;

use commit_identity::{AGENT, CLEAN, PERSON, configured, judged, kinds};
use mockspace_lint_rules::AgentMode;

// --- the refusal, which is the default ------------------------------------------------------

#[test]
fn an_agent_author_is_refused_when_a_human_was_in_the_loop() {
    let found = judged(&[], AgentMode::Assistant, CLEAN, Some(AGENT), Some(PERSON));
    assert_eq!(kinds(&found), vec!["identity"], "got: {found:?}");
    let text = &found[0].1;
    assert!(
        text.contains("the author is an agent identity"),
        "names the role. got: {text}"
    );
    assert!(
        !text.contains("committer"),
        "only the author is at fault. got: {text}"
    );
    assert!(text.contains(AGENT), "names the identity. got: {text}");
    assert!(text.contains("assistant"), "names the mode. got: {text}");
}

#[test]
fn an_agent_committer_is_refused_when_a_human_was_in_the_loop() {
    let found = judged(&[], AgentMode::Assistant, CLEAN, Some(PERSON), Some(AGENT));
    assert_eq!(kinds(&found), vec!["identity"], "got: {found:?}");
    let text = &found[0].1;
    assert!(
        text.contains("the committer is an agent identity"),
        "names the role. got: {text}"
    );
    assert!(
        !text.contains("the author is"),
        "only the committer is at fault. got: {text}"
    );
    assert!(text.contains(AGENT), "names the identity. got: {text}");
}

#[test]
fn one_agent_identity_in_both_fields_is_one_finding_naming_both() {
    // What a container whose global identity was set to the agent produces. Two
    // findings for one identity would double the noise on a push of dozens.
    let found = judged(&[], AgentMode::Assistant, CLEAN, Some(AGENT), Some(AGENT));
    assert_eq!(kinds(&found), vec!["identity"], "got: {found:?}");
    assert!(
        found[0].1.contains("the author and committer are"),
        "got: {}",
        found[0].1
    );
}

#[test]
fn two_different_agent_identities_are_two_findings() {
    let found = judged(
        &[],
        AgentMode::Assistant,
        CLEAN,
        Some(AGENT),
        Some("Copilot <copilot@github.com>"),
    );
    assert_eq!(
        kinds(&found),
        vec!["identity", "identity"],
        "got: {found:?}"
    );
}

// --- absent is not an agent -------------------------------------------------------------

#[test]
fn a_commit_with_no_identity_is_not_judged() {
    // A forge body has none, and neither has a commit the gate could not read
    // one for. Absent is not an agent, and refusing it would make every pull
    // request body fail.
    for (author, committer) in [(None, None), (Some(PERSON), None), (None, Some(PERSON))] {
        let found = judged(&[], AgentMode::Assistant, CLEAN, author, committer);
        assert!(found.is_empty(), "got: {found:?}");
    }
}

// --- the policy is the mode's, the same one a byline answers to -----------------------------

const CLAUDE_GLOB: &str = "Claude *<noreply@anthropic.com>";

/// A message carrying the byline that same glob requires, so the requirement
/// that headless work records one does not speak and only the identity does.
const WITH_BYLINE: &str =
    "feat: a subject\n\nA plain body.\n\nCo-Authored-By: Claude Opus <noreply@anthropic.com>\n";

#[test]
fn an_agent_identity_is_permitted_under_an_autonomous_glob_that_allows_it() {
    let found = judged(
        &[("autonomous", CLAUDE_GLOB)],
        AgentMode::Autonomous,
        WITH_BYLINE,
        Some(AGENT),
        Some(AGENT),
    );
    assert!(found.is_empty(), "the permit path. got: {found:?}");
}

#[test]
fn the_same_identity_under_the_same_config_is_refused_with_a_human_in_the_loop() {
    // The two modes have to genuinely differ, or the predicate is decoration.
    // `assistant` is left empty, which permits nothing.
    let found = judged(
        &[("autonomous", CLAUDE_GLOB)],
        AgentMode::Assistant,
        CLEAN,
        Some(AGENT),
        Some(AGENT),
    );
    assert_eq!(kinds(&found), vec!["identity"], "got: {found:?}");
}

#[test]
fn an_agent_identity_the_glob_does_not_match_is_refused_even_under_autonomous() {
    // Headless mode permits one specific identity, not any agent.
    let found = judged(
        &[("autonomous", CLAUDE_GLOB)],
        AgentMode::Autonomous,
        WITH_BYLINE,
        Some("Copilot <copilot@github.com>"),
        Some("Copilot <copilot@github.com>"),
    );
    assert_eq!(kinds(&found), vec!["identity"], "got: {found:?}");
    assert!(
        found[0].1.contains(CLAUDE_GLOB),
        "names the pattern. got: {}",
        found[0].1
    );
}

#[test]
fn a_permitted_identity_does_not_stand_in_for_the_byline_headless_work_requires() {
    // The identity is permitted, and the message still carries no trailer, so
    // the requirement the autonomous glob sets is unmet. Provenance in the
    // message and provenance in the commit object are two records, and one
    // does not satisfy a requirement written about the other.
    let found = judged(
        &[("autonomous", CLAUDE_GLOB)],
        AgentMode::Autonomous,
        CLEAN,
        Some(AGENT),
        Some(AGENT),
    );
    assert_eq!(kinds(&found), vec!["missing-byline"], "got: {found:?}");
}

/// The value of `autonomous` in a preset, as shipped.
fn autonomous_glob_of(preset: &str) -> String {
    preset
        .lines()
        .find_map(|l| l.strip_prefix("autonomous = \""))
        .and_then(|rest| rest.strip_suffix('"'))
        .unwrap_or_else(|| panic!("the preset sets no autonomous glob:\n{preset}"))
        .to_string()
}

#[test]
fn a_shipped_preset_permits_the_identities_its_tool_really_commits_as() {
    // The globs are read out of the presets, so a preset that stops matching what
    // its tool commits as fails here and is not copied into a test that still
    // passes. The Copilot cases are the two forms a Copilot agent commit takes:
    // the bot account, and the author the coding agent writes, which is named
    // `Copilot` and carries no `[bot]` at all.
    let claude = autonomous_glob_of(include_str!("../presets/attribution-claude.toml"));
    let copilot = autonomous_glob_of(include_str!("../presets/attribution-copilot.toml"));

    let by = |glob: &str, who: &str| {
        judged(
            &[("autonomous", glob)],
            AgentMode::Autonomous,
            // a byline the glob under test also permits, to keep the requirement quiet
            &format!("feat: x\n\nCo-Authored-By: {who}\n"),
            Some(who),
            Some(who),
        )
    };
    for who in [
        "Copilot <198982749+Copilot@users.noreply.github.com>",
        "copilot-swe-agent[bot] <198982749+Copilot@users.noreply.github.com>",
        "GitHub Copilot <copilot@github.com>",
    ] {
        let found = by(&copilot, who);
        assert!(
            found.is_empty(),
            "{who} under the Copilot preset: {found:?}"
        );
    }
    for who in [AGENT, "Claude Opus 4.5 <noreply@anthropic.com>"] {
        let found = by(&claude, who);
        assert!(found.is_empty(), "{who} under the Claude preset: {found:?}");
    }
    // and each is refused under the other's preset
    assert_eq!(
        kinds(&by(
            &claude,
            "Copilot <198982749+Copilot@users.noreply.github.com>"
        )),
        vec!["byline", "identity"]
    );
    assert_eq!(kinds(&by(&copilot, AGENT)), vec!["byline", "identity"]);
}

// --- what the finding tells the person who reads it ----------------------------------------

#[test]
fn the_advice_under_a_human_in_the_loop_remakes_every_commit_in_the_range() {
    // `--reset-author` repairs only the tip, and the push gate sees a range, so
    // the advice that stops at it leaves the push refused on the next commit down.
    let found = judged(&[], AgentMode::Assistant, CLEAN, Some(AGENT), Some(AGENT));
    let text = &found[0].1;
    assert!(text.contains("human in the loop"), "got: {text}");
    assert!(
        text.contains("user.name") && text.contains("user.email"),
        "got: {text}"
    );
    assert!(
        text.contains("git rebase --exec"),
        "how to remake each commit. got: {text}"
    );
    assert!(text.contains("--reset-author"), "got: {text}");
    assert!(
        text.contains("only the tip"),
        "says why a bare amend is not enough. got: {text}"
    );
}

#[test]
fn the_advice_under_autonomous_mode_does_not_say_a_human_was_in_the_loop() {
    // Headless work has no human in the loop, so telling it the commit is the
    // human's is wrong, and the way out is an identity the mode permits.
    let found = judged(
        &[("autonomous", CLAUDE_GLOB)],
        AgentMode::Autonomous,
        WITH_BYLINE,
        Some("Copilot <copilot@github.com>"),
        Some("Copilot <copilot@github.com>"),
    );
    assert_eq!(kinds(&found), vec!["identity"], "{found:?}");
    let text = &found[0].1;
    assert!(!text.contains("human in the loop"), "got: {text}");
    assert!(text.contains(CLAUDE_GLOB), "names the pattern. got: {text}");
    assert!(
        text.contains("git rebase --exec"),
        "how to remake each commit. got: {text}"
    );
}

// --- the finding is declared ---------------------------------------------------------------

#[test]
fn the_identity_finding_kind_is_declared_so_a_project_can_address_it() {
    // A kind a lint emits and does not declare cannot be given its own severity
    // in `[lints.message-attribution]`, which is the one place a project says
    // how hard it wants this refused.
    let (p, _) = configured(&[]);
    let lint = p
        .message_lints
        .iter()
        .find(|l| l.name() == "message-attribution")
        .expect("the pack ships message-attribution");
    let declared = lint.finding_kinds();
    assert!(declared.contains(&"identity"), "declared: {declared:?}");

    let found = judged(&[], AgentMode::Assistant, CLEAN, Some(AGENT), Some(PERSON));
    assert!(!found.is_empty());
    for (kind, _) in &found {
        assert!(
            declared.contains(&kind.as_str()),
            "`{kind}` is emitted and not declared"
        );
    }
}
