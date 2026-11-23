//! The registry's side of a search: the rows that carry the question, the keywords they lend it,
//! the documents the rows cite, and the day the canon last spoke on the subject.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use corpus::Query;
use mockspace::tool::ToolContext;
use muisti::repo::RepoIndex;

/// Fields that are citations rather than prose. A hit in one says a document or a
/// row was named, not that a subject was discussed: `provenance` names the note a
/// row came from, and `cites`, `superseded_by` and `subject` name other rows.
///
/// **They are excluded from the text search and read for something else.** A
/// provenance entry names the file a row came from, so the set of them is the
/// set of research this registry has actually absorbed, and the difference
/// between that set and what a search returns is the transcription nobody has
/// done. That difference is the whole output of this tool.
pub(crate) const NOT_PROSE: &[&str] =
    &["provenance", "cites", "superseded_by", "subject", "related"];

/// How many registry keywords may join the question.
///
/// A cap rather than a filter on quality, because quality is not decidable here:
/// a keyword is a word somebody thought worth searching for, and the tool cannot
/// tell which of a row's fifteen bears on this question. What it can do is prefer
/// the rows that carry most of the question and stop at a number where the
/// ranking still discriminates.
pub(crate) const EXPANSION: usize = 24;

/// The day the canon last changed on this subject: the latest commit to any file that holds a
/// row binding it. A research note older than this was written before the canon last spoke.
pub(crate) fn canon_changed(index: &RepoIndex, binding: &[String]) -> Option<i64> {
    binding
        .iter()
        .filter_map(|q| index.row(q).ok().flatten())
        .map(|r| r.path)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .filter_map(|file| index.changed_day(&file))
        .max()
}

/// Each row as the report names it, a superseded tenet followed by what supersedes it.
///
/// A superseded tenet stays whole in the canon so its replacement is read with it in
/// view. Listed bare beside the rows that bind, it reads as one of them.
pub(crate) fn marked(ctx: &ToolContext<'_>, rows: &[String]) -> Vec<String> {
    rows.iter()
        .map(|q| {
            match ctx.registry.field(q, "superseded_by").map(str::trim) {
                Some(s) if !s.is_empty() => {
                    format!("{q}  superseded by {}", s.trim_matches(['[', ']', '"']))
                },
                _ => q.clone(),
            }
        })
        .collect()
}

/// Rows carrying the subject, and every keyword those rows declare.
///
/// The keywords come from the rows that matched, not from the whole registry. A
/// question about licensing should not be expanded with the vocabulary of the
/// boot chain, and expanding from everything is how a search stops
/// discriminating.
pub(crate) fn registry_pass(
    ctx: &ToolContext<'_>,
    index: &RepoIndex,
    probe: &Query,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    // The same expansion the corpus gets, applied to rows. Matching the raw
    // phrase as a substring was the first version and it failed on the one
    // question this tool was built for: no row contains the string "two
    // surfaces data store", so nothing matched, nothing was harvested, and the
    // corpus search ran with no registry vocabulary at all. A row is a document
    // like any other and is matched like one.
    let mut scored: Vec<(usize, String, Vec<String>)> = Vec::new();

    let namespaces: Vec<String> = ctx.registry.namespaces().map(str::to_string).collect();
    for ns in &namespaces {
        for q in ctx.registry.rows_in(ns) {
            let Some(row) = ctx.registry.row(q) else {
                continue;
            };
            let mut prose = q.replace('_', " ");
            let mut declared: Vec<String> = Vec::new();
            for (name, value) in row {
                if NOT_PROSE.contains(&name.as_str()) {
                    continue;
                }
                if name == "keywords" {
                    // A `string[]` arrives as one rendered value, so both
                    // separators are split and the quoting stripped.
                    for k in value.split(['\n', ',']) {
                        let k = k.trim().trim_matches(['"', '[', ']', ' ']);
                        if k.len() > 2 {
                            declared.push(k.to_lowercase());
                        }
                    }
                }
                prose.push(' ');
                prose.push_str(value);
            }
            let prose = prose.to_lowercase();
            let hits = probe
                .terms
                .iter()
                .filter(|t| !Query::find(&t.stem, &prose).is_empty())
                .count();
            if hits > 0 {
                scored.push((hits, q.clone(), declared));
            }
        }
    }

    scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let rows: Vec<String> = scored.iter().map(|(_, q, _)| q.clone()).collect();

    // Keywords come only from the rows that carry most of the question. A row
    // matching one common word of it is not about the subject, and harvesting
    // from every such row is how a search stops discriminating: on this
    // registry, taking keywords from every partial match produced ninety terms
    // and a ranking indistinguishable from a word count.
    let best = scored.first().map_or(0, |(h, ..)| *h);
    let floor = best.saturating_sub(1).max(1);
    let binding: Vec<String> = scored
        .iter()
        .filter(|(h, ..)| *h >= floor)
        .map(|(_, q, _)| q.clone())
        .collect();
    // Ranked by how much of the question their own row carries, then capped.
    // Uncapped, one broad row's keyword list put `cars`, `guitars` and `kitchen
    // appliances` into a search about a data store, and ninety terms is a
    // ranking that no longer discriminates.
    let mut ranked: Vec<(usize, String)> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (hits, _, declared) in &scored {
        if *hits < floor {
            continue;
        }
        for k in declared {
            if seen.insert(k.clone()) {
                ranked.push((*hits, k.clone()));
            }
        }
    }
    // How many documents carry a keyword, asked of the same index the ranking
    // will use. A phrase is not a key there, so it is measured on its rarest
    // word: `mere aggregation` is as rare as `aggregation` at worst, and taking
    // the rarest is the closest single-key answer available without scanning.
    let df = |k: &str| {
        corpus::terms::words(k)
            .iter()
            .map(|w| index.doc_frequency(w).unwrap_or(0))
            .min()
            .unwrap_or(0)
    };
    (rows, rank_keywords(ranked, df, EXPANSION), binding)
}

/// Which of the harvested keywords are worth a slot in the question.
///
/// **Ordered by the row's own hits first, then by how much a keyword
/// discriminates.** The row score stays the primary key because a keyword from a
/// row carrying two words of the question is about the subject, while a rare word
/// from a row that barely matched is a coincidence.
///
/// The tie-break is what this fixes. It was alphabetical, and on a one-word
/// subject every matching row ties at one hit, so the tie-break decided the whole
/// expansion and the cap fell wherever the alphabet ran out. A licensing question
/// searched `aggregate`, `bundling` and `copyleft`, and never searched `gpl`,
/// `licence`, `mpl` or `spdx`, which sit on the same rows and are the words the
/// subject is actually argued in.
///
/// **Rarest first among the terms the corpus carries, and terms it carries
/// nowhere go last.** Rarity is the discriminating power the ranker already
/// weighs, so this picks the keywords that will decide a placement rather than
/// the ones that will match everything. Sorting purely by rarity would fill every
/// slot with terms present in no document, which searches for nothing.
pub(crate) fn rank_keywords(
    mut candidates: Vec<(usize, String)>,
    df: impl Fn(&str) -> usize,
    cap: usize,
) -> Vec<String> {
    let mut ranked: Vec<(usize, usize, String)> = candidates
        .drain(..)
        .map(|(hits, k)| {
            let d = df(&k);
            // A term in no document is dead: last, and ordered among its own
            // kind by the alphabet so a run is reproducible.
            (hits, if d == 0 { usize::MAX } else { d }, k)
        })
        .collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    ranked.truncate(cap);
    ranked.into_iter().map(|(_, _, k)| k).collect()
}

/// Every research file the registry cites, and how many rows cite it.
///
/// **The join that makes this tool answer a question rather than list files.** A
/// document the registry already cites has been read by somebody who wrote a row
/// from it; one it has never cited is material this canon has not absorbed, and
/// telling those apart is the difference between a search and a worklist.
///
/// A citation is `root::segment::segment::#anchor`, and the roots are declared in
/// `mockspace.toml` rather than handed to a tool, so both shapes are tried and
/// whichever names a real document wins. That mirrors what `provenance-sweep`
/// does with the same problem, and for the same reason: a second copy of the
/// root table here would drift from the first with nothing reporting it.
pub(crate) fn cited_files(ctx: &ToolContext<'_>) -> BTreeMap<String, usize> {
    let mock = mock_prefix(ctx.mock_dir, ctx.repo_root);
    let mut cited: BTreeMap<String, usize> = BTreeMap::new();
    for ns in ctx.registry.namespaces().collect::<Vec<_>>() {
        for q in ctx.registry.rows_in(ns) {
            let Some(row) = ctx.registry.row(q) else {
                continue;
            };
            for (name, value) in row {
                if !NOT_PROSE.contains(&name.as_str()) {
                    continue;
                }
                for entry in value.split(['\n', ',']) {
                    if let Some(path) = citation_path(entry, |p| ctx.mock_dir.join(p).is_file())
                        .map(|p| join(&mock, &p))
                    {
                        *cited.entry(path).or_insert(0) += 1;
                    }
                }
            }
        }
    }
    cited
}

/// Where a provenance citation points, from the mock directory, given a way to ask what exists.
///
/// A citation is `root::segment::segment::#anchor` and the roots are declared in
/// `mockspace.toml`, which a tool is handed no parsed copy of. So both shapes
/// are tried and whichever names a real document wins: the root's own directory
/// under `research/`, and `research/` itself, which is what the panel root is.
#[must_use]
pub fn citation_path(entry: &str, exists: impl Fn(&str) -> bool) -> Option<String> {
    let entry = entry.trim().trim_matches(['"', '[', ']', ' ']);
    let parts: Vec<&str> = entry.split("::").collect();
    // Three at minimum: a root, at least one path segment, and an anchor. Fewer
    // is not a citation and is left alone rather than guessed at.
    if parts.len() < 3 {
        return None;
    }
    let root = parts[0];
    let tail = parts[1 .. parts.len() - 1].join("/");
    for base in [format!("research/{root}"), "research".to_string()] {
        let candidate = format!("{base}/{tail}.md");
        if exists(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// The mock directory as a path from the repository's root, the prefix the index shows its
/// documents under.
#[must_use]
pub fn mock_prefix(mock_dir: &Path, repo_root: &Path) -> String {
    mock_dir
        .strip_prefix(repo_root)
        .unwrap_or(mock_dir)
        .to_string_lossy()
        .replace('\\', "/")
        .trim_matches('/')
        .to_string()
}

/// Two relative paths joined, the first of them possibly empty.
fn join(a: &str, b: &str) -> String {
    if a.is_empty() { b.to_string() } else { format!("{a}/{b}") }
}
