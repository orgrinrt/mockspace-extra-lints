//! The value of a Rust string literal, read from its source text.
//!
//! The lints that look at an attribute or a macro argument get the literal as the parse hands it
//! over, quotes, hashes and escapes included, and what they compare is the string it means.

/// The value of a string literal, raw or not. `None` for anything that is not one.
pub(crate) fn literal(text: &str) -> Option<String> {
    if let Some(rest) = text.strip_prefix('r') {
        let hashes = rest.bytes().take_while(|&b| b == b'#').count();
        let inner = rest[hashes..].strip_prefix('"')?;
        let end = format!("\"{}", "#".repeat(hashes));
        return inner.strip_suffix(end.as_str()).map(str::to_string);
    }
    text.strip_prefix('"')?.strip_suffix('"').map(unescape)
}

/// A string literal's body with its escapes read, a backslash at the end of a line taking the
/// whitespace of the next with it.
fn unescape(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\n' | '\r') => {
                while chars.next_if(|c| c.is_whitespace()).is_some() {}
            }
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('0') => out.push('\0'),
            Some(c @ ('"' | '\'' | '\\')) => out.push(c),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_are_read_the_way_the_compiler_reads_them() {
        assert_eq!(unescape(r#"a \"b\" \\ \n\t"#), "a \"b\" \\ \n\t");
        assert_eq!(unescape("one \\\n     two"), "one two");
        assert_eq!(unescape("one \\\r\n\t two"), "one two");
        assert_eq!(literal("\"x\""), Some("x".to_string()));
        assert_eq!(literal("r\"x\\y\""), Some("x\\y".to_string()));
        assert_eq!(literal("r##\"a\"#b\"##"), Some("a\"#b".to_string()));
        assert_eq!(literal("concat!(\"x\")"), None);
    }
}
