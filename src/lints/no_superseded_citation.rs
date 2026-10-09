//! Lint: a design or a source file never cites a canon row that something else has superseded.
//!
//! A project whose canon is typed rows keeps a row it has replaced, marked with the rows that
//! replace it (`superseded_by`), so the next canon is built with the old one in view. The kept row
//! no longer binds. A design or a piece of code that still names it is written to a rule nobody
//! holds any more, and nothing else notices: the registry's own checks read the rows, not the text
//! that cites them. This lint reads the text.
//!
//! What it reads: every text file under the repository (`.md.tmpl`, `.md`, `.rs`, `.toml`,
//! `.slang`) except the canon's own files (`canon_paths`), the design rounds and research notes
//! that are the record of what was once true, build output, vendored sources and generated files.
//! What it counts as a citation: the row's slug as a whole word, backticked or not. What it lets
//! through: a sentence that says the row is superseded, since a design may explain what a row was
//! replaced by without relying on it.
//!
//! A project names the field with a `superseded_by` field on its rows. A repository whose registry
//! has none declares nothing superseded and the lint answers nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use mockspace_lint_rules::{Lint, LintError, RegistryView, RepoContext, RepoLint, Severity};

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "no-superseded-citation";

/// The field a registry row carries to say what replaced it.
const FIELD: &str = "superseded_by";

/// The separator the engine joins a list field with.
const JOIN: &str = ", ";

/// The extensions whose files are read.
const READ: &[&str] = &["rs", "tmpl", "md", "toml", "slang"];

/// Directory names never entered, wherever they sit.
const SKIPPED: &[&str] = &["target", "vendor", "node_modules"];

/// The directories under the mock directory that are the record of what was once true.
const RECORD: &[&str] = &["design_rounds", "research"];

/// What a generated file carries in its first lines.
const GENERATED: &str = "AUTO-GENERATED";

/// How many lines at the top of a file the generated marker may sit in.
const HEADER: usize = 12;

pub struct NoSupersededCitation;

impl Lint for NoSupersededCitation {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

impl RepoLint for NoSupersededCitation {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        let replaced = replaced(ctx.registry);
        if replaced.is_empty() {
            return Vec::new();
        }
        let mut files = Vec::new();
        let record = RECORD
            .iter()
            .filter_map(|d| ctx.mock_dir.join(d).strip_prefix(ctx.repo_root).ok().map(slashed))
            .collect::<Vec<_>>();
        walk(ctx.repo_root, ctx.repo_root, &record, ctx.canon_paths, &mut files);
        files.sort();
        let mut out = Vec::new();
        for rel in files {
            let Ok(text) = fs::read_to_string(ctx.repo_root.join(&rel)) else {
                continue;
            };
            if text.lines().take(HEADER).any(|l| l.contains(GENERATED)) {
                continue;
            }
            let crate_name = rel
                .split('/')
                .find(|c| ctx.all_crates.contains(*c))
                .unwrap_or("workspace")
                .to_string();
            for (line, old, successors) in citations(&text, &replaced) {
                let mut e = LintError::error(
                    crate_name.clone(),
                    line,
                    NAME,
                    format!(
                        "`{rel}` cites `{old}`, which {} supersede{}; write to the successor, or say in the sentence that the row is superseded",
                        names(&successors),
                        if successors.len() == 1 { "s" } else { "" },
                    ),
                );
                e.path = Some(rel.clone());
                out.push(e);
            }
        }
        out
    }
}

/// `` `a`, `b` ``: the successors, each backticked.
fn names(successors: &[String]) -> String {
    successors
        .iter()
        .map(|s| format!("`{s}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Every replaced row's slug, with the rows that stand in its place now: a successor that is itself
/// replaced is followed to what replaced it, so the name given is one that binds.
fn replaced(reg: &RegistryView) -> BTreeMap<String, Vec<String>> {
    let mut direct: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for ns in reg.namespaces() {
        for q in reg.rows_in(ns) {
            let Some(raw) = reg.field(q, FIELD) else {
                continue;
            };
            let to: Vec<String> = raw
                .split(JOIN)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.rsplit("::").next().unwrap_or(s).to_string())
                .collect();
            if !to.is_empty() {
                let slug = q.rsplit("::").next().unwrap_or(q).to_string();
                direct.insert(slug, to);
            }
        }
    }
    direct
        .keys()
        .map(|slug| {
            let mut seen = BTreeSet::from([slug.clone()]);
            let mut live = Vec::new();
            follow(&direct, slug, &mut seen, &mut live);
            (slug.clone(), live)
        })
        .collect()
}

/// The first rows down `slug`'s replacements that nothing replaces, in order, each once.
fn follow(
    direct: &BTreeMap<String, Vec<String>>,
    slug: &str,
    seen: &mut BTreeSet<String>,
    live: &mut Vec<String>,
) {
    for next in &direct[slug] {
        match direct.get(next) {
            Some(_) if seen.insert(next.clone()) => follow(direct, next, seen, live),
            Some(_) => {}
            None if !live.contains(next) => live.push(next.clone()),
            None => {}
        }
    }
}

/// A path with forward slashes, whatever the platform.
fn slashed(p: &Path) -> String {
    p.components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// The files to read under `dir`, relative to `root`.
fn walk(
    root: &Path,
    dir: &Path,
    record: &[String],
    canon_paths: &[String],
    out: &mut Vec<String>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        let rel = slashed(rel);
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if path.is_dir() {
            if name.starts_with('.') || SKIPPED.contains(&name) || record.contains(&rel) {
                continue;
            }
            walk(root, &path, record, canon_paths, out);
        } else if reads(name) && !canon_paths.iter().any(|g| glob(g, &rel)) {
            out.push(rel);
        }
    }
}

/// Whether a file of this name is one the lint reads.
fn reads(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, ext)| READ.contains(&ext))
}

/// Whether `text` matches `pattern`, where `*` stands for any run of characters but a slash and
/// `**` for any run at all.
fn glob(pattern: &str, text: &str) -> bool {
    fn go(p: &[u8], t: &[u8]) -> bool {
        match p {
            [] => t.is_empty(),
            [b'*', b'*', rest @ ..] => (0..=t.len()).any(|i| go(rest, &t[i..])),
            [b'*', rest @ ..] => {
                let run = t.iter().take_while(|&&c| c != b'/').count();
                (0..=run).any(|i| go(rest, &t[i..]))
            }
            [c, rest @ ..] => t.first() == Some(c) && go(rest, &t[1..]),
        }
    }
    go(pattern.as_bytes(), text.as_bytes())
}

/// Whether a character can sit inside a row's slug.
fn id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Every place the text names a replaced row as a whole word, outside a sentence that says it is
/// superseded: its line, the row, and what stands in its place.
fn citations(text: &str, replaced: &BTreeMap<String, Vec<String>>) -> Vec<(usize, String, Vec<String>)> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        for (slug, live) in replaced {
            for (at, _) in line.match_indices(slug.as_str()) {
                let before = line[..at].chars().next_back();
                let after = line[at + slug.len()..].chars().next();
                if before.is_some_and(id_char) || after.is_some_and(id_char) {
                    continue;
                }
                if !says_superseded(&lines, i, at, slug.len()) {
                    out.push((i + 1, slug.clone(), live.clone()));
                }
            }
        }
    }
    out.sort();
    out
}

/// Whether the sentence around the match, kept inside its paragraph, says "superseded".
fn says_superseded(lines: &[&str], line: usize, at: usize, len: usize) -> bool {
    let first = (0..=line).rev().take_while(|&j| !lines[j].trim().is_empty()).last().unwrap_or(line);
    let last = (line..lines.len()).take_while(|&j| !lines[j].trim().is_empty()).last().unwrap_or(line);
    let para = lines[first..=last].join("\n");
    let off = lines[first..line].iter().map(|l| l.len() + 1).sum::<usize>() + at;
    let stop = |i: usize| {
        matches!(para.as_bytes()[i], b'.' | b'!' | b'?')
            && para.as_bytes().get(i + 1).is_none_or(|c| c.is_ascii_whitespace())
    };
    let from = (0..off).rev().find(|&i| stop(i)).map_or(0, |i| i + 1);
    let to = (off + len..para.len()).find(|&i| stop(i)).unwrap_or(para.len());
    para[from..to].to_ascii_lowercase().contains("supersed")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn registry(rows: &[(&str, &[(&str, &str)])]) -> RegistryView {
        let rows = rows
            .iter()
            .map(|(q, f)| {
                let f = f
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                    .collect::<BTreeMap<_, _>>();
                ((*q).to_string(), f)
            })
            .collect::<BTreeMap<_, _>>();
        RegistryView::new(rows, BTreeMap::new())
    }

    /// Two replaced rows, one of them replaced twice over, and a live one.
    fn canon() -> RegistryView {
        registry(&[
            ("tenet::old", &[("superseded_by", "mid")]),
            ("tenet::mid", &[("superseded_by", "new")]),
            ("tenet::new", &[("what", "x")]),
            ("tenet::split", &[("superseded_by", "left, right")]),
            ("tenet::left", &[]),
            ("tenet::right", &[]),
        ])
    }

    fn write(root: &Path, rel: &str, text: &str) {
        let p = root.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn run(root: &Path, canon_paths: &[String]) -> Vec<LintError> {
        let reg = canon();
        let crates = BTreeSet::from(["alpha".to_string()]);
        let ctx = RepoContext {
            mock_dir: &root.join("mock"),
            repo_root: root,
            all_crates: &crates,
            src_dirs: &[],
            invocation: None,
            canon_paths,
            open_panels: &[],
            registry: &reg,
        };
        NoSupersededCitation.check_repo(&ctx)
    }

    fn messages(root: &Path) -> Vec<String> {
        run(root, &[]).into_iter().map(|e| e.message).collect()
    }

    fn dir(tag: &str) -> tempfile::TempDir {
        tempfile::Builder::new().prefix(tag).tempdir().unwrap()
    }

    #[test]
    fn the_default_blocks_every_gate() {
        assert_eq!(NoSupersededCitation.default_severity(), Severity::HARD_ERROR);
    }

    #[test]
    fn a_design_citing_a_superseded_row_is_reported_at_its_line_with_the_live_successor() {
        let d = dir("design");
        write(
            d.path(),
            "mock/crates/alpha/DESIGN.md.tmpl",
            "# alpha\n\nIt stands after `old`, and nothing else.\n",
        );
        let found = run(d.path(), &[]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].line, 3);
        assert_eq!(found[0].crate_name, "alpha");
        assert_eq!(
            found[0].path.as_deref(),
            Some("mock/crates/alpha/DESIGN.md.tmpl")
        );
        assert!(found[0].message.contains("`old`"), "{:?}", found[0].message);
        assert!(found[0].message.contains("`new`"), "{:?}", found[0].message);
        assert!(!found[0].message.contains("`mid`"), "{:?}", found[0].message);
        assert_eq!(found[0].severity, Severity::HARD_ERROR);
    }

    #[test]
    fn source_citing_a_superseded_row_is_reported_bare_or_backticked() {
        let d = dir("source");
        write(
            d.path(),
            "mock/crates/alpha/src/lib.rs",
            "//! After old.\nfn f() {} // `old` again\n",
        );
        let found = run(d.path(), &[]);
        assert_eq!(found.len(), 2, "{found:?}");
        assert_eq!(found[0].line, 1);
        assert_eq!(found[1].line, 2);
    }

    #[test]
    fn a_row_split_into_two_names_both_successors() {
        let d = dir("split");
        write(d.path(), "mock/crates/alpha/src/lib.rs", "//! After `split`.\n");
        let m = messages(d.path());
        assert_eq!(m.len(), 1, "{m:?}");
        assert!(m[0].contains("`left`") && m[0].contains("`right`"), "{m:?}");
    }

    #[test]
    fn a_live_row_and_a_longer_word_that_holds_the_slug_are_not_citations() {
        let d = dir("live");
        write(
            d.path(),
            "mock/crates/alpha/DESIGN.md.tmpl",
            "After `new`, and `older`, and `not_old`, and `old_enough`.\n",
        );
        assert!(messages(d.path()).is_empty());
    }

    #[test]
    fn a_sentence_saying_the_row_is_superseded_may_name_it() {
        let d = dir("said");
        write(
            d.path(),
            "mock/crates/alpha/DESIGN.md.tmpl",
            "This replaces what `old` said, which is superseded by `new`. It also leans on\n`old` here.\n",
        );
        let found = run(d.path(), &[]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].line, 2);
    }

    #[test]
    fn the_canon_the_rounds_the_research_and_generated_files_are_not_read() {
        let d = dir("skipped");
        write(d.path(), "mock/registry/tenet/a.toml", "what = \"old\"\n");
        write(d.path(), "mock/design_rounds/1_topic.md", "old\n");
        write(d.path(), "mock/research/1_x/NOTES.md", "old\n");
        write(d.path(), "mock/target/debug/x.rs", "// old\n");
        write(d.path(), "vendor/x/lib.rs", "// old\n");
        write(
            d.path(),
            "docs/ALPHA.md",
            "<!--\n  AUTO-GENERATED: DO NOT EDIT\n-->\nold\n",
        );
        let canon_paths = vec!["mock/registry/tenet/*.toml".to_string()];
        let found = run(d.path(), &canon_paths);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn content_and_root_files_outside_the_crates_are_read() {
        let d = dir("content");
        write(d.path(), "content/pack/items.toml", "# after old\n");
        write(d.path(), "mock/DESIGN.md.tmpl", "After `old`.\n");
        let found = run(d.path(), &[]);
        assert_eq!(found.len(), 2, "{found:?}");
    }

    #[test]
    fn a_registry_with_nothing_superseded_finds_nothing() {
        let d = dir("none");
        write(d.path(), "mock/DESIGN.md.tmpl", "After `old`.\n");
        let reg = registry(&[("tenet::old", &[("what", "x")])]);
        let crates = BTreeSet::new();
        let ctx = RepoContext {
            mock_dir: &d.path().join("mock"),
            repo_root: d.path(),
            all_crates: &crates,
            src_dirs: &[],
            invocation: None,
            canon_paths: &[],
            open_panels: &[],
            registry: &reg,
        };
        assert!(NoSupersededCitation.check_repo(&ctx).is_empty());
    }
}
