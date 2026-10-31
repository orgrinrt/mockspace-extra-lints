//! Lint: a git dependency on a first-party repository tracks a branch, never a fixed commit or tag.
//!
//! A dependency spelled with `rev` or `tag` names one point in a repository's history, and it
//! stays there while the repository moves on. For a repository of the same estate that is a stale
//! hash the moment the next fix lands on its `dev`, and nothing says so: the build is green against
//! code that no longer exists at the head. The first-party repositories are one body of work, so a
//! dependency on one tracks the head of the branch the work lands on, and the lockfile records the
//! commit that head was when it was last resolved.
//!
//! A repository is first-party when its url names the `orgrinrt` or the `hiisi-digital` account on
//! GitHub, in the https, ssh or scp spelling. A table is read in a `Cargo.toml` or a
//! `mockspace.toml`, as an inline table (`name = { git = "...", rev = "..." }`, on one line or
//! across several) or as a table of its own (`[dependencies.name]`), wherever it sits: a
//! dependency, a `[patch]` entry or a `[lint-crates]` entry. A `rev` or a `tag` beside a `git` url
//! of such a repository is the finding. A third-party repository is not read, and neither is a
//! lockfile, which records a commit by design.
//!
//! The record directories (`design_rounds` and `research` under the mock directory) are not read,
//! nor is anything under a `research/sketches` directory wherever it sits: a sketch pins the
//! revision it measured, and the pin is the evidence of what it measured.

use std::collections::BTreeMap;
use std::fs;

use mockspace_lint_rules::{Lint, LintError, RepoContext, RepoLint, Severity};

use crate::repo_files::{paths, record_dirs, WORKSPACE};

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "no-first-party-rev-pin";

/// The files whose tables are read.
const MANIFESTS: &[&str] = &["Cargo.toml", "mockspace.toml"];

/// The GitHub accounts the first-party repositories sit under, lowercase.
const ACCOUNTS: &[&str] = &["orgrinrt", "hiisi-digital"];

/// The keys that fix a dependency to one point in history.
const FIXING: &[&str] = &["rev", "tag"];

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
            .filter_map(|rel| fs::read_to_string(ctx.repo_root.join(&rel)).ok().map(|t| (rel, t)))
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

fn judge(files: &[(String, String)]) -> Vec<LintError> {
    let mut out = Vec::new();
    for (rel, text) in files {
        for table in tables(text) {
            let Some(url) = table.fields.get("git") else {
                continue;
            };
            if !first_party(url) {
                continue;
            }
            for key in FIXING.iter().filter(|k| table.fields.contains_key(**k)) {
                let mut e = LintError::error(
                    WORKSPACE.to_string(),
                    table.line,
                    NAME,
                    format!(
                        "a dependency on the first-party repository `{url}` names a fixed `{key}`; it tracks the head of a branch instead (`branch = \"dev\"`), and the lockfile records the commit"
                    ),
                );
                e.path = Some(rel.clone());
                out.push(e);
            }
        }
    }
    out
}

/// Whether a git url names a repository of one of the first-party accounts.
fn first_party(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    let Some(at) = lower.find("github.com") else {
        return false;
    };
    let rest = lower[at + "github.com".len()..].trim_start_matches(['/', ':']);
    let account = rest.split('/').next().unwrap_or("");
    ACCOUNTS.contains(&account)
}

/// One table as written, with the line it opens on.
struct Table {
    line: usize,
    fields: BTreeMap<String, String>,
}

/// Every table with a `git` key in `text`: the inline tables and the tables with a header of their
/// own. This reads the shapes a manifest uses for a dependency and nothing of TOML beyond them.
fn tables(text: &str) -> Vec<Table> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut header: Option<Table> = None;
    let mut i = 0;
    while i < lines.len() {
        let line = strip_comment(lines[i]).trim();
        if line.starts_with('[') {
            out.extend(header.take());
            header = Some(Table { line: i + 1, fields: BTreeMap::new() });
        } else if let Some((key, value)) = line.split_once('=') {
            let value = value.trim();
            if value.starts_with('{') {
                // An inline table runs to its closing brace, which may be several lines on.
                let mut body = value.to_string();
                let start = i;
                while !braces_closed(&body) && i + 1 < lines.len() {
                    i += 1;
                    body.push(' ');
                    body.push_str(strip_comment(lines[i]).trim());
                }
                out.push(Table { line: start + 1, fields: inline_fields(&body) });
            } else if let Some(h) = header.as_mut() {
                h.fields.insert(key.trim().trim_matches('"').to_string(), unquote(value));
            }
        }
        i += 1;
    }
    out.extend(header);
    out
}

/// Whether every `{` in `body` outside a string has its `}`.
fn braces_closed(body: &str) -> bool {
    let mut depth = 0_i32;
    let mut in_string = false;
    for c in body.chars() {
        match c {
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => depth -= 1,
            _ => {}
        }
    }
    depth <= 0
}

/// The `key = "value"` pairs of an inline table, at its top level.
fn inline_fields(body: &str) -> BTreeMap<String, String> {
    let inner = body.trim().trim_start_matches('{');
    let inner = inner.rsplit_once('}').map_or(inner, |(head, _)| head);
    let mut out = BTreeMap::new();
    let mut depth = 0_i32;
    let mut in_string = false;
    let mut piece = String::new();
    let mut pieces = Vec::new();
    for c in inner.chars() {
        match c {
            '"' => in_string = !in_string,
            '{' | '[' if !in_string => depth += 1,
            '}' | ']' if !in_string => depth -= 1,
            ',' if !in_string && depth == 0 => {
                pieces.push(std::mem::take(&mut piece));
                continue;
            }
            _ => {}
        }
        piece.push(c);
    }
    pieces.push(piece);
    for p in pieces {
        if let Some((k, v)) = p.split_once('=') {
            out.insert(k.trim().trim_matches('"').to_string(), unquote(v.trim()));
        }
    }
    out
}

fn unquote(v: &str) -> String {
    v.trim().trim_matches('"').to_string()
}

/// A line without its trailing comment, leaving a `#` inside a string alone.
fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_string = !in_string,
            '#' if !in_string => return &line[..i],
            _ => {}
        }
    }
    line
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;

    use mockspace_lint_rules::RegistryView;

    use super::*;

    fn write(root: &Path, rel: &str, text: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn check(root: &Path) -> Vec<LintError> {
        let crates = BTreeSet::from(["alpha".to_string()]);
        let reg = RegistryView::new(BTreeMap::new(), BTreeMap::new());
        let ctx = RepoContext {
            mock_dir: &root.join("mock"),
            repo_root: root,
            all_crates: &crates,
            src_dirs: &[],
            invocation: None,
            canon_paths: &[],
            open_panels: &[],
            registry: &reg,
            verdicts: None,
        };
        NoFirstPartyRevPin.check_repo(&ctx)
    }

    fn at(rel: &str, body: &str) -> Vec<LintError> {
        let d = tempfile::tempdir().unwrap();
        write(d.path(), rel, body);
        check(d.path())
    }

    fn manifest(body: &str) -> Vec<LintError> {
        at("mock/crates/alpha/Cargo.toml", body)
    }

    #[test]
    fn a_rev_on_a_first_party_https_dependency_is_refused() {
        let found = manifest(
            "[dependencies]\ntomlfuse = { git = \"https://github.com/orgrinrt/tomlfuse\", rev = \"1773dfb\" }\n",
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].line, 2);
        assert_eq!(found[0].path.as_deref(), Some("mock/crates/alpha/Cargo.toml"));
        assert!(found[0].message.contains("`rev`"), "{found:?}");
    }

    #[test]
    fn a_rev_on_a_first_party_ssh_dependency_is_refused() {
        let found = manifest(
            "[dependencies]\nl = { git = \"ssh://git@github.com/orgrinrt/letkajenkka.git\", rev = \"52a6117\" }\n",
        );
        assert_eq!(found.len(), 1, "{found:?}");
    }

    #[test]
    fn a_tag_on_a_dependency_of_the_other_first_party_account_is_refused() {
        let found = manifest(
            "[dependencies]\nm = { git = \"git@github.com:hiisi-digital/mockspace.git\", tag = \"v0.1.0\" }\n",
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].message.contains("`tag`"), "{found:?}");
    }

    #[test]
    fn a_table_with_a_header_of_its_own_is_read() {
        let found = manifest(
            "[dependencies.tomlfuse]\ngit = \"https://github.com/orgrinrt/tomlfuse\"\nrev = \"1773dfb\"\n",
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].line, 1);
    }

    #[test]
    fn an_inline_table_across_lines_is_read() {
        let found = manifest(
            "[dependencies]\ntomlfuse = {\n    git = \"https://github.com/orgrinrt/tomlfuse\",\n    rev = \"1773dfb\", # the old one\n}\n",
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].line, 2);
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
    fn an_account_name_inside_another_accounts_path_is_not_first_party() {
        let found = manifest(
            "[dependencies]\nx = { git = \"https://github.com/someone/orgrinrt\", rev = \"0123abc\" }\n",
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_sketch_keeps_its_pin() {
        let body = "[dependencies]\ntomlfuse = { git = \"https://github.com/orgrinrt/tomlfuse\", rev = \"1773dfb\" }\n";
        assert!(at("mock/research/sketches/tomlfuse/Cargo.toml", body).is_empty());
        assert!(at("elsewhere/research/sketches/x/Cargo.toml", body).is_empty());
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
    fn every_pin_in_a_manifest_is_reported_once() {
        let found = manifest(
            "[dependencies]\na = { git = \"https://github.com/orgrinrt/a\", rev = \"1\" }\nb = { git = \"https://github.com/orgrinrt/b\", branch = \"dev\" }\nc = { git = \"https://github.com/hiisi-digital/c\", tag = \"v1\" }\n",
        );
        let lines: Vec<usize> = found.iter().map(|e| e.line).collect();
        assert_eq!(lines, vec![2, 4], "{found:?}");
    }
}
