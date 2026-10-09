//! What names an agent, from a commit's author, committer or a `Co-Authored-By`.
//!
//! The recogniser reads what only an agent carries, and never a word that happens
//! to sit inside somebody's name. A substring match refuses `Devin Smith`,
//! `Claude Monet` and `Haider Ali` on every commit they make, and at a commit gate
//! that is a person blocked from their own work. What an agent carries is one of
//! three things, tried in this order:
//!
//! - a marker, matched anywhere in the name or the mailbox: `[bot]` is not
//!   something a person writes into either by accident;
//! - a mailbox an agent commits from, matched whole as a glob. A vendor's domain
//!   is not one, so a person writing from it is a person;
//! - a tool's own name, which makes the identity an agent's only as the whole of a
//!   name: optionally behind vendor words, then the tool, then nothing but the
//!   words that ride along with a tool and versions. `Claude Opus 4.1`, `GitHub
//!   Copilot` and `Gemini Code Assist` are the tools, and `Claude Monet`, `Max
//!   Claude` and `Claude Max` are people.
//!
//! A tool whose name is also somebody's given name counts only with a second
//! signal: a vendor word before it, a companion or a version after it, or one of
//! the mailboxes. A person whose whole name is `Claude` or `Devin` is therefore a
//! person, and `Claude Code` is the tool.
//!
//! A group in parentheses is read on its own, as a tool's name when it is one
//! (`Jane Smith (aider)`, which is how aider tags a name) and left out of the name
//! when it is not (`Claude 3.5 Sonnet (new)`).
//!
//! # Configuration
//!
//! An entry is told apart by its shape. One holding `@` is a mailbox, a glob over
//! the whole mailbox. One opening with `[` is a marker. Anything else is a tool's
//! name, its words as spelled. Three keys on the lint carry them: `agent_identities`
//! replaces the whole list, `extra_agent_identities` adds to it, and `not_agents`
//! names people who are never to be read as agents, as a name, a mailbox glob, or
//! `Name <mailbox>`.
//!
//! The lists and the verdicts are held to the conformance table that ships with
//! `mockspace-lint-rules`, which a second implementation of the recogniser, in shell, is
//! held to as well, so the two cannot move apart without a suite failing.

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

/// Tools whose name counts as the whole of a name with nothing else.
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
];

/// Tools whose name is also somebody's given name, which count only with a second
/// signal.
pub(crate) const DEFAULT_GIVEN_NAMES: &[&str] = &[
    "claude", "devin", "gemini", "amp", "cursor", "jules", "cody", "bard", "kiro", "junie",
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

/// The configured recogniser: a list of entries, more entries added to it, and the
/// people never to be read as agents.
#[derive(Debug, Clone)]
pub(crate) struct AgentIdentities {
    entries:    Vec<String>,
    extra:      Vec<String>,
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
            not_agents: Vec::new(),
        }
    }
}

/// A tool's name as its words, and whether it needs a second signal.
struct Tool {
    words: Vec<String>,
    given: bool,
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

    /// Name the people who are never to be read as agents.
    pub(crate) fn exclude(&mut self, people: Vec<String>) {
        self.not_agents = people;
    }

    /// Whether `value`, a `Name <mailbox>` or a bare name or mailbox, is an agent.
    pub(crate) fn names_an_agent(&self, value: &str) -> bool {
        let (name, mailbox) = split_identity(value);
        let name = name.to_lowercase();
        let mailbox = mailbox.to_lowercase();

        if self.is_excluded(&name, &mailbox) {
            return false;
        }

        let mut tools: Vec<Tool> = Vec::new();
        for entry in self.entries.iter().chain(&self.extra) {
            let entry = entry.trim().to_lowercase();
            if entry.is_empty() {
                continue;
            }
            if entry.starts_with('[') {
                if name.contains(&entry) || mailbox.contains(&entry) {
                    return true;
                }
            } else if entry.contains('@') {
                if !mailbox.is_empty() && glob_matches(&entry, &mailbox) {
                    return true;
                }
            } else {
                let words = words_of(&entry);
                if !words.is_empty() {
                    let given = DEFAULT_GIVEN_NAMES.iter().any(|g| words_of(g) == words);
                    tools.push(Tool {
                        words,
                        given,
                    });
                }
            }
        }

        // A group in parentheses is read on its own, as a tool's name when it is
        // one and left out of the name when it is not.
        let mut rest = name;
        while let Some((open, close)) = innermost_group(&rest) {
            if is_a_tool(&words_of(&rest[open + 1 .. close]), &tools) {
                return true;
            }
            rest.replace_range(open ..= close, " ");
        }
        is_a_tool(&words_of(&rest), &tools)
    }

    /// Whether the project said this identity is a person.
    fn is_excluded(&self, name: &str, mailbox: &str) -> bool {
        self.not_agents.iter().any(|entry| {
            let entry = entry.trim().to_lowercase();
            if entry.contains('<') {
                let (n, m) = split_identity(&entry);
                words_of(n) == words_of(name) && glob_matches(m, mailbox)
            } else if entry.contains('@') {
                !mailbox.is_empty() && glob_matches(&entry, mailbox)
            } else {
                !entry.is_empty() && words_of(&entry) == words_of(name)
            }
        })
    }
}

/// Whether `words` are a tool's name: optionally behind vendor words, then a tool,
/// then only companions and versions.
fn is_a_tool(words: &[String], tools: &[Tool]) -> bool {
    let mut k = 0;
    while k < words.len() {
        if tool_starts_at(words, k, tools) {
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

/// Whether a tool's name starts at word `k` and is followed by nothing but
/// companions and versions, with the second signal a given-name tool wants.
fn tool_starts_at(words: &[String], k: usize, tools: &[Tool]) -> bool {
    tools.iter().any(|tool| {
        let end = k + tool.words.len();
        if words.len() < end || words[k .. end] != tool.words[..] {
            return false;
        }
        let tail = &words[end ..];
        if !tail.iter().all(|w| is_a_companion_or_version(w)) {
            return false;
        }
        !tool.given || k > 0 || !tail.is_empty()
    })
}

fn is_a_companion_or_version(word: &str) -> bool {
    COMPANION_WORDS.contains(&word)
        || word.starts_with(|c: char| c.is_ascii_digit())
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
/// its two brackets.
fn innermost_group(text: &str) -> Option<(usize, usize)> {
    let close = text.find(')')?;
    let open = text[.. close].rfind('(')?;
    Some((open, close))
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
