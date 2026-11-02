//! Lint: a `[lints.*]` table of `mockspace.toml` names no `crates` list.
//!
//! A lint binds every crate. A list of the crates a lint is let loose on inverts that: a crate made tomorrow
//! is outside every gate that lists its crates, and the list only ever grows by somebody remembering to add
//! to it. What a lint cannot yet hold is named the other way round, in `exempt_crates`, which is short,
//! temporary and tied to the work that ends it.
//!
//! A table that holds the key `crates` is refused, at the line the key stands on. A lint set `off` at the
//! commit, the build and the push is left alone: it runs nowhere, so whatever its table lists binds nothing,
//! and a lint a project turns off because it was written for another kind of crate is no exemption. A lint
//! with only some of the three gates `off` is not left alone, since it still runs.
//!
//! The table is read as text, a table header and its keys, so a `crates` key inside a nested table such as
//! `[lints.name.rule.x]` is not read as the lint's own.

use mockspace_lint_rules::{Lint, LintError, RepoContext, RepoLint, Severity};

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "a-lint-admits-no-crate-list";

/// The key that admits crates.
const ADMIT: &str = "crates";

/// The gates a lint is set at.
const GATES: &[&str] = &["commit", "build", "push"];

pub struct ALintAdmitsNoCrateList;

impl Lint for ALintAdmitsNoCrateList {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

impl RepoLint for ALintAdmitsNoCrateList {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        let text = std::fs::read_to_string(ctx.repo_root.join("mockspace.toml")).unwrap_or_default();
        check(&text)
    }
}

/// One `[lints.<name>]` table as read: its name, the gates it sets and the line its `crates` key is on.
#[derive(Default)]
struct Table {
    name:  String,
    gates: Vec<(String, String)>,
    admit: Option<usize>,
}

/// The tables of the text that name a lint, in order.
fn tables(text: &str) -> Vec<Table> {
    let mut out: Vec<Table> = Vec::new();
    let mut current: Option<Table> = None;
    for (i, raw) in text.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or_default().trim();
        if line.starts_with('[') && !line.starts_with("[\"") {
            out.extend(current.take());
            let name = line.trim_matches(|c| c == '[' || c == ']').trim();
            if let Some(lint) = name.strip_prefix("lints.").filter(|n| !n.contains('.')) {
                current = Some(Table { name: lint.to_owned(), ..Table::default() });
            }
            continue;
        }
        let Some(table) = current.as_mut() else {
            continue;
        };
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key == ADMIT {
            table.admit = Some(i + 1);
        } else if GATES.contains(&key) {
            let level = value.trim().trim_matches('"').to_owned();
            table.gates.push((key.to_owned(), level));
        }
    }
    out.extend(current);
    out
}

/// Whether the table sets every gate and every one of them `off`.
fn is_off(table: &Table) -> bool {
    GATES.iter().all(|g| table.gates.iter().any(|(k, v)| k == g && v == "off"))
}

fn check(text: &str) -> Vec<LintError> {
    tables(text)
        .iter()
        .filter(|t| !is_off(t))
        .filter_map(|t| {
            let line = t.admit?;
            let mut e = LintError::error(
                t.name.clone(),
                line,
                NAME,
                format!(
                    "`[lints.{}]` names a `crates` list: a lint binds every crate, so name the ones it cannot hold \
                     yet in `exempt_crates` instead, each for a while and for a ticket",
                    t.name
                ),
            );
            e.path = Some("mockspace.toml".to_owned());
            Some(e)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADMITTING: &str = "[lints.file-size]\ncommit = \"error\"\nbuild = \"error\"\npush = \"error\"\ncrates = [\n    \"a\",\n]\n";

    #[test]
    fn a_table_with_a_crates_list_is_refused_at_the_line_of_the_key() {
        let f = check(ADMITTING);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!((f[0].crate_name.as_str(), f[0].line), ("file-size", 5));
        assert_eq!(f[0].path.as_deref(), Some("mockspace.toml"));
    }

    #[test]
    fn the_same_table_with_exempt_crates_instead_passes() {
        let text = ADMITTING.replace("crates = [", "exempt_crates = [");
        assert!(check(&text).is_empty());
    }

    #[test]
    fn a_table_with_no_list_passes() {
        assert!(check("[lints.file-size]\ncommit = \"error\"\nmax_lines = \"500\"\n").is_empty());
    }

    #[test]
    fn a_lint_off_at_every_gate_is_left_alone_and_one_off_at_two_is_not() {
        let off = "[lints.no-std]\ncommit = \"off\"\nbuild = \"off\"\npush = \"off\"\ncrates = [\"a\"]\n";
        assert!(check(off).is_empty());
        let two = "[lints.no-std]\ncommit = \"off\"\nbuild = \"off\"\npush = \"error\"\ncrates = [\"a\"]\n";
        assert_eq!(check(two).len(), 1);
        let none = "[lints.no-std]\ncrates = [\"a\"]\n";
        assert_eq!(check(none).len(), 1, "a table that sets no gate is not off");
    }

    #[test]
    fn each_admitting_table_is_a_finding_and_a_neighbour_without_one_is_not() {
        let text = format!("{ADMITTING}\n[lints.export-count]\ncommit = \"error\"\n\n[lints.single-source]\ncrates = [\"b\"]\n");
        let f = check(&text);
        let names: Vec<_> = f.iter().map(|e| e.crate_name.as_str()).collect();
        assert_eq!(names, ["file-size", "single-source"]);
    }

    #[test]
    fn a_crates_key_of_another_kind_of_table_is_not_a_lints_own() {
        let text = "[lints.x.rule.y]\ncrates = [\"a\"]\n\n[workspace]\ncrates = [\"b\"]\n\n[lints.z]\nexempt_crates = [\"c\"]\n";
        assert!(check(text).is_empty());
    }

    #[test]
    fn a_comment_naming_the_key_is_not_the_key() {
        assert!(check("[lints.a]\n# crates = [\"a\"]\ncommit = \"error\"\n").is_empty());
    }
}
