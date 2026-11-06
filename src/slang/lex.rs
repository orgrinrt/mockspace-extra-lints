//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The tokens, comments and preprocessor lines of a Slang source, read in one pass.
//!
//! Every Slang lint and tool reads a file through this pass, so a comment, a string and a line that
//! carries code are told apart in one place. A line comment is the text after `//`, a block comment
//! is one entry per source line it spans, a string is one token whose text is what is between the
//! quotes, and a preprocessor line (with its continuations) is kept whole and never tokenised, since
//! a macro body holds braces that are no scope.

/// One token of the code, with the line it starts on (from 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: Kind,
    pub line: usize,
}

/// What a token is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    /// A name or a keyword.
    Ident(String),
    /// A number, or a character literal, as written.
    Number(String),
    /// A string literal, without its quotes.
    Str(String),
    /// Any other single character. `::`, `->` and `>>` are several of these.
    Punct(char),
}

impl Token {
    /// The identifier this is, if it is one.
    pub fn ident(&self) -> Option<&str> {
        match &self.kind {
            Kind::Ident(s) => Some(s),
            _ => None,
        }
    }

    /// Whether this is the punctuation character `c`.
    pub fn is(&self, c: char) -> bool {
        self.kind == Kind::Punct(c)
    }
}

/// One line of a comment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// The source line, from 1.
    pub line: usize,
    /// What the comment says, without its `//`, `///`, `//!`, `/*` or leading `*`.
    pub text: String,
    /// Whether it is a documentation comment: `///`, `//!`, `/**` or `/*!`.
    pub doc:  bool,
}

/// A preprocessor line, continuations joined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directive {
    /// The line it starts on, from 1.
    pub line: usize,
    /// The line from the `#`, trimmed.
    pub text: String,
}

/// What one pass over a source gives.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Lexed {
    pub tokens:     Vec<Token>,
    pub comments:   Vec<Comment>,
    pub directives: Vec<Directive>,
    /// How many lines carry a token or a directive: the lines that are neither blank nor only a
    /// comment.
    pub code_lines: usize,
}

/// Reads `text` in one pass.
pub fn lex(text: &str) -> Lexed {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Lexed::default();
    let mut coded: Vec<usize> = Vec::new();
    let mut line = 1usize;
    let mut i = 0usize;
    let mut line_start = true;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            line += 1;
            line_start = true;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            i = line_comment(&chars, i, line, &mut out);
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            let (next, lines) = block_comment(&chars, i, line, &mut out);
            i = next;
            line += lines;
        } else if c == '#' && line_start {
            let (next, lines) = directive(&chars, i, line, &mut out);
            coded.push(line);
            i = next;
            line += lines;
        } else {
            line_start = false;
            coded.push(line);
            let (token, next, lines) = token(&chars, i, line);
            out.tokens.push(token);
            i = next;
            line += lines;
        }
    }
    coded.dedup();
    out.code_lines = coded.len();
    out
}

fn line_comment(chars: &[char], at: usize, line: usize, out: &mut Lexed) -> usize {
    let mut end = at;
    while end < chars.len() && chars[end] != '\n' {
        end += 1;
    }
    let body: String = chars[at + 2 .. end].iter().collect();
    let doc = body.starts_with('/') && !body.starts_with("//") || body.starts_with('!');
    let text = if doc { &body[1 ..] } else { &body[..] };
    out.comments.push(Comment {
        line,
        text: text.trim().to_string(),
        doc,
    });
    end
}

/// Returns where the comment ends and how many newlines it crossed.
fn block_comment(chars: &[char], at: usize, line: usize, out: &mut Lexed) -> (usize, usize) {
    let mut end = at + 2;
    while end < chars.len() && !(chars[end] == '*' && chars.get(end + 1) == Some(&'/')) {
        end += 1;
    }
    let close = end.min(chars.len());
    let body: String = chars[at + 2 .. close].iter().collect();
    let doc = body.starts_with('*') && !body.starts_with("*/") || body.starts_with('!');
    let mut crossed = 0;
    for (n, raw) in body.split('\n').enumerate() {
        crossed = n;
        let mut text = raw.trim();
        if n == 0 && doc {
            text = text[1 ..].trim();
        }
        text = text.strip_prefix('*').map_or(text, str::trim);
        if !text.is_empty() {
            out.comments.push(Comment {
                line: line + n,
                text: text.to_string(),
                doc,
            });
        }
    }
    ((close + 2).min(chars.len()), crossed)
}

fn directive(chars: &[char], at: usize, line: usize, out: &mut Lexed) -> (usize, usize) {
    let mut end = at;
    let mut crossed = 0;
    let mut text = String::new();
    while end < chars.len() {
        if chars[end] == '\n' {
            if text.ends_with('\\') {
                text.pop();
                crossed += 1;
                end += 1;
                continue;
            }
            break;
        }
        text.push(chars[end]);
        end += 1;
    }
    out.directives.push(Directive {
        line,
        text: text.trim().to_string(),
    });
    (end, crossed)
}

/// Returns the token at `at`, where the next one starts and how many newlines it crossed.
fn token(chars: &[char], at: usize, line: usize) -> (Token, usize, usize) {
    let c = chars[at];
    if c == '"' {
        let mut end = at + 1;
        let mut text = String::new();
        let mut crossed = 0;
        while end < chars.len() && chars[end] != '"' {
            if chars[end] == '\\' && end + 1 < chars.len() {
                text.push(chars[end + 1]);
                crossed += usize::from(chars[end + 1] == '\n');
                end += 2;
                continue;
            }
            crossed += usize::from(chars[end] == '\n');
            text.push(chars[end]);
            end += 1;
        }
        let kind = Kind::Str(text);
        return (Token { kind, line }, (end + 1).min(chars.len()), crossed);
    }
    if c == '\'' {
        let mut end = at + 1;
        while end < chars.len() && chars[end] != '\'' && chars[end] != '\n' {
            end += if chars[end] == '\\' { 2 } else { 1 };
        }
        let text: String = chars[at .. end.min(chars.len())].iter().collect();
        return (Token { kind: Kind::Number(text), line }, (end + 1).min(chars.len()), 0);
    }
    if c.is_ascii_alphabetic() || c == '_' {
        let mut end = at;
        while end < chars.len() && (chars[end].is_ascii_alphanumeric() || chars[end] == '_') {
            end += 1;
        }
        let word: String = chars[at .. end].iter().collect();
        return (Token { kind: Kind::Ident(word), line }, end, 0);
    }
    if c.is_ascii_digit() {
        let mut end = at;
        while end < chars.len() && (chars[end].is_ascii_alphanumeric() || chars[end] == '_' || chars[end] == '.')
        {
            end += 1;
        }
        let text: String = chars[at .. end].iter().collect();
        return (Token { kind: Kind::Number(text), line }, end, 0);
    }
    (Token { kind: Kind::Punct(c), line }, at + 1, 0)
}
