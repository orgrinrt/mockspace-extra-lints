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
//! directory name, `lints`, `tools`, `benches`, `content` or `workspace`. The list is the debt, named, and
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
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mockspace::{Item, Lint, LintError, RegistryView, RepoContext, RepoLint, Severity, View};

use crate::canon_rows::list;
use crate::canon_tree::files;
use crate::gate_config::table;
use crate::retired_words::{Retiring, scanned, split_text, unit_of};
use crate::touched_lines::{self, Touched};

pub fn repo_lint() -> Box<dyn RepoLint> {
    Box::new(ARetiredWordStaysOut)
}

const NAME: &str = "a-retired-word-stays-out";

struct ARetiredWordStaysOut;

impl Lint for ARetiredWordStaysOut {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
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
        let mut out = check(view, repo, ctx.registry(), config, &change);
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
        .filter(|q| list(reg, q, "superseded_by").is_empty())
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
fn judge(rel: &Path, body: &str, rules: &[Retiring]) -> Vec<LintError> {
    split_text(rel, body, rules)
        .0
        .into_iter()
        .map(|hit| {
            let path = hit.path.to_string_lossy().replace('\\', "/");
            let mut e = LintError::error(
                unit_of(&hit.path),
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

/// Every scanned file under `repo`, keyed under `rules`.
fn scanned_files(view: &View<'_>, repo: &Path, rules: &[Retiring]) -> Vec<Scanned> {
    let rules_key = fingerprint(rules);
    files(view, repo)
        .into_iter()
        .filter(|rel| scanned(rel))
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
fn judge_files(view: &View<'_>, repo: &Path, rules: &[Retiring]) -> Vec<LintError> {
    view.each(&scanned_files(view, repo, rules), |v, file| {
        let Some(body) = v.text(repo.join(&file.rel)) else {
            return Vec::new();
        };
        judge(&file.rel, body, rules)
    })
    .into_iter()
    .flatten()
    .collect()
}

fn check(view: &View<'_>, repo: &Path, reg: &RegistryView, config: &str, change: &Change) -> Vec<LintError> {
    let rules = retiring(reg);
    if rules.is_empty() {
        return Vec::new();
    }
    let debt: BTreeSet<String> = table(config, NAME).exempt_crates.into_iter().collect();
    let mut holding: BTreeSet<String> = BTreeSet::new();
    let mut out = Vec::new();
    for mut e in judge_files(view, repo, &rules) {
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
mod tests {
    use mockspace::reading::Reading;

    use super::*;
    use crate::canon_rows::testkit::{check as run_lint, git, scratch, view, write};

    /// What [`check`] finds over `d`, read through a reading of its own.
    fn checked(d: &Path, reg: &RegistryView, config: &str, change: &Change) -> Vec<LintError> {
        check(&Reading::new(d).view(), d, reg, config, change)
    }

    fn reg() -> RegistryView {
        view(&[
            ("tenet::there_is_no_zorblax", &[("rules_out", "zorblax")]),
            ("tenet::the_zorblax_grows", &[
                ("rules_out", "grows"),
                ("superseded_by", "there_is_no_zorblax"),
            ]),
            ("tenet::the_ground_is_one", &[("what", "One ground.")]),
        ])
    }

    fn config(exempt: &[&str]) -> String {
        let list: Vec<String> = exempt.iter().map(|u| format!("\"{u}\"")).collect();
        format!("[lints.{NAME}]\nexempt_crates = [{}]\n", list.join(", "))
    }

    fn repo() -> std::path::PathBuf {
        let d = scratch("retired-word");
        write(
            &d,
            "mock/crates/kaski-a/DESIGN.md.tmpl",
            "a zorblax stands here\n",
        );
        write(
            &d,
            "mock/crates/kaski-a/src/lib.rs",
            "pub struct Quillfern;\n",
        );
        write(
            &d,
            "mock/crates/kaski-b/src/lib.rs",
            "pub struct ZorblaxCell;\n",
        );
        write(&d, "mock/research/x/NOTES.md", "the zorblax was a window\n");
        d
    }

    #[test]
    fn a_unit_not_on_the_list_is_held_from_its_first_line() {
        let d = repo();
        let found = checked(&d, &reg(), &config(&[]), &Change::default());
        let units: Vec<&str> = found.iter().map(|e| e.crate_name.as_str()).collect();
        assert_eq!(units, ["kaski-a", "kaski-b"], "{found:?}");
        assert!(found[0].message.contains("there_is_no_zorblax"));
    }

    #[test]
    fn a_unit_on_the_list_is_let_through_and_the_others_still_fail() {
        let d = repo();
        let found = checked(&d, &reg(), &config(&["kaski-a"]), &Change::default());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].crate_name, "kaski-b");
    }

    #[test]
    fn a_unit_on_the_list_with_nothing_left_is_a_finding_so_the_list_only_shrinks() {
        let d = repo();
        let found = checked(
            &d,
            &reg(),
            &config(&["kaski-a", "kaski-b", "kaski-c"]),
            &Change::default(),
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].message.contains("`kaski-c` is listed"));
    }

    #[test]
    fn the_control_a_tree_that_conforms_has_no_finding_and_a_superseded_tenet_binds_nothing() {
        let d = scratch("retired-word-clean");
        write(
            &d,
            "mock/crates/kaski-a/DESIGN.md.tmpl",
            "the quillfern stands, and it grows\n",
        );
        assert!(checked(&d, &reg(), &config(&[]), &Change::default()).is_empty());
        let none = view(&[("tenet::the_ground_is_one", &[("what", "One ground.")])]);
        assert!(checked(&repo(), &none, &config(&[]), &Change::default()).is_empty());
    }


    #[test]
    fn a_files_key_moves_with_the_rules_so_a_tenet_ruling_out_another_word_judges_every_file_again() {
        // Mutation: the key leaves the rules out, so a finding saved under one set of words replays under another.
        let d = repo();
        let keys = |reg: &RegistryView| -> Vec<String> {
            scanned_files(&Reading::new(&d).view(), &d, &retiring(reg))
                .into_iter()
                .map(|f| f.key)
                .collect()
        };
        let wider = view(&[("tenet::there_is_no_quillfern", &[("rules_out", "quillfern")])]);
        let (before, again, after) = (keys(&reg()), keys(&reg()), keys(&wider));
        assert_eq!(before.len(), 3, "{before:?}");
        assert_eq!(before, again, "the same rules over the same files keep their keys");
        assert!(before.iter().all(|k| !after.contains(k)), "{before:?} {after:?}");
    }

    #[test]
    fn the_lint_judging_each_file_through_its_own_view_finds_what_the_tree_holds() {
        let d = repo();
        write(&d, "mockspace.toml", &config(&[]));
        let units: Vec<String> = run_lint(&ARetiredWordStaysOut, &d, &reg())
            .into_iter()
            .filter(|e| !e.message.contains("cannot tell which lines"))
            .map(|e| format!("{}:{}", e.crate_name, e.line))
            .collect();
        assert_eq!(units, ["kaski-a:1", "kaski-b:1"]);
    }


    const LIB: &str = "mock/crates/kaski-a/src/lib.rs";

    /// A repository whose `dev` holds `kaski-a` with two lines naming `zorblax` and one that does not, with
    /// `mockspace.toml` listing `exempt`, on a branch `work` cut from it.
    fn on_work(exempt: &[&str]) -> std::path::PathBuf {
        let d = scratch("retired-word-change");
        git(&d, &["init", "-q", "-b", "dev"]);
        write(
            &d,
            LIB,
            "pub struct ZorblaxCell;\npub fn zorblax_a() {}\npub fn calm() {}\n",
        );
        write(&d, "mockspace.toml", &config(exempt));
        git(&d, &["add", "mock", "mockspace.toml"]);
        git(&d, &["commit", "-q", "-m", "base"]);
        git(&d, &["switch", "-q", "-c", "work"]);
        d
    }

    /// What the lint finds over a work tree, the change read from git.
    fn judged(d: &Path) -> Vec<LintError> {
        let change = change_of(&Reading::new(d).view(), d).expect("a change");
        let config = std::fs::read_to_string(d.join("mockspace.toml")).expect("a config");
        checked(d, &reg(), &config, &change)
    }

    #[test]
    fn a_listed_unit_with_no_change_is_let_through_for_the_lines_it_holds() {
        let d = on_work(&["kaski-a"]);
        let found = judged(&d);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_line_a_change_adds_to_a_listed_unit_is_refused_and_the_lines_it_held_still_stand() {
        // Mutation: the touched lines are not consulted, which is the gate that never fired.
        let d = on_work(&["kaski-a"]);
        write(
            &d,
            LIB,
            "pub struct ZorblaxCell;\npub fn zorblax_a() {}\npub fn calm() {}\npub fn zorblax_b() {}\n",
        );
        let found = judged(&d);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(
            (found[0].crate_name.as_str(), found[0].line),
            ("kaski-a", 4)
        );
        assert!(
            found[0].message.contains("adds or touches"),
            "{}",
            found[0].message
        );
        assert!(found[0].message.contains("there_is_no_zorblax"));
    }

    #[test]
    fn a_line_a_change_touches_in_a_listed_unit_is_refused_though_it_named_the_word_before() {
        let d = on_work(&["kaski-a"]);
        write(
            &d,
            LIB,
            "pub struct ZorblaxCell(u32);\npub fn zorblax_a() {}\npub fn calm() {}\n",
        );
        let found = judged(&d);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].line, 1);
    }

    #[test]
    fn a_listed_unit_can_shrink_so_a_change_that_clears_a_line_is_let_through() {
        let d = on_work(&["kaski-a"]);
        write(
            &d,
            LIB,
            "pub struct Cell;\npub fn zorblax_a() {}\npub fn calm() {}\n",
        );
        let found = judged(&d);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_listed_unit_whose_changed_lines_name_nothing_is_let_through() {
        let d = on_work(&["kaski-a"]);
        write(
            &d,
            LIB,
            "pub struct ZorblaxCell;\npub fn zorblax_a() {}\npub fn calm() {}\npub fn quiet() {}\n",
        );
        let found = judged(&d);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_new_file_in_a_listed_unit_is_refused_whole() {
        let d = on_work(&["kaski-a"]);
        write(
            &d,
            "mock/crates/kaski-a/src/more.rs",
            "pub fn calm() {}\npub fn zorblax_c() {}\n",
        );
        let found = judged(&d);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].line, 2);
    }

    #[test]
    fn a_name_added_to_the_list_is_refused_and_one_taken_off_is_not() {
        // Mutation: the list at the base is not read, so every name on it is new or none is.
        let d = on_work(&["kaski-a", "kaski-b"]);
        write(&d, "mockspace.toml", &config(&["kaski-a"]));
        let found = judged(&d);
        assert!(
            found.iter().all(|e| !e.message.contains("is added to")),
            "{found:?}"
        );
        write(
            &d,
            "mockspace.toml",
            &config(&["kaski-a", "kaski-b", "kaski-c"]),
        );
        let found = judged(&d);
        let added: Vec<&str> = found
            .iter()
            .filter(|e| e.message.contains("is added to"))
            .map(|e| e.crate_name.as_str())
            .collect();
        assert_eq!(added, ["kaski-c"], "{found:?}");
    }

    #[test]
    fn a_unit_off_the_list_is_held_from_its_first_line_with_or_without_a_change() {
        let d = on_work(&[]);
        let found = judged(&d);
        let lines: Vec<usize> = found.iter().map(|e| e.line).collect();
        assert_eq!(lines, [1, 2], "{found:?}");
    }

    #[test]
    fn a_change_that_cannot_be_read_is_a_finding_and_the_unlisted_units_are_still_held() {
        // Mutation: an unreadable change is read as none, which lets every listed unit through unwatched.
        let d = repo();
        write(&d, "mockspace.toml", &config(&[]));
        let found = run_lint(&ARetiredWordStaysOut, &d, &reg());
        assert!(
            found
                .iter()
                .any(|e| e.message.contains("cannot tell which lines")),
            "{found:?}"
        );
        assert!(found.iter().any(|e| e.crate_name == "kaski-b"), "{found:?}");
    }

    #[test]
    fn the_default_blocks_every_gate() {
        assert_eq!(
            ARetiredWordStaysOut.default_severity(),
            Severity::HARD_ERROR
        );
    }
}
