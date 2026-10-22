//! The word lists the recogniser reads, and nothing else.
//!
//! Each is held to the conformance table that ships with `mockspace-lint-rules`,
//! entry by entry, by the unit tests beside them.

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

/// Endings only a private network's domains have, written without the dot, so a
/// mailbox at one is a machine's. A domain that is `localhost`, or one label with no
/// dot, is a machine's as well.
pub(crate) const MACHINE_SUFFIXES: &[&str] =
    &["local", "localdomain", "lan", "internal", "home.arpa"];

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
    "chat",
    "workspace",
    "bugbot",
    "action",
    "sdk",
    "on",
    "the",
];
