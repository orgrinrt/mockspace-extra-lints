//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `mock ticket-add <title>`: a ticket and its first move, written into the registry.
//!
//! Silent on success. What it refuses, and why, is the `tickets` crate's rules; this only reads the
//! command line and hands the request over.

use mockspace::tool::{ArgSpec, Purpose, Tool, ToolContext, ToolReport};

/// The maker.
pub struct TicketAdd;

impl Tool for TicketAdd {
    fn name(&self) -> &'static str {
        "ticket-add"
    }

    fn description(&self) -> &'static str {
        "writes a ticket and its first move into the registry"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Make {
            writes: &["mock/registry/ticket/items/*.toml"],
            roots:  &[],
        }
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[ArgSpec {
            name:        "title",
            required:    true,
            description: "what the ticket is, in a line",
        }]
    }

    fn value_flags(&self) -> &[&'static str] {
        tickets::args::ADD_FLAGS
    }

    fn help(&self) -> &'static str {
        "Flags, each taking a value: --kind (task by default), --state (the kind's initial by default), --holder, --id (a slug of the title by default), --what, --options (the options of a question, separated by a vertical bar, since an option's words hold commas, with no cap on how many), --choice (one or many: whether the question takes one pick or several, one when absent), --needs and --related (ns::slug, as tenet::id and subject::slug), --repos, --tags (comma separated, a tag being a free string that marks the ticket), --part-of, --span (a span of the planning layer, written as a member row) and --core, --set, --priority, --branch, --brief, --session, --rank, --why, and --by and --at (the author and the time of the first move: the environment's channel name and now by default). A move into a state that needs a holder wants --holder and the kind's required fields."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let request = match tickets::args::add(ctx.args) {
            Ok(r) => r,
            Err(e) => return ToolReport::inconclusive(e),
        };
        if let Err(e) = tickets::refs::Known::of(ctx.registry).refuse(&request.fields) {
            return ToolReport::inconclusive(e);
        }
        match tickets::write::add(&tickets::registry_dir(ctx.mock_dir), &request) {
            Ok(_) => ToolReport::reported("", 1),
            Err(e) => ToolReport::inconclusive(e),
        }
    }
}

mockspace::lint_pack! {
    tools: [TicketAdd],
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use mockspace::RegistryView;
    use mockspace::tool::{Outcome, Tool, ToolContext, ToolReport};
    use tickets::testkit::Scratch;
    use tickets::{load, registry_dir};

    use super::TicketAdd;

    fn run(s: &Scratch, args: &[&str]) -> ToolReport {
        let crates = BTreeSet::new();
        let registry = RegistryView::default();
        TicketAdd.run(&ToolContext {
            mock_dir: &s.mock,
            repo_root: &s.mock,
            all_crates: &crates,
            src_dirs: &[],
            args,
            stdin: None,
            registry: &registry,
        })
    }

    const WHO: &[&str] = &["--by", "tester", "--at", "2026-10-23T10:00:00Z"];

    fn with(first: &[&'static str]) -> Vec<&'static str> {
        first.iter().copied().chain(WHO.iter().copied()).collect()
    }

    #[test]
    fn a_row_that_validates_is_added_and_nothing_is_printed() {
        let s = Scratch::new();
        let r = run(&s, &with(&["Plant the beans"]));
        assert!(matches!(r.outcome, Outcome::Clean {
            examined: 1,
        }));
        assert!(r.output.is_empty());
        let store = load(&registry_dir(&s.mock)).unwrap();
        assert_eq!(
            store.tickets["plant_the_beans"].one("title"),
            "Plant the beans"
        );
        assert_eq!(store.state_of("plant_the_beans").unwrap().id, "backlog");
    }

    #[test]
    fn a_state_the_kind_does_not_declare_is_refused_and_writes_nothing() {
        let s = Scratch::new();
        let r = run(&s, &with(&["Plant", "--state", "limbo"]));
        assert!(matches!(r.outcome, Outcome::Inconclusive { .. }));
        assert!(load(&registry_dir(&s.mock)).unwrap().tickets.is_empty());
    }

    #[test]
    fn a_link_to_a_subject_or_tenet_the_canon_lacks_is_refused_and_writes_nothing() {
        let s = Scratch::new();
        let canon = RegistryView::new(
            std::collections::BTreeMap::from([(
                "subject::engine".to_string(),
                std::collections::BTreeMap::new(),
            )]),
            std::collections::BTreeMap::new(),
        );
        let crates = BTreeSet::new();
        let go = |related: &'static str| {
            let args = with(&["Plant", "--related", related]);
            TicketAdd.run(&ToolContext {
                mock_dir:   &s.mock,
                repo_root:  &s.mock,
                all_crates: &crates,
                src_dirs:   &[],
                args:       &args,
                stdin:      None,
                registry:   &canon,
            })
        };
        let bad = go("subject::enigne");
        match bad.outcome {
            Outcome::Inconclusive {
                ref reason,
            } => assert!(reason.contains("subject::enigne"), "{reason}"),
            ref o => panic!("{o:?}"),
        }
        assert!(load(&registry_dir(&s.mock)).unwrap().tickets.is_empty());
        assert!(matches!(
            go("subject::engine").outcome,
            Outcome::Clean { .. }
        ));
    }

    #[test]
    fn a_need_that_is_no_ticket_is_refused() {
        let s = Scratch::new();
        let r = run(&s, &with(&["Plant", "--needs", "nothing_here"]));
        assert!(matches!(r.outcome, Outcome::Inconclusive { .. }));
        assert!(load(&registry_dir(&s.mock)).unwrap().tickets.is_empty());
    }

    #[test]
    fn a_flag_the_command_does_not_take_is_refused() {
        let s = Scratch::new();
        let r = run(&s, &with(&["Plant", "--nope"]));
        assert!(matches!(r.outcome, Outcome::Inconclusive { .. }));
    }
}
