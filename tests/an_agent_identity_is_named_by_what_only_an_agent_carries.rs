//! What `message-attribution` takes for an agent identity, row by row.
//!
//! The rows are the conformance table that ships with `mockspace-lint-rules`, the
//! one every recogniser of an agent identity is held to. Each is run through the
//! pack as the engine drives it, as an author, as a committer and as the value of
//! a `Co-Authored-By`, so what the recogniser decides and what the lint does with
//! it are tested together and neither can regress behind the other. The four keys
//! a project sets to move the line are here too.
//!
//! What the lint does with an agent once it is found, the mode's policy and the
//! advice in the finding, is `a_commit_identity_is_judged_like_a_byline.rs`.

mod commit_identity;

use commit_identity::{AGENT, CLEAN, PERSON, configured, judged, kinds};
use mockspace_lint_rules::AgentMode;
use mockspace_lint_rules::agent_identity_conformance::table;

/// A message carrying `who` as a co-author and nothing else.
fn co_authored(who: &str) -> String {
    format!("feat: a subject\n\nCo-Authored-By: {who}\n")
}

// --- the rows -----------------------------------------------------------------------------

#[test]
fn the_shared_table_is_read_and_has_both_verdicts() {
    let t = table();
    assert!(!t.people.is_empty() && !t.agents.is_empty() && !t.keyed.is_empty());
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
        let found = judged(&[], AgentMode::Assistant, &co_authored(who), None, None);
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
        let found = judged(&[], AgentMode::Assistant, &co_authored(who), None, None);
        assert!(
            !found.is_empty() && found.iter().all(|(k, _)| k == "byline" || k == "advert"),
            "{who} as a co-author: {found:?}"
        );
    }
}

#[test]
fn a_given_name_tool_is_an_agent_with_a_second_signal_and_a_person_without() {
    // The edge the second signal exists for. `Claude` and `Devin` on their own are
    // somebody's name, and a companion after them, a mailbox the tool commits from
    // or the tool's own address is the tool. They are rows of the table as well;
    // this names the reason.
    for who in [
        "Claude <c.dupont@example.com>",
        "Claude <c.dupont@notanthropic.com>",
        "Devin <d.patel@patel.example>",
        "Claude Max <c@example.com>",
        "Claude Monet <claude@anthropic.com>",
    ] {
        let found = judged(&[], AgentMode::Assistant, CLEAN, Some(who), Some(who));
        assert!(found.is_empty(), "{who}: {found:?}");
    }
    for who in [
        "Claude Code <c@example.com>",
        "Claude Code Action <c@example.com>",
        "Claude <claude@localhost>",
        "Claude <12345+claude@users.noreply.github.com>",
        "Claude <claude@anthropic.com>",
        "Devin <devin@cognition.ai>",
        "Gemini <gemini@google.com>",
        "Amp <noreply@ampcode.com>",
    ] {
        let found = judged(&[], AgentMode::Assistant, CLEAN, Some(who), None);
        assert_eq!(kinds(&found), vec!["identity"], "{who}: {found:?}");
    }
}

#[test]
fn a_tool_that_is_no_given_name_is_an_agent_with_any_words_after_it() {
    for who in ["Copilot Chat", "Copilot Workspace", "Cursor Bugbot", "Cursor"] {
        let found = judged(&[], AgentMode::Assistant, CLEAN, Some(who), None);
        assert_eq!(kinds(&found), vec!["identity"], "{who}: {found:?}");
    }
    // and a tool's name inside somebody's is none
    let found = judged(
        &[],
        AgentMode::Assistant,
        CLEAN,
        Some("Smith Copilot <smith@example.com>"),
        None,
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_group_in_parentheses_is_read_for_the_tags_tools_write_and_nothing_else() {
    for who in ["Jane Doe (OpenAI)", "Jane Doe (Anthropic)", "Alex (Droid)", "Alex (Cline)"] {
        let found = judged(&[], AgentMode::Assistant, CLEAN, Some(who), Some(who));
        assert!(found.is_empty(), "{who}: {found:?}");
    }
    let found = judged(
        &[],
        AgentMode::Assistant,
        CLEAN,
        Some("Paul Gauthier (aider)"),
        None,
    );
    assert_eq!(kinds(&found), vec!["identity"], "{found:?}");
}

// --- the four keys ------------------------------------------------------------------------

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
fn agent_names_names_the_names_that_are_agents_whatever_the_mailbox() {
    // The default lets these through, and the key is how a project that knows its
    // own build hosts says they are an agent's.
    for (key, identity) in &table().keyed {
        let found = judged(
            &[],
            AgentMode::Assistant,
            CLEAN,
            Some(identity),
            Some(identity),
        );
        assert!(found.is_empty(), "{identity} by default: {found:?}");
        let found = judged(
            &[],
            AgentMode::Assistant,
            &co_authored(identity),
            None,
            None,
        );
        assert!(
            found.is_empty(),
            "{identity} as a co-author by default: {found:?}"
        );

        let params = [("agent_names", key.as_str())];
        let as_author = judged(&params, AgentMode::Assistant, CLEAN, Some(identity), None);
        assert_eq!(
            kinds(&as_author),
            vec!["identity"],
            "{identity}: {as_author:?}"
        );
        let as_committer = judged(&params, AgentMode::Assistant, CLEAN, None, Some(identity));
        assert_eq!(
            kinds(&as_committer),
            vec!["identity"],
            "{identity}: {as_committer:?}"
        );
        let as_co_author = judged(
            &params,
            AgentMode::Assistant,
            &co_authored(identity),
            None,
            None,
        );
        assert_eq!(
            kinds(&as_co_author),
            vec!["byline"],
            "{identity}: {as_co_author:?}"
        );
    }
    // several names, each the whole name and nothing longer
    let params = [("agent_names", "Claude, Devin")];
    for who in ["Claude <root@buildhost.local>", "devin <root@buildhost.local>"] {
        let found = judged(&params, AgentMode::Assistant, CLEAN, Some(who), None);
        assert_eq!(kinds(&found), vec!["identity"], "{who}: {found:?}");
    }
    for who in ["Claude Monet <root@buildhost.local>", "Gemini <root@buildhost.local>"] {
        let found = judged(&params, AgentMode::Assistant, CLEAN, Some(who), None);
        assert!(found.is_empty(), "{who}: {found:?}");
    }
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
    let found = judged(
        &[("not_agents", "Cody Agent")],
        AgentMode::Assistant,
        &co_authored(identity),
        None,
        None,
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn not_agents_excuses_an_agents_mailbox_only_by_the_whole_identity() {
    // `Claude` excusing `Claude <noreply@anthropic.com>` is the finding: a bare
    // name or a bare mailbox there would excuse every agent that commits as that
    // name or from that mailbox, so only `Name <mailbox>` does.
    for entry in ["Claude", "noreply@anthropic.com", "Claude Code"] {
        let found = judged(
            &[("not_agents", entry)],
            AgentMode::Assistant,
            CLEAN,
            Some(AGENT),
            Some(AGENT),
        );
        assert_eq!(kinds(&found), vec!["identity"], "`{entry}`: {found:?}");
        let found = judged(
            &[("not_agents", entry)],
            AgentMode::Assistant,
            &co_authored(AGENT),
            None,
            None,
        );
        assert_eq!(
            kinds(&found),
            vec!["byline"],
            "`{entry}` as a co-author: {found:?}"
        );
    }
    let params = [("not_agents", AGENT)];
    let found = judged(
        &params,
        AgentMode::Assistant,
        CLEAN,
        Some(AGENT),
        Some(AGENT),
    );
    assert!(found.is_empty(), "the whole identity: {found:?}");
    let found = judged(
        &params,
        AgentMode::Assistant,
        CLEAN,
        Some("Claude Code <noreply@anthropic.com>"),
        None,
    );
    assert_eq!(
        kinds(&found),
        vec!["identity"],
        "another name at the mailbox: {found:?}"
    );
}

#[test]
fn the_four_keys_are_declared_so_a_project_can_set_them() {
    let (p, _) = configured(&[]);
    let lint = p
        .message_lints
        .iter()
        .find(|l| l.name() == "message-attribution")
        .expect("the pack ships message-attribution");
    for key in ["agent_identities", "extra_agent_identities", "agent_names", "not_agents"] {
        assert!(
            lint.config_keys().contains(&key),
            "`{key}` is not declared: {:?}",
            lint.config_keys()
        );
    }
}
