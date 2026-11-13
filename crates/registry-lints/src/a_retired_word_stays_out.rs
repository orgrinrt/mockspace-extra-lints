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

use std::collections::BTreeSet;
use std::path::Path;

use mockspace::{Lint, LintError, RegistryView, RepoContext, RepoLint, Severity};

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
        let config =
            std::fs::read_to_string(ctx.repo_root.join("mockspace.toml")).unwrap_or_default();
        let (change, unread) = match change_of(ctx.repo_root) {
            Ok(change) => (change, None),
            Err(why) => (Change::default(), Some(why)),
        };
        let mut out = check(ctx.repo_root, ctx.registry, &config, Some(ctx), &change);
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
fn change_of(repo: &Path) -> Result<Change, String> {
    let read = touched_lines::read(repo)?;
    let at_base = touched_lines::file_at(repo, &read.base, "mockspace.toml").unwrap_or_default();
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

/// The findings of every scanned file, each file's replayed from the store when its path, its text and the
/// rules are what they were. `ctx` is `None` where there is no store to ask.
fn judge_files(repo: &Path, rules: &[Retiring], ctx: Option<&RepoContext>) -> Vec<LintError> {
    let rules_key = fingerprint(rules);
    let mut out = Vec::new();
    for rel in files(repo).into_iter().filter(|rel| scanned(rel)) {
        let Ok(body) = std::fs::read_to_string(repo.join(&rel)) else {
            continue;
        };
        let path = rel.to_string_lossy().replace('\\', "/");
        let found = match ctx {
            Some(ctx) => {
                let parts: [&[u8]; 3] = [path.as_bytes(), body.as_bytes(), rules_key.as_bytes()];
                ctx.remembered(NAME, &parts, || judge(&rel, &body, rules))
            },
            None => judge(&rel, &body, rules),
        };
        out.extend(found);
    }
    out
}

fn check(
    repo: &Path,
    reg: &RegistryView,
    config: &str,
    ctx: Option<&RepoContext>,
    change: &Change,
) -> Vec<LintError> {
    let rules = retiring(reg);
    if rules.is_empty() {
        return Vec::new();
    }
    let debt: BTreeSet<String> = table(config, NAME).exempt_crates.into_iter().collect();
    let mut holding: BTreeSet<String> = BTreeSet::new();
    let mut out = Vec::new();
    for mut e in judge_files(repo, &rules, ctx) {
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
    use std::cell::{Cell, RefCell};
    use std::collections::HashMap;

    use mockspace::verdict::VerdictCache;

    use super::*;
    use crate::canon_rows::testkit::{ctx_with, git, scratch, view, write};

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
        let found = check(&d, &reg(), &config(&[]), None, &Change::default());
        let units: Vec<&str> = found.iter().map(|e| e.crate_name.as_str()).collect();
        assert_eq!(units, ["kaski-a", "kaski-b"], "{found:?}");
        assert!(found[0].message.contains("there_is_no_zorblax"));
    }

    #[test]
    fn a_unit_on_the_list_is_let_through_and_the_others_still_fail() {
        let d = repo();
        let found = check(&d, &reg(), &config(&["kaski-a"]), None, &Change::default());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].crate_name, "kaski-b");
    }

    #[test]
    fn a_unit_on_the_list_with_nothing_left_is_a_finding_so_the_list_only_shrinks() {
        let d = repo();
        let found = check(
            &d,
            &reg(),
            &config(&["kaski-a", "kaski-b", "kaski-c"]),
            None,
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
        assert!(check(&d, &reg(), &config(&[]), None, &Change::default()).is_empty());
        let none = view(&[("tenet::the_ground_is_one", &[("what", "One ground.")])]);
        assert!(check(&repo(), &none, &config(&[]), None, &Change::default()).is_empty());
    }

    /// A store of saved verdicts that counts how often it answered and how often it was written.
    #[derive(Default)]
    struct Saved {
        kept:   RefCell<HashMap<String, String>>,
        hits:   Cell<usize>,
        stores: Cell<usize>,
    }

    impl VerdictCache for Saved {
        fn digest(&self, parts: &[&[u8]]) -> String {
            parts
                .iter()
                .map(|p| format!("{}:{}", p.len(), String::from_utf8_lossy(p)))
                .collect::<Vec<_>>()
                .join("|")
        }

        fn replay(&self, key: &str) -> Option<String> {
            let found = self.kept.borrow().get(key).cloned();
            if found.is_some() {
                self.hits.set(self.hits.get() + 1);
            }
            found
        }

        fn remember(&self, key: &str, encoded: &str) {
            self.stores.set(self.stores.get() + 1);
            self.kept
                .borrow_mut()
                .insert(key.to_owned(), encoded.to_owned());
        }
    }

    /// Puts `d` under git with its `mockspace.toml` committed on `dev`, once, so a change can be read.
    fn versioned(d: &Path) {
        if d.join(".git").exists() {
            return;
        }
        git(d, &["init", "-q", "-b", "dev"]);
        git(d, &["add", "mockspace.toml"]);
        git(d, &["commit", "-q", "-m", "base"]);
    }

    /// What the lint finds over `d` through a store, as `(unit, line, message)`.
    fn run(d: &Path, reg: &RegistryView, saved: &Saved) -> Vec<(String, usize, String)> {
        write(d, "mockspace.toml", &config(&[]));
        versioned(d);
        ARetiredWordStaysOut
            .check_repo(&ctx_with(d, reg, Some(saved)))
            .into_iter()
            .map(|e| (e.crate_name, e.line, e.message))
            .collect()
    }

    /// What the lint finds over `d` with no store at all.
    fn fresh(d: &Path, reg: &RegistryView) -> Vec<(String, usize, String)> {
        check(d, reg, &config(&[]), None, &Change::default())
            .into_iter()
            .map(|e| (e.crate_name, e.line, e.message))
            .collect()
    }

    #[test]
    fn an_unchanged_tree_replays_every_file_and_finds_what_a_fresh_run_finds() {
        let d = repo();
        let (r, saved) = (reg(), Saved::default());
        let first = run(&d, &r, &saved);
        assert_eq!(
            first,
            fresh(&d, &r),
            "a first run through the store changes nothing"
        );
        assert_eq!(
            (saved.hits.get(), saved.stores.get()),
            (0, 3),
            "three files are read"
        );
        let second = run(&d, &r, &saved);
        assert_eq!(second, first);
        assert_eq!(
            (saved.hits.get(), saved.stores.get()),
            (3, 3),
            "every file replays, none is judged"
        );
    }

    #[test]
    fn a_changed_file_is_judged_again_and_the_unchanged_ones_replay() {
        let d = repo();
        let (r, saved) = (reg(), Saved::default());
        run(&d, &r, &saved);
        write(
            &d,
            "mock/crates/kaski-c/src/lib.rs",
            "pub struct Quillfern;\n",
        );
        let added = run(&d, &r, &saved);
        assert_eq!(added, fresh(&d, &r));
        assert_eq!(
            (saved.hits.get(), saved.stores.get()),
            (3, 4),
            "only the new file is judged"
        );
        write(
            &d,
            "mock/crates/kaski-b/src/lib.rs",
            "pub struct ZorblaxCell;\n// and again\n",
        );
        let moved = run(&d, &r, &saved);
        assert_eq!(moved, fresh(&d, &r));
        assert_eq!(
            (saved.hits.get(), saved.stores.get()),
            (6, 5),
            "the edited file alone is judged"
        );
        write(
            &d,
            "mock/crates/kaski-b/src/lib.rs",
            "pub struct Quillfern;\n",
        );
        let cleared = run(&d, &r, &saved);
        assert_eq!(cleared, fresh(&d, &r));
        assert!(
            cleared.iter().all(|(unit, ..)| unit != "kaski-b"),
            "{cleared:?}"
        );
    }

    #[test]
    fn a_tenet_ruling_out_another_word_judges_every_file_again() {
        let d = repo();
        let (r, saved) = (reg(), Saved::default());
        run(&d, &r, &saved);
        let wider = view(&[("tenet::there_is_no_quillfern", &[(
            "rules_out",
            "quillfern",
        )])]);
        let found = run(&d, &wider, &saved);
        assert_eq!(found, fresh(&d, &wider));
        assert_eq!(
            saved.stores.get(),
            6,
            "the words moved, so nothing saved before applies"
        );
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
        let change = change_of(d).expect("a change");
        let config = std::fs::read_to_string(d.join("mockspace.toml")).expect("a config");
        check(d, &reg(), &config, None, &change)
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
        let found = ARetiredWordStaysOut.check_repo(&ctx_with(&d, &reg(), None));
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
