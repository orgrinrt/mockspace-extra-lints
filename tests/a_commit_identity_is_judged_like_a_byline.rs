//! A commit whose author or committer is an agent is judged by
//! `message-attribution` under the policy that judges an agent `Co-Authored-By`.
//!
//! An agent committing under its own name leaves no trailer and no advert in the
//! message, so a check that reads only the message passes it, and the author and
//! committer are where the claim is made.
//!
//! Every case here goes through the pack as the engine drives it: collected,
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

const AGENT: &str = "Claude <noreply@anthropic.com>";
const PERSON: &str = "Jane Smith <jane@example.com>";

/// The pack, configured as the engine configures it, with `params` as the
/// `[lints.message-attribution]` section.
fn configured(params: &[(&str, &str)]) -> (LintPack, LintConfig) {
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
fn judged(
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
const CLEAN: &str = "feat: a subject\n\nA plain body.\n";

fn kinds(found: &[(String, String)]) -> Vec<&str> {
    found.iter().map(|(k, _)| k.as_str()).collect()
}

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

// --- what counts as an agent: the shared conformance table --------------------------------
//
// The rows are the table that ships with `mockspace-lint-rules`, the one a second
// implementation of the recogniser, in shell, is held to as well. Every row is run
// through the pack as the engine drives it, as an author, as a committer and as the
// value of a `Co-Authored-By`, so what the recogniser decides and what the lint does
// with it are tested together and neither can regress behind the other.

use mockspace_lint_rules::agent_identity_conformance::table;

#[test]
fn the_shared_table_is_read_and_has_both_verdicts() {
    let t = table();
    assert!(!t.people.is_empty() && !t.agents.is_empty());
}

#[test]
fn no_person_row_is_refused_as_an_author_or_a_committer_under_either_mode() {
    for mode in [AgentMode::Assistant, AgentMode::Autonomous] {
        for who in &table().people {
            let found = judged(&[], mode, CLEAN, Some(who), Some(who));
            assert!(
                found.is_empty(),
                "{who} under {mode:?} was refused: {found:?}"
            );
        }
    }
}

#[test]
fn no_person_row_is_refused_as_a_co_author() {
    // The trailer path asks the same question of the same list, so it had the same
    // false positives, and a co-author who is a real person is the case the lint
    // has always said it never touches.
    for who in &table().people {
        let message = format!("feat: a subject\n\nCo-Authored-By: {who}\n");
        let found = judged(&[], AgentMode::Assistant, &message, None, None);
        assert!(
            found.is_empty(),
            "{who} as a co-author was refused: {found:?}"
        );
    }
}

#[test]
fn every_agent_row_is_refused_as_an_author_and_as_a_committer() {
    for who in &table().agents {
        let as_author = judged(&[], AgentMode::Assistant, CLEAN, Some(who), Some(PERSON));
        assert_eq!(
            kinds(&as_author),
            vec!["identity"],
            "{who} as the author: {as_author:?}"
        );
        let as_committer = judged(&[], AgentMode::Assistant, CLEAN, Some(PERSON), Some(who));
        assert_eq!(
            kinds(&as_committer),
            vec!["identity"],
            "{who} as the committer: {as_committer:?}"
        );
    }
}

#[test]
fn every_agent_row_is_refused_as_a_co_author() {
    // Either as a byline or, where the whole line is one a tool advertises with,
    // as an advert: both are a refusal, and a row the trailer path passes is the
    // regression this guards.
    for who in &table().agents {
        let message = format!("feat: a subject\n\nCo-Authored-By: {who}\n");
        let found = judged(&[], AgentMode::Assistant, &message, None, None);
        assert!(
            !found.is_empty() && found.iter().all(|(k, _)| k == "byline" || k == "advert"),
            "{who} as a co-author: {found:?}"
        );
    }
}

#[test]
fn a_person_whose_whole_name_is_a_given_name_tool_is_not_refused() {
    // The edge the second signal exists for. `Claude` and `Devin` on their own are
    // somebody's name, and `Claude Code` or a mailbox the tool commits from is the
    // tool. They are rows of the table as well; this names the reason.
    for who in [
        "Claude <claude@example.com>",
        "Devin <devin@patel.example>",
        "Claude Max <c@example.com>",
    ] {
        let found = judged(&[], AgentMode::Assistant, CLEAN, Some(who), Some(who));
        assert!(found.is_empty(), "{who}: {found:?}");
    }
    let found = judged(
        &[],
        AgentMode::Assistant,
        CLEAN,
        Some("Claude Code <c@example.com>"),
        None,
    );
    assert_eq!(kinds(&found), vec!["identity"], "got: {found:?}");
}

// --- the three keys -----------------------------------------------------------------------

#[test]
fn agent_identities_replaces_the_whole_list_and_says_so() {
    // The existing key, kept as the full replacement. What it costs is written down
    // here so it is a known cost: a list naming only a tool disarms every default
    // mailbox and marker with it, which is why the additive key exists.
    let params = [("agent_identities", "robotron")];
    let found = judged(
        &params,
        AgentMode::Assistant,
        CLEAN,
        Some("Robotron <r@robotron.test>"),
        Some(PERSON),
    );
    assert_eq!(kinds(&found), vec!["identity"], "{found:?}");
    for who in [
        "Copilot <x@example.com>",
        "dependabot[bot] <x@example.com>",
        "Dev Container <noreply@anthropic.com>",
    ] {
        let found = judged(
            &params,
            AgentMode::Assistant,
            CLEAN,
            Some(who),
            Some(PERSON),
        );
        assert!(
            found.is_empty(),
            "{who} should be disarmed by a full replacement: {found:?}"
        );
    }
}

#[test]
fn extra_agent_identities_adds_to_the_defaults_and_disarms_none_of_them() {
    let params = [(
        "extra_agent_identities",
        "robotron,robot@robotron.test,[auto]",
    )];
    for who in [
        "Robotron CLI <x@example.com>",
        "Build <robot@robotron.test>",
        "ci[auto] <x@example.com>",
        // every default arm still holds
        "Copilot <x@example.com>",
        "dependabot[bot] <x@example.com>",
        "Dev Container <noreply@anthropic.com>",
    ] {
        let found = judged(
            &params,
            AgentMode::Assistant,
            CLEAN,
            Some(who),
            Some(PERSON),
        );
        assert_eq!(kinds(&found), vec!["identity"], "{who}: {found:?}");
    }
    let found = judged(
        &params,
        AgentMode::Assistant,
        CLEAN,
        Some("Robert Robotron-Smith <rob@example.com>"),
        Some("Claude Monet <monet@example.org>"),
    );
    assert!(
        found.is_empty(),
        "a name merely holding the word: {found:?}"
    );
}

#[test]
fn not_agents_names_the_people_who_are_never_read_as_agents() {
    let identity = "Cody Agent <cody@example.com>";
    let found = judged(
        &[],
        AgentMode::Assistant,
        CLEAN,
        Some(identity),
        Some(identity),
    );
    assert_eq!(
        kinds(&found),
        vec!["identity"],
        "without the key: {found:?}"
    );

    for entry in ["Cody Agent", "cody@example.com", "Cody Agent <cody@example.com>"] {
        let found = judged(
            &[("not_agents", entry)],
            AgentMode::Assistant,
            CLEAN,
            Some(identity),
            Some(identity),
        );
        assert!(found.is_empty(), "`{entry}`: {found:?}");
    }
    // and as a co-author too
    let message = format!("feat: x\n\nCo-Authored-By: {identity}\n");
    let found = judged(
        &[("not_agents", "Cody Agent")],
        AgentMode::Assistant,
        &message,
        None,
        None,
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn the_three_keys_are_declared_so_a_project_can_set_them() {
    let (p, _) = configured(&[]);
    let lint = p
        .message_lints
        .iter()
        .find(|l| l.name() == "message-attribution")
        .expect("the pack ships message-attribution");
    for key in ["agent_identities", "extra_agent_identities", "not_agents"] {
        assert!(
            lint.config_keys().contains(&key),
            "`{key}` is not declared: {:?}",
            lint.config_keys()
        );
    }
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
