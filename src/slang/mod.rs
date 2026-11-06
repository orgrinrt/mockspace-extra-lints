//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The one reading of Slang that every lint and tool of the pack takes.
//!
//! A lint about shaders needs the same few facts a lint about Rust gets from its parse: where the
//! comments are, which lines carry code, what the file declares and what it imports. They are read
//! here, once, so no lint carries a parser of its own and two of them cannot disagree about what a
//! comment or a declaration is. [`read`] gives a [`SlangFile`]; [`sources`] walks a repository for its
//! `.slang` sources (as [`sources`]) and names the crate each sits in.
//!
//! What is read is the top level: the `module` name, `import`, `__include` and `implementing`, the
//! functions, types, constants and globals with their visibility, attributes, parameter types and
//! return type, and every comment with its line. A body is skipped as balanced braces.

mod files;
mod items;
mod lex;
mod members;
mod text;
#[cfg(test)]
mod tests;

pub use files::{SlangSource, sources};
pub use items::{Import, ImportForm, Item, ItemKind, Member, Visibility};
pub use lex::{Comment, Directive};

/// One Slang source, read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlangFile {
    /// The name `module name;` gives, and its line.
    pub module:     Option<(String, usize)>,
    pub imports:    Vec<Import>,
    pub items:      Vec<Item>,
    pub comments:   Vec<Comment>,
    /// The preprocessor lines, among them any `#include`.
    pub directives: Vec<Directive>,
    /// The lines that are neither blank nor only a comment.
    pub code_lines: usize,
}

/// Reads one source.
pub fn read(text: &str) -> SlangFile {
    let lexed = lex::lex(text);
    let parsed = items::parse(&lexed.tokens);
    SlangFile {
        module:     parsed.module,
        imports:    parsed.imports,
        items:      parsed.items,
        comments:   lexed.comments,
        directives: lexed.directives,
        code_lines: lexed.code_lines,
    }
}

impl SlangFile {
    /// The functions this file declares an entry for, with their stage.
    pub fn entries(&self) -> impl Iterator<Item = (&Item, &str)> {
        self.items
            .iter()
            .filter_map(|i| i.stage().map(|stage| (i, stage)))
    }

    /// The declarations other modules can use.
    pub fn exports(&self) -> impl Iterator<Item = &Item> {
        self.items.iter().filter(|i| i.exported())
    }
}
