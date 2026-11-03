//! Lint: a `[lints.*]` table of `mockspace.toml` names no `crates` list.
//!
//! A lint binds every crate. A list of the crates a lint is let loose on inverts that: a crate made tomorrow
//! is outside every gate that lists its crates, and the list only ever grows by somebody remembering to add
//! to it. What a lint cannot yet hold is named the other way round, in `exempt_crates`, which is short,
//! temporary and tied to the work that ends it.
//!
//! A table that holds the key `crates` is refused, at the line the key stands on. The file is read as TOML,
//! so a lint written as an inline table, with a dotted key or with a quoted key is read as the same table
//! the engine reads. A `crates` key inside a nested table such as `[lints.name.rule.x]` is not the lint's own.
//!
//! A lint that is off at the commit, the build and the push is left alone: it runs nowhere, so whatever its
//! table lists binds nothing, and a lint a project turns off because it was written for another kind of crate
//! is no exemption. Each gate is worked out as the engine works it out: the gate's own key, else the
//! `severity` preset, else off when some other gate is set, and a name is read without regard to case. A lint
//! with a gate that still runs is not left alone.

use std::path::PathBuf;

use mockspace_lint_rules::{
    Level,
    Lint,
    LintError,
    RepoContext,
    RepoLint,
    Severity,
    parse_severity,
};
use toml_edit::{Item, TableLike};

use crate::manifest_toml::{line_of, parse};
use crate::repo_files::WORKSPACE;

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "a-lint-admits-no-crate-list";

/// The name of the config file.
const CONFIG: &str = "mockspace.toml";

/// The key that admits crates.
const ADMIT: &str = "crates";

/// The configured lint.
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
        let path = config_path(ctx);
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let rel = path.strip_prefix(ctx.repo_root).unwrap_or(&path);
        check_file(&text, &rel.to_string_lossy().replace('\\', "/"))
    }
}

/// The config the engine reads: `mockspace.toml` in the mock directory when it is there, else the one at
/// the repository root.
fn config_path(ctx: &RepoContext) -> PathBuf {
    let in_mock = ctx.mock_dir.join(CONFIG);
    if in_mock.is_file() { in_mock } else { ctx.repo_root.join(CONFIG) }
}

/// The level a gate has: its own key, else what the `severity` preset says, else off when another gate
/// is set. A lint that sets none of them has the lint's own default, which is not known here, so it runs.
fn gates(table: &dyn TableLike) -> Option<[Level; 3]> {
    let level = |key: &str| {
        table
            .get(key)
            .and_then(Item::as_str)
            .and_then(Level::from_str_name)
    };
    let own = ["commit", "build", "push"].map(level);
    let preset = table
        .get("severity")
        .and_then(Item::as_str)
        .and_then(parse_severity);
    let base = match preset {
        Some(preset) => preset,
        None if own.iter().any(Option::is_some) => Severity::OFF,
        None => return None,
    };
    Some([
        own[0].unwrap_or(base.on_commit),
        own[1].unwrap_or(base.on_build),
        own[2].unwrap_or(base.on_push),
    ])
}

/// Whether the table switches the lint off at every gate.
fn is_off(table: &dyn TableLike) -> bool {
    gates(table).is_some_and(|levels| levels.iter().all(|level| *level == Level::Pass))
}

/// The findings in the text of the config at `rel`, a path from the repository root.
fn check_file(text: &str, rel: &str) -> Vec<LintError> {
    let Some(doc) = parse(text) else {
        return Vec::new();
    };
    let Some(lints) = doc.as_table().get("lints").and_then(Item::as_table_like) else {
        return Vec::new();
    };
    lints
        .iter()
        .filter_map(|(name, entry)| {
            let table = entry.as_table_like().filter(|table| !is_off(*table))?;
            let (key, _) = table.get_key_value(ADMIT)?;
            let mut e = LintError::error(
                WORKSPACE.to_owned(),
                line_of(text, key.span()),
                NAME,
                format!(
                    "`[lints.{name}]` names a `crates` list: a lint binds every crate, so name the ones it cannot hold \
                     yet in `exempt_crates` instead, each for a while and for a ticket"
                ),
            );
            e.path = Some(rel.to_owned());
            Some(e)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use mockspace_lint_rules::RegistryView;

    use super::*;

    const ADMITTING: &str = "[lints.file-size]\ncommit = \"error\"\nbuild = \"error\"\npush = \"error\"\ncrates = [\n    \"a\",\n]\n";

    fn check(text: &str) -> Vec<LintError> {
        check_file(text, CONFIG)
    }

    fn lines(text: &str) -> Vec<usize> {
        check(text).iter().map(|e| e.line).collect()
    }

    #[test]
    fn a_table_with_a_crates_list_is_refused_at_the_line_of_the_key() {
        let f = check(ADMITTING);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].line, 5);
        assert_eq!(f[0].path.as_deref(), Some("mockspace.toml"));
        assert!(
            f[0].message.contains("`[lints.file-size]`"),
            "{}",
            f[0].message
        );
    }

    #[test]
    fn the_finding_belongs_to_the_workspace_and_not_to_a_crate_named_for_the_lint() {
        // Mutation: the lint's name as the crate, which an `exempt_crates` list would then match.
        let f = check("[lints.\"file-size\"]\ncrates = [\"a\"]\n");
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].crate_name, WORKSPACE);
        assert!(
            f[0].message.contains("`[lints.file-size]`"),
            "quotes kept: {}",
            f[0].message
        );
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
    fn every_spelling_of_the_table_the_engine_reads_is_read() {
        // The engine takes each of these as `crates` under the lint `file-size`.
        // Mutation: the file read line by line as text, which sees only the header form.
        assert_eq!(
            lines("[lints]\nfile-size = { crates = [\"a\"] }\n"),
            [2],
            "inline table"
        );
        assert_eq!(
            lines("[lints]\nfile-size.crates = [\"a\"]\n"),
            [2],
            "dotted key"
        );
        assert_eq!(
            lines("[lints.file-size]\n\"crates\" = [\"a\"]\n"),
            [2],
            "quoted key"
        );
        assert_eq!(
            lines("[lints]\n\"file-size\" = { \"crates\" = [\"a\"] }\n"),
            [2],
            "both quoted"
        );
        assert_eq!(
            lines("[lints]\nfile-size = {\n    commit = \"error\",\n    crates = [\"a\"],\n}\n"),
            [4],
            "inline table across lines"
        );
        assert_eq!(
            lines("lints.file-size.crates = [\"a\"]\n"),
            [1],
            "dotted from the root"
        );
    }

    #[test]
    fn a_lint_off_at_every_gate_is_left_alone_and_one_off_at_two_is_not() {
        let off =
            "[lints.no-std]\ncommit = \"off\"\nbuild = \"off\"\npush = \"off\"\ncrates = [\"a\"]\n";
        assert!(check(off).is_empty());
        let two = "[lints.no-std]\ncommit = \"off\"\nbuild = \"off\"\npush = \"error\"\ncrates = [\"a\"]\n";
        assert_eq!(check(two).len(), 1);
        let none = "[lints.no-std]\ncrates = [\"a\"]\n";
        assert_eq!(check(none).len(), 1, "a table that sets no gate is not off");
    }

    #[test]
    fn a_severity_of_off_switches_the_lint_off_as_the_engine_reads_it() {
        // Mutation: only the three gate keys read, so `severity` is no switch.
        assert!(check("[lints.no-std]\nseverity = \"off\"\ncrates = [\"a\"]\n").is_empty());
        assert!(check("[lints]\nno-std = { severity = \"off\", crates = [\"a\"] }\n").is_empty());
        // A gate set on top of it runs, so the lint is not off.
        let on = "[lints.no-std]\nseverity = \"off\"\ncommit = \"error\"\ncrates = [\"a\"]\n";
        assert_eq!(check(on).len(), 1, "one gate runs");
        // A severity that runs somewhere, with one gate turned off, still runs at the others.
        let some = "[lints.no-std]\nseverity = \"error\"\ncommit = \"off\"\ncrates = [\"a\"]\n";
        assert_eq!(check(some).len(), 1, "build and push still run");
    }

    #[test]
    fn a_gate_set_alone_leaves_the_others_off_as_the_engine_reads_it() {
        // `commit = "off"` alone leaves build and push at the off the engine starts a gated lint from.
        assert!(check("[lints.no-std]\ncommit = \"off\"\ncrates = [\"a\"]\n").is_empty());
        assert_eq!(
            check("[lints.no-std]\ncommit = \"error\"\ncrates = [\"a\"]\n").len(),
            1
        );
    }

    #[test]
    fn the_names_are_read_without_regard_to_case() {
        // Mutation: the value compared as written.
        assert!(check("[lints.no-std]\nseverity = \"OFF\"\ncrates = [\"a\"]\n").is_empty());
        assert!(check("[lints.no-std]\ncommit = \"Off\"\nbuild = \" off \"\npush = \"OFF\"\ncrates = [\"a\"]\n").is_empty());
    }

    #[test]
    fn each_admitting_table_is_a_finding_and_a_neighbour_without_one_is_not() {
        let text = format!(
            "{ADMITTING}\n[lints.export-count]\ncommit = \"error\"\n\n[lints.single-source]\ncrates = [\"b\"]\n"
        );
        let f = check(&text);
        let names: Vec<_> = f
            .iter()
            .map(|e| e.message.split('`').nth(1).unwrap().to_owned())
            .collect();
        assert_eq!(names, ["[lints.file-size]", "[lints.single-source]"]);
    }

    #[test]
    fn a_crates_key_of_another_kind_of_table_is_not_a_lints_own() {
        let text = "[lints.x.rule.y]\ncrates = [\"a\"]\n\n[workspace]\ncrates = [\"b\"]\n\n[lints.z]\nexempt_crates = [\"c\"]\n";
        assert!(check(text).is_empty());
    }

    #[test]
    fn a_lint_given_as_a_preset_name_has_no_list_to_refuse() {
        assert!(check("[lints]\nfile-size = \"error\"\nno-std = \"off\"\n").is_empty());
    }

    #[test]
    fn a_comment_naming_the_key_is_not_the_key() {
        assert!(check("[lints.a]\n# crates = [\"a\"]\ncommit = \"error\"\n").is_empty());
    }

    #[test]
    fn a_file_that_is_not_toml_has_nothing_to_report() {
        assert!(check("[lints.a\ncrates = [").is_empty());
    }

    /// The lint run over a repository whose mock directory is `<root>/mock`.
    fn run(root: &std::path::Path) -> Vec<LintError> {
        let crates = BTreeSet::new();
        let reg = RegistryView::new(BTreeMap::new(), BTreeMap::new());
        let ctx = RepoContext {
            mock_dir:    &root.join("mock"),
            repo_root:   root,
            all_crates:  &crates,
            src_dirs:    &[],
            invocation:  None,
            canon_paths: &[],
            open_panels: &[],
            registry:    &reg,
            verdicts:    None,
        };
        ALintAdmitsNoCrateList.check_repo(&ctx)
    }

    fn write(root: &std::path::Path, rel: &str, text: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn the_lint_reads_the_mockspace_toml_at_the_root_of_the_repository() {
        // Mutation: `check_repo` reading some other path or no file.
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "mockspace.toml", ADMITTING);
        let f = run(dir.path());
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].line, 5);
        assert_eq!(f[0].path.as_deref(), Some("mockspace.toml"));
        let empty = tempfile::tempdir().unwrap();
        assert!(run(empty.path()).is_empty());
    }

    #[test]
    fn the_lint_reads_the_mockspace_toml_in_the_mock_directory_when_there_is_one() {
        // The engine reads the config in the mock directory first and falls back to the root.
        // Mutation: only the root read.
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "mock/mockspace.toml", ADMITTING);
        let f = run(dir.path());
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].path.as_deref(), Some("mock/mockspace.toml"));
    }

    #[test]
    fn a_config_in_the_mock_directory_is_the_one_read_and_the_one_at_the_root_is_not() {
        // Mutation: the root preferred, or both read.
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "mockspace.toml", ADMITTING);
        write(
            dir.path(),
            "mock/mockspace.toml",
            "[lints.file-size]\ncommit = \"error\"\n",
        );
        assert!(run(dir.path()).is_empty());
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "mock/mockspace.toml",
            "[lints.file-size]\ncommit = \"error\"\n",
        );
        write(dir.path(), "mockspace.toml", ADMITTING);
        assert!(run(dir.path()).is_empty(), "whichever is written first");
    }
}
