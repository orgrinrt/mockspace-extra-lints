//! What names an agent, from a commit's author, committer or a `Co-Authored-By`.
//!
//! The recogniser reads what only an agent carries, and never a word that happens
//! to sit inside somebody's name. A substring match refuses `Devin Smith`,
//! `Claude Monet` and `Haider Ali` on every commit they make, and at a commit gate
//! that is a person blocked from their own work. What an agent carries is one of
//! these, tried in this order:
//!
//! - a marker, matched anywhere in the name or the mailbox: `[bot]` is not
//!   something a person writes into either by accident;
//! - a mailbox an agent commits from, matched whole as a glob. A vendor's domain
//!   is not one, so a person writing from it is a person;
//! - a tag a tool writes into a name, which is all a group in parentheses is read
//!   for. `Paul Gauthier (aider)` is the tool, `Jane Doe (OpenAI)` is Jane Doe, and
//!   a group holding anything else is left out of the name, so `Claude 3.5 Sonnet
//!   (new)` is read as `Claude 3.5 Sonnet`;
//! - a name the project lists in `agent_names`, matched against the whole name
//!   whatever the mailbox;
//! - a name that starts with a tool's own, behind vendor words if any, and is
//!   followed by nothing but the words that ride along with a tool and versions:
//!   `Copilot Chat`, `Cursor Bugbot`, `Claude Code on the web` and `GPT 4o` are
//!   the tools. Any other word after it is a word of somebody's name, so
//!   `Copilot Smith`, `Cline, John` and `Claude Monet` are people, and a tool
//!   inside somebody's name is none (`Smith Copilot`).
//!
//! A tool alone is an agent, except where its name is also somebody's given name
//! (`Claude`, `Devin`, `Gemini` and the rest of `DEFAULT_GIVEN_NAMES`). Those want a
//! second signal when nothing follows them, since a person can be called that: a
//! vendor word in front (`Google Gemini`), or a mailbox that is the tool's. The
//! mailbox is the tool's when its domain is one of the tool's `VENDOR_DOMAINS`,
//! matched whole, or when its local part is the tool's word at a mailbox that is a
//! machine's: a domain that is `localhost`, one label with no dot, or one ending
//! in one of `MACHINE_SUFFIXES`. The same local part at any other domain is a
//! person called that, `Devin <devin@acme.com>`, and so is GitHub's private
//! address, `Devin <12345+devin@users.noreply.github.com>`, which every account
//! has.
//!
//! What the default lets through is a bare given name behind a mailbox that is
//! neither the tool's nor on the list: `claude <root@buildhost.local>`, which was
//! found, and a person's own `Devin <devin@acme.com>`, which has to stay a person.
//! A project that knows its own build hosts names the name in `agent_names`, and
//! only a name it lists is read that way.
//!
//! # Configuration
//!
//! An entry is told apart by its shape. One holding `@` is a mailbox, a glob over
//! the whole mailbox. One opening with `[` is a marker. Anything else is a tool's
//! name, its words as spelled. Four keys on the lint carry them: `agent_identities`
//! replaces the whole list, `extra_agent_identities` adds to it, `agent_names`
//! names whole names that are agents whatever the mailbox, and `not_agents` names
//! people who are never to be read as agents.
//!
//! A `not_agents` entry is a `Name <mailbox>`, a mailbox glob, or a name. Only the
//! `Name <mailbox>` form excuses an identity whose mailbox is on the list, at a
//! vendor's domain or carries a marker, since a bare name or a bare mailbox there
//! would excuse every agent that commits as that name or from that mailbox.
//!
//! The lists and the verdicts are held to the conformance table that ships with
//! `mockspace-lint-rules`, which every other recogniser of an agent identity is
//! held to as well, so none of them can move apart from the rest without a suite
//! failing.

use crate::util::glob_matches;

mod lists;
mod mailbox;
mod words;

use lists::{DEFAULT_GIVEN_NAMES, DEFAULT_MAILBOXES, DEFAULT_MARKERS, DEFAULT_TOOLS, GROUP_TAGS};
use mailbox::Mailbox;
use words::{Tool, innermost_group, is_a_tool, split_identity, words_of};

/// The configured recogniser: a list of entries, more entries added to it, the
/// names the project says are agents, and the people never to be read as agents.
#[derive(Debug, Clone)]
pub(crate) struct AgentIdentities {
    entries:    Vec<String>,
    extra:      Vec<String>,
    names:      Vec<String>,
    not_agents: Vec<String>,
}

impl Default for AgentIdentities {
    fn default() -> Self {
        let entries = DEFAULT_MARKERS
            .iter()
            .chain(DEFAULT_MAILBOXES)
            .chain(DEFAULT_TOOLS)
            .chain(DEFAULT_GIVEN_NAMES)
            .map(|s| (*s).to_string())
            .collect();
        Self {
            entries,
            extra: Vec::new(),
            names: Vec::new(),
            not_agents: Vec::new(),
        }
    }
}

/// What the entries say, sorted by what they are.
struct Lists {
    markers: Vec<String>,
    globs:   Vec<String>,
    tools:   Vec<Tool>,
}

impl AgentIdentities {
    /// Replace the whole list. The defaults are gone, every arm of them.
    pub(crate) fn replace(&mut self, entries: Vec<String>) {
        self.entries = entries;
    }

    /// Add to the list, whatever it is.
    pub(crate) fn extend(&mut self, entries: Vec<String>) {
        self.extra = entries;
    }

    /// Name the whole names that are agents whatever the mailbox.
    pub(crate) fn name_agents(&mut self, names: Vec<String>) {
        self.names = names;
    }

    /// Name the people who are never to be read as agents.
    pub(crate) fn exclude(&mut self, people: Vec<String>) {
        self.not_agents = people;
    }

    /// Whether `value`, a `Name <mailbox>` or a bare name or mailbox, is an agent.
    pub(crate) fn names_an_agent(&self, value: &str) -> bool {
        let (name, mailbox) = split_identity(value);
        let name = name.to_lowercase();
        let mailbox = mailbox.to_lowercase();
        let parts = Mailbox::of(&mailbox);
        let lists = self.lists();

        let marked = lists
            .markers
            .iter()
            .any(|m| name.contains(m.as_str()) || mailbox.contains(m.as_str()));
        let listed = !mailbox.is_empty() && lists.globs.iter().any(|g| glob_matches(g, &mailbox));

        // A mailbox an agent commits from is not something a bare name or a bare
        // mailbox in `not_agents` can excuse, only the whole identity can.
        let agents_mailbox = listed
            || parts.is_at_a_vendor()
            || (!mailbox.is_empty() && lists.markers.iter().any(|m| mailbox.contains(m.as_str())));
        if self.is_excluded(&name, &mailbox, agents_mailbox) {
            return false;
        }
        if marked || listed {
            return true;
        }

        // A group in parentheses is read for a tag and for nothing else, and what
        // is not a tag is left out of the name.
        let mut rest = name;
        while let Some((open, close)) = innermost_group(&rest) {
            let group = words_of(&rest[open + 1 .. close]);
            if GROUP_TAGS.iter().any(|tag| words_of(tag) == group) {
                return true;
            }
            rest.replace_range(open ..= close, " ");
        }

        let words = words_of(&rest);
        if !words.is_empty() && self.names.iter().any(|n| words_of(n) == words) {
            return true;
        }
        is_a_tool(&words, &lists.tools, &parts)
    }

    /// The entries, sorted by shape: a marker, a mailbox glob, or a tool's name.
    fn lists(&self) -> Lists {
        let mut lists = Lists {
            markers: Vec::new(),
            globs:   Vec::new(),
            tools:   Vec::new(),
        };
        for entry in self.entries.iter().chain(&self.extra) {
            let entry = entry.trim().to_lowercase();
            if entry.is_empty() {
                continue;
            }
            if entry.starts_with('[') {
                lists.markers.push(entry);
            } else if entry.contains('@') {
                lists.globs.push(entry);
            } else {
                let words = words_of(&entry);
                if !words.is_empty() {
                    let given = DEFAULT_GIVEN_NAMES.iter().any(|g| words_of(g) == words);
                    lists.tools.push(Tool {
                        words,
                        given,
                    });
                }
            }
        }
        lists
    }

    /// Whether the project said this identity is a person. When the mailbox is one
    /// an agent commits from, only an entry naming both the name and the mailbox
    /// says so.
    fn is_excluded(&self, name: &str, mailbox: &str, agents_mailbox: bool) -> bool {
        self.not_agents.iter().any(|entry| {
            let entry = entry.trim().to_lowercase();
            if entry.contains('<') {
                let (n, m) = split_identity(&entry);
                words_of(n) == words_of(name) && glob_matches(m, mailbox)
            } else if agents_mailbox {
                false
            } else if entry.contains('@') {
                !mailbox.is_empty() && glob_matches(&entry, mailbox)
            } else {
                !entry.is_empty() && words_of(&entry) == words_of(name)
            }
        })
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod config_tests;

#[cfg(test)]
mod mailbox_tests;
