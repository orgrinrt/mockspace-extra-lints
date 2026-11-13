//! Lint: a registry schema holds its relations in one `ref[]` field, never in a field per target.
//!
//! A relation between rows is one field typed `ref[]`, whichever namespace the other row is in. A
//! schema that also carries a field per target namespace (`bevy`, `upstream`, `canon` and the like,
//! typed as that namespace's slugs) has two places to write the same kind of link, and a link
//! written in either is read by nobody who looks in the other. Nothing in the engine refuses it, so
//! the schema in `mockspace.toml` is what this reads.
//!
//! A namespace is held to it when it declares the relation field, named by `relation` and `related`
//! by default, with the type `ref[]`. In such a namespace a field typed as a list of another
//! namespace's slugs, the namespace's own included (`bevy[]`, `tenet[]`, `ticket[]` in `ticket`), is a
//! finding, unless its name is one of the structural
//! fields, which say what the row itself is rather than what it relates to: `superseded_by`,
//! `migrated_from`, `qualified_by`, `cites`, `needs` and `part_of` by default, replaced by the
//! `structural` list where a project names one.
//!
//! A namespace that declares no relation field is not read, since it has no one field to fold into.
//! A field that is a single slug (`kind`, `subject`, `tier`) is an attribute of the row and is not
//! read either.
//!
//! The lint reads `mockspace.toml` at the repository root. A repository with none, or a schema it
//! cannot parse, gets no finding.

use std::collections::{BTreeSet, HashMap};
use std::fs;

use mockspace_lint_rules::{Lint, LintError, RepoContext, RepoLint, Severity};

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "one-relation-field";

const RELATION_KEY: &str = "relation";
const STRUCTURAL_KEY: &str = "structural";

const DEFAULT_RELATION: &str = "related";
const DEFAULT_STRUCTURAL: &[&str] =
    &["superseded_by", "migrated_from", "qualified_by", "cites", "needs", "part_of"];

/// The namespaces the engine adds to a schema of its own.
const BUILTIN: &[&str] = &["vocab", "reference"];

const PER_TARGET: &str = "per-target";

/// The configured lint.
pub struct OneRelationField {
    relation: String,
    structural: Vec<String>,
}

impl Default for OneRelationField {
    fn default() -> Self {
        Self {
            relation: DEFAULT_RELATION.to_string(),
            structural: DEFAULT_STRUCTURAL.iter().map(|s| (*s).to_string()).collect(),
        }
    }
}

impl Lint for OneRelationField {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }

    fn finding_kinds(&self) -> &[&str] {
        &[PER_TARGET]
    }

    fn config_keys(&self) -> &[&str] {
        &[RELATION_KEY, STRUCTURAL_KEY]
    }

    fn configure(&mut self, params: &HashMap<String, String>) {
        if let Some(v) = params.get(RELATION_KEY) {
            self.relation = v.trim().to_string();
        }
        if let Some(v) = params.get(STRUCTURAL_KEY) {
            self.structural =
                v.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect();
        }
    }
}

impl RepoLint for OneRelationField {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        let Ok(text) = fs::read_to_string(ctx.repo_root.join("mockspace.toml")) else {
            return Vec::new();
        };
        let Ok(doc) = text.parse::<toml::Table>() else {
            return Vec::new();
        };
        let Some(spaces) = doc
            .get("registry")
            .and_then(|r| r.get("namespace"))
            .and_then(toml::Value::as_array)
        else {
            return Vec::new();
        };
        // the engine adds `vocab` and `reference` to every schema, so a list of their slugs is a target too
        let mut names: BTreeSet<&str> = BUILTIN.iter().copied().collect();
        names.extend(spaces.iter().filter_map(|n| n.get("key").and_then(toml::Value::as_str)));
        let mut out = Vec::new();
        for space in spaces {
            let Some(key) = space.get("key").and_then(toml::Value::as_str) else {
                continue;
            };
            let fields: Vec<(&str, &str)> = space
                .get("field")
                .and_then(toml::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|f| {
                    Some((
                        f.get("name")?.as_str()?,
                        f.get("type")?.as_str()?,
                    ))
                })
                .collect();
            if !fields.iter().any(|(n, t)| *n == self.relation && *t == "ref[]") {
                continue;
            }
            for (name, ty) in &fields {
                let Some(target) = ty.strip_suffix("[]") else {
                    continue;
                };
                if !names.contains(target) || self.structural.iter().any(|s| s == name) {
                    continue;
                }
                let mut e = LintError::error(
                    "mockspace.toml".to_string(),
                    0,
                    NAME,
                    format!(
                        "namespace `{key}` field `{name}` lists `{target}` beside its `{}` field; write the link in `{}` as `{target}::<slug>`, or name `{name}` in `structural` if it says what the row is rather than what it relates to",
                        self.relation, self.relation
                    ),
                );
                e.finding_kind = Some(PER_TARGET);
                e.path = Some("mockspace.toml".to_string());
                out.push(e);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests;
