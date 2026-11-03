//! TOML read the way the engine reads it, for the lints that look at a manifest or `mockspace.toml`.
//!
//! A lint that reads such a file as text sees the spellings its author thought of and misses the
//! rest: the engine takes `name = { key = 1 }`, a dotted key `name.key = 1` and a quoted key as the
//! same table, and a lint about what a table holds has to take them as well. This parses with the
//! library the engine uses, at the revision that reads TOML 1.1 as cargo now does (an inline table may
//! span lines and end in a comma), and keeps the byte spans, so a finding still names the line a key
//! stands on.

use std::ops::Range;

use toml_edit::Document;

/// The file as a document, or nothing when it is not TOML. A file that is not TOML is one the engine
/// and cargo refuse to load, so no lint has anything to add about it.
pub(crate) fn parse(text: &str) -> Option<Document<&str>> {
    Document::parse(text).ok()
}

/// The line, counted from one, that a span starts on. A span the parser did not record is line one.
pub(crate) fn line_of(text: &str, span: Option<Range<usize>>) -> usize {
    span.map_or(1, |s| {
        text[.. s.start.min(text.len())].matches('\n').count() + 1
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_span_is_read_as_the_line_it_starts_on() {
        let text = "a = 1\nb = 2\n\nc = 3\n";
        let doc = parse(text).unwrap();
        let line = |key: &str| line_of(text, doc.as_table().get_key_value(key).unwrap().0.span());
        assert_eq!((line("a"), line("b"), line("c")), (1, 2, 4));
    }

    #[test]
    fn a_file_that_is_not_toml_is_nothing() {
        assert!(parse("a = = 1").is_none());
    }

    #[test]
    fn a_span_that_was_not_recorded_is_line_one() {
        assert_eq!(line_of("a\nb\n", None), 1);
    }
}
