//! What the four keys do to the recogniser: replace the list, add to it, name whole
//! names that are agents, and excuse people.
//!
//! The lists and the rows they read are `agent_identity_tests.rs`, held to the
//! conformance table. These are the configured behaviours on top, and the line
//! `not_agents` may not cross: an identity at a mailbox an agent commits from is
//! excused by the whole identity and by nothing shorter.

use mockspace_lint_rules::agent_identity_conformance::table;

use super::AgentIdentities;

/// What a mailbox glob stands for once its stars are filled in.
fn instance(glob: &str) -> String {
    glob.replace('*', "x")
}

#[test]
fn replacing_the_list_drops_the_defaults_and_keeps_the_rules_for_what_is_left() {
    let mut ids = AgentIdentities::default();
    ids.replace(vec![
        "robotron".into(),
        "robot@robotron.test".into(),
        "[auto]".into(),
    ]);
    assert!(ids.names_an_agent("Robotron CLI <x@example.com>"));
    assert!(ids.names_an_agent("Build <robot@robotron.test>"));
    assert!(ids.names_an_agent("ci[auto] <x@example.com>"));
    assert!(!ids.names_an_agent("Robert Robotron-Smith <rob@example.com>"));
    // and the defaults are gone, which is what replacing means
    assert!(!ids.names_an_agent("Copilot <x@example.com>"));
    assert!(!ids.names_an_agent("dependabot[bot] <x@example.com>"));
    assert!(!ids.names_an_agent("Dev Container <noreply@anthropic.com>"));
    // the tags are how a name is read and not entries in the list, so they stay
    assert!(ids.names_an_agent("Paul Gauthier (aider)"));
}

#[test]
fn extending_the_list_adds_to_the_defaults_and_disarms_none_of_them() {
    let mut ids = AgentIdentities::default();
    ids.extend(vec![
        "robotron".into(),
        "robot@robotron.test".into(),
        "[auto]".into(),
    ]);
    assert!(ids.names_an_agent("Robotron CLI <x@example.com>"));
    assert!(ids.names_an_agent("Build <robot@robotron.test>"));
    assert!(ids.names_an_agent("ci[auto] <x@example.com>"));
    // the defaults, every arm of them
    assert!(ids.names_an_agent("Copilot <x@example.com>"));
    assert!(ids.names_an_agent("dependabot[bot] <x@example.com>"));
    assert!(ids.names_an_agent("Dev Container <noreply@anthropic.com>"));
    assert!(!ids.names_an_agent("Claude Monet <monet@example.org>"));
}

#[test]
fn names_the_project_lists_are_independent_of_the_list() {
    let mut ids = AgentIdentities::default();
    ids.replace(vec!["robotron".into()]);
    ids.name_agents(vec!["Claude".into()]);
    assert!(ids.names_an_agent("claude <root@buildhost.local>"));
    assert!(!ids.names_an_agent("Copilot <x@example.com>"));
}

#[test]
fn a_person_the_project_names_is_never_read_as_an_agent() {
    let mut ids = AgentIdentities::default();
    assert!(ids.names_an_agent("Cody Agent <cody@example.com>"));
    ids.exclude(vec!["Cody Agent".into()]);
    assert!(
        !ids.names_an_agent("Cody Agent <cody@example.com>"),
        "by name"
    );
    assert!(
        !ids.names_an_agent("cody agent <someone@else.example>"),
        "in any case"
    );
    // a name is matched whole, so it excuses nobody else
    assert!(ids.names_an_agent("Cody Code <cody@example.com>"));
}

#[test]
fn a_person_can_be_named_by_mailbox_or_by_the_whole_identity() {
    let mut ids = AgentIdentities::default();
    ids.exclude(vec![
        "devin@patel.example".into(),
        "Copilot <me@home.example>".into(),
    ]);
    assert!(
        !ids.names_an_agent("Devin AI <devin@patel.example>"),
        "by mailbox"
    );
    assert!(
        !ids.names_an_agent("Copilot <me@home.example>"),
        "by name and mailbox"
    );
    assert!(
        ids.names_an_agent("Copilot <other@home.example>"),
        "the same name, another mailbox"
    );
    assert!(ids.names_an_agent("Devin AI <devin@cognition-labs.com>"));
}

#[test]
fn a_name_the_project_excuses_is_excused_where_only_the_name_is_an_agents() {
    // A marker in the name is the one signal a name can excuse, since the project
    // named exactly that name. Nothing else about the identity is an agent's.
    let mut ids = AgentIdentities::default();
    ids.exclude(vec!["Jane [bot] Smith".into(), "Cursor".into()]);
    assert!(!ids.names_an_agent("Jane [bot] Smith <j@example.com>"));
    assert!(!ids.names_an_agent("Cursor <cursor@example.com>"));
}

#[test]
fn a_bare_name_or_mailbox_never_excuses_an_identity_at_an_agents_mailbox() {
    let t = table();
    // the finding as it was found: `Claude` excused `Claude <noreply@anthropic.com>`
    let mut ids = AgentIdentities::default();
    ids.exclude(vec!["Claude".into()]);
    assert!(ids.names_an_agent("Claude <noreply@anthropic.com>"));
    assert!(ids.names_an_agent("Claude Code <noreply@anthropic.com>"));
    assert!(
        !ids.names_an_agent("Claude <c.dupont@example.com>"),
        "nothing else is an agent's"
    );

    for m in &t.mailboxes {
        let at = instance(m);
        let who = format!("Dev Container <{at}>");
        for entry in ["Dev Container", at.as_str()] {
            let mut ids = AgentIdentities::default();
            ids.exclude(vec![entry.into()]);
            assert!(ids.names_an_agent(&who), "{who} excused by `{entry}`");
        }
    }
    for (tool, domain) in &t.vendors {
        let who = format!("{tool} <x@{domain}>");
        for entry in [tool.clone(), format!("x@{domain}")] {
            let mut ids = AgentIdentities::default();
            ids.exclude(vec![entry.clone()]);
            assert!(ids.names_an_agent(&who), "{who} excused by `{entry}`");
        }
    }
    // a marker in the mailbox is the same signal as a mailbox on the list
    let mut ids = AgentIdentities::default();
    ids.exclude(vec!["Jane Smith".into()]);
    assert!(ids.names_an_agent("Jane Smith <1234+thing[bot]@users.noreply.github.com>"));
}

#[test]
fn the_whole_identity_excuses_an_identity_at_an_agents_mailbox() {
    let mut ids = AgentIdentities::default();
    ids.exclude(vec![
        "Claude <noreply@anthropic.com>".into(),
        "Gemini <gemini@google.com>".into(),
        "Devin <*@cognition.ai>".into(),
    ]);
    assert!(!ids.names_an_agent("Claude <noreply@anthropic.com>"));
    assert!(!ids.names_an_agent("Gemini <gemini@google.com>"));
    assert!(
        !ids.names_an_agent("Devin <anyone@cognition.ai>"),
        "by a glob"
    );
    // and only that identity: another name at the mailbox, or the name elsewhere
    assert!(ids.names_an_agent("Claude Code <noreply@anthropic.com>"));
    assert!(ids.names_an_agent("Claude <claude@anthropic.com>"));
    assert!(ids.names_an_agent("Gemini CLI <gemini@google.com>"));
}

#[test]
fn a_name_that_is_listed_and_excused_is_excused() {
    // The project said both, and the person is the one it names last in intent: the
    // exclusion is read first.
    let mut ids = AgentIdentities::default();
    ids.name_agents(vec!["Claude".into()]);
    ids.exclude(vec!["Claude".into()]);
    assert!(!ids.names_an_agent("Claude <root@buildhost.local>"));
}
