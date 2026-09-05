//! Lint: a name a consumer has to write is re-exported from the crate whose
//! signature makes them write it.
//!
//! Owning a dependency means owning the names it puts in your own API. A
//! public function that takes a `riimu_face::Face` has made every caller
//! depend on `riimu_face`, unless `Face` is reachable from here, and a
//! consumer made to add a second crate to call a function it already depends
//! on has been handed a version the kit chose for it. The second spelling of
//! the crate underneath is also a second package in the graph, whose types do
//! not unify with the ones this crate was compiled against.
//!
//! Two tiers, and the second is the one that gets forgotten. The first is
//! this crate's own signatures: every foreign type, trait or alias in a
//! public parameter, return, field, bound, where clause or alias has a `pub
//! use` here. The second is what a re-export carries with it: re-exporting
//! `Face` hands a consumer every public method on `Face`, so what those
//! methods take and return is a name the consumer has to write too, and it
//! has to be reachable from here on the same terms. That tier reads the
//! dependency's source through `cargo metadata`, and stops at the
//! dependency's own declarations: a name that crate took from one further
//! down is one crate further than this reads.
//!
//! Crates of the same workspace are not foreign, on the reasoning a kit gives:
//! a consumer names the kit's crates and nothing under them, and a sibling is
//! one of the kit's. `core`, `std` and `alloc` are never foreign.
//!
//! Escape hatch (single line): `// lint:allow(re-export-foreign-names)
//! reason: ...; tracked: #N`, on the signature or on the `pub use`.

use std::collections::{BTreeMap, BTreeSet};

use mockspace_lint_rules::{CrateLint, Lint, LintContext, LintError, Severity};
use tree_sitter::Node;

use crate::dep_surface::{Dependencies, dependencies, exposures, is_reserved, sources};
use crate::paths::{
    Imports,
    as_root,
    for_each_kind,
    generic_names,
    imports_of,
    is_plain_pub,
    modules_of,
    parse,
    re_exports_of,
    type_names,
    under_cfg_test,
    use_leaves,
};
use crate::util::{err_in_file, line_lint_allowed, txt};

pub const NAME: &str = "re-export-foreign-names";

/// The crates that stand where `core` and `alloc` would in this stack, and
/// are named by every consumer for that reason.
///
/// A `USize` or a `Maybe` in a kit's signature is not a leak of the crate
/// underneath: it is the stack's own vocabulary, which the consumer already
/// depends on the way it depends on `core`, and re-exporting it from every
/// crate that says a count would be the same name reachable from twenty
/// places. The workspace's `no-alloc-no-std-framing` rule states it: arvo,
/// hilavitkutin and notko are the explicit replacement for `std` and `alloc`.
/// Matched by root and by the root's family, so `arvo_bits` is `arvo`'s.
pub const FOUNDATION: &[&str] = &["notko", "arvo", "hilavitkutin"];

pub struct ReExportForeignNames;

impl Lint for ReExportForeignNames {
    /// Walks `all_sources` itself, so the dispatcher hands it the crate once.
    fn per_file(&self) -> bool {
        false
    }

    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

/// What the crate as a whole knows, gathered before any file is judged.
struct Crate {
    re_exports: BTreeSet<(String, String)>,
    modules:    BTreeSet<String>,
    deps:       BTreeSet<String>,
    siblings:   BTreeSet<String>,
}

impl Crate {
    /// Whether a path root names another crate this one depends on.
    fn is_foreign(&self, root: &str) -> bool {
        if is_reserved(root)
            || is_foundation(root)
            || self.modules.contains(root)
            || self.siblings.contains(root)
        {
            return false;
        }
        self.deps.is_empty() || self.deps.contains(root)
    }

    fn re_exported(&self, root: &str, name: &str) -> bool {
        self.re_exports
            .contains(&(root.to_string(), name.to_string()))
            || self
                .re_exports
                .contains(&(root.to_string(), "*".to_string()))
    }
}

/// Whether a path root is one of the stack's foundation crates or a member of
/// one's family.
fn is_foundation(root: &str) -> bool {
    FOUNDATION.iter().any(|f| {
        root == *f
            || root
                .strip_prefix(f)
                .is_some_and(|rest| rest.starts_with('_'))
    })
}

impl CrateLint for ReExportForeignNames {
    fn check(&self, ctx: &LintContext) -> Vec<LintError> {
        if ctx.should_skip_proc_macro_source_lint() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut krate = Crate {
            re_exports: BTreeSet::new(),
            modules:    BTreeSet::new(),
            deps:       ctx.deps.iter().map(|d| as_root(d)).collect(),
            siblings:   ctx.all_crates.iter().map(|c| as_root(c)).collect(),
        };
        let trees: Vec<_> = ctx
            .all_sources
            .iter()
            .filter_map(|f| parse(&f.text).map(|t| (f, t)))
            .collect();
        for (file, tree) in &trees {
            re_exports_of(tree.root_node(), &file.text, &mut krate.re_exports);
            modules_of(tree.root_node(), &file.text, &mut krate.modules);
            if let Some(stem) = file.rel_path.file_stem().and_then(|s| s.to_str()) {
                krate.modules.insert(stem.to_string());
            }
        }

        for (file, tree) in &trees {
            let rel = file.rel_path.display().to_string();
            let imports = imports_of(tree.root_node(), &file.text);
            own_signatures(
                tree.root_node(),
                &file.text,
                &rel,
                &imports,
                &krate,
                ctx,
                &mut out,
            );
        }

        carried_surfaces(&trees, &krate, ctx, &mut out);
        out
    }
}

/// Tier one: foreign names in this crate's own public signatures.
fn own_signatures(
    root: Node,
    src: &str,
    rel: &str,
    imports: &Imports,
    krate: &Crate,
    ctx: &LintContext,
    out: &mut Vec<LintError>,
) {
    let mut seen = BTreeSet::new();
    for kind in [
        "function_item",
        "function_signature_item",
        "struct_item",
        "enum_item",
        "type_item",
        "const_item",
        "static_item",
        "trait_item",
    ] {
        for_each_kind(root, kind, &mut |item| {
            if under_cfg_test(item, src) || !reachable(item, src) {
                return;
            }
            let line = item.start_position().row + 1;
            if line_lint_allowed(src.lines().nth(line - 1).unwrap_or(""), NAME) {
                return;
            }
            let label = item
                .child_by_field_name("name")
                .map(|n| txt(n, src).to_string())
                .unwrap_or_else(|| "<unnamed>".to_string());
            let mut generics = BTreeSet::new();
            generic_names(item, src, &mut generics);
            // An item inside an impl inherits the impl's generics.
            let mut up = item.parent();
            while let Some(p) = up {
                if p.kind() == "impl_item" || p.kind() == "trait_item" {
                    generic_names(p, src, &mut generics);
                }
                up = p.parent();
            }
            let mut names = Vec::new();
            for part in signature_parts(item, src) {
                type_names(part, src, &mut names);
            }
            for named in names {
                if named.name == "Self" || generics.contains(&named.name) {
                    continue;
                }
                // Every crate the name could have come from. A qualified
                // path's root is a crate or an alias of a module in one, and a
                // bare name is whatever the file's `use` lines bound it to,
                // all of them where the file bound it more than once.
                let candidates: Vec<(String, String)> = match &named.root {
                    Some(root) => {
                        match imports.named.get(root) {
                            Some(bindings) => {
                                bindings
                                    .iter()
                                    .map(|b| (b.root.clone(), named.name.clone()))
                                    .collect()
                            },
                            None => vec![(root.clone(), named.name.clone())],
                        }
                    },
                    None => {
                        match imports.named.get(&named.name) {
                            Some(bindings) => {
                                bindings
                                    .iter()
                                    .map(|b| (b.root.clone(), b.name.clone()))
                                    .collect()
                            },
                            None => continue,
                        }
                    },
                };
                let Some((root, original)) = candidates.into_iter().find(|(root, original)| {
                    krate.is_foreign(root) && !krate.re_exported(root, original)
                }) else {
                    continue;
                };
                if !seen.insert((line, original.clone())) {
                    continue;
                }
                out.push(err_in_file(
                    ctx,
                    rel,
                    line,
                    NAME,
                    format!(
                        "`{original}` from `{root}` is in the public signature of `{label}` and \
                         is not re-exported; a consumer cannot name it without depending on \
                         `{root}`, so add `pub use {root}::...::{original}` here, under any name"
                    ),
                ));
            }
        });
    }
}

/// The nodes of an item a consumer reads a type out of.
fn signature_parts<'a>(item: Node<'a>, src: &str) -> Vec<Node<'a>> {
    let mut parts = Vec::new();
    match item.kind() {
        "function_item" | "function_signature_item" => {
            for field in ["parameters", "return_type"] {
                if let Some(n) = item.child_by_field_name(field) {
                    parts.push(n);
                }
            }
            let mut c = item.walk();
            for child in item.children(&mut c) {
                if child.kind() == "where_clause" {
                    parts.push(child);
                }
            }
        },
        "struct_item" | "enum_item" => {
            if let Some(body) = item.child_by_field_name("body") {
                public_fields(body, src, &mut parts);
            }
        },
        "type_item" => {
            if let Some(n) = item.child_by_field_name("type") {
                parts.push(n);
            }
        },
        "const_item" | "static_item" => {
            if let Some(n) = item.child_by_field_name("type") {
                parts.push(n);
            }
        },
        "trait_item" => {
            if let Some(bounds) = item.child_by_field_name("bounds") {
                parts.push(bounds);
            }
            if let Some(body) = item.child_by_field_name("body") {
                for_each_kind(body, "associated_type", &mut |assoc| {
                    if let Some(bounds) = assoc.child_by_field_name("bounds") {
                        parts.push(bounds);
                    }
                });
            }
        },
        _ => {},
    }
    parts
}

fn public_fields<'a>(body: Node<'a>, src: &str, into: &mut Vec<Node<'a>>) {
    match body.kind() {
        "field_declaration_list" => {
            let mut c = body.walk();
            for field in body.named_children(&mut c) {
                if field.kind() == "field_declaration" && is_plain_pub(field, src) {
                    if let Some(ty) = field.child_by_field_name("type") {
                        into.push(ty);
                    }
                }
            }
        },
        "ordered_field_declaration_list" => {
            let mut c = body.walk();
            for child in body.named_children(&mut c) {
                if child.kind() == "visibility_modifier" && txt(child, src).trim() == "pub" {
                    if let Some(ty) = child.next_named_sibling() {
                        into.push(ty);
                    }
                }
            }
        },
        "enum_variant_list" => {
            for_each_kind(body, "enum_variant", &mut |variant| {
                if let Some(vb) = variant.child_by_field_name("body") {
                    // Every field of a public enum's variant is public.
                    let mut c = vb.walk();
                    for field in vb.named_children(&mut c) {
                        match field.kind() {
                            "field_declaration" => {
                                if let Some(ty) = field.child_by_field_name("type") {
                                    into.push(ty);
                                }
                            },
                            "visibility_modifier"
                            | "line_comment"
                            | "block_comment"
                            | "attribute_item" => {},
                            _ => into.push(field),
                        }
                    }
                }
            });
        },
        _ => {},
    }
}

/// Whether a consumer can reach the item: it is `pub`, and where it sits in a
/// trait it is public with the trait; where it sits in a trait impl it is the
/// trait's signature and not this crate's.
fn reachable(item: Node, src: &str) -> bool {
    match item.parent().map(|p| p.kind()) {
        Some("declaration_list") => {
            let owner = item.parent().and_then(|p| p.parent());
            match owner.map(|o| o.kind()) {
                Some("trait_item") => owner.is_some_and(|o| is_plain_pub(o, src)),
                Some("impl_item") => {
                    owner.is_some_and(|o| o.child_by_field_name("trait").is_none())
                        && is_plain_pub(item, src)
                },
                _ => is_plain_pub(item, src),
            }
        },
        _ => is_plain_pub(item, src),
    }
}

/// Tier two: the surfaces of the foreign items this crate re-exports.
fn carried_surfaces(
    trees: &[(&mockspace_lint_rules::CrateSourceFile, tree_sitter::Tree)],
    krate: &Crate,
    ctx: &LintContext,
    out: &mut Vec<LintError>,
) {
    // Which foreign items are re-exported, and from which line, so the finding
    // lands on the `pub use` that made the promise.
    let mut carried: BTreeMap<(String, String), (String, usize)> = BTreeMap::new();
    for (file, tree) in trees {
        let rel = file.rel_path.display().to_string();
        for_each_kind(tree.root_node(), "use_declaration", &mut |node| {
            if !is_plain_pub(node, &file.text) {
                return;
            }
            let line = node.start_position().row + 1;
            if line_lint_allowed(file.text.lines().nth(line - 1).unwrap_or(""), NAME) {
                return;
            }
            for (_, root, original) in use_leaves(node, &file.text) {
                if original == "*" || !krate.is_foreign(&root) {
                    continue;
                }
                carried
                    .entry((root, original))
                    .or_insert((rel.clone(), line));
            }
        });
    }
    if carried.is_empty() {
        return;
    }

    let Dependencies {
        roots,
    } = match dependencies(ctx.workspace_root, ctx.crate_name) {
        Ok(d) => d,
        Err(why) => {
            // The promise cannot be checked, and a check that cannot run says
            // so rather than passing: every re-export below would otherwise
            // read as verified.
            let (rel, line) = carried.values().next().cloned().unwrap_or_default();
            out.push(err_in_file(
                ctx,
                rel,
                line,
                NAME,
                format!("the surfaces this crate re-exports could not be read: {why}"),
            ));
            return;
        },
    };

    let mut read: BTreeMap<String, crate::dep_surface::Sources> = BTreeMap::new();
    for ((root, item), (rel, line)) in &carried {
        let Some(dir) = roots.get(root) else { continue };
        let src = read.entry(root.clone()).or_insert_with(|| sources(dir));
        // A re-exported name the dependency does not declare is a module or a
        // function or a macro, none of which carries a surface this reads.
        if !src.declared.contains(item) {
            continue;
        }
        for exposure in exposures(src, item) {
            if exposure.name == *item || krate.re_exported(root, &exposure.name) {
                continue;
            }
            out.push(err_in_file(
                ctx,
                rel,
                *line,
                NAME,
                format!(
                    "`{item}` is re-exported from `{root}` and its public surface names \
                     `{}` through {}, which is not re-exported; a consumer holding a `{item}` \
                     cannot name it without depending on `{root}`, so add `pub use \
                     {root}::...::{}` beside `{item}`, under any name",
                    exposure.name, exposure.through, exposure.name
                ),
            ));
        }
    }
}
