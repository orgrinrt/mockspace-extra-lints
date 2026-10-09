//! A commit whose author or committer is an agent is judged by
//! `message-attribution` under the policy that judges an agent `Co-Authored-By`.
//!
//! On 9 October 221 commits on one repository's remote branches, and 16 on a
//! pull request, were found with an agent as author and committer, because the
//! containers' global git identity was set to that. Their messages carried no
//! trailer and no advert, so every message check passed them, and the only thing
//! that read the two fields was a scan run at review time.
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
    let ctx = MessageContext {
        domain: MessageDomain::CommitMessage,
        mode,
        message,
        origin: "abc1234 feat: a subject",
        repo_root: Path::new("/tmp"),
        invocation: None,
        author,
        committer,
    };
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
    // What a container whose global identity was set to the agent produces, and
    // what the 237 commits were. Two findings for one identity would double the
    // noise on a push of dozens.
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

#[test]
fn the_shipped_presets_permit_their_own_tools_identity_and_nothing_else() {
    // The globs are copied from the presets and the presets are read here, so
    // the two cannot drift apart without this failing.
    let claude = include_str!("../presets/attribution-claude.toml");
    let copilot = include_str!("../presets/attribution-copilot.toml");
    assert!(claude.contains(&format!("autonomous = \"{CLAUDE_GLOB}\"")));
    assert!(copilot.contains("autonomous = \"*copilot*[bot]*\""));

    let copilot_bot = "copilot-swe-agent[bot] <198982749+Copilot@users.noreply.github.com>";
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
    assert!(by(CLAUDE_GLOB, AGENT).is_empty());
    assert!(by("*copilot*[bot]*", copilot_bot).is_empty());
    // and each is refused under the other's preset
    assert_eq!(kinds(&by(CLAUDE_GLOB, copilot_bot)), vec![
        "byline", "identity"
    ]);
    assert_eq!(kinds(&by("*copilot*[bot]*", AGENT)), vec![
        "byline", "identity"
    ]);
}

// --- what counts as an agent: what only an agent carries -----------------------------------
//
// Recognised by the mailbox an agent commits from, a `[bot]` marker, or a name that
// is wholly a tool's own name (with the model and product words that ride along
// with one), and never by a word appearing inside somebody's name. A substring
// match refused Devin Smith, Hubbard Jones (bard), Haider Ali (aider), Cody Brown,
// Claude Monet, Anders Android (droid), Jules Verne, Mistral Winds and Ada Lombard
// in a probe of the review scanner's matcher, and at a commit gate that is a
// person blocked on every commit they make.

/// Real people, including every name the substring matcher refused in the probe,
/// the owner's own, and some ordinary ones. None of them is an agent.
const PEOPLE: &[&str] = &[
    "Devin Smith <devin.smith@example.com>",
    "Hubbard Jones <hubbard@example.com>",
    "Haider Ali <haider@example.com>",
    "Cody Brown <cody@example.com>",
    "Claude Monet <claude.monet@example.org>",
    "Anders Android <anders@example.com>",
    "Jules Verne <jules@example.com>",
    "Mistral Winds <mistral@example.com>",
    "Ada Lombard <ada@example.com>",
    "O. R. Toimela <ort@hiisi.digital>",
    "Jane Smith <jane@example.com>",
    "Matti Meikalainen <matti@example.fi>",
    "Maria Garcia Lopez <maria@example.es>",
    "Li Wei <li.wei@example.cn>",
    "Some One <12345+someone@users.noreply.github.com>",
    "GitHub <noreply@github.com>",
    // a person at a vendor, writing from the vendor's own domain: the domain is
    // not what only an agent carries
    "Jane Roe <jane.roe@anthropic.com>",
    "Bob Poe <bob@openai.com>",
    // a parenthesis that is not a tool's name
    "Jane Smith (she/her) <jane@example.com>",
];

#[test]
fn no_person_is_refused_as_an_author_or_a_committer_under_either_mode() {
    for mode in [AgentMode::Assistant, AgentMode::Autonomous] {
        for who in PEOPLE {
            let found = judged(&[], mode, CLEAN, Some(who), Some(who));
            assert!(
                found.is_empty(),
                "{who} under {mode:?} was refused: {found:?}"
            );
        }
    }
}

#[test]
fn no_person_is_refused_as_a_co_author_either() {
    // The trailer path asks the same question of the same list, so it had the
    // same false positives, and a co-author who is a real person is the case the
    // lint has always said it never touches.
    for who in PEOPLE {
        let message = format!("feat: a subject\n\nCo-Authored-By: {who}\n");
        let found = judged(&[], AgentMode::Assistant, &message, None, None);
        assert!(
            found.is_empty(),
            "{who} as a co-author was refused: {found:?}"
        );
    }
}

#[test]
fn an_agent_is_recognised_by_what_only_an_agent_carries() {
    let agents = [
        // the identity of the 237 commits, and the names Claude Code goes by
        "Claude <noreply@anthropic.com>",
        "Claude Code <noreply@anthropic.com>",
        "Claude Opus 4.1 <noreply@anthropic.com>",
        "Claude Sonnet 5.5 <claude@example.com>",
        "claude-code[bot] <claude-code@users.noreply.github.com>",
        // a tool's own name, wholly
        "Copilot <198982749+Copilot@users.noreply.github.com>",
        "GitHub Copilot <copilot@example.com>",
        "Codex <codex@example.com>",
        "ChatGPT <chatgpt@example.com>",
        "GPT-4o <gpt@example.com>",
        "Gemini CLI <gemini@example.com>",
        "Cursor Agent <cursoragent@cursor.com>",
        "Devin AI <devin@cognition-labs.com>",
        "aider <aider@aider.chat>",
        "SWE-agent <swe@example.com>",
        // a bot marker, on the name or on the mailbox
        "dependabot[bot] <49699333+dependabot[bot]@users.noreply.github.com>",
        "A Name <1234+thing[bot]@users.noreply.github.com>",
        "copilot-swe-agent[bot] <198982749+Copilot@users.noreply.github.com>",
        // an agent's own mailbox under a name that says nothing
        "Dev Container <noreply@anthropic.com>",
        "Dev Container <copilot@github.com>",
        // the tag aider appends to a person's name
        "Jane Smith (aider) <jane@example.com>",
    ];
    for who in agents {
        let found = judged(&[], AgentMode::Assistant, CLEAN, Some(who), Some(PERSON));
        assert_eq!(
            kinds(&found),
            vec!["identity"],
            "{who} was not refused: {found:?}"
        );
    }
}

#[test]
fn a_co_author_naming_an_agent_by_what_only_an_agent_carries_is_a_byline() {
    for who in ["Claude Opus 4.1 <noreply@anthropic.com>", "dependabot[bot] <a@b.test>"] {
        let message = format!("feat: a subject\n\nCo-Authored-By: {who}\n");
        let found = judged(&[], AgentMode::Assistant, &message, None, None);
        assert_eq!(kinds(&found), vec!["byline"], "{who}: {found:?}");
    }
}

#[test]
fn a_person_who_commits_under_a_tools_bare_name_reads_as_that_tool() {
    // The one edge that is left, written down so it is a known one. A name that
    // is wholly a tool's own name is the tool's, and somebody whose whole
    // `user.name` is `Devin` is refused until the project says what an agent is.
    let devin = "Devin <devin@patel.example>";
    let found = judged(&[], AgentMode::Assistant, CLEAN, Some(devin), Some(devin));
    assert_eq!(kinds(&found), vec!["identity"], "got: {found:?}");

    let found = judged(
        &[("agent_identities", "claude,copilot")],
        AgentMode::Assistant,
        CLEAN,
        Some(devin),
        Some(devin),
    );
    assert!(found.is_empty(), "got: {found:?}");
}

#[test]
fn a_project_that_redefines_what_an_agent_is_redefines_it_for_identities_too() {
    let found = judged(
        &[("agent_identities", "robotron")],
        AgentMode::Assistant,
        CLEAN,
        Some(AGENT),
        Some("Robotron <r@robotron.test>"),
    );
    // Claude reads as a person under this list, and Robotron as an agent.
    assert_eq!(kinds(&found), vec!["identity"], "got: {found:?}");
    assert!(
        found[0].1.contains("the committer is"),
        "got: {}",
        found[0].1
    );
}

#[test]
fn a_project_can_name_an_agents_mailbox_and_a_marker_as_well_as_its_name() {
    let params = [("agent_identities", "robotron,robot@robotron.test,[auto]")];
    for who in [
        "Robotron Pro <x@example.com>",
        "Build <robot@robotron.test>",
        "ci[auto] <x@example.com>",
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
        Some(PERSON),
    );
    assert!(
        found.is_empty(),
        "a name merely holding the word: {found:?}"
    );
}

#[test]
fn every_identity_the_review_scanner_names_is_refused_here_too() {
    // The scanner in `mockspace/lib/attribution.sh` is the reference for what
    // names an agent, and these are the vendors its own suite
    // (`it_names_the_vendors_across_families`) holds it to, written as the
    // identities a commit would carry. A vendor it knows and this lint does not
    // is a commit that clears the hook and fails the review.
    let named = [
        "Claude Opus 5 <noreply@anthropic.com>",
        "Copilot <copilot@github.com>",
        "ChatGPT <noreply@openai.com>",
        "Cursor Agent <agent@cursor.com>",
        "google-labs-jules[bot] <jules@google.com>",
        "Devin AI <devin@cognition-labs.com>",
        "aider <aider@aider.chat>",
        "Amp <amp@ampcode.com>",
        "Grok <grok@x.ai>",
        "dependabot[bot] <support@github.com>",
    ];
    for who in named {
        let found = judged(&[], AgentMode::Assistant, CLEAN, Some(who), Some(PERSON));
        assert_eq!(
            kinds(&found),
            vec!["identity"],
            "{who} was not refused: {found:?}"
        );
    }
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
