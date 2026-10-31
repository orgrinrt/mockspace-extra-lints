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
//! What it counts as a citation: the row's slug as a whole word, where a hyphen is part of the
//! word, and a slug that is one plain lowercase word only when backticked. What it lets through: a
//! sentence that says the row is superseded, kept inside its block and, in a file that is not prose,
//! inside its line, since a design may explain what a row was replaced by without relying on it. A
//! slug that a live row of another namespace also carries is not read, since a text cannot tell them
//! apart. The canon files are the ones `canon_paths` names in `mockspace.toml`, matched with the same
//! glob as the engine matches them with, so a bare name there reaches any depth.
//!
//! A project names the field in its `mockspace.toml`, under `[lints.no-superseded-citation]`,
//! because what a registry calls its replacement edge is the project's own choice and nothing here
//! knows it. The edge runs one of two ways and the project names whichever its registry has, or
//! both: `field` is carried by the replaced row and lists its successors (`superseded_by`),
//! `supersedes_field` is carried by the replacing row and lists what it replaces (`supersedes`).
//! There is no default: a project that has named neither gets one finding at `mockspace.toml`
//! while the registry has rows, and a registry with no row carrying a named field declares nothing
//! superseded.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;

use mockspace_lint_rules::{glob_match, Lint, LintError, RegistryView, RepoContext, RepoLint, Severity};

use crate::repo_files::{paths, record_dirs};

/// The lint's name, as a project's `[lints.*]` table spells it.
pub const NAME: &str = "no-superseded-citation";

/// The key under `[lints.no-superseded-citation]` that names the field a registry row carries to say
/// what replaced it.
const FIELD_KEY: &str = "field";

/// The key under `[lints.no-superseded-citation]` that names the field a registry row carries to say
/// which older rows it replaces, for a registry whose edge runs from the new row to the old one.
const SUPERSEDES_KEY: &str = "supersedes_field";

/// The separator the engine joins a list field with.
const JOIN: &str = ", ";

/// The extensions whose files are read.
const READ: &[&str] = &["rs", "tmpl", "md", "toml", "slang"];

/// What a generated file carries in its first lines.
const GENERATED: &str = "AUTO-GENERATED";

/// How many lines at the top of a file the generated marker may sit in.
const HEADER: usize = 12;

/// The configured lint. Each field is empty until the project names it, and the lint reads the edge
/// in each direction it is given: `field` runs from the replaced row to its successors and
/// `supersedes` from the replacing row to what it replaces. With neither named the lint answers
/// with one finding at the config.
#[derive(Default)]
pub struct NoSupersededCitation {
    field: String,
    supersedes: String,
}

impl Lint for NoSupersededCitation {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }

    fn config_keys(&self) -> &[&str] {
        &[FIELD_KEY, SUPERSEDES_KEY]
    }

    fn configure(&mut self, params: &HashMap<String, String>) {
        if let Some(v) = params.get(FIELD_KEY) {
            self.field = v.trim().to_string();
        }
        if let Some(v) = params.get(SUPERSEDES_KEY) {
            self.supersedes = v.trim().to_string();
        }
    }
}

impl RepoLint for NoSupersededCitation {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        if self.field.is_empty() && self.supersedes.is_empty() {
            return unconfigured(ctx.registry);
        }
        let replaced = replaced(ctx.registry, &self.field, &self.supersedes);
        if replaced.is_empty() {
            return Vec::new();
        }
        let record = record_dirs(ctx.repo_root, ctx.mock_dir);
        let files = paths(ctx.repo_root, &record, |name, rel| {
            reads(name) && !ctx.canon_paths.iter().any(|g| glob_match(g, rel))
        });
        // The one fact every file's verdict shares: which rows are replaced and by what. A file's
        // findings follow from its own text and this, so a file unchanged under an unchanged table
        // is not read again.
        let table = format!("{replaced:?}");
        let mut read: Vec<(String, String, String)> = Vec::new();
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
            read.push((rel, crate_name, text));
        }
        // Every file is asked at once, so the ones that moved are judged together on the machine's
        // threads. The keys are the ones asking per file made, so saved verdicts replay across.
        let parts: Vec<Vec<&[u8]>> = read
            .iter()
            .map(|(rel, crate_name, text)| {
                vec![rel.as_bytes(), crate_name.as_bytes(), table.as_bytes(), text.as_bytes()]
            })
            .collect();
        ctx.remembered_batch(NAME, &parts, |n| {
            let (rel, crate_name, text) = &read[n];
            file_findings(rel, crate_name, text, &replaced)
        })
        .into_iter()
        .flatten()
        .collect()
    }
}

/// The findings in one file's text against the table of replaced rows.
fn file_findings(
    rel: &str,
    crate_name: &str,
    text: &str,
    replaced: &BTreeMap<String, Vec<String>>,
) -> Vec<LintError> {
    let mut out = Vec::new();
    let per_line = !(rel.ends_with(".md") || rel.ends_with(".tmpl"));
    for (line, old, successors) in citations(text, replaced, per_line) {
        let message = if successors.is_empty() {
            format!(
                "`{rel}` cites `{old}`, whose replacements lead back to it and never to a row that binds; repair the registry"
            )
        } else {
            format!(
                "`{rel}` cites `{old}`, which {} supersede{}; write to the successor, or say in the sentence that the row is superseded",
                names(&successors),
                if successors.len() == 1 { "s" } else { "" },
            )
        };
        let mut e = LintError::error(crate_name.to_string(), line, NAME, message);
        e.path = Some(rel.to_string());
        out.push(e);
    }
    out
}

/// The one finding of a project whose registry has rows and which has not named the field, since
/// the lint would otherwise answer nothing and look like a pass.
fn unconfigured(reg: &RegistryView) -> Vec<LintError> {
    if !reg.namespaces().any(|ns| !reg.rows_in(ns).is_empty()) {
        return Vec::new();
    }
    let mut e = LintError::error(
        "workspace".to_string(),
        1,
        NAME,
        format!(
            "the registry has rows but neither `{FIELD_KEY}` nor `{SUPERSEDES_KEY}` is named under `[lints.{NAME}]` in `mockspace.toml`, so no row is read as superseded"
        ),
    );
    e.path = Some("mockspace.toml".to_string());
    vec![e]
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
///
/// The edge is read in each direction a field is named for: `forward` on a row lists its successors,
/// `backward` on a row lists the rows it replaces. A name left empty is not read. A bare name in
/// `backward` is a row of the replacing row's own namespace.
fn replaced(reg: &RegistryView, forward: &str, backward: &str) -> BTreeMap<String, Vec<String>> {
    let mut direct: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut gone: BTreeSet<String> = BTreeSet::new();
    let mut rows: Vec<(&str, &str)> = Vec::new();
    for ns in reg.namespaces() {
        for q in reg.rows_in(ns) {
            rows.push((ns, q));
            let slug = slug_of(q);
            let to = listed(reg, q, forward);
            if !to.is_empty() {
                gone.insert(q.clone());
                for s in to {
                    edge(&mut direct, slug, slug_of(s));
                }
            }
            for old in listed(reg, q, backward) {
                let old_q = if old.contains("::") { old.to_string() } else { format!("{ns}::{old}") };
                gone.insert(old_q);
                edge(&mut direct, slug_of(old), slug);
            }
        }
    }
    // A text names a row by its slug alone, so a slug a live row in another namespace also
    // carries cannot be told apart and is not read as replaced.
    let live: BTreeSet<&str> = rows
        .iter()
        .filter(|(_, q)| !gone.contains(*q))
        .map(|(_, q)| slug_of(q))
        .collect();
    direct.retain(|slug, _| !live.contains(slug.as_str()));
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

/// A row's slug: what follows its last `::`.
fn slug_of(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

/// The names row `q` lists in `field`, none when the field is not named or the row lacks it.
fn listed<'a>(reg: &'a RegistryView, q: &str, field: &str) -> Vec<&'a str> {
    if field.is_empty() {
        return Vec::new();
    }
    reg.field(q, field)
        .unwrap_or_default()
        .split(JOIN)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect()
}

/// Records that `old` is replaced by `new`, once.
fn edge(direct: &mut BTreeMap<String, Vec<String>>, old: &str, new: &str) {
    let to = direct.entry(old.to_string()).or_default();
    if !to.iter().any(|t| t == new) {
        to.push(new.to_string());
    }
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

/// Whether a file of this name is one the lint reads.
fn reads(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, ext)| READ.contains(&ext))
}

/// Whether a character can sit inside a row's slug.
fn id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// Whether a slug is a single plain word, which prose uses for other things and so is only a
/// citation when backticked.
fn plain(slug: &str) -> bool {
    slug.chars().all(|c| c.is_ascii_lowercase())
}

/// Every place the text names a replaced row as a whole word, outside a sentence that says it is
/// superseded: its line, the row, and what stands in its place. A slug that is a plain word counts
/// only when backticked. With `per_line`, the sentence is cut at the line, for files that are not
/// prose.
fn citations(
    text: &str,
    replaced: &BTreeMap<String, Vec<String>>,
    per_line: bool,
) -> Vec<(usize, String, Vec<String>)> {
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
                if plain(slug) && !(before == Some('`') && after == Some('`')) {
                    continue;
                }
                if !says_superseded(&lines, i, at, slug.len(), per_line) {
                    out.push((i + 1, slug.clone(), live.clone()));
                }
            }
        }
    }
    out.sort();
    out
}

/// Whether a line opens a block of its own: a heading, a table row or a list item.
fn block_start(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with(['#', '|'])
        || ["- ", "* ", "+ "].iter().any(|m| t.starts_with(m))
}

/// Whether a line is a block by itself: a heading or a table row.
fn alone(line: &str) -> bool {
    line.trim_start().starts_with(['#', '|'])
}

/// Whether the sentence around the match, kept inside its block, says "superseded".
fn says_superseded(lines: &[&str], line: usize, at: usize, len: usize, per_line: bool) -> bool {
    let (mut first, mut last) = (line, line);
    if !per_line {
        while first > 0 && !lines[first - 1].trim().is_empty() && !block_start(lines[first]) && !alone(lines[first - 1]) {
            first -= 1;
        }
        while last + 1 < lines.len() && !lines[last + 1].trim().is_empty() && !block_start(lines[last + 1]) && !alone(lines[last]) {
            last += 1;
        }
    }
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
mod replay;
#[cfg(test)]
mod tests;
