//! Lint: a git dependency on a first-party repository tracks a branch, never a fixed commit.
//!
//! A dependency spelled with `rev` names one point in a repository's history, and it stays there
//! while the repository moves on. For a first-party repository that is a stale hash the moment the
//! next fix lands on its `dev`, and a hash that does not exist at all once the repository is moved
//! to a new home, which keeps its branch names and loses the rest. The build is green against code
//! that no longer exists at the head, or it cannot fetch. The first-party repositories are one body
//! of work, so a dependency on one tracks the head of the branch the work lands on, and the lockfile
//! records the commit that head was when it was last resolved.
//!
//! A repository is first-party when its url has the host `github.com` and the `orgrinrt` or the
//! `hiisi-digital` account, in the https, ssh or scp spelling. The files are read as TOML, so a
//! table is found as an inline table (on one line or across several), as a table of its own
//! (`[dependencies.name]`) and with dotted keys (`name.git = "..."`), wherever it sits: a
//! dependency, a `[patch]` entry or a `[lint-crates]` entry. A `rev` beside a `git` url of such a
//! repository is the finding, at the line of the `rev`. A `tag` is let through, since a tag is a
//! name and a rehouse does not break a name the way it breaks a hash. A third-party repository is
//! not read, and neither is a lockfile, which records a commit by design.
//!
//! The record directories (`design_rounds` and `research` under the mock directory) are not read,
//! nor is anything under a `research/sketches` directory wherever it sits: a sketch pins the
//! revision it measured, and the pin is the evidence of what it measured.

use std::fs;
use std::ops::Range;

use mockspace_lint_rules::{Lint, LintError, RepoContext, RepoLint, Severity};
use toml_edit::{Item, TableLike};

use crate::manifest_toml::{line_of, parse};
use crate::repo_files::{WORKSPACE, paths, record_dirs};

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "no-first-party-rev-pin";

/// The files whose tables are read.
const MANIFESTS: &[&str] = &["Cargo.toml", "mockspace.toml"];

/// The GitHub host the first-party repositories sit on, lowercase.
const HOST: &str = "github.com";

/// The GitHub accounts the first-party repositories sit under, lowercase.
const ACCOUNTS: &[&str] = &["orgrinrt", "hiisi-digital"];

/// The key that fixes a dependency to one point in history.
const FIXING: &str = "rev";

/// The path segment pair a sketch directory is, wherever it sits.
const SKETCHES: &str = "research/sketches";

/// The configured lint.
#[derive(Default)]
pub struct NoFirstPartyRevPin;

impl Lint for NoFirstPartyRevPin {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

impl RepoLint for NoFirstPartyRevPin {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        let skip = record_dirs(ctx.repo_root, ctx.mock_dir);
        let found = paths(ctx.repo_root, &skip, |name, rel| {
            MANIFESTS.contains(&name) && !in_sketches(rel)
        });
        let files: Vec<(String, String)> = found
            .into_iter()
            .filter_map(|rel| {
                fs::read_to_string(ctx.repo_root.join(&rel))
                    .ok()
                    .map(|t| (rel, t))
            })
            .collect();
        // The verdict follows from the text of the manifests alone, so it replays when none moved.
        let mut parts: Vec<&[u8]> = Vec::new();
        for (rel, text) in &files {
            parts.push(rel.as_bytes());
            parts.push(text.as_bytes());
        }
        ctx.remembered(NAME, &parts, || judge(&files))
    }
}

/// Whether a path from the repository root sits under a sketch directory.
fn in_sketches(rel: &str) -> bool {
    let padded = format!("/{rel}");
    padded.contains(&format!("/{SKETCHES}/"))
}

/// A `rev` on a first-party dependency: the url, and where the key stands.
struct Pin {
    url:  String,
    span: Option<Range<usize>>,
}

fn judge(files: &[(String, String)]) -> Vec<LintError> {
    let mut out = Vec::new();
    for (rel, text) in files {
        let Some(doc) = parse(text) else {
            continue;
        };
        let mut pins = Vec::new();
        walk_table(doc.as_table(), &mut pins);
        pins.sort_by_key(|pin| pin.span.as_ref().map(|s| s.start));
        for pin in pins {
            let mut e = LintError::error(
                WORKSPACE.to_string(),
                line_of(text, pin.span),
                NAME,
                format!(
                    "a dependency on the first-party repository `{}` names a fixed `{FIXING}`; it tracks the head of a branch instead (`branch = \"dev\"`), and the lockfile records the commit",
                    pin.url
                ),
            );
            e.path = Some(rel.clone());
            out.push(e);
        }
    }
    out
}

/// Every table below this one, this one included, that has a first-party `git` url and a `rev`.
fn walk_table(table: &dyn TableLike, found: &mut Vec<Pin>) {
    found.extend(pin_of(table));
    for (_, child) in table.iter() {
        walk_item(child, found);
    }
}

fn walk_item(item: &Item, found: &mut Vec<Pin>) {
    if let Some(table) = item.as_table_like() {
        walk_table(table, found);
    } else if let Some(tables) = item.as_array_of_tables() {
        tables.iter().for_each(|table| walk_table(table, found));
    } else if let Some(array) = item.as_array() {
        array
            .iter()
            .filter_map(|value| value.as_inline_table())
            .for_each(|table| walk_table(table, found));
    }
}

/// The pin a table makes, when it has a first-party `git` url and a `rev`.
fn pin_of(table: &dyn TableLike) -> Option<Pin> {
    let url = table.get("git")?.as_str()?;
    if !first_party(url) {
        return None;
    }
    let (key, _) = table.get_key_value(FIXING)?;
    Some(Pin {
        url:  url.to_owned(),
        span: key.span(),
    })
}

/// Whether a git url names a repository of one of the first-party accounts on GitHub: the host is
/// `github.com` exactly, and the account is the first segment of the path, in the url spelling
/// (`https://`, `ssh://`, with or without a user, a password or a port) and the scp spelling
/// (`git@github.com:account/repo`).
fn first_party(url: &str) -> bool {
    let rest = url
        .trim()
        .split_once("://")
        .map_or(url.trim(), |(_, rest)| rest);
    let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host_part = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let (host, account) = match host_part.split_once(':') {
        // After the colon is a port when it is all digits, and the account in the scp spelling.
        Some((host, tail)) if !tail.is_empty() && !tail.bytes().all(|b| b.is_ascii_digit()) => {
            (host, tail)
        },
        Some((host, _)) => (host, path.split('/').next().unwrap_or("")),
        None => (host_part, path.split('/').next().unwrap_or("")),
    };
    host.eq_ignore_ascii_case(HOST) && ACCOUNTS.contains(&account.to_ascii_lowercase().as_str())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;

    use mockspace_lint_rules::RegistryView;

    use super::*;
    use crate::test_support::Kept;

    fn write(root: &Path, rel: &str, text: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn check_with(root: &Path, store: Option<&Kept>) -> Vec<LintError> {
        let crates = BTreeSet::from(["alpha".to_string()]);
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
            verdicts:    store.map(|s| s as &dyn mockspace_lint_rules::verdict::VerdictCache),
        };
        NoFirstPartyRevPin.check_repo(&ctx)
    }

    fn check(root: &Path) -> Vec<LintError> {
        check_with(root, None)
    }

    fn at(rel: &str, body: &str) -> Vec<LintError> {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), rel, body);
        check(d.path())
    }

    fn manifest(body: &str) -> Vec<LintError> {
        at("mock/crates/alpha/Cargo.toml", body)
    }

    fn lines(found: &[LintError]) -> Vec<usize> {
        found.iter().map(|e| e.line).collect()
    }

    #[test]
    fn a_rev_on_a_first_party_https_dependency_is_refused_at_the_line_of_the_rev() {
        let found = manifest(
            "[dependencies]\ntomlfuse = { git = \"https://github.com/orgrinrt/tomlfuse\", rev = \"1773dfb\" }\n",
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].line, 2);
        assert_eq!(
            found[0].path.as_deref(),
            Some("mock/crates/alpha/Cargo.toml")
        );
        assert!(found[0].message.contains("`rev`"), "{found:?}");
        assert_eq!(found[0].crate_name, WORKSPACE);
    }

    #[test]
    fn a_rev_on_a_first_party_ssh_dependency_is_refused() {
        let found = manifest(
            "[dependencies]\nl = { git = \"ssh://git@github.com/orgrinrt/letkajenkka.git\", rev = \"52a6117\" }\n",
        );
        assert_eq!(found.len(), 1, "{found:?}");
    }

    #[test]
    fn a_rev_on_a_dependency_of_the_other_first_party_account_is_refused() {
        let found = manifest(
            "[dependencies]\nm = { git = \"git@github.com:hiisi-digital/mockspace.git\", rev = \"0123abc\" }\n",
        );
        assert_eq!(found.len(), 1, "{found:?}");
    }

    #[test]
    fn a_tag_is_let_through_since_a_name_survives_a_rehouse() {
        // Mutation: `tag` among the keys that fix a dependency.
        let found = manifest(
            "[dependencies]\nm = { git = \"git@github.com:hiisi-digital/mockspace.git\", tag = \"v0.1.0\" }\n",
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_dependency_with_both_a_rev_and_a_tag_is_one_finding_for_the_rev() {
        let found = manifest(
            "[dependencies]\nm = { git = \"https://github.com/orgrinrt/m\", tag = \"v1\", rev = \"0123abc\" }\n",
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].message.contains("`rev`"), "{found:?}");
    }

    #[test]
    fn a_table_with_a_header_of_its_own_is_read_at_the_line_of_the_rev() {
        let found = manifest(
            "[dependencies.tomlfuse]\ngit = \"https://github.com/orgrinrt/tomlfuse\"\nrev = \"1773dfb\"\n",
        );
        assert_eq!(lines(&found), [3], "{found:?}");
    }

    #[test]
    fn a_table_written_with_dotted_keys_is_read() {
        // Mutation: the file read line by line, which sees an inline table and a header and no more.
        let found = manifest(
            "[dependencies]\ntomlfuse.git = \"https://github.com/orgrinrt/tomlfuse\"\ntomlfuse.rev = \"1773dfb\"\n",
        );
        assert_eq!(lines(&found), [3], "{found:?}");
        let root = at(
            "mock/crates/alpha/Cargo.toml",
            "dependencies.tomlfuse.git = \"https://github.com/orgrinrt/tomlfuse\"\ndependencies.tomlfuse.rev = \"1\"\n",
        );
        assert_eq!(lines(&root), [2], "{root:?}");
    }

    #[test]
    fn an_inline_table_across_lines_is_read() {
        let found = manifest(
            "[dependencies]\ntomlfuse = {\n    git = \"https://github.com/orgrinrt/tomlfuse\",\n    rev = \"1773dfb\", # the old one\n}\n",
        );
        assert_eq!(lines(&found), [4], "{found:?}");
    }

    #[test]
    fn a_patch_entry_is_read() {
        // The readme says a `[patch]` entry is read, so a test says it is.
        let found = manifest(
            "[patch.crates-io]\nm = { git = \"https://github.com/orgrinrt/m\", rev = \"1\" }\n\n[patch.\"https://github.com/orgrinrt/x\"]\nx = { git = \"https://github.com/orgrinrt/x\", rev = \"2\" }\n",
        );
        assert_eq!(lines(&found), [2, 5], "{found:?}");
    }

    #[test]
    fn a_table_in_an_array_of_tables_is_read() {
        // Mutation: the entries of an array of tables not walked.
        let found = at(
            "mockspace.toml",
            "[[extra]]\ngit = \"https://github.com/orgrinrt/x\"\nrev = \"1\"\n\n[[extra]]\ngit = \"https://github.com/orgrinrt/y\"\nrev = \"2\"\n",
        );
        assert_eq!(lines(&found), [3, 7], "{found:?}");
        let inline = at(
            "mockspace.toml",
            "extra = [{ git = \"https://github.com/orgrinrt/x\", rev = \"1\" }]\n",
        );
        assert_eq!(lines(&inline), [1], "an inline table in an array");
    }

    #[test]
    fn findings_follow_the_order_written_when_a_table_is_opened_again_later() {
        // Mutation: the findings left in the order the document tree is walked, which finishes a table
        // before it goes on to the next.
        let found = manifest(
            "[dependencies.a]\nx = { git = \"https://github.com/orgrinrt/x\", rev = \"1\" }\n\n[dependencies.b]\ny = { git = \"https://github.com/orgrinrt/y\", rev = \"2\" }\n\n[dependencies.a.z]\ngit = \"https://github.com/orgrinrt/z\"\nrev = \"3\"\n",
        );
        assert_eq!(lines(&found), [2, 5, 9], "{found:?}");
    }

    #[test]
    fn a_lint_crates_entry_in_mockspace_toml_is_read() {
        let found = at(
            "mockspace.toml",
            "[lint-crates]\nask = { git = \"ssh://git@github.com/orgrinrt/muisti.git\", rev = \"abc1234\" }\n",
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].path.as_deref(), Some("mockspace.toml"));
    }

    #[test]
    fn a_branch_is_let_through() {
        let found = manifest(
            "[dependencies]\ntomlfuse = { git = \"https://github.com/orgrinrt/tomlfuse\", branch = \"dev\" }\n",
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_third_party_rev_is_let_through() {
        let found = manifest(
            "[dependencies]\nparry = { git = \"https://github.com/dimforge/parry\", rev = \"0123abc\" }\n",
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn the_host_is_github_com_and_no_host_that_contains_it() {
        // Mutation: `github.com` found anywhere in the url.
        for url in [
            "https://notgithub.com/orgrinrt/x",
            "https://github.com.example.org/orgrinrt/x",
            "https://example.org/github.com/orgrinrt/x",
            "https://gitlab.com/orgrinrt/x",
            "git@notgithub.com:orgrinrt/x.git",
        ] {
            assert!(!first_party(url), "{url}");
        }
    }

    #[test]
    fn the_account_is_the_first_segment_of_the_path() {
        assert!(!first_party("https://github.com/someone/orgrinrt"));
        assert!(!first_party("https://github.com/orgrinrt-fork/x"));
        assert!(!first_party("git@github.com:someone/orgrinrt.git"));
    }

    #[test]
    fn every_spelling_of_a_first_party_url_is_first_party() {
        for url in [
            "https://github.com/orgrinrt/x",
            "https://github.com/orgrinrt/x.git",
            "ssh://git@github.com/orgrinrt/x.git",
            "ssh://git@github.com:22/hiisi-digital/x.git",
            "git@github.com:orgrinrt/x.git",
            "https://user:token@github.com/orgrinrt/x",
            "https://github.com:443/orgrinrt/x",
            "https://GitHub.com/OrgRinrt/x",
            "github.com/orgrinrt/x",
        ] {
            assert!(first_party(url), "{url}");
        }
    }

    #[test]
    fn a_record_directory_keeps_its_pin() {
        // The design rounds and the research under the mock directory are the record of earlier work.
        // Mutation: the record directories walked like the rest.
        let body = "[dependencies]\ntomlfuse = { git = \"https://github.com/orgrinrt/tomlfuse\", rev = \"1773dfb\" }\n";
        assert!(at("mock/research/notes/x/Cargo.toml", body).is_empty());
        assert!(at("mock/design_rounds/202610010000_x/Cargo.toml", body).is_empty());
    }

    #[test]
    fn a_sketch_keeps_its_pin_wherever_it_sits_and_other_research_elsewhere_does_not() {
        // The two arms are told apart: a sketch under the mock directory is skipped as a record, and a
        // sketch elsewhere only by its own rule. Mutation: the sketch exemption removed.
        let body = "[dependencies]\ntomlfuse = { git = \"https://github.com/orgrinrt/tomlfuse\", rev = \"1773dfb\" }\n";
        assert!(at("mock/research/sketches/tomlfuse/Cargo.toml", body).is_empty());
        assert!(at("elsewhere/research/sketches/x/Cargo.toml", body).is_empty());
        assert_eq!(
            at("elsewhere/research/notes/x/Cargo.toml", body).len(),
            1,
            "only sketches are let through"
        );
    }

    #[test]
    fn a_path_dependency_and_a_registry_version_are_let_through() {
        let found = manifest(
            "[dependencies]\na = { path = \"../a\" }\nb = { version = \"=0.1.0\" }\nc = \"1\"\n",
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_comment_that_looks_like_a_pin_is_not_one() {
        let found = manifest(
            "[dependencies]\n# tomlfuse = { git = \"https://github.com/orgrinrt/tomlfuse\", rev = \"1773dfb\" }\n",
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn every_pin_in_a_manifest_is_reported_once_in_the_order_written() {
        let found = manifest(
            "[dependencies]\na = { git = \"https://github.com/orgrinrt/a\", rev = \"1\" }\nb = { git = \"https://github.com/orgrinrt/b\", branch = \"dev\" }\nc = { git = \"https://github.com/hiisi-digital/c\", rev = \"2\" }\n",
        );
        assert_eq!(lines(&found), [2, 4], "{found:?}");
    }

    #[test]
    fn a_file_that_is_not_toml_has_nothing_to_report() {
        assert!(manifest("[dependencies\na = { git = ").is_empty());
    }

    #[test]
    fn a_warm_run_replays_the_verdict_and_an_edited_manifest_is_judged_again() {
        // Mutation: the verdict not saved, or saved without the text of the manifests.
        let d = tempfile::tempdir().unwrap();
        let store = Kept::default();
        let pinned =
            "[dependencies]\na = { git = \"https://github.com/orgrinrt/a\", rev = \"1\" }\n";
        write(d.path(), "mock/crates/alpha/Cargo.toml", pinned);
        let cold = check_with(d.path(), Some(&store));
        assert_eq!(cold.len(), 1, "{cold:?}");
        assert_eq!(
            (store.hits(NAME), store.misses(NAME)),
            (0, 1),
            "a cold run replays nothing"
        );
        store.clear_counts();
        let warm = check_with(d.path(), Some(&store));
        assert_eq!(lines(&warm), lines(&cold));
        assert_eq!(
            (store.hits(NAME), store.misses(NAME)),
            (1, 0),
            "a warm run is one replay"
        );
        store.clear_counts();
        write(
            d.path(),
            "mock/crates/alpha/Cargo.toml",
            "[dependencies]\na = { git = \"https://github.com/orgrinrt/a\", branch = \"dev\" }\n",
        );
        let edited = check_with(d.path(), Some(&store));
        assert!(edited.is_empty(), "{edited:?}");
        assert_eq!(
            (store.hits(NAME), store.misses(NAME)),
            (0, 1),
            "an edited manifest is judged again"
        );
    }
}
