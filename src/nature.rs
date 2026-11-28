//! What a crate is by its own nature, which decides where a stack lint applies.
//!
//! A stack lint holds a crate to the stack's own vocabulary: no `std`, no heap, no bare primitive,
//! the arvo types. Where it applies is never a list in a configuration. It is read off the crate:
//! what its root declares and what it is built from.
//!
//! - A crate that declares `#![no_std]` has said it lives without `std`, so `no-std` and
//!   `no-alloc` hold it to that. One that does not declare it has said nothing either lint could
//!   hold it to.
//! - A crate of the stack is one of the crates the vocabulary lives in, `arvo`, `notko` and
//!   `hilavitkutin-str`, or a `#![no_std]` crate built on one of them. A `std` crate that depends
//!   on the vocabulary is a consumer, and the ban is not enforced on consumers, who store whatever
//!   they want (`the_no_raw_primitives_lint_is_internal_and_is_not_enforced_on_consumers`). A crate
//!   with no edge to the vocabulary at all is not of the stack either.
//! - Each bare-type lint refuses a primitive and names its replacement, and each replacement lives
//!   in one of those crates: the numeric types in `arvo`, `Maybe`, `Outcome` and `Just` in `notko`,
//!   `Str` in `hilavitkutin-str`. The lint holds a crate of the stack only where the crate that owns
//!   its replacement is reachable from the crate's own source: the crate is one of that owner's
//!   family, or its manifest names the owner as a dependency. A finding asking for a type the crate
//!   cannot name has no fix in that crate, so it is not a finding there. That gives
//!   `arvo-types-only` and `no-bare-numeric` to `arvo`, `no-bare-option` and `no-bare-result` to
//!   `notko`, `no-bare-string` to `hilavitkutin-str`, and `no-public-raw-field` to the owner of each
//!   field type it refuses, `arvo` for a number and `hilavitkutin-str` for a string.
//! - `no-bare-static-str` is not one of them. Its fix in a crate without `hilavitkutin-str` is to
//!   gate the item behind `#[cfg(debug_assertions)]`, which needs nothing from another crate, so it
//!   holds every crate of the stack and only its message reads the edge: `Str` where the crate has
//!   it, the gate alone where it does not. `no-std` and `no-alloc` read the attribute alone, since
//!   what they ask for, `core` and storage that does not allocate, a `no_std` crate always has.
//!
//! The dependency test is one reading of the manifest, [`names_as_dependency`], used by both: by
//! [`is_of_the_stack`] for the edge to the vocabulary and by [`reaches_the_owner`] for the edge to
//! one owner. An edge counts when the crate's own source can name what it brings:
//!
//! - `[dependencies]` and `[target.<cfg>.dependencies]` count, a rename by the package it names, a
//!   `workspace = true` entry by its key.
//! - `[dev-dependencies]` do not: they build the tests, examples and benches, and the lints read
//!   `src/`.
//! - `[build-dependencies]` do not: they build `build.rs`, and nothing of theirs is in scope in
//!   `src/`.
//! - An `optional = true` dependency does not: the source written for a build without the feature
//!   cannot name it, and a line scan cannot tell that source from the gated part. A crate taking
//!   the owner behind a feature is held where it takes it unconditionally.

use mockspace_lint_rules::LintContext;

use crate::util::Category;

/// A crate the stack's vocabulary lives in, named by the package name its family opens with.
///
/// The family is the name itself and every package opening with the name and a hyphen, so `arvo`
/// is `arvo`, `arvo-bits` and `arvo-format`, and nothing called `arvoisa`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Owner(&'static str);

impl Owner {
    /// The numeric types, `bool` among them.
    pub const ARVO: Self = Self("arvo");
    /// `Str` and its compile-time handles.
    pub const HILAVITKUTIN_STR: Self = Self("hilavitkutin-str");
    /// `Maybe`, `Outcome` and `Just`.
    pub const NOTKO: Self = Self("notko");

    /// The package name the family opens with, as a finding names it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        self.0
    }

    /// Whether `package` is of this family.
    #[must_use]
    pub fn owns(self, package: &str) -> bool {
        package == self.0
            || package
                .strip_prefix(self.0)
                .is_some_and(|rest| rest.starts_with('-'))
    }
}

/// The crates the stack's vocabulary lives in
/// (`the_stack_supplies_its_own_vocabulary_for_each_bare_primitive_in_arvo_notko_and_the_string_crate`).
const VOCABULARY: [Owner; 3] = [Owner::ARVO, Owner::NOTKO, Owner::HILAVITKUTIN_STR];

/// Whether the crate's root declares `#![no_std]`, read from the root's prelude, where an inner
/// attribute can stand. A crate whose root cannot be read declares nothing.
#[must_use]
pub fn declares_no_std(ctx: &LintContext) -> bool {
    ["src/lib.rs", "src/main.rs"]
        .iter()
        .find_map(|f| ctx.view().text(ctx.crate_dir().join(f)))
        .is_some_and(|root| crate_prelude(root).contains("#![no_std]"))
}

/// The crate's own manifest, or nothing where it cannot be read.
fn manifest<'a>(ctx: &LintContext<'a>) -> Option<&'a toml::Table> {
    match ctx.view().toml(ctx.crate_dir().join("Cargo.toml")) {
        Some(Ok(manifest)) => Some(manifest),
        _ => None,
    }
}

/// Whether the crate is of the stack: one of the vocabulary's own crates, or a `#![no_std]` crate
/// that depends on one. A manifest that cannot be read says nothing, so the crate is not taken to
/// be of the stack.
#[must_use]
pub fn is_of_the_stack(ctx: &LintContext) -> bool {
    manifest(ctx).is_some_and(|m| manifest_is_of_the_stack(m, declares_no_std(ctx)))
}

/// Whether a bare-type lint guarding `category` holds the crate: the crate is of the stack, the
/// crate that owns the category's replacement is reachable from it, and it is not the crate that
/// introduces the category, whose own internals are where the replacement is defined.
#[must_use]
pub fn is_held_to(ctx: &LintContext, category: Category) -> bool {
    is_of_the_stack(ctx) && reaches_the_owner(ctx, category.owner) && !ctx.introduces(category.name)
}

/// Whether the crate can name what `owner` brings: it is of the owner's family, or its manifest
/// names the owner as a dependency by [`names_as_dependency`].
#[must_use]
pub fn reaches_the_owner(ctx: &LintContext, owner: Owner) -> bool {
    manifest(ctx).is_some_and(|m| manifest_reaches(m, owner))
}

/// [`is_of_the_stack`] over a parsed manifest and whether the crate's root declares `#![no_std]`.
#[must_use]
pub fn manifest_is_of_the_stack(manifest: &toml::Table, no_std: bool) -> bool {
    if own_name(manifest).is_some_and(|own| VOCABULARY.iter().any(|v| v.owns(own))) {
        return true;
    }
    no_std
        && names_as_dependency(manifest, |package| {
            VOCABULARY.iter().any(|v| v.owns(package))
        })
}

/// [`reaches_the_owner`] over a parsed manifest.
#[must_use]
pub fn manifest_reaches(manifest: &toml::Table, owner: Owner) -> bool {
    own_name(manifest).is_some_and(|own| owner.owns(own))
        || names_as_dependency(manifest, |package| owner.owns(package))
}

fn own_name(manifest: &toml::Table) -> Option<&str> {
    manifest
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(toml::Value::as_str)
}

/// Whether the manifest names a dependency whose package `wanted` accepts, counting only an edge
/// the crate's own source can name: `[dependencies]` on any target, not `optional`. A rename counts
/// by the package it names, a `workspace = true` entry by its key. `[dev-dependencies]` build the
/// tests and `[build-dependencies]` build `build.rs`, so neither is read.
#[must_use]
pub fn names_as_dependency(manifest: &toml::Table, wanted: impl Fn(&str) -> bool) -> bool {
    let top = manifest.get("dependencies").and_then(toml::Value::as_table);
    let per_target = manifest
        .get("target")
        .and_then(toml::Value::as_table)
        .into_iter()
        .flat_map(|targets| targets.values())
        .filter_map(|target| target.get("dependencies").and_then(toml::Value::as_table));
    top.into_iter()
        .chain(per_target)
        .flat_map(|table| table.iter())
        .any(|(key, spec)| {
            let optional = spec
                .get("optional")
                .and_then(toml::Value::as_bool)
                .unwrap_or(false);
            let package = spec
                .get("package")
                .and_then(toml::Value::as_str)
                .unwrap_or(key);
            !optional && wanted(package)
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
