//! What a crate is by its own nature, which decides where a stack lint applies.
//!
//! A stack lint holds a crate to the stack's own vocabulary: no `std`, no heap, no bare primitive,
//! the arvo types. Where it applies is never a list in a configuration. It is read off the crate:
//! what its root declares and what it is built from.
//!
//! - A crate that declares `#![no_std]` has said it lives without `std`, so `no-std` and
//!   `no-alloc` hold it to that. One that does not declare it has said nothing either lint could
//!   hold it to.
//! - A crate of the stack is one of the crates the vocabulary lives in, `arvo` and `notko`, or a
//!   `#![no_std]` crate built on one of them. The bare-primitive lints, `arvo-types-only` and
//!   `no-public-raw-field` ask a crate to use that vocabulary, which only a crate that has it can
//!   do. A `std` crate that depends on the vocabulary is a consumer, and the ban is not enforced on
//!   consumers, who store whatever they want (`the_no_raw_primitives_lint_is_internal_and_is_not_enforced_on_consumers`).
//!   A crate with no edge to the vocabulary at all is not of the stack either.

use mockspace_lint_rules::LintContext;

/// The crates the stack's vocabulary lives in, by the name their packages open with
/// (`the_stack_supplies_its_own_vocabulary_for_each_bare_primitive_in_arvo_notko_and_the_string_crate`).
const VOCABULARY: [&str; 2] = ["arvo", "notko"];

/// Whether the crate's root declares `#![no_std]`, read from the root's prelude, where an inner
/// attribute can stand. A crate whose root cannot be read declares nothing.
#[must_use]
pub fn declares_no_std(ctx: &LintContext) -> bool {
    ["src/lib.rs", "src/main.rs"]
        .iter()
        .find_map(|f| ctx.view().text(ctx.crate_dir().join(f)))
        .is_some_and(|root| crate_prelude(root).contains("#![no_std]"))
}

/// Whether the crate is of the stack: one of the vocabulary's own crates, or a `#![no_std]` crate
/// that depends on one. A manifest that cannot be read says nothing, so the crate is not taken to
/// be of the stack.
#[must_use]
pub fn is_of_the_stack(ctx: &LintContext) -> bool {
    let Some(Ok(manifest)) = ctx.view().toml(ctx.crate_dir().join("Cargo.toml")) else {
        return false;
    };
    manifest_is_of_the_stack(manifest, declares_no_std(ctx))
}

/// [`is_of_the_stack`] over a parsed manifest and whether the crate's root declares `#![no_std]`.
#[must_use]
pub fn manifest_is_of_the_stack(manifest: &toml::Table, no_std: bool) -> bool {
    let own = manifest
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(toml::Value::as_str);
    if own.is_some_and(of_the_vocabulary) {
        return true;
    }
    no_std && depends_on_the_vocabulary(manifest)
}

/// Whether a normal or build dependency, on any target, is one of the vocabulary's crates. A
/// renamed dependency counts by the package it names; a dev-dependency builds tests, not the crate.
fn depends_on_the_vocabulary(manifest: &toml::Table) -> bool {
    let mut tables: Vec<&toml::Table> = Vec::new();
    for key in ["dependencies", "build-dependencies"] {
        if let Some(t) = manifest.get(key).and_then(toml::Value::as_table) {
            tables.push(t);
        }
    }
    if let Some(targets) = manifest.get("target").and_then(toml::Value::as_table) {
        for target in targets.values().filter_map(toml::Value::as_table) {
            for key in ["dependencies", "build-dependencies"] {
                if let Some(t) = target.get(key).and_then(toml::Value::as_table) {
                    tables.push(t);
                }
            }
        }
    }
    tables.iter().flat_map(|t| t.iter()).any(|(name, spec)| {
        let package = spec
            .get("package")
            .and_then(toml::Value::as_str)
            .unwrap_or(name);
        of_the_vocabulary(package)
    })
}

/// Whether a package name is one of the vocabulary's crates: the name itself, or the name and a
/// hyphen opening it.
fn of_the_vocabulary(package: &str) -> bool {
    VOCABULARY.iter().any(|v| {
        package == *v
            || package
                .strip_prefix(v)
                .is_some_and(|rest| rest.starts_with('-'))
    })
}

/// The part of a crate root that may carry an inner attribute.
///
/// Everything up to and including the last line before the first item, which is where the language
/// allows `#![...]` and nowhere else. A module doc runs as long as it runs, so a fixed line count is
/// a guess about how much prose a crate is allowed to open with, and this reads the shape instead.
///
/// The window used to be thirty lines. A crate whose module doc ran to thirty-one reported its root
/// as missing an attribute that was on line thirty-two, which is the whole reason this function
/// exists.
///
/// Conservative at the boundary: an unrecognised line ends the prelude.
#[must_use]
pub fn crate_prelude(source: &str) -> String {
    let mut kept = Vec::new();
    for line in source.lines() {
        let text = line.trim_start();
        // `#!` and not `#[`: an outer attribute opens an item, and an item is exactly what ends the
        // prelude.
        let is_prelude = text.is_empty()
            || text.starts_with("//")
            || text.starts_with("#!")
            || text.starts_with("/*")
            || text.starts_with('*');
        if !is_prelude {
            break;
        }
        kept.push(line);
    }
    kept.join("\n")
}

#[cfg(test)]
mod tests;
