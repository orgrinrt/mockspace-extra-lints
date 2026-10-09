//! The recogniser is held to the conformance table, entry by entry.
//!
//! The table ships with `mockspace-lint-rules`, at the revision this pack is built
//! against, and a second implementation of the same recogniser, in shell, is held to
//! the same file. A test that walked this module's own lists would pass with an entry
//! deleted from them, so every list is compared with the table's, and then every
//! entry is exercised on its own: removing one from the lists turns a test red in
//! two places.

use std::collections::BTreeSet;

use mockspace_lint_rules::agent_identity_conformance::{AgentIdentityConformance, table};

use super::{
    AgentIdentities,
    COMPANION_WORDS,
    DEFAULT_GIVEN_NAMES,
    DEFAULT_MAILBOXES,
    DEFAULT_MARKERS,
    DEFAULT_TOOLS,
    HEAD_WORDS,
};

fn agent(who: &str) -> bool {
    AgentIdentities::default().names_an_agent(who)
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

fn rows(items: &[String]) -> BTreeSet<String> {
    items.iter().cloned().collect()
}

/// What a mailbox glob stands for once its stars are filled in.
fn instance(glob: &str) -> String {
    glob.replace('*', "x")
}

fn t() -> AgentIdentityConformance {
    table()
}

#[test]
fn the_table_is_read_and_has_rows_in_every_list() {
    // The guard under all of it: a table that parsed to nothing would make every
    // loop below pass having asserted nothing.
    let t = t();
    for (name, n) in [
        ("markers", t.markers.len()),
        ("mailboxes", t.mailboxes.len()),
        ("tools", t.tools.len()),
        ("given", t.given.len()),
        ("heads", t.heads.len()),
        ("companions", t.companions.len()),
        ("people", t.people.len()),
        ("agents", t.agents.len()),
    ] {
        assert!(n > 0, "the table has no {name}");
    }
}

#[test]
fn the_lists_are_exactly_the_tables_lists() {
    let t = t();
    assert_eq!(set(DEFAULT_MARKERS), rows(&t.markers), "markers");
    assert_eq!(set(DEFAULT_MAILBOXES), rows(&t.mailboxes), "mailboxes");
    assert_eq!(set(DEFAULT_TOOLS), rows(&t.tools), "tools");
    assert_eq!(set(DEFAULT_GIVEN_NAMES), rows(&t.given), "given names");
    assert_eq!(set(HEAD_WORDS), rows(&t.heads), "head words");
    assert_eq!(set(COMPANION_WORDS), rows(&t.companions), "companion words");
}

#[test]
fn each_tool_is_the_whole_of_a_name_and_nothing_longer() {
    for tool in &t().tools {
        assert!(agent(&format!("{tool} <x@example.com>")), "{tool}");
        assert!(agent(&tool.to_uppercase()), "{tool} in capitals");
        // a word of somebody's name on either side is a person
        assert!(
            !agent(&format!("{tool} Smith <x@example.com>")),
            "{tool} Smith"
        );
        assert!(
            !agent(&format!("Smith {tool} <x@example.com>")),
            "Smith {tool}"
        );
    }
}

#[test]
fn each_given_name_tool_wants_a_second_signal() {
    let t = t();
    for g in &t.given {
        assert!(!agent(&format!("{g} <x@example.com>")), "{g} alone");
        assert!(!agent(g), "{g} bare");
        // a version, a companion after it, a vendor word before it, a mailbox
        assert!(agent(&format!("{g} 4.1 <x@example.com>")), "{g} 4.1");
        assert!(agent(&format!("{g} v2")), "{g} v2");
        for c in &t.companions {
            assert!(agent(&format!("{g} {c} <x@example.com>")), "{g} {c}");
        }
        for h in &t.heads {
            assert!(agent(&format!("{h} {g} <x@example.com>")), "{h} {g}");
        }
        for m in &t.mailboxes {
            assert!(agent(&format!("{g} <{}>", instance(m))), "{g} at {m}");
        }
        // and a word that is none of those makes it a person
        assert!(!agent(&format!("{g} Monet <x@example.com>")), "{g} Monet");
        assert!(!agent(&format!("Max {g} <x@example.com>")), "Max {g}");
    }
}

#[test]
fn each_companion_counts_after_a_tool_and_never_before_one() {
    let t = t();
    for tool in t.tools.iter().chain(&t.given) {
        for c in &t.companions {
            assert!(agent(&format!("{tool} {c} <x@example.com>")), "{tool} {c}");
            // in front of the tool it is a word of somebody's name, unless it is a
            // vendor word, or is itself a tool and so a name of its own
            if t.heads.contains(c) || t.tools.contains(c) || t.given.contains(c) {
                continue;
            }
            assert!(!agent(&format!("{c} {tool} <x@example.com>")), "{c} {tool}");
        }
    }
}

#[test]
fn each_head_word_stands_before_a_tool_and_names_nobody_alone() {
    let t = t();
    for h in &t.heads {
        for tool in t.tools.iter().chain(&t.given) {
            assert!(agent(&format!("{h} {tool} <x@example.com>")), "{h} {tool}");
        }
        if !t.tools.contains(h) {
            assert!(!agent(&format!("{h} <x@example.com>")), "{h} alone");
        }
    }
}

#[test]
fn each_mailbox_is_matched_whole_whatever_the_name() {
    for m in &t().mailboxes {
        let at = instance(m);
        assert!(agent(&format!("Dev Container <{at}>")), "{m}");
        assert!(agent(&at), "{m} bare");
        // an exact address is not matched by a longer one that ends with it
        if !m.contains('*') {
            assert!(!agent(&format!("Dev Container <x{m}>")), "x{m}");
            assert!(
                !agent(&format!("Dev Container <{m}.example>")),
                "{m}.example"
            );
        }
    }
}

#[test]
fn each_marker_is_matched_in_the_name_and_in_the_mailbox() {
    for m in &t().markers {
        assert!(
            agent(&format!("thing{m} <x@example.com>")),
            "{m} in the name"
        );
        assert!(
            agent(&format!("A Name <1234+thing{m}@users.noreply.github.com>")),
            "{m} in the mailbox"
        );
    }
}

#[test]
fn each_person_row_is_a_person_in_every_form_it_arrives_in() {
    for p in &t().people {
        assert!(!agent(p), "{p}");
        assert!(!agent(&format!("{p} 1791567502 +0000")), "{p} with a date");
    }
}

#[test]
fn each_agent_row_is_an_agent_in_every_form_it_arrives_in() {
    for a in &t().agents {
        assert!(agent(a), "{a}");
        assert!(agent(&format!("{a} 1791567502 +0000")), "{a} with a date");
    }
}

#[test]
fn a_group_in_parentheses_is_read_on_its_own() {
    // a tool's name inside it is the tool's, and anything else is left out
    assert!(agent("Jane Smith (aider) <jane@example.com>"));
    assert!(agent("Jane Smith (Claude Code) <jane@example.com>"));
    assert!(!agent("Jane Smith (she/her) <jane@example.com>"));
    assert!(
        !agent("Jane Smith (devin) <jane@example.com>"),
        "a given name with no second signal"
    );
    assert!(agent("Claude 3.5 Sonnet (new)"));
}

// --- configuration ------------------------------------------------------------------

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
fn the_exclusion_wins_over_a_marker_and_a_mailbox() {
    // The project said this is a person, and it is the project that knows.
    let mut ids = AgentIdentities::default();
    ids.exclude(vec![
        "noreply@anthropic.com".into(),
        "Jane [bot] Smith".into(),
    ]);
    assert!(!ids.names_an_agent("Claude Code <noreply@anthropic.com>"));
    assert!(!ids.names_an_agent("Jane [bot] Smith <j@example.com>"));
}
