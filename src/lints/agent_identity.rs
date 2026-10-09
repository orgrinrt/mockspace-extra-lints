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
//! - a name that starts with a tool's own, behind vendor words if any. A tool that
//!   is not somebody's given name makes an agent of any words after it (`Copilot
//!   Chat`, `Cursor Bugbot`), and a tool inside somebody's name is none (`Smith
//!   Copilot`).
//!
//! A tool whose name is also somebody's given name (`Claude`, `Devin`, `Gemini`
//! and the rest of `DEFAULT_GIVEN_NAMES`) is read more carefully, since a person
//! can be called that. It is an agent when a companion word is anywhere after it
//! (`Claude Code Action`, `Claude Agent SDK`, `Claude Code on the web`), when
//! nothing but versions follow it (`Claude 3.5`), or when nothing follows it and
//! either a vendor word stands in front (`Google Gemini`) or the mailbox is the
//! tool's: its local part is the tool's word (the login after `NNN+` for a
//! `users.noreply.github.com` mailbox), or its domain is one of the tool's
//! `VENDOR_DOMAINS`, matched whole. Any other word after it is a word of
//! somebody's name, so `Claude Monet`, `Claude Max` and `Claude Pro` are people.
//!
//! What the default lets through is a bare given name behind a mailbox that is
//! neither the tool's nor on the list, `claude <root@buildhost.local>` being the
//! one found, and a project that knows its own build hosts names the name in
//! `agent_names`. Only a name it lists is read that way.
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

/// Markers only a bot carries, matched anywhere in the name or the mailbox.
pub(crate) const DEFAULT_MARKERS: &[&str] = &["[bot]"];

/// Mailboxes an agent commits from, globs over the whole mailbox.
pub(crate) const DEFAULT_MAILBOXES: &[&str] = &[
    "noreply@anthropic.com",
    "noreply@openai.com",
    "copilot@github.com",
    "cursoragent@cursor.com",
    "agent@cursor.com",
    "amp@ampcode.com",
    "grok@x.ai",
    "openhands@all-hands.dev",
    "*-bot@*",
];

/// Tags a tool writes into a name, which is all a group in parentheses is read
/// for. Closed: a group holding anything else says nothing about the name.
pub(crate) const GROUP_TAGS: &[&str] = &["aider"];

/// Tools that are not somebody's given name, an agent's as the start of a name.
pub(crate) const DEFAULT_TOOLS: &[&str] = &[
    "copilot",
    "chatgpt",
    "gpt",
    "codex",
    "grok",
    "xai",
    "x ai",
    "windsurf",
    "codeium",
    "tabnine",
    "supermaven",
    "codewhisperer",
    "amazon q",
    "antigravity",
    "opencode",
    "openhands",
    "replit",
    "ghostwriter",
    "phind",
    "deepseek",
    "qwen",
    "ollama",
    "llama",
    "aider",
    "cline",
    "roo code",
    "roo cline",
    "lovable",
    "droid",
    "sourcegraph",
    "anysphere",
    "cognition labs",
    "jetbrains ai",
    "blackbox ai",
    "continue dev",
    "augment code",
    "augmentcode",
    "bolt new",
    "v0 dev",
    "factory ai",
    "swe agent",
    "ai assistant",
    "coding agent",
    "llm agent",
    "crush bot",
    "goose bot",
    "anthropic",
    "openai",
    "opus",
    "sonnet",
    "haiku",
    "cursor",
];

/// Tools whose name is also somebody's given name, which count only with a second
/// signal.
pub(crate) const DEFAULT_GIVEN_NAMES: &[&str] =
    &["claude", "devin", "gemini", "amp", "jules", "cody", "bard", "kiro", "junie"];

/// A given-name tool and a domain it commits from, one pair per domain. The domain
/// is matched whole, so a subdomain or a longer name ending with it is no match.
pub(crate) const VENDOR_DOMAINS: &[(&str, &str)] = &[
    ("claude", "anthropic.com"),
    ("claude", "claude.com"),
    ("claude", "claude.ai"),
    ("devin", "cognition.ai"),
    ("devin", "devin.ai"),
    ("gemini", "google.com"),
    ("amp", "ampcode.com"),
    ("jules", "jules.google"),
    ("cody", "sourcegraph.com"),
    ("bard", "google.com"),
    ("kiro", "kiro.dev"),
    ("junie", "jetbrains.com"),
];

/// Vendor words allowed in front of a tool's name.
pub(crate) const HEAD_WORDS: &[&str] =
    &["github", "google", "anthropic", "openai", "microsoft", "amazon", "aws"];

/// Words allowed after a tool's name: a model, a product, a surface. Not a word
/// that is also somebody's name, so `max` and `pro` are not here.
pub(crate) const COMPANION_WORDS: &[&str] = &[
    "code",
    "agent",
    "ai",
    "bot",
    "assistant",
    "assist",
    "cli",
    "app",
    "coding",
    "via",
    "slack",
    "web",
    "desktop",
    "ide",
    "extension",
    "connector",
    "integration",
    "autofix",
    "review",
    "reviewer",
    "new",
    "preview",
    "beta",
    "opus",
    "sonnet",
    "haiku",
    "instant",
    "flash",
    "turbo",
    "lite",
    "thinking",
    "mini",
];

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

/// A tool's name as its words, and whether it needs a second signal.
struct Tool {
    words: Vec<String>,
    given: bool,
}

/// What the entries say, sorted by what they are.
struct Lists {
    markers: Vec<String>,
    globs:   Vec<String>,
    tools:   Vec<Tool>,
}

/// A mailbox as the two halves a given-name tool is judged on.
#[derive(Default)]
struct Mailbox {
    local:  String,
    domain: String,
}

impl Mailbox {
    /// The local part, which is the login after `NNN+` at GitHub's noreply domain,
    /// and the domain. A value with no `@` has neither.
    fn of(mailbox: &str) -> Self {
        let Some((local, domain)) = mailbox.rsplit_once('@') else {
            return Self::default();
        };
        let login = (domain == "users.noreply.github.com")
            .then(|| local.split_once('+'))
            .flatten()
            .filter(|(id, login)| {
                !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) && !login.is_empty()
            })
            .map(|(_, login)| login);
        Self {
            local:  login.unwrap_or(local).to_string(),
            domain: domain.to_string(),
        }
    }

    /// Whether the mailbox is a given-name tool's: its local part is the tool's
    /// word, or its domain is one of the tool's vendor domains. One with no domain
    /// is nobody's.
    fn is_the_tools(&self, tool: &str) -> bool {
        !self.domain.is_empty()
            && (self.local == tool
                || VENDOR_DOMAINS
                    .iter()
                    .any(|(t, d)| *t == tool && *d == self.domain))
    }

    /// Whether the domain is one a given-name tool commits from, whichever tool.
    fn is_at_a_vendor(&self) -> bool {
        !self.domain.is_empty() && VENDOR_DOMAINS.iter().any(|(_, d)| *d == self.domain)
    }
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

/// Whether `words` start with a tool's name, optionally behind vendor words.
fn is_a_tool(words: &[String], tools: &[Tool], mailbox: &Mailbox) -> bool {
    let mut k = 0;
    while k < words.len() {
        if tool_starts_at(words, k, tools, mailbox) {
            return true;
        }
        // one more vendor word in front, or the words are not a tool's
        if !HEAD_WORDS.contains(&words[k].as_str()) {
            return false;
        }
        k += 1;
    }
    false
}

/// Whether a tool's name starts at word `k`. A tool that is not a given name makes
/// an agent of whatever follows it. One that is wants a companion after it,
/// versions alone after it, or nothing after it and a vendor word before it or a
/// mailbox that is the tool's.
fn tool_starts_at(words: &[String], k: usize, tools: &[Tool], mailbox: &Mailbox) -> bool {
    tools.iter().any(|tool| {
        let end = k + tool.words.len();
        if words.len() < end || words[k .. end] != tool.words[..] {
            return false;
        }
        if !tool.given {
            return true;
        }
        let tail = &words[end ..];
        if tail.is_empty() {
            return k > 0 || mailbox.is_the_tools(&tool.words.join(" "));
        }
        tail.iter().any(|w| COMPANION_WORDS.contains(&w.as_str()))
            || tail.iter().all(|w| is_a_version(w))
    })
}

/// A word opening with a digit, or `v` and digits.
fn is_a_version(word: &str) -> bool {
    word.starts_with(|c: char| c.is_ascii_digit())
        || (word.len() > 1
            && word.starts_with('v')
            && word[1 ..].chars().all(|c| c.is_ascii_digit()))
}

/// The lowercase alphanumeric words of `text`.
fn words_of(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// The first group in parentheses that holds no other, as the byte positions of
/// its two brackets. A closing bracket with nothing open before it is skipped, and
/// an opening one followed by another is not the group.
fn innermost_group(text: &str) -> Option<(usize, usize)> {
    let mut open = None;
    for (i, c) in text.char_indices() {
        match c {
            '(' => open = Some(i),
            ')' => {
                if let Some(o) = open {
                    return Some((o, i));
                }
            },
            _ => {},
        }
    }
    None
}

/// The name and the mailbox of an identity: `Name <mailbox>`, or a bare mailbox
/// when it holds an `@` and no spaces, or a bare name.
fn split_identity(value: &str) -> (&str, &str) {
    let value = value.trim();
    if let (Some(open), Some(close)) = (value.rfind('<'), value.rfind('>')) {
        if open < close {
            return (value[.. open].trim(), value[open + 1 .. close].trim());
        }
    }
    if value.contains('@') && !value.contains(char::is_whitespace) {
        return ("", value);
    }
    (value, "")
}

#[cfg(test)]
#[path = "agent_identity_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "agent_identity_config_tests.rs"]
mod config_tests;
