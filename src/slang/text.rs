//! Tokens turned back into the text a signature is compared by.

use super::lex::{Kind, Token};

/// The type of each parameter in the tokens between a function's parentheses.
pub(super) fn split_params(tokens: &[Token]) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut from = 0usize;
    for (n, t) in tokens.iter().enumerate() {
        match &t.kind {
            Kind::Punct('(' | '[' | '<') => depth += 1,
            Kind::Punct(')' | ']' | '>') => depth -= 1,
            Kind::Punct(',') if depth == 0 => {
                out.push(param_type(&tokens[from .. n]));
                from = n + 1;
            },
            _ => {},
        }
    }
    if from < tokens.len() {
        out.push(param_type(&tokens[from ..]));
    }
    out.retain(|p| !p.is_empty());
    out
}

/// Whether the token at `n` is a `:` that is neither half of a `::`.
pub(super) fn lone_colon(tokens: &[Token], n: usize) -> bool {
    tokens[n].is(':')
        && !tokens.get(n + 1).is_some_and(|x| x.is(':'))
        && !n.checked_sub(1).and_then(|p| tokens.get(p)).is_some_and(|x| x.is(':'))
}

/// One parameter without its name, its semantic and its default.
fn param_type(tokens: &[Token]) -> String {
    let end = (0 .. tokens.len())
        .find(|&n| lone_colon(tokens, n) || tokens[n].is('='))
        .unwrap_or(tokens.len());
    let span = &tokens[.. end];
    let named = span.len() > 1 && span.last().is_some_and(|t| t.ident().is_some());
    render(if named { &span[.. span.len() - 1] } else { span })
}

/// Tokens as text: words apart by a space, punctuation against what it follows, a string in quotes.
pub(super) fn render(tokens: &[Token]) -> String {
    let mut out = String::new();
    let mut word_before = false;
    for t in tokens {
        let word = match &t.kind {
            Kind::Ident(s) | Kind::Number(s) => Some(s.clone()),
            Kind::Str(s) => Some(format!("\"{s}\"")),
            Kind::Punct(c) => {
                out.push(*c);
                word_before = false;
                None
            },
        };
        if let Some(word) = word {
            if word_before {
                out.push(' ');
            }
            out.push_str(&word);
            word_before = true;
        }
    }
    out
}
