//! Lint: a git dependency on a repository the consumer names tracks a branch, never a fixed commit.
//!
//! A dependency spelled with `rev` or `tag` names one point in a repository's history, and it stays
//! there while the repository moves on. For a repository of the same body of work that is a stale
//! pin the moment the next fix lands, and a hash that does not exist at all once the repository is
//! moved to a new home, which keeps its branch names and loses the rest. What counts as such a
//! repository, which branch the work lands on and where a pin is meant to stay are the consumer's
//! to say, and the lint knows none of them:
//!
//! - `accounts`, a comma-separated list of `host/account` (`github.com/acme`), names the
//!   repositories read. A git url is one of them when its host and the first segment of its path
//!   match an entry, in the https, ssh or scp spelling. With no `accounts` the lint finds nothing.
//! - `branch`, optional, is the branch a dependency on such a repository tracks. With it set, a
//!   dependency that names no branch, or another branch, is a finding too. Without it only a fixed
//!   point is.
//! - `exempt`, a comma-separated list of path globs from the repository root, names the manifests
//!   not read, such as the records of earlier work, where a pin is evidence of what was built.
//!
//! The severity is the consumer's, set at each gate in the lint's table. The files read are
//! `Cargo.toml` and `mockspace.toml`, as TOML, so a table is found as an inline table (on one line
//! or across several), as a table of its own (`[dependencies.name]`) and with dotted keys
//! (`name.git = "..."`), wherever it sits: a dependency, a `[patch]` entry or a `[lint-crates]`
//! entry. A fixed `rev` or `tag` is reported at the line of the key, a missing or other branch at
//! the line of the `git` key. A lockfile is not read, since it records a commit by design.

use std::collections::HashMap;
use std::fs;
use std::ops::Range;

use mockspace_lint_rules::{glob_match_anchored, Lint, LintError, Reach, RepoContext, RepoLint, Severity};
use toml_edit::{Item, TableLike};

use crate::manifest_toml::{line_of, parse};
use crate::repo_files::{WORKSPACE, paths};

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "a-git-dependency-is-not-pinned";

/// The files whose tables are read.
const MANIFESTS: &[&str] = &["Cargo.toml", "mockspace.toml"];

/// What the lint reads, as the paths its reach declares: every manifest it walks.
const READS: &[&str] = &["**/Cargo.toml", "**/mockspace.toml"];

/// The keys that fix a dependency to one point in history, the first that is written the finding.
const FIXING: &[&str] = &["rev", "tag"];

const ACCOUNTS_KEY: &str = "accounts";
const BRANCH_KEY: &str = "branch";
const EXEMPT_KEY: &str = "exempt";

/// The configured lint.
#[derive(Default)]
pub struct AGitDependencyIsNotPinned {
    /// Lowercase `(host, account)` pairs.
    accounts: Vec<(String, String)>,
    /// The `accounts` entries that are not `host/account`, each reported, since a typo would
    /// otherwise leave the lint looking unconfigured.
    bad:      Vec<String>,
    /// The branch a dependency tracks, when the consumer names one.
    branch:   Option<String>,
    /// Path globs from the repository root.
    exempt:   Vec<String>,
}

fn list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

impl Lint for AGitDependencyIsNotPinned {
    fn name(&self) -> &'static str {
        NAME
    }

    fn reach(&self) -> Reach {
        Reach::Paths(READS)
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }

    fn config_keys(&self) -> &[&str] {
        &[ACCOUNTS_KEY, BRANCH_KEY, EXEMPT_KEY]
    }

    fn configure(&mut self, params: &HashMap<String, String>) {
        if let Some(v) = params.get(ACCOUNTS_KEY) {
            self.accounts.clear();
            self.bad.clear();
            for entry in list(v) {
                match entry.split_once('/') {
                    Some((host, account)) if !host.is_empty() && !account.is_empty() => self
                        .accounts
                        .push((host.to_ascii_lowercase(), account.to_ascii_lowercase())),
                    _ => self.bad.push(entry),
                }
            }
        }
        if let Some(v) = params.get(BRANCH_KEY) {
            self.branch = Some(v.trim().to_string()).filter(|b| !b.is_empty());
        }
        if let Some(v) = params.get(EXEMPT_KEY) {
            self.exempt = list(v);
        }
    }
}

impl RepoLint for AGitDependencyIsNotPinned {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        if !self.bad.is_empty() {
            return self
                .bad
                .iter()
                .map(|entry| {
                    LintError::error(
                        WORKSPACE.to_string(),
                        0,
                        NAME,
                        format!("the `accounts` entry `{entry}` is not `host/account`, so the lint reads nothing for it"),
                    )
                })
                .collect();
        }
        if self.accounts.is_empty() {
            return Vec::new();
        }
        let found = paths(ctx.repo_root, &[], |name, rel| {
            MANIFESTS.contains(&name) && !self.exempt.iter().any(|g| glob_match_anchored(g, rel))
        });
        let files: Vec<(String, String)> = found
            .into_iter()
            .filter_map(|rel| {
                fs::read_to_string(ctx.repo_root.join(&rel))
                    .ok()
                    .map(|t| (rel, t))
            })
            .collect();
        // The verdict follows from the text of the manifests and the three keys, so it replays when
        // none moved.
        let config = format!(
            "{:?}|{}|{:?}",
            self.accounts,
            self.branch.as_deref().unwrap_or(""),
            self.exempt
        );
        let mut parts: Vec<&[u8]> = vec![config.as_bytes()];
        for (rel, text) in &files {
            parts.push(rel.as_bytes());
            parts.push(text.as_bytes());
        }
        ctx.remembered(NAME, &parts, || self.judge(&files))
    }
}

/// What is wrong with a dependency on a repository of the consumer's.
enum Fault {
    /// A `rev` or a `tag`, by the key's name.
    Fixed(&'static str),
    /// No branch, or a branch other than the one named.
    Branch(Option<String>),
}

/// A finding: the url, the fault and where it stands.
struct Pin {
    url:   String,
    fault: Fault,
    span:  Option<Range<usize>>,
}

impl AGitDependencyIsNotPinned {
    fn judge(&self, files: &[(String, String)]) -> Vec<LintError> {
        let mut out = Vec::new();
        for (rel, text) in files {
            let Some(doc) = parse(text) else {
                continue;
            };
            let mut pins = Vec::new();
            self.walk_table(doc.as_table(), &mut pins);
            pins.sort_by_key(|pin| pin.span.as_ref().map(|s| s.start));
            for pin in pins {
                let message = match &pin.fault {
                    Fault::Fixed(key) => format!(
                        "a dependency on `{}` names a fixed `{key}`; it tracks the head of a branch instead, and the lockfile records the commit",
                        pin.url
                    ),
                    Fault::Branch(None) => format!(
                        "a dependency on `{}` names no branch; it tracks `{}`",
                        pin.url,
                        self.branch.as_deref().unwrap_or("")
                    ),
                    Fault::Branch(Some(other)) => format!(
                        "a dependency on `{}` tracks `{other}`; it tracks `{}`",
                        pin.url,
                        self.branch.as_deref().unwrap_or("")
                    ),
                };
                let mut e =
                    LintError::error(WORKSPACE.to_string(), line_of(text, pin.span), NAME, message);
                e.path = Some(rel.clone());
                out.push(e);
            }
        }
        out
    }

    /// Every table below this one, this one included, that has a `git` url of a configured account
    /// and a fault.
    fn walk_table(&self, table: &dyn TableLike, found: &mut Vec<Pin>) {
        found.extend(self.pin_of(table));
        for (_, child) in table.iter() {
            self.walk_item(child, found);
        }
    }

    fn walk_item(&self, item: &Item, found: &mut Vec<Pin>) {
        if let Some(table) = item.as_table_like() {
            self.walk_table(table, found);
        } else if let Some(tables) = item.as_array_of_tables() {
            tables.iter().for_each(|table| self.walk_table(table, found));
        } else if let Some(array) = item.as_array() {
            array
                .iter()
                .filter_map(|value| value.as_inline_table())
                .for_each(|table| self.walk_table(table, found));
        }
    }

    /// The finding a table makes: a fixed point first, else the branch against the one named.
    fn pin_of(&self, table: &dyn TableLike) -> Option<Pin> {
        let (git_key, git) = table.get_key_value("git")?;
        let url = git.as_str()?;
        if !self.first_party(url) {
            return None;
        }
        for key in FIXING {
            if let Some((fixed, _)) = table.get_key_value(key) {
                return Some(Pin {
                    url:   url.to_owned(),
                    fault: Fault::Fixed(key),
                    span:  fixed.span(),
                });
            }
        }
        let wanted = self.branch.as_deref()?;
        let have = table.get(BRANCH_KEY).and_then(Item::as_str);
        (have != Some(wanted)).then(|| Pin {
            url:   url.to_owned(),
            fault: Fault::Branch(have.map(str::to_owned)),
            span:  git_key.span(),
        })
    }

    /// Whether a git url names a repository of one of the configured accounts: the host matches
    /// exactly and the account is the first segment of the path, in the url spelling (`https://`,
    /// `ssh://`, with or without a user, a password or a port) and the scp spelling
    /// (`git@host:account/repo`).
    fn first_party(&self, url: &str) -> bool {
        let has_scheme = url.contains("://");
        let rest = url
            .trim()
            .split_once("://")
            .map_or(url.trim(), |(_, rest)| rest);
        let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
        let host_part = authority
            .rsplit_once('@')
            .map_or(authority, |(_, host)| host);
        let (host, account) = match host_part.split_once(':') {
            // With a scheme, after the colon is a port when it is all digits; without one, the
            // scp spelling has no port and what follows is the account, whatever its characters.
            Some((host, tail))
                if !tail.is_empty() && (!has_scheme || !tail.bytes().all(|b| b.is_ascii_digit())) =>
            {
                (host, tail)
            },
            Some((host, _)) => (host, path.split('/').next().unwrap_or("")),
            None => (host_part, path.split('/').next().unwrap_or("")),
        };
        let (host, account) = (host.to_ascii_lowercase(), account.to_ascii_lowercase());
        self.accounts.iter().any(|(h, a)| *h == host && *a == account)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;

    use mockspace_lint_rules::RegistryView;

    use super::*;
    use crate::test_support::Kept;

    const OURS: &str = "github.com/acme,github.com/other";

    fn write(root: &Path, rel: &str, text: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn lint(params: &[(&str, &str)]) -> AGitDependencyIsNotPinned {
        let mut lint = AGitDependencyIsNotPinned::default();
        let map: HashMap<String, String> = params
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        lint.configure(&map);
        lint
    }

    fn check_with(
        lint: &AGitDependencyIsNotPinned,
        root: &Path,
        store: Option<&Kept>,
    ) -> Vec<LintError> {
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
        lint.check_repo(&ctx)
    }

    fn at(lint: &AGitDependencyIsNotPinned, rel: &str, body: &str) -> Vec<LintError> {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), rel, body);
        check_with(lint, d.path(), None)
    }

    /// The pack configured the way a consumer of one body of work would, no branch.
    fn manifest(body: &str) -> Vec<LintError> {
        at(&lint(&[("accounts", OURS)]), "mock/crates/alpha/Cargo.toml", body)
    }

    fn on_branch(body: &str) -> Vec<LintError> {
        at(
            &lint(&[("accounts", OURS), ("branch", "main")]),
            "mock/crates/alpha/Cargo.toml",
            body,
        )
    }

    fn lines(found: &[LintError]) -> Vec<usize> {
        found.iter().map(|e| e.line).collect()
    }

    #[test]
    fn an_unconfigured_lint_finds_nothing() {
        // Mutation: the lint falls back to accounts of its own, or reads every git dependency.
        let body = "[dependencies]\na = { git = \"https://github.com/acme/a\", rev = \"1\" }\n";
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "mock/crates/alpha/Cargo.toml", body);
        let found = check_with(&AGitDependencyIsNotPinned::default(), d.path(), None);
        assert!(found.is_empty(), "{found:?}");
        let found = check_with(&lint(&[("branch", "main")]), d.path(), None);
        assert!(found.is_empty(), "a branch alone names no repository: {found:?}");
        assert!(!manifest(body).is_empty(), "the control: configured, the same file is a finding");
    }

    #[test]
    fn a_rev_on_a_configured_https_dependency_is_refused_at_the_line_of_the_rev() {
        let found = manifest(
            "[dependencies]\ntomlfuse = { git = \"https://github.com/acme/tomlfuse\", rev = \"1773dfb\" }\n",
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].line, 2);
        assert_eq!(found[0].path.as_deref(), Some("mock/crates/alpha/Cargo.toml"));
        assert!(found[0].message.contains("`rev`"), "{found:?}");
        assert_eq!(found[0].crate_name, WORKSPACE);
        assert!(!found[0].message.contains("dev"), "no branch name of ours in the advice");
    }

    #[test]
    fn a_tag_is_a_fixed_point_as_a_rev_is() {
        // Mutation: a tag is let through.
        let found = manifest("[dependencies]\na = { git = \"https://github.com/acme/a\", tag = \"v1\" }\n");
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].message.contains("`tag`"), "{found:?}");
    }

    #[test]
    fn a_dependency_with_both_a_rev_and_a_tag_is_one_finding() {
        let found = manifest(
            "[dependencies]\na = { git = \"https://github.com/acme/a\", rev = \"1\", tag = \"v1\" }\n",
        );
        assert_eq!(found.len(), 1, "{found:?}");
    }

    #[test]
    fn every_spelling_of_a_configured_url_is_read() {
        for url in [
            "https://github.com/acme/a",
            "https://user:pw@github.com/acme/a.git",
            "ssh://git@github.com/acme/a",
            "ssh://git@github.com:22/acme/a",
            "git@github.com:acme/a.git",
            "HTTPS://GitHub.com/ACME/a",
        ] {
            let found = manifest(&format!(
                "[dependencies]\na = {{ git = \"{url}\", rev = \"1\" }}\n"
            ));
            assert_eq!(found.len(), 1, "{url}: {found:?}");
        }
    }

    #[test]
    fn the_host_and_the_account_are_both_the_configured_ones() {
        // Mutation: the account alone, or a host that contains the configured one, matches.
        for url in [
            "https://github.com.evil.example/acme/a",
            "https://gitlab.com/acme/a",
            "https://github.com/acmeish/a",
            "https://github.com/x/acme",
            "git@github.com:stranger/a.git",
        ] {
            let found = manifest(&format!(
                "[dependencies]\na = {{ git = \"{url}\", rev = \"1\" }}\n"
            ));
            assert!(found.is_empty(), "{url}: {found:?}");
        }
        let elsewhere = at(
            &lint(&[("accounts", "git.example.org/acme")]),
            "mock/crates/alpha/Cargo.toml",
            "[dependencies]\na = { git = \"https://git.example.org/acme/a\", rev = \"1\" }\nb = { git = \"https://github.com/acme/b\", rev = \"1\" }\n",
        );
        assert_eq!(lines(&elsewhere), vec![2], "the host is the consumer's: {elsewhere:?}");
    }

    #[test]
    fn a_branch_is_let_through_when_none_is_configured() {
        assert!(manifest("[dependencies]\na = { git = \"https://github.com/acme/a\", branch = \"any\" }\n").is_empty());
        assert!(manifest("[dependencies]\na = { git = \"https://github.com/acme/a\" }\n").is_empty());
    }

    #[test]
    fn with_a_branch_configured_a_missing_or_other_branch_is_a_finding() {
        // Mutation: the branch is not compared, or a missing one is let through.
        let missing = on_branch("[dependencies]\na = { git = \"https://github.com/acme/a\" }\n");
        assert_eq!(lines(&missing), vec![2], "{missing:?}");
        assert!(missing[0].message.contains("no branch"), "{missing:?}");
        assert!(missing[0].message.contains("`main`"), "{missing:?}");
        let other = on_branch("[dependencies]\na = { git = \"https://github.com/acme/a\", branch = \"dev\" }\n");
        assert_eq!(other.len(), 1, "{other:?}");
        assert!(other[0].message.contains("tracks `dev`"), "{other:?}");
        let right = on_branch("[dependencies]\na = { git = \"https://github.com/acme/a\", branch = \"main\" }\n");
        assert!(right.is_empty(), "{right:?}");
        let fixed = on_branch("[dependencies]\na = { git = \"https://github.com/acme/a\", rev = \"1\", branch = \"main\" }\n");
        assert_eq!(fixed.len(), 1, "a fixed point is the one finding: {fixed:?}");
    }

    #[test]
    fn a_table_with_a_header_of_its_own_is_read_at_the_line_of_the_rev() {
        let found = manifest(
            "[dependencies.a]\ngit = \"https://github.com/acme/a\"\nrev = \"1\"\n",
        );
        assert_eq!(lines(&found), vec![3], "{found:?}");
    }

    #[test]
    fn a_table_written_with_dotted_keys_is_read() {
        let found = manifest("[dependencies]\na.git = \"https://github.com/acme/a\"\na.rev = \"1\"\n");
        assert_eq!(lines(&found), vec![3], "{found:?}");
    }

    #[test]
    fn an_inline_table_across_lines_is_read() {
        let found = manifest(
            "[dependencies]\na = {\n    git = \"https://github.com/acme/a\",\n    rev = \"1\",\n}\n",
        );
        assert_eq!(lines(&found), vec![4], "{found:?}");
    }

    #[test]
    fn a_patch_entry_and_a_lint_crates_entry_are_read() {
        let patch = manifest("[patch.crates-io]\na = { git = \"https://github.com/acme/a\", rev = \"1\" }\n");
        assert_eq!(patch.len(), 1, "{patch:?}");
        let lint_crates = at(
            &lint(&[("accounts", OURS)]),
            "mockspace.toml",
            "[lint-crates]\npack = { git = \"https://github.com/acme/p\", rev = \"1\" }\n",
        );
        assert_eq!(lint_crates.len(), 1, "{lint_crates:?}");
    }

    #[test]
    fn a_table_in_an_array_of_tables_is_read() {
        let found = manifest("[[bin]]\nname = \"x\"\n[[bin]]\ngit = \"https://github.com/acme/a\"\nrev = \"1\"\n");
        assert_eq!(lines(&found), vec![5], "{found:?}");
    }

    #[test]
    fn findings_follow_the_order_written() {
        let found = manifest(
            "[dependencies]\nb = { git = \"https://github.com/acme/b\", rev = \"1\" }\na = { git = \"https://github.com/acme/a\", rev = \"2\" }\n",
        );
        assert_eq!(lines(&found), vec![2, 3], "{found:?}");
    }

    #[test]
    fn a_path_dependency_a_registry_version_and_a_comment_are_let_through() {
        assert!(manifest("[dependencies]\na = { path = \"../a\" }\nb = \"1\"\n").is_empty());
        assert!(manifest("[dependencies]\n# a = { git = \"https://github.com/acme/a\", rev = \"1\" }\n").is_empty());
    }

    #[test]
    fn a_file_that_is_not_toml_has_nothing_to_report() {
        assert!(manifest("[dependencies\na = { git = ").is_empty());
    }

    #[test]
    fn an_exempt_path_keeps_its_pin_and_nothing_else_does() {
        // Mutation: the exempt globs are ignored, or a directory of ours is exempt by default.
        let body = "[dependencies]\na = { git = \"https://github.com/acme/a\", rev = \"1\" }\n";
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "mock/research/sketches/x/Cargo.toml", body);
        write(d.path(), "mock/design_rounds/old/Cargo.toml", body);
        let none = lint(&[("accounts", OURS)]);
        assert_eq!(check_with(&none, d.path(), None).len(), 2, "no default exemption");
        write(d.path(), "mock/crates/alpha/Cargo.toml", body);
        let some = lint(&[("accounts", OURS), ("exempt", "mock/research/**, mock/design_rounds/**")]);
        let found = check_with(&some, d.path(), None);
        // Mutation: any exempt config silences the whole repository.
        assert_eq!(
            found.iter().filter_map(|e| e.path.as_deref()).collect::<Vec<_>>(),
            ["mock/crates/alpha/Cargo.toml"],
            "a manifest outside the globs is still read: {found:?}"
        );
    }

    #[test]
    fn exempt_globs_are_anchored_at_the_root_with_a_star_that_stops_at_a_slash_and_keep_case() {
        // Mutation: the pack's other glob dialect, which folds case and lets `*` cross `/`.
        let body = "[dependencies]\na = { git = \"https://github.com/acme/a\", rev = \"1\" }\n";
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "mock/crates/alpha/Cargo.toml", body);
        let cross = lint(&[("accounts", OURS), ("exempt", "mock/*/Cargo.toml")]);
        assert_eq!(check_with(&cross, d.path(), None).len(), 1, "`*` stops at a slash");
        let cased = lint(&[("accounts", OURS), ("exempt", "MOCK/**")]);
        assert_eq!(check_with(&cased, d.path(), None).len(), 1, "the case is kept");
        let right = lint(&[("accounts", OURS), ("exempt", "mock/*/*/Cargo.toml")]);
        assert!(check_with(&right, d.path(), None).is_empty(), "the control: the right glob exempts it");
    }

    #[test]
    fn an_all_digit_account_is_an_account_in_the_scp_spelling_and_a_port_with_a_scheme() {
        // Mutation: the port guess runs without a scheme.
        let digits = lint(&[("accounts", "github.com/1234")]);
        let scp = at(
            &digits,
            "mock/crates/alpha/Cargo.toml",
            "[dependencies]\na = { git = \"git@github.com:1234/a.git\", rev = \"1\" }\n",
        );
        assert_eq!(scp.len(), 1, "{scp:?}");
        let ported = at(
            &lint(&[("accounts", "github.com/acme")]),
            "mock/crates/alpha/Cargo.toml",
            "[dependencies]\na = { git = \"ssh://git@github.com:1234/acme/a.git\", rev = \"1\" }\n",
        );
        assert_eq!(ported.len(), 1, "a port with a scheme is still skipped: {ported:?}");
    }

    #[test]
    fn an_accounts_entry_that_is_not_host_slash_account_is_reported() {
        // Mutation: the entry is dropped and the lint looks unconfigured.
        let found = at(
            &lint(&[("accounts", "acme")]),
            "mock/crates/alpha/Cargo.toml",
            "[dependencies]\na = { path = \"../a\" }\n",
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].message.contains("`acme`"), "{found:?}");
        let both = lint(&[("accounts", "github.com/acme,acme,/x")]);
        let d = tempfile::tempdir().unwrap();
        write(d.path(), "mock/crates/alpha/Cargo.toml", "[dependencies]\n");
        assert_eq!(check_with(&both, d.path(), None).len(), 2, "each bad entry once");
    }

    #[test]
    fn a_warm_run_replays_the_verdict_and_an_edit_or_a_config_change_is_judged_again() {
        // Mutation: the verdict is saved without the text of the manifests, or without the config.
        let d = tempfile::tempdir().unwrap();
        let store = Kept::default();
        let lint_a = lint(&[("accounts", OURS)]);
        let pinned = "[dependencies]\na = { git = \"https://github.com/acme/a\", rev = \"1\" }\n";
        write(d.path(), "mock/crates/alpha/Cargo.toml", pinned);
        let cold = check_with(&lint_a, d.path(), Some(&store));
        assert_eq!(cold.len(), 1, "{cold:?}");
        assert_eq!((store.hits(NAME), store.misses(NAME)), (0, 1), "a cold run replays nothing");
        store.clear_counts();
        let warm = check_with(&lint_a, d.path(), Some(&store));
        assert_eq!(lines(&warm), lines(&cold));
        assert_eq!((store.hits(NAME), store.misses(NAME)), (1, 0), "a warm run is one replay");
        store.clear_counts();
        let other = lint(&[("accounts", "github.com/elsewhere")]);
        let moved = check_with(&other, d.path(), Some(&store));
        assert!(moved.is_empty(), "{moved:?}");
        assert_eq!((store.hits(NAME), store.misses(NAME)), (0, 1), "another config is judged again");
        store.clear_counts();
        write(
            d.path(),
            "mock/crates/alpha/Cargo.toml",
            "[dependencies]\na = { git = \"https://github.com/acme/a\", branch = \"dev\" }\n",
        );
        let edited = check_with(&lint_a, d.path(), Some(&store));
        assert!(edited.is_empty(), "{edited:?}");
        assert_eq!((store.hits(NAME), store.misses(NAME)), (0, 1), "an edited manifest is judged again");
    }
}
