//! What the mailbox and the project's names add to a name.
//!
//! A given-name tool behind the mailbox that is its own is an agent, and a person
//! called the same at an ordinary domain is not. A name the project lists is an
//! agent whatever the mailbox. Both are held to the conformance table, and each
//! arm goes red when its row is taken out of the table or its rule is broken.

use mockspace_lint_rules::agent_identity_conformance::table;

use super::AgentIdentities;

fn agent(who: &str) -> bool {
    AgentIdentities::default().names_an_agent(who)
}

/// Whether `who` is an agent when the project lists `names` in `agent_names`.
fn named(who: &str, names: &[&str]) -> bool {
    let mut ids = AgentIdentities::default();
    ids.name_agents(names.iter().map(|n| (*n).to_string()).collect());
    ids.names_an_agent(who)
}

#[test]
fn a_given_name_is_read_behind_a_mailbox_that_is_the_tools() {
    let t = table();
    for g in &t.given {
        let up = g.to_uppercase();
        // the local part is the tool's word, whole, at a mailbox that is a
        // machine's: localhost, a domain of one label, or a domain ending in one
        // of the machine endings in the table
        for at in ["localhost", "buildbox"] {
            assert!(agent(&format!("{g} <{g}@{at}>")), "{g} at {at}");
        }
        assert!(agent(&format!("{up} <{up}@BUILDBOX>")), "{g} in capitals");
        for ending in &t.machines {
            assert!(
                agent(&format!("{g} <{g}@ci.{ending}>")),
                "{g} at ci.{ending}"
            );
            assert!(
                agent(&format!("{up} <{up}@CI.{}>", ending.to_uppercase())),
                "{g} at CI.{ending} in capitals"
            );
            // the ending has to end the domain, and a dot has to come before it
            for at in [
                format!("ci.{ending}.example.com"),
                format!("{ending}.example.com"),
                format!("ci{ending}.example.com"),
            ] {
                assert!(!agent(&format!("{g} <{g}@{at}>")), "{g} at {at}");
            }
        }
        // the same local part at an ordinary domain is a person called that
        for at in ["example.com", "acme.com", "gmail.com"] {
            assert!(!agent(&format!("{g} <{g}@{at}>")), "{g} at {at}");
        }
        assert!(
            !agent(&format!("{up} <{up}@EXAMPLE.COM>")),
            "{g} at EXAMPLE.COM"
        );
        // and so is the private address GitHub gives every account, with the
        // number or without it, the number being no part of the tool's word
        for who in [
            format!("{g} <12345+{g}@users.noreply.github.com>"),
            format!("{up} <12345+{up}@USERS.NOREPLY.GITHUB.COM>"),
            format!("{g} <{g}@users.noreply.github.com>"),
            format!("{g} <12345+{g}@example.com>"),
            format!("{g} <12345+{g}@ci.local>"),
        ] {
            assert!(!agent(&who), "{who}");
        }
        // the local part is the tool's word whole
        for who in [
            format!("{g} <x{g}@localhost>"),
            format!("{g} <{g}.x@localhost>"),
            format!("{g} <x{g}@ci.local>"),
        ] {
            assert!(!agent(&who), "{who}");
        }
        // the mailbox is the whole of the signal, so it needs the whole of the name
        assert!(
            !agent(&format!("{g} Monet <{g}@localhost>")),
            "{g} Monet at localhost"
        );
        assert!(
            !agent(&format!("{g} Monet <{g}@ci.local>")),
            "{g} Monet at ci.local"
        );
        // a bare mailbox has no name, a name with no mailbox has no domain, and a
        // mailbox with no domain is nobody's
        assert!(!agent(&format!("{g}@localhost")), "{g}@localhost bare");
        assert!(!agent(g), "{g} bare");
        assert!(!agent(&format!("{g} <{g}@>")), "{g} at nothing");
    }
    // a vendor's domain, matched whole, and only the vendor of that tool
    for (tool, domain) in &t.vendors {
        assert!(agent(&format!("{tool} <x@{domain}>")), "{tool} at {domain}");
        assert!(
            agent(&format!(
                "{} <x@{}>",
                tool.to_uppercase(),
                domain.to_uppercase()
            )),
            "{tool} at {domain} in capitals"
        );
        assert!(!agent(&format!("{tool} <x@not{domain}>")), "not{domain}");
        assert!(
            !agent(&format!("{tool} <x@mail.{domain}>")),
            "mail.{domain}"
        );
        assert!(
            !agent(&format!("{tool} <x@{domain}.example>")),
            "{domain}.example"
        );
        assert!(
            !agent(&format!("{tool} Monet <x@{domain}>")),
            "{tool} Monet at {domain}"
        );
    }
    for g in &t.given {
        for (tool, domain) in &t.vendors {
            // a domain that is another tool's vendor, and not this one's
            if tool != g && !t.vendors.contains(&(g.clone(), domain.clone())) {
                assert!(!agent(&format!("{g} <x@{domain}>")), "{g} at {domain}");
            }
        }
    }
}

#[test]
fn a_name_the_project_lists_is_read_whatever_the_mailbox() {
    let t = table();
    for (key, identity) in &t.keyed {
        // the default lets the row through, and the name is what stops it
        assert!(!agent(identity), "{identity} by default");
        assert!(!named(identity, &[]), "{identity} with no names");
        assert!(named(identity, &[key]), "{identity} named {key}");
    }
    for g in &t.given {
        let who = format!("{g} <root@buildhost.local>");
        assert!(!agent(&who), "{who} by default");
        assert!(named(&who, &[g]), "{who} named {g}");
        assert!(
            named(&who, &[&g.to_uppercase()]),
            "{who} named {g} in capitals"
        );
        assert!(named(&who, &["Zed", g]), "{who} among names");
        // another name names another identity
        assert!(!named(&who, &["Zed"]), "{who} named Zed");
        assert!(!named(&who, &[""]), "{who} named nothing");
        // the whole name, not its start, and a group in parentheses is no part of it
        assert!(
            !named(&format!("{g} Monet <root@buildhost.local>"), &[g]),
            "{g} Monet named {g}"
        );
        assert!(
            named(&format!("{g} (she/her) <root@buildhost.local>"), &[g]),
            "{g} (she/her) named {g}"
        );
    }
    // a name the project lists is any name, and the other nets still hold
    assert!(named("Build Host <root@buildhost.local>", &["Build Host"]));
    assert!(!named("Build Host <root@buildhost.local>", &["Build"]));
    assert!(named("Copilot <x@example.com>", &["Zed"]));
    assert!(!named("Jane Smith <jane@example.com>", &["Zed"]));
}
