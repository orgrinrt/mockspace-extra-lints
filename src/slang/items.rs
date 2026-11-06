//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The module header, the imports and the top-level declarations of a Slang source.
//!
//! This reads what a file says about itself at its top level, and nothing inside a body: a function's
//! statements, a struct's members and an initialiser are skipped as balanced braces. A declaration is
//! a function (a header with a parenthesis before any `=`), a `struct`, `interface`, `enum`,
//! `class`, `extension`, `cbuffer` or `typealias`, a `static const` value, or any other global
//! (a resource, a groupshared array). The contents of a `namespace` are read as top-level.

use super::lex::{Kind, Token};
use super::members::members;
use super::text::{render, split_params};

/// How a declaration may be reached from another module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Marked `public`: what a module offers to the files that import it.
    Public,
    /// Marked `internal`.
    Internal,
    /// Marked `private`.
    Private,
    /// Carries none of the three.
    Unmarked,
}

/// What a declaration is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ItemKind {
    Func,
    Struct,
    Interface,
    Enum,
    Extension,
    Alias,
    Const,
    Buffer,
    /// A global that is none of the above: a resource, a groupshared array, a uniform.
    Global,
}

/// One top-level declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub kind:       ItemKind,
    pub name:       String,
    /// The line the name stands on, from 1.
    pub line:       usize,
    pub visibility: Visibility,
    /// The attributes in front of it, each as written between its brackets, such as
    /// `shader("compute")`.
    pub attributes: Vec<String>,
    /// For a function, the type of each parameter, as written and without the name.
    pub params:     Vec<String>,
    /// For a function, the return type as written. Empty for anything else.
    pub ret:        String,
    /// For a struct, its fields; for an enum, its variants; for an interface, its methods.
    pub members:    Vec<Member>,
    /// For a type, the generic parameters as written between its angle brackets, empty without.
    pub generics:   String,
}

/// One member of a type, as `name` over `body`: a field's type, a variant's value, a method's
/// signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub name: String,
    pub body: String,
}

impl Item {
    /// The stage this declares an entry for, from a `shader("stage")` attribute.
    pub fn stage(&self) -> Option<&str> {
        self.attributes.iter().find_map(|a| {
            let rest = a.strip_prefix("shader")?.trim_start().strip_prefix('(')?;
            let rest = rest.trim().strip_prefix('"')?;
            rest.split('"').next()
        })
    }

    /// Whether other modules can use it: marked `public`.
    pub fn exported(&self) -> bool {
        self.visibility == Visibility::Public
    }
}

/// How a file names another module to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportForm {
    /// `import name;`
    Import,
    /// `__include name;`, or `__include "path";`
    Include,
    /// `implementing name;`: this file is a part of that module.
    Implementing,
}

/// A module a file names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Import {
    /// The module as written, a dotted name or the text of a path.
    pub name: String,
    pub line: usize,
    pub form: ImportForm,
}

/// What the scan finds.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Parsed {
    /// The name `module name;` gives, and its line.
    pub module:  Option<(String, usize)>,
    pub imports: Vec<Import>,
    pub items:   Vec<Item>,
}

const MODIFIERS: &[&str] = &[
    "public",
    "private",
    "internal",
    "export",
    "static",
    "const",
    "inline",
    "extern",
    "uniform",
    "groupshared",
    "precise",
    "volatile",
    "shared",
    "in",
    "out",
    "inout",
    "__exported",
    "unsafe",
    "override",
    "virtual",
    "sealed",
    "__generic",
];

pub(super) struct Scan<'a> {
    pub(super) t: &'a [Token],
    pub(super) i: usize,
}

/// Reads the top level of a token stream.
pub fn parse(tokens: &[Token]) -> Parsed {
    let mut out = Parsed::default();
    let mut scan = Scan { t: tokens, i: 0 };
    scan.scope(&mut out);
    out
}

impl<'a> Scan<'a> {
    fn peek(&self) -> Option<&'a Token> {
        self.t.get(self.i)
    }

    fn at(&self, c: char) -> bool {
        self.peek().is_some_and(|t| t.is(c))
    }

    fn ident(&self) -> Option<&'a str> {
        self.peek().and_then(Token::ident)
    }

    /// Reads declarations until the scope closes or the tokens end.
    fn scope(&mut self, out: &mut Parsed) {
        while let Some(t) = self.peek() {
            if t.is('}') {
                return;
            }
            if t.is(';') {
                self.i += 1;
                continue;
            }
            let before = self.i;
            self.declaration(out);
            if self.i == before {
                self.i += 1;
            }
        }
    }

    /// An attribute run `[a] [b(1)]`, each as the text between its brackets.
    pub(super) fn attributes(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        while self.at('[') {
            let start = self.i + 1;
            self.skip_group('[', ']');
            out.push(render(&self.t[start .. self.i.saturating_sub(1).max(start)]));
        }
        out
    }

    /// Skips a balanced group whose opener is under the cursor, to just past its closer.
    fn skip_group(&mut self, open: char, close: char) {
        let mut depth = 0usize;
        while let Some(t) = self.peek() {
            self.i += 1;
            if t.is(open) {
                depth += 1;
            } else if t.is(close) {
                depth -= 1;
                if depth == 0 {
                    return;
                }
            }
        }
    }

    /// Skips to just past the next `;`, over balanced braces and parentheses.
    fn skip_statement(&mut self) {
        while let Some(t) = self.peek() {
            if t.is('{') {
                self.skip_group('{', '}');
            } else if t.is('(') {
                self.skip_group('(', ')');
            } else {
                self.i += 1;
                if t.is(';') {
                    return;
                }
            }
        }
    }

    fn declaration(&mut self, out: &mut Parsed) {
        let attributes = self.attributes();
        let mut visibility = Visibility::Unmarked;
        let mut is_const = false;
        while let Some(word) = self.ident().filter(|w| MODIFIERS.contains(w)) {
            match word {
                "public" => visibility = Visibility::Public,
                "internal" => visibility = Visibility::Internal,
                "private" => visibility = Visibility::Private,
                "const" => is_const = true,
                "__generic" if self.t.get(self.i + 1).is_some_and(|t| t.is('<')) => {
                    self.i += 1;
                    self.skip_angles();
                    continue;
                },
                _ => {},
            }
            self.i += 1;
        }
        let Some(word) = self.ident() else {
            self.skip_statement();
            return;
        };
        let line = self.peek().map_or(0, |t| t.line);
        let make = |kind, name: String, line| Item {
            kind,
            name,
            line,
            visibility,
            attributes: attributes.clone(),
            params: Vec::new(),
            ret: String::new(),
            members: Vec::new(),
            generics: String::new(),
        };
        match word {
            "module" | "implementing" | "import" | "__include" => self.reference(word, out),
            "namespace" => {
                self.i += 1;
                while self.peek().is_some_and(|t| !t.is('{') && !t.is(';')) {
                    self.i += 1;
                }
                if self.at('{') {
                    self.i += 1;
                    self.scope(out);
                    self.i += usize::from(self.at('}'));
                }
            },
            "using" => self.skip_statement(),
            "struct" | "class" | "interface" | "enum" | "cbuffer" | "tbuffer" | "extension" => {
                let kind = match word {
                    "interface" => ItemKind::Interface,
                    "enum" => ItemKind::Enum,
                    "extension" => ItemKind::Extension,
                    "cbuffer" | "tbuffer" => ItemKind::Buffer,
                    _ => ItemKind::Struct,
                };
                self.i += 1;
                let (name, name_line) = self.type_name();
                let mut item = make(kind, name, name_line);
                self.skip_body(&mut item);
                out.items.push(item);
            },
            "typealias" | "typedef" => {
                self.i += 1;
                let start = self.i;
                self.skip_statement();
                let span = &self.t[start .. self.i];
                let name = if word == "typealias" {
                    span.first().and_then(Token::ident)
                } else {
                    span.iter().rev().find_map(Token::ident)
                };
                if let Some(name) = name {
                    out.items.push(make(ItemKind::Alias, name.to_string(), line));
                }
            },
            _ => self.value_or_function(out, attributes.clone(), visibility, is_const),
        }
    }

    /// `import a.b;`, `module a;`, `implementing a;`, `__include "a";`.
    fn reference(&mut self, word: &str, out: &mut Parsed) {
        let line = self.peek().map_or(0, |t| t.line);
        self.i += 1;
        let start = self.i;
        self.skip_statement();
        let span = &self.t[start .. self.i.saturating_sub(1).max(start)];
        let name: String = span
            .iter()
            .map(|t| match &t.kind {
                Kind::Ident(s) | Kind::Str(s) => s.clone(),
                Kind::Punct(c) => c.to_string(),
                Kind::Number(_) => String::new(),
            })
            .collect();
        if name.is_empty() {
            return;
        }
        match word {
            "module" => out.module = Some((name, line)),
            "implementing" => {
                out.imports.push(Import { name, line, form: ImportForm::Implementing });
            },
            "__include" => out.imports.push(Import { name, line, form: ImportForm::Include }),
            _ => out.imports.push(Import { name, line, form: ImportForm::Import }),
        }
    }

    /// The name of a type declaration, and its line: the identifier after the keyword, or the whole
    /// target type of an `extension`.
    fn type_name(&mut self) -> (String, usize) {
        let line = self.peek().map_or(0, |t| t.line);
        let start = self.i;
        while self.peek().is_some_and(|t| !t.is('{') && !t.is(';') && !t.is(':') && !t.is('<')) {
            self.i += 1;
        }
        let span = &self.t[start .. self.i];
        let name = span
            .iter()
            .find_map(|t| t.ident().filter(|w| *w != "class"))
            .unwrap_or_default()
            .to_string();
        (name, line)
    }

    /// Past the bases and the braces of a type declaration, and the `;` or declarators after it,
    /// giving the item its generics and its members on the way.
    fn skip_body(&mut self, item: &mut Item) {
        let header = self.i;
        while self.peek().is_some_and(|t| !t.is('{') && !t.is(';')) {
            self.i += 1;
        }
        if self.t.get(header).is_some_and(|t| t.is('<')) {
            let mut depth = 0usize;
            let close = self.t[header .. self.i]
                .iter()
                .position(|t| {
                    depth += usize::from(t.is('<'));
                    depth -= usize::from(t.is('>'));
                    depth == 0
                })
                .map_or(self.i, |c| header + c);
            item.generics = render(&self.t[header + 1 .. close]);
        }
        if self.at('{') {
            let body = self.i + 1;
            self.skip_group('{', '}');
            item.members = members(item.kind, &self.t[body .. self.i.saturating_sub(1).max(body)]);
        }
        while self.peek().is_some_and(|t| !t.is(';') && !t.is('}') && t.ident().is_none()) {
            self.i += 1;
        }
        if self.at(';') {
            self.i += 1;
        }
    }

    fn skip_angles(&mut self) {
        let mut depth = 0usize;
        while let Some(t) = self.peek() {
            self.i += 1;
            if t.is('<') {
                depth += 1;
            } else if t.is('>') {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return;
                }
            }
        }
    }

    /// A function, a `static const` value or another global.
    fn value_or_function(
        &mut self,
        out: &mut Parsed,
        attributes: Vec<String>,
        visibility: Visibility,
        is_const: bool,
    ) {
        let start = self.i;
        let mut paren = None;
        let mut assign = None;
        let mut annotated = None;
        while let Some(t) = self.peek() {
            let previous = self.i.checked_sub(1).and_then(|p| self.t.get(p));
            let next = self.t.get(self.i + 1);
            if t.is('<') && assign.is_none() && annotated.is_none() {
                self.skip_angles();
            } else if t.is(':')
                && annotated.is_none()
                && !next.is_some_and(|x| x.is(':'))
                && !previous.is_some_and(|x| x.is(':'))
            {
                annotated = Some(self.i);
                self.i += 1;
            } else if t.is('(') && paren.is_none() && assign.is_none() && annotated.is_none() {
                paren = Some(self.i);
                self.skip_group('(', ')');
            } else if t.is('(') {
                self.skip_group('(', ')');
            } else if t.is('=') && assign.is_none() && paren.is_none() {
                assign = Some(self.i);
                self.i += 1;
            } else if t.is('{') && assign.is_none() {
                break;
            } else if t.is('{') {
                self.skip_group('{', '}');
            } else if t.is(';') || t.is('}') {
                break;
            } else {
                self.i += 1;
            }
        }
        let item = match paren {
            Some(open) => self.function(start, open, attributes, visibility),
            None => {
                let end = [annotated, assign].into_iter().flatten().min().unwrap_or(self.i);
                let name = self.t[start .. end].iter().rev().find_map(|t| t.ident().map(|n| (n, t.line)));
                name.map(|(name, line)| Item {
                    kind: if is_const { ItemKind::Const } else { ItemKind::Global },
                    name: name.to_string(),
                    line,
                    visibility,
                    attributes,
                    params: Vec::new(),
                    ret: String::new(),
                    members: Vec::new(),
                    generics: String::new(),
                })
            },
        };
        out.items.extend(item);
        if self.at('{') {
            self.skip_group('{', '}');
        } else if self.at(';') {
            self.i += 1;
        }
    }

    pub(super) fn function(
        &self,
        start: usize,
        open: usize,
        attributes: Vec<String>,
        visibility: Visibility,
    ) -> Option<Item> {
        let mut name_at = open.checked_sub(1)?;
        if self.t[name_at].is('>') {
            let mut depth = 0usize;
            while name_at > start {
                if self.t[name_at].is('>') {
                    depth += 1;
                } else if self.t[name_at].is('<') {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                name_at -= 1;
            }
            name_at = name_at.checked_sub(1)?;
        }
        let name_tok = &self.t[name_at];
        let name = name_tok.ident()?;
        let close = self.matching(open);
        let params = split_params(&self.t[open + 1 .. close]);
        let ret = render(&self.t[start .. name_at]);
        Some(Item {
            kind: ItemKind::Func,
            name: name.to_string(),
            line: name_tok.line,
            visibility,
            attributes,
            params,
            ret,
            members: Vec::new(),
            generics: String::new(),
        })
    }

    /// The index of the `)` closing the `(` at `open`.
    fn matching(&self, open: usize) -> usize {
        let mut depth = 0usize;
        for (n, t) in self.t[open ..].iter().enumerate() {
            if t.is('(') {
                depth += 1;
            } else if t.is(')') {
                depth -= 1;
                if depth == 0 {
                    return open + n;
                }
            }
        }
        self.t.len()
    }
}
