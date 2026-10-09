//! The recogniser is held to the conformance table, entry by entry.
//!
//! The table ships with `mockspace-lint-rules`, at the revision this pack is built
//! against, and every other recogniser of an agent identity is held to the same
//! file. A test that walked this module's own lists would pass with an entry
//! deleted from them, so every list is compared with the table's, and then every
//! entry is exercised on its own: removing one from the lists turns a test red in
//! two places, and removing a row from the table turns the comparison red.

use std::collections::BTreeSet;

use mockspace_lint_rules::agent_identity_conformance::{AgentIdentityConformance, table};

use super::AgentIdentities;
use super::lists::{
    COMPANION_WORDS,
    DEFAULT_GIVEN_NAMES,
    DEFAULT_MAILBOXES,
    DEFAULT_MARKERS,
    DEFAULT_TOOLS,
    GROUP_TAGS,
    HEAD_WORDS,
    MACHINE_SUFFIXES,
    VENDOR_DOMAINS,
};

fn agent(who: &str) -> bool {
    AgentIdentities::default().names_an_agent(who)
}

/// Whether `who` is an agent when the project lists `names` in `agent_names`.
fn named(who: &str, names: &[&str]) -> bool {
    let mut ids = AgentIdentities::default();
    ids.name_agents(names.iter().map(|n| (*n).to_string()).collect());
    ids.names_an_agent(who)
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
        ("tags", t.tags.len()),
        ("tools", t.tools.len()),
        ("given", t.given.len()),
        ("vendors", t.vendors.len()),
        ("machines", t.machines.len()),
        ("heads", t.heads.len()),
        ("companions", t.companions.len()),
        ("keyed", t.keyed.len()),
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
    assert_eq!(set(GROUP_TAGS), rows(&t.tags), "tags");
    assert_eq!(set(DEFAULT_TOOLS), rows(&t.tools), "tools");
    assert_eq!(set(DEFAULT_GIVEN_NAMES), rows(&t.given), "given names");
    assert_eq!(
        VENDOR_DOMAINS
            .iter()
            .map(|(tool, domain)| ((*tool).to_string(), (*domain).to_string()))
            .collect::<BTreeSet<_>>(),
        t.vendors.iter().cloned().collect::<BTreeSet<_>>(),
        "vendor domains"
    );
    assert_eq!(set(MACHINE_SUFFIXES), rows(&t.machines), "machine endings");
    assert_eq!(set(HEAD_WORDS), rows(&t.heads), "head words");
    assert_eq!(set(COMPANION_WORDS), rows(&t.companions), "companion words");
}

#[test]
fn each_tool_is_the_start_of_a_name_and_nothing_inside_one() {
    let t = t();
    for tool in &t.tools {
        // alone, and in capitals, and behind a mailbox of nobody's
        assert!(agent(&format!("{tool} <x@example.com>")), "{tool}");
        assert!(agent(&tool.to_uppercase()), "{tool} in capitals");
        // followed by nothing but companions and versions, a surface or a model
        assert!(
            agent(&format!("{tool} Chat <x@example.com>")),
            "{tool} Chat"
        );
        assert!(agent(&format!("{tool} 4.1")), "{tool} 4.1");
        assert!(agent(&format!("{tool} v2 Chat")), "{tool} v2 Chat");
        // followed by a word of somebody's name, with or without a comma, it is theirs
        assert!(
            !agent(&format!("{tool} Smith <x@example.com>")),
            "{tool} Smith"
        );
        assert!(!agent(&format!("{tool}, Smith")), "{tool}, Smith");
        // and one companion among such words does not turn them into the tool's
        assert!(!agent(&format!("{tool} Chat Smith")), "{tool} Chat Smith");
        assert!(!agent(&format!("{tool} Smith Chat")), "{tool} Smith Chat");
        assert!(
            !agent(&format!("{tool} Zed Chat Smith")),
            "{tool} Zed Chat Smith"
        );
        // a word of somebody's name in front of it makes it a name inside theirs
        assert!(
            !agent(&format!("Smith {tool} <x@example.com>")),
            "Smith {tool}"
        );
    }
    for c in &t.companions {
        assert!(agent(&format!("Copilot {c}")), "Copilot {c}");
        assert!(!agent(&format!("Copilot Smith {c}")), "Copilot Smith {c}");
    }
}

#[test]
fn each_given_name_tool_wants_a_second_signal() {
    let t = t();
    for g in &t.given {
        // alone it is a person's name
        assert!(!agent(&format!("{g} <x@example.com>")), "{g} alone");
        assert!(!agent(g), "{g} bare");
        // with a vendor word before it, or a mailbox on the list, it is the tool
        for h in &t.heads {
            assert!(agent(&format!("{h} {g} <x@example.com>")), "{h} {g}");
        }
        for m in &t.mailboxes {
            assert!(agent(&format!("{g} <{}>", instance(m))), "{g} at {m}");
        }
        // followed by nothing but companions and versions it is the tool, as any
        // tool is
        assert!(agent(&format!("{g} 4.1 <x@example.com>")), "{g} 4.1");
        assert!(agent(&format!("{g} v2")), "{g} v2");
        assert!(
            agent(&format!("{g} 4.1 {}", t.companions[0])),
            "{g} 4.1 {}",
            t.companions[0]
        );
        for c in &t.companions {
            assert!(agent(&format!("{g} {c} <x@example.com>")), "{g} {c}");
            assert!(agent(&format!("{g} {c} 4.1")), "{g} {c} 4.1");
            // a word of somebody's name anywhere among them makes it a person's
            assert!(
                !agent(&format!("{g} Zed {c} <x@example.com>")),
                "{g} Zed {c}"
            );
            assert!(
                !agent(&format!("{g} {c} Zed <x@example.com>")),
                "{g} {c} Zed"
            );
            assert!(!agent(&format!("{g} {c} Zed {c}")), "{g} {c} Zed {c}");
        }
        // and a word that is none of those is a person's, a version included
        assert!(!agent(&format!("{g} Monet <x@example.com>")), "{g} Monet");
        assert!(
            !agent(&format!("{g} 4.1 Monet <x@example.com>")),
            "{g} 4.1 Monet"
        );
        assert!(!agent(&format!("{g} Monet 4.1")), "{g} Monet 4.1");
        assert!(!agent(&format!("Max {g} <x@example.com>")), "Max {g}");
    }
}

#[test]
fn a_group_in_parentheses_is_read_for_a_tag_and_for_nothing_else() {
    let t = t();
    for tag in &t.tags {
        assert!(agent(&format!("Jane Doe ({tag})")), "({tag})");
        assert!(
            agent(&format!(
                "Jane Doe ({}) <jane@example.com>",
                tag.to_uppercase()
            )),
            "({tag}) in capitals"
        );
        assert!(
            agent(&format!("Jane (she/her) ({tag}) Doe")),
            "({tag}) after another"
        );
        // exactly the tag, so a group holding more is left out of the name
        assert!(!agent(&format!("Jane Doe ({tag} extra)")), "({tag} extra)");
        assert!(!agent(&format!("Jane Doe ({tag}-ish)")), "({tag}-ish)");
    }
    // a tool's name, a given name or a vendor in a group is none of the tags
    for word in t.tools.iter().chain(&t.given).chain(&t.heads) {
        if t.tags.contains(word) {
            continue;
        }
        assert!(!agent(&format!("Jane Doe ({word})")), "({word})");
        assert!(
            !agent(&format!(
                "Jane Doe ({}) <jane@example.com>",
                word.to_uppercase()
            )),
            "({word}) in capitals"
        );
    }
    // and a group that is left out of a name leaves the rest of it to be read
    assert!(agent("Copilot (she/her)"));
    assert!(agent("Claude 3.5 Sonnet (new)"));
    assert!(!agent("Claude Monet (aider-ish)"));
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
        // a name the project lists that is not the person's changes nothing
        assert!(!named(p, &["Zed"]), "{p} with another name listed");
    }
}

#[test]
fn each_agent_row_is_an_agent_in_every_form_it_arrives_in() {
    for a in &t().agents {
        assert!(agent(a), "{a}");
        assert!(agent(&format!("{a} 1791567502 +0000")), "{a} with a date");
    }
}
