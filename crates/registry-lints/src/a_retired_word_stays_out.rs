//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `a-retired-word-stays-out`: no design, code or content line names a word a binding tenet rules out.
//!
//! A tenet lists in `rules_out` the words and names it rules out. A design can cite that tenet by id, and
//! pass every check that reads ids, while describing the very thing the tenet rules out, so this reads the
//! words. What is read, how a word is matched and what lets a line through is `retired_words.rs`, which
//! the `canon` tool's verb `contradicts` reads as well: the verb reports the whole of it, and this gates.
//!
//! The mutation order forbids rewriting a design before the code beneath it is gone, so a word a tenet has
//! only just ruled out is in many places that cannot be cleared in one change. The units still holding it
//! are named in `exempt_crates` of this lint's table in `mockspace.toml`, a unit being a crate by its
//! directory name, `lints`, `tools`, `benches`, a content directory or `workspace`. The list is the debt, named, and
//! it only shrinks: a unit leaves it in the commit that clears its last line, a unit on it that holds no
//! line any more is a finding, and a unit that is not on it is held from its first line. Other gates here
//! ratchet by naming the clean crates they bind; this one names the unclean ones, because a new crate is
//! then bound from the start rather than missed until somebody lists it.
//!
//! A unit on the list is let through only for the lines it already holds. A line a change adds or touches
//! names no retired word in any unit, listed or not, so a listed unit can only shrink its count, and no name
//! joins the list: a name on it that the change's base did not carry is a finding. What a change adds or
//! touches is read from git by `touched_lines.rs`, the work tree against where the change started. Where
//! that cannot be read, the lint says so as a finding of its own and the units off the list are held all
//! the same.
//!
//! The lint reads the whole repository, so the engine saves no verdict for it as a whole. What it finds in
//! one file depends on that file's path and text and on the words the binding tenets rule out, and on
//! nothing else, so each scanned file's findings are saved on those three and replayed while they stand.
//! The debt list is applied afterwards, outside what is saved, which is why a unit leaving it costs no
//! file a second reading.

use std::borrow::Cow;
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use mockspace::{Item, Lint, LintError, RegistryView, RepoContext, RepoLint, Severity, View};

use crate::canon_rows::list;
use crate::canon_tree::files;
use crate::gate_config::table;
use crate::layout::{CONTENT_DIRS, ContentDirs, Layout};
use crate::retired_words::{Retiring, scanned, split_text, unit_of};
use crate::standing::binding;
use crate::touched_lines::{self, Touched};

pub fn repo_lint() -> Box<dyn RepoLint> {
    Box::new(ARetiredWordStaysOut::default())
}

const NAME: &str = "a-retired-word-stays-out";

/// The lint, holding the content directories its table names, which are read as the content tier.
#[derive(Default)]
struct ARetiredWordStaysOut {
    content: ContentDirs,
}

impl Lint for ARetiredWordStaysOut {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }

    fn config_keys(&self) -> &[&str] {
        &[CONTENT_DIRS]
    }

    fn configure(&mut self, params: &HashMap<String, String>) {
        self.content.configure_lint(params);
    }
}

impl RepoLint for ARetiredWordStaysOut {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        let view = ctx.view();
        let repo = ctx.repo_root;
        let config = view.text(repo.join("mockspace.toml")).unwrap_or_default();
        let (change, unread) = match change_of(view, repo) {
            Ok(change) => (change, None),
            Err(why) => (Change::default(), Some(why)),
        };
        let layout = Layout::of(ctx.mock_dir, repo, &self.content);
        let mut out = check(view, repo, ctx.registry(), config, &change, &layout);
        out.extend(unread.map(|why| unreadable(&why)));
        out
    }
}

/// What the change under judgement adds or touches, and the debt list it started from.
#[derive(Debug, Default)]
struct Change {
    /// The lines the change adds or touches.
    touched:   Touched,
    /// The names `exempt_crates` held at the change's base, where it is known.
    base_debt: Option<BTreeSet<String>>,
}

/// The change at `repo` as the lint judges it.
fn change_of(view: &View<'_>, repo: &Path) -> Result<Change, String> {
    let read = touched_lines::read(view, repo)?;
    let at_base = touched_lines::file_at(view, repo, &read.base, "mockspace.toml").unwrap_or_default();
    Ok(Change {
        touched:   read.touched,
        base_debt: Some(table(&at_base, NAME).exempt_crates.into_iter().collect()),
    })
}

/// The finding for a change that could not be read, which the lint raises rather than let the listed
/// units through unwatched.
fn unreadable(why: &str) -> LintError {
    let mut e = LintError::error(
        "workspace".to_owned(),
        0,
        NAME,
        format!(
            "cannot tell which lines this change adds or touches, so a listed unit is not held to them: {why}"
        ),
    );
    e.path = Some("mockspace.toml".to_owned());
    e
}

/// The binding tenets that rule out a word, as the scan reads them.
pub fn retiring(reg: &RegistryView) -> Vec<Retiring> {
    reg.rows_in("tenet")
        .iter()
        .filter(|q| binding(reg, q))
        .filter_map(|q| {
            let terms = list(reg, q, "rules_out");
            (!terms.is_empty()).then(|| {
                Retiring {
                    id:    q.rsplit("::").next().unwrap_or(q).to_owned(),
                    terms: terms.into_iter().map(str::to_owned).collect(),
                }
            })
        })
        .collect()
}

/// The rules as bytes, so a verdict saved under one set of banned words is not read under another.
fn fingerprint(rules: &[Retiring]) -> String {
    rules
        .iter()
        .map(|r| format!("{}\0{}", r.id, r.terms.join("\0")))
        .collect::<Vec<_>>()
        .join("\u{1}")
}

/// Every line of one file that names a word a tenet rules out, held or not by the debt list, as the
/// findings the lint would raise for a unit not on it. Depends on the file's path and text and on the
/// rules, and on nothing else, which is what lets it be saved per file.
fn judge(rel: &Path, body: &str, rules: &[Retiring], layout: &Layout) -> Vec<LintError> {
    split_text(rel, body, rules, layout)
        .0
        .into_iter()
        .map(|hit| {
            let path = hit.path.to_string_lossy().replace('\\', "/");
            let mut e = LintError::error(
                unit_of(&hit.path, layout),
                hit.line,
                NAME,
                format!(
                    "`{path}` names `{}`, which `{}` rules out: `{}`. Rewrite the line; `mock canon \
                     contradicts {path}` lists the rest of the file",
                    hit.term, hit.rule, hit.text
                ),
            );
            e.path = Some(path);
            e
        })
        .collect()
}

/// One scanned file, judged through a view of its own. Its key holds what the judgement takes from
/// outside the file: the path, which the file's text is read by, and the rules it is judged under.
struct Scanned {
    rel: PathBuf,
    key: String,
}

impl Item for Scanned {
    fn key(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.key)
    }
}

/// Every scanned file under `repo`, keyed under `rules` and the layout it is read by.
fn scanned_files(view: &View<'_>, repo: &Path, rules: &[Retiring], layout: &Layout) -> Vec<Scanned> {
    let rules_key = format!("{}\u{3}{layout:?}", fingerprint(rules));
    files(view, repo)
        .into_iter()
        .filter(|rel| scanned(rel, layout))
        .map(|rel| {
            let key = format!("{rules_key}\u{2}{}", rel.to_string_lossy().replace('\\', "/"));
            Scanned {
                rel,
                key,
            }
        })
        .collect()
}

/// The findings of every scanned file, each judged through its own view, so a file whose text is what
/// it was under the same rules is replayed rather than judged again.
fn judge_files(view: &View<'_>, repo: &Path, rules: &[Retiring], layout: &Layout) -> Vec<LintError> {
    view.each(&scanned_files(view, repo, rules, layout), |v, file| {
        let Some(body) = v.text(repo.join(&file.rel)) else {
            return Vec::new();
        };
        judge(&file.rel, body, rules, layout)
    })
    .into_iter()
    .flatten()
    .collect()
}

fn check(
    view: &View<'_>,
    repo: &Path,
    reg: &RegistryView,
    config: &str,
    change: &Change,
    layout: &Layout,
) -> Vec<LintError> {
    let rules = retiring(reg);
    if rules.is_empty() {
        return Vec::new();
    }
    let debt: BTreeSet<String> = table(config, NAME).exempt_crates.into_iter().collect();
    let mut holding: BTreeSet<String> = BTreeSet::new();
    let mut out = Vec::new();
    for mut e in judge_files(view, repo, &rules, layout) {
        holding.insert(e.crate_name.clone());
        if !debt.contains(&e.crate_name) {
            out.push(e);
        } else if e
            .path
            .as_deref()
            .is_some_and(|p| change.touched.holds(p, e.line))
        {
            e.message = format!(
                "{} `{}` is on the debt list of `[lints.{NAME}]`, which lets the lines it already holds stand and \
                 no line a change adds or touches",
                e.message, e.crate_name
            );
            out.push(e);
        }
    }
    if let Some(base) = &change.base_debt {
        for unit in debt.difference(base) {
            let mut e = LintError::error(
                unit.clone(),
                0,
                NAME,
                format!(
                    "`{unit}` is added to `exempt_crates` of `[lints.{NAME}]`, which only shrinks: a unit the \
                     change's base did not list is held from its first line, so clear its lines instead"
                ),
            );
            e.path = Some("mockspace.toml".to_owned());
            out.push(e);
        }
    }
    for unit in debt.difference(&holding) {
        let mut e = LintError::error(
            unit.clone(),
            0,
            NAME,
            format!(
                "`{unit}` is listed in `exempt_crates` of `[lints.{NAME}]` and holds no line naming a word a \
                 tenet rules out; remove it from the list, which only shrinks"
            ),
        );
        e.path = Some("mockspace.toml".to_owned());
        out.push(e);
    }
    out
}

#[cfg(test)]
#[path = "a_retired_word_stays_out_tests.rs"]
mod tests;
