//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Which statements would need editing when the implementation moves.
//!
//! The canon is intent. A row saying what the game is for survives a total rewrite of everything
//! under it; a row naming the component that does the work does not, and the day that component is
//! replaced the row is quietly false. Ported from kamu.
//!
//! A tool because it ranks candidates for a judgement. Naming a mechanism is not wrong on its own,
//! and whether a sentence has crossed from intent into mechanism is a reading.
//!
//! Two signals, reported separately because they mean different things.
//!
//!   - A named piece of the implementation. kamu enumerates its mechanisms as rows; kaski does not
//!     and has no use for such a namespace. What moves when kaski's implementation moves is its
//!     crates and the types its designs declare, so those are what a statement is matched against:
//!     every crate, and every identifier a `DESIGN.md.tmpl` under `mock/crates/` writes in backticks.
//!   - Implementation-shaped text: a path, a Rust path, a flag, a file extension, a call. These name
//!     a spelling rather than a thing, and a spelling is the first part of an implementation to move.

use std::collections::{BTreeMap, BTreeSet};

use mockspace::tool::{ArgSpec, NotALint, Purpose, Tool, ToolContext, ToolReport};

use registry_lints::tree_walk;

/// The fields carrying the canon's own statements, read out of the schema.
///
/// **This was a hand-written list of seven names**, and it had already fallen
/// out of step in the way both sibling tools document their own tables falling
/// out of step. It could not see `does`, which is the field op ordered added for
/// what a thing does; it could not see `note` or `gap`, which are 341 rows
/// between them; and it named nothing on `term` at all. Twenty-six fields are
/// declared and it read seven.
///
/// Three properties in the declaration decide it, and each is there for its own
/// reason rather than for this tool:
///
///   - **`type = "string"`.** A relation names a row and is checked by the
///     citation lint; a `string[]` is a list of search words.
///   - **no `values`.** A closed enumeration is a label, so `kind` picks from
///     three words and none of them is a statement.
///   - **not `visibility = "internal"`.** A quotation is op's exact words and is
///     the one thing here nobody may edit, which is the opposite of what this
///     tool is looking for. `key` addresses a document the reader cannot open
///     and `source` records a derivation. All three are marked in the schema,
///     so none has to be named here.
///
/// Scanned rather than parsed as TOML, which the crate has no dependency for.
/// A field block belongs to the namespace whose header last opened, which is
/// what array-of-tables nesting means and what makes the last field of one
/// namespace look like the first of the next if you read it any other way.
fn statement_fields(schema: &str) -> BTreeMap<String, BTreeSet<String>> {
    #[derive(PartialEq)]
    enum Block {
        None,
        Namespace,
        Field,
    }

    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut ns = String::new();
    let mut block = Block::None;
    let (mut name, mut ty, mut enumerated, mut internal) =
        (String::new(), String::new(), false, false);

    let flush = |ns: &str,
                 name: &mut String,
                 ty: &mut String,
                 enumerated: &mut bool,
                 internal: &mut bool,
                 out: &mut BTreeMap<String, BTreeSet<String>>| {
        if !ns.is_empty() && !name.is_empty() && ty == "string" && !*enumerated && !*internal {
            out.entry(ns.to_string())
                .or_default()
                .insert(std::mem::take(name));
        }
        name.clear();
        ty.clear();
        *enumerated = false;
        *internal = false;
    };

    for line in schema.lines() {
        let line = line.trim();
        if line == "[[registry.namespace]]" || line == "[[registry.namespace.field]]" {
            if block == Block::Field {
                flush(
                    &ns,
                    &mut name,
                    &mut ty,
                    &mut enumerated,
                    &mut internal,
                    &mut out,
                );
            }
            block = if line.ends_with("field]]") { Block::Field } else { Block::Namespace };
            continue;
        }
        let Some((k, v)) = line.split_once(" = ") else {
            continue;
        };
        let v = v.trim().trim_matches('"');
        match (&block, k) {
            (Block::Namespace, "key") => ns = v.to_string(),
            (Block::Field, "name") => name = v.to_string(),
            (Block::Field, "type") => ty = v.to_string(),
            (Block::Field, "values") => enumerated = true,
            (Block::Field, "visibility") => internal = v == "internal",
            _ => {},
        }
    }
    if block == Block::Field {
        flush(
            &ns,
            &mut name,
            &mut ty,
            &mut enumerated,
            &mut internal,
            &mut out,
        );
    }
    out
}

/// The `s` on a counted noun. One definition, because two copies of the same
/// rule in one report is how two sentences about one run stop agreeing.
fn plural(n: usize) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// Whether the text names a declared type, written as the design writes it.
///
/// Case-sensitive, where [`names`] is not: the designs declare `Situation`, `Knowledge` and `Sky`,
/// and the canon uses each of those words as a word. A type written mid-sentence with its capital is
/// the type; the same word opening a sentence is capitalised by the sentence, so a one-word name
/// counts only where the sentence did not put its capital there. A name with a second capital or a
/// digit, `HeatLattice`, is never a sentence's doing and counts anywhere.
fn names_type(text: &str, name: &str) -> bool {
    let one_word = !name
        .chars()
        .skip(1)
        .any(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
    text.match_indices(name).any(|(at, _)| {
        let before = text[.. at].chars().next_back();
        let after = text[at + name.len() ..].chars().next();
        if before.is_some_and(char::is_alphanumeric) || after.is_some_and(char::is_alphanumeric) {
            return false;
        }
        !one_word || !opens_a_sentence(&text[.. at])
    })
}

/// Whether what precedes a word leaves it opening a sentence: nothing before it, or a full stop,
/// a question or an exclamation, a colon or a semicolon, with only spaces, quotes and brackets
/// between.
fn opens_a_sentence(before: &str) -> bool {
    match before
        .chars()
        .rev()
        .find(|c| !c.is_whitespace() && !matches!(c, '"' | '\'' | '(' | '[' | '`'))
    {
        None => true,
        Some(c) => matches!(c, '.' | '!' | '?' | ':' | ';'),
    }
}

/// Whether the text names a crate: the whole name, not the start of a longer one.
///
/// Its own boundary because a crate's name carries hyphens, so `kaski-fire` must not be found in
/// `kaski-fire-lints`, which the alphanumeric boundary of [`names`] would allow.
fn names_crate(lower: &str, krate: &str) -> bool {
    let part = |c: char| c.is_alphanumeric() || c == '-' || c == '_';
    lower.match_indices(krate).any(|(at, _)| {
        !lower[.. at].chars().next_back().is_some_and(part)
            && !lower[at + krate.len() ..].chars().next().is_some_and(part)
    })
}

/// Every identifier the designs of the crates under the source directories write in backticks, with
/// the crates whose design writes it, and how many designs were read.
///
/// An identifier is a capital and then at least two more letters or digits: a type, a trait, a
/// variant. A path or a call in backticks is not one, and a statement naming either is caught by
/// [`SHAPES`] instead.
fn design_types(src_dirs: &[std::path::PathBuf]) -> (BTreeMap<String, Vec<String>>, usize) {
    let mut out: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut read = 0usize;
    for (krate, dir) in src_dirs.iter().flat_map(|d| crate::tree_walk::subdirs(d)) {
        let Ok(text) = std::fs::read_to_string(dir.join("DESIGN.md.tmpl")) else {
            continue;
        };
        read += 1;
        for (i, span) in text.split('`').enumerate() {
            let is_ident = span.len() >= 3
                && span.starts_with(|c: char| c.is_ascii_uppercase())
                && span.chars().all(|c| c.is_ascii_alphanumeric());
            if i % 2 == 1 && is_ident {
                let from = out.entry(span.to_string()).or_default();
                if !from.contains(&krate) {
                    from.push(krate.clone());
                }
            }
        }
    }
    (out, read)
}

/// The directories at the repository root, each with its slash, which a path in the repository starts
/// at. Read from the tree, since which directories those are is the repository's own; a hidden
/// directory, the build directory and a root [`SHAPES`] already names are left out.
fn path_roots(repo_root: &std::path::Path) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(repo_root)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter(|n| !n.starts_with('.') && n != "target")
        .map(|n| format!("{n}/"))
        .filter(|r| !SHAPES.iter().any(|(s, _)| s == r))
        .collect();
    out.sort();
    out
}

/// Text shapes that name a spelling rather than a thing.
///
/// Each is matched as a substring on purpose: a version or a path fragment is not a word and does not
/// sit on word boundaries. Every member was chosen because it cannot occur in a sentence about intent
/// without naming something concrete.
///
/// kamu matched a bare `/` as a path separator. kaski writes alternatives with one, `Being/Genome`,
/// `realm/culture/region`, and every slash in its statements when this was ported was one of those,
/// so a path is matched by the roots the repository's paths start at instead, which [`path_roots()`]
/// reads from the tree, and by `src/`, which every crate's paths pass through.
const SHAPES: &[(&str, &str)] = &[
    ("::", "a Rust path"),
    ("src/", "a path in the repository"),
    ("--", "a command-line flag"),
    (".toml", "a file extension"),
    (".rs", "a file extension"),
    (".md", "a file extension"),
    (".wgsl", "a file extension"),
    (".slang", "a file extension"),
    ("()", "a call"),
];

pub struct IntentOrMechanism;

impl Tool for IntentOrMechanism {
    fn name(&self) -> &'static str {
        "intent-or-mechanism"
    }

    fn description(&self) -> &'static str {
        "statements naming a crate, a type a design declares, or a spelling that moves when the \
         implementation does"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Check(NotALint::NoFailingCase)
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[ArgSpec {
            name:        "namespace",
            required:    false,
            description: "rank one namespace rather than all of them",
        }]
    }

    fn help(&self) -> &'static str {
        "Each hit is `namespace::slug field: what was found`. Which fields count as statements is \
         read from the schema: a string field that is neither a closed enumeration nor marked \
         internal. What a statement is matched against is every crate and every type a design \
         declares in backticks. Naming one is not wrong on its own, so read every hit: the question \
         is whether the sentence still holds after the thing it names is replaced."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let only = ctx.args.first().copied();
        let mut out = String::new();
        let mut flagged = 0usize;
        let mut examined = 0usize;
        // A statement field the row leaves empty. Counted because a total over the rows that filled
        // them reads as a total over the rows.
        let mut absent = 0usize;
        // Namespaces the engine serves that the schema does not declare.
        let mut undeclared: Vec<String> = Vec::new();

        let Ok(schema) = std::fs::read_to_string(ctx.repo_root.join("mockspace.toml")) else {
            return ToolReport::inconclusive(
                "no `mockspace.toml` to read the field declarations from. Which fields carry a \
                 statement is read from the schema rather than listed here, so without it there is \
                 nothing to examine and a clean report would be about the instrument.",
            );
        };
        let fields = statement_fields(&schema);
        if fields.is_empty() {
            return ToolReport::inconclusive(
                "the schema declares no statement field, meaning no string field that is neither a \
                 closed enumeration nor internal. Either the schema moved or this stopped being able \
                 to read it.",
            );
        }

        let (types, designs) = design_types(ctx.src_dirs);
        let roots = path_roots(ctx.repo_root);
        if types.is_empty() {
            return ToolReport::inconclusive(format!(
                "no design declares a type: {designs} `DESIGN.md.tmpl` read under {}, and none \
                 writes an identifier in backticks. Half of what this looks for could not be looked \
                 for, so a clean report would be about the instrument rather than the canon.",
                ctx.src_dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>().join(", ")
            ));
        }
        let crates: Vec<String> = ctx.all_crates.iter().map(|c| c.to_lowercase()).collect();

        let mut declared_namespaces = 0usize;
        let mut declared_fields = 0usize;

        let namespaces: Vec<String> = ctx.registry.namespaces().map(str::to_string).collect();
        for ns in &namespaces {
            if only.is_some_and(|o| o != ns.as_str()) {
                continue;
            }
            let Some(declared) = fields.get(ns) else {
                let n = ctx.registry.rows_in(ns).len();
                undeclared.push(format!("{ns} ({n} row{})", plural(n)));
                continue;
            };
            declared_namespaces += 1;
            declared_fields += declared.len();
            for q in ctx.registry.rows_in(ns) {
                for field in declared {
                    let Some(text) = ctx.registry.field(q, field) else {
                        absent += 1;
                        continue;
                    };
                    examined += 1;
                    let lower = text.to_lowercase();
                    let mut found: Vec<String> = Vec::new();
                    for (name, from) in &types {
                        if names_type(text, name) {
                            found.push(format!(
                                "names `{name}` ({}'s design)",
                                from.join("'s and ")
                            ));
                        }
                    }
                    for c in &crates {
                        if names_crate(&lower, c) {
                            found.push(format!("names the crate `{c}`"));
                        }
                    }
                    for (shape, what) in SHAPES {
                        if text.contains(shape) {
                            found.push(format!("{what} (`{shape}`)"));
                        }
                    }
                    for root in &roots {
                        if text.contains(root.as_str()) {
                            found.push(format!("a path in the repository (`{root}`)"));
                        }
                    }
                    if !found.is_empty() {
                        flagged += 1;
                        out.push_str(&format!("{q} {field}: {}\n", found.join(", ")));
                    }
                }
            }
        }

        if examined == 0 && declared_namespaces == 0 && !undeclared.is_empty() {
            return ToolReport::inconclusive(format!(
                "`mockspace.toml` declares no fields for {}, so nothing was examined. Which fields \
                 carry a statement is read from the schema, and for these it is the schema that is \
                 silent rather than the rows.",
                undeclared.join(", ")
            ));
        }
        if examined == 0 {
            return ToolReport::inconclusive(
                "no statement field was examined, so this establishes nothing about the canon. \
                 Either the registry is empty or no row fills a field the schema declares as a \
                 statement.",
            );
        }

        out.push_str(&format!(
            "\n{flagged} of {examined} statements name something that can move.\n"
        ));
        out.push_str(&format!(
            "  Read from the schema: {declared_fields} statement field{} across \
             {declared_namespaces} namespace{}, matched against {} type{} declared in {designs} \
             design{} and {} crate{}. Empty on their row: {absent} declared field{}.\n",
            plural(declared_fields),
            plural(declared_namespaces),
            types.len(),
            plural(types.len()),
            plural(designs),
            crates.len(),
            plural(crates.len()),
            plural(absent),
        ));
        if !undeclared.is_empty() {
            out.push_str(&format!(
                "  Not examined at all: {}. The engine serves these and `mockspace.toml` declares no \
                 fields for them, so nothing here can say which of their fields carry a \
                 statement.\n",
                undeclared.join(", ")
            ));
        }
        ToolReport::reported(out, examined)
    }
}

#[cfg(test)]
mod tests;

mockspace::lint_pack! {
    tools: [IntentOrMechanism],
}
