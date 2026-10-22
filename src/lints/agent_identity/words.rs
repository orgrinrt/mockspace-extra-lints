//! Reading a name as words, and the words as a tool's.

use super::lists::{COMPANION_WORDS, HEAD_WORDS};
use super::mailbox::Mailbox;

/// A tool's name as its words, and whether it needs a second signal.
pub(super) struct Tool {
    pub(super) words: Vec<String>,
    pub(super) given: bool,
}

/// Whether `words` start with a tool's name, optionally behind vendor words, and
/// carry nothing after it but companions and versions.
pub(super) fn is_a_tool(words: &[String], tools: &[Tool], mailbox: &Mailbox) -> bool {
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

/// Whether a tool's name starts at word `k`, with nothing after it but companions
/// and versions. A tool that is not a given name is an agent alone. One that is
/// wants, alone, a vendor word before it or a mailbox that is the tool's.
fn tool_starts_at(words: &[String], k: usize, tools: &[Tool], mailbox: &Mailbox) -> bool {
    tools.iter().any(|tool| {
        let end = k + tool.words.len();
        if words.len() < end || words[k .. end] != tool.words[..] {
            return false;
        }
        let tail = &words[end ..];
        if tail.is_empty() {
            return !tool.given || k > 0 || mailbox.is_the_tools(&tool.words.join(" "));
        }
        tail.iter()
            .all(|w| COMPANION_WORDS.contains(&w.as_str()) || is_a_version(w))
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
pub(super) fn words_of(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// The first group in parentheses that holds no other, as the byte positions of
/// its two brackets. A closing bracket with nothing open before it is skipped, and
/// an opening one followed by another is not the group.
pub(super) fn innermost_group(text: &str) -> Option<(usize, usize)> {
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
pub(super) fn split_identity(value: &str) -> (&str, &str) {
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
