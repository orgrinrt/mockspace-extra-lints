//! The members of a type declaration: a struct's fields and methods, an enum's variants, an
//! interface's methods.

use super::items::{ItemKind, Member, Scan, Visibility};
use super::lex::Token;
use super::text::{lone_colon, render};

/// The members the tokens of a type's body declare.
pub(super) fn members(kind: ItemKind, body: &[Token]) -> Vec<Member> {
    match kind {
        ItemKind::Enum => variants(body),
        ItemKind::Struct | ItemKind::Interface | ItemKind::Buffer => statements(body),
        _ => Vec::new(),
    }
}

/// One variant per top-level comma: its name over the value it is given.
fn variants(body: &[Token]) -> Vec<Member> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut from = 0usize;
    let mut take = |span: &[Token]| {
        let Some(name) = span.iter().find_map(Token::ident) else {
            return;
        };
        let value = span
            .iter()
            .position(|t| t.is('='))
            .map_or(String::new(), |eq| render(&span[eq + 1 ..]));
        out.push(Member { name: name.to_string(), body: value });
    };
    for (n, t) in body.iter().enumerate() {
        if t.is('(') || t.is('[') || t.is('{') {
            depth += 1;
        } else if t.is(')') || t.is(']') || t.is('}') {
            depth -= 1;
        } else if t.is(',') && depth == 0 {
            take(&body[from .. n]);
            from = n + 1;
        }
    }
    take(&body[from ..]);
    out
}

/// A statement ends at a `;` outside any group, or at the `}` closing a method body.
fn statements(body: &[Token]) -> Vec<Member> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut from = 0usize;
    let mut paren_seen = false;
    for (n, t) in body.iter().enumerate() {
        if t.is('(') {
            paren_seen = true;
            depth += 1;
        } else if t.is('[') || t.is('{') {
            depth += 1;
        } else if t.is(')') || t.is(']') {
            depth -= 1;
        } else if t.is('}') {
            depth -= 1;
            if depth == 0 && paren_seen {
                out.extend(statement(&body[from ..= n]));
                from = n + 1;
                paren_seen = false;
            }
        } else if t.is(';') && depth == 0 {
            out.extend(statement(&body[from .. n]));
            from = n + 1;
            paren_seen = false;
        }
    }
    out
}

const NOT_PART_OF_A_TYPE: &[&str] = &[
    "public", "private", "internal", "static", "const", "inline", "uniform", "precise", "volatile",
    "override", "virtual", "sealed", "in", "out", "inout",
];

/// A method (a parenthesis before any `=`) as its name over its signature, a field as its name over
/// its type.
fn statement(tokens: &[Token]) -> Option<Member> {
    let mut scan = Scan { t: tokens, i: 0 };
    scan.attributes();
    let start = scan.i;
    let mut at = start;
    while tokens.get(at).and_then(Token::ident).is_some_and(|w| NOT_PART_OF_A_TYPE.contains(&w)) {
        at += 1;
    }
    let body = &tokens[at ..];
    let end = body.iter().position(|t| t.is('=') || t.is('(')).unwrap_or(body.len());
    if body.get(end).is_some_and(|t| t.is('(')) {
        let method = Scan { t: tokens, i: 0 }.function(at, at + end, Vec::new(), Visibility::Unmarked)?;
        return Some(Member {
            name: method.name,
            body: format!("({}) {}", method.params.join(", "), method.ret).trim_end().to_string(),
        });
    }
    let head = &body[.. end];
    let stop = (0 .. head.len()).find(|&n| lone_colon(head, n)).unwrap_or(head.len());
    let span = &head[.. stop];
    let name_at = span.iter().rposition(|t| t.ident().is_some())?;
    Some(Member {
        name: span[name_at].ident()?.to_string(),
        body: render(&span[.. name_at]),
    })
}
