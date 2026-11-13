//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `a-live-row-names-no-replaced-row-without-lineage`: a tenet that still binds names no replaced row,
//! unless its `lineage` says it does so on purpose and its content has not changed since.
//!
//! The canon row `a_cite_of_a_replaced_row_stands_as_lineage_only_while_what_it_is_attached_to_is_unchanged`
//! settles the intent and leaves the telling-apart to this check. A `lineage` entry is
//! `<replaced id>:<fingerprint>`, the fingerprint being `canon_rows::fingerprint` of the row's `what` and
//! `because`. Any edit of either invalidates every entry, so a cite can only stand while the sentence it
//! sits in is the sentence it was marked on, and the finding that says so names the row it should point
//! at now.
//!
//! Three things are refused: a replaced row named in `what`, `because`, `cites` or a `tenet::` link in
//! `related` with no entry; an entry whose fingerprint no longer matches; and an entry naming a row
//! that is not replaced or that the row no longer names. A replaced row is history and is not held to
//! this.
//!
//! The one abuse it cannot see is re-stamping a fingerprint without retargeting, which shows in a diff as
//! a bare `lineage` edit and is for a reader to look for.

use mockspace::{Lint, LintError, RegistryView, RepoContext, RepoLint, Severity};

use crate::canon_rows::{finding, fingerprint, linked, list, named, slug, text};

pub fn repo_lint() -> Box<dyn RepoLint> {
    Box::new(ALiveRowNamesNoReplacedRowWithoutLineage)
}

const NAME: &str = "a-live-row-names-no-replaced-row-without-lineage";

struct ALiveRowNamesNoReplacedRowWithoutLineage;

impl Lint for ALiveRowNamesNoReplacedRowWithoutLineage {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

impl RepoLint for ALiveRowNamesNoReplacedRowWithoutLineage {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        check(ctx.registry())
    }
}

/// Where a replaced row's chain of successors ends: the first row down it that nothing replaces.
fn terminal<'a>(reg: &'a RegistryView, id: &'a str) -> &'a str {
    let mut at = id;
    // A cycle is a defect of its own; the bound only keeps this from looping on one.
    for _ in 0 .. 64 {
        let next = list(reg, &format!("tenet::{at}"), "superseded_by");
        match next.first() {
            Some(n) => at = n,
            None => break,
        }
    }
    at
}

fn replaced(reg: &RegistryView, id: &str) -> bool {
    reg.row(&format!("tenet::{id}")).is_some()
        && !list(reg, &format!("tenet::{id}"), "superseded_by").is_empty()
}

fn check(reg: &RegistryView) -> Vec<LintError> {
    let is_replaced = |w: &str| replaced(reg, w);
    let mut out = Vec::new();
    for q in reg.rows_in("tenet") {
        let id = slug(q);
        if !list(reg, q, "superseded_by").is_empty() {
            continue;
        }
        let what = text(reg, q, "what").unwrap_or_default();
        let because = text(reg, q, "because").unwrap_or_default();
        let mut names: Vec<&str> = Vec::new();
        for w in [what, because] {
            for n in named(w, is_replaced) {
                if n != id && !names.contains(&n) {
                    names.push(n);
                }
            }
        }
        let linked_tenets = linked(reg, q, "related", "tenet");
        for n in list(reg, q, "cites").into_iter().chain(linked_tenets) {
            if is_replaced(n) && !names.contains(&n) {
                names.push(n);
            }
        }
        let now = fingerprint(what, because);
        let entries = list(reg, q, "lineage");
        for n in &names {
            let held = entries
                .iter()
                .find_map(|e| e.split_once(':').filter(|(r, _)| r == n));
            match held {
                None => out.push(finding(
                    NAME,
                    format!(
                        "`{q}` names `{n}`, a replaced row now ending at `{t}`; point it at `{t}`, or mark the cite as lineage with `lineage = [\"{n}:{now}\"]` if the sentence is deliberately about the replaced row",
                        t = terminal(reg, n)
                    ),
                )),
                Some((_, fp)) if fp != now => out.push(finding(
                    NAME,
                    format!(
                        "`{q}` marks `{n}` as lineage and its content has changed since; the cite now maps to what it means today, so point it at `{t}`",
                        t = terminal(reg, n)
                    ),
                )),
                Some(_) => {}
            }
        }
        for e in &entries {
            let Some((r, _)) = e.split_once(':') else {
                out.push(finding(
                    NAME,
                    format!("`{q}` has a `lineage` entry `{e}` that is not `<replaced id>:<fingerprint>`"),
                ));
                continue;
            };
            if !is_replaced(r) {
                out.push(finding(
                    NAME,
                    format!("`{q}` marks `{r}` as lineage and no tenet by that id is replaced; drop the entry"),
                ));
            } else if !names.contains(&r) {
                out.push(finding(
                    NAME,
                    format!("`{q}` marks `{r}` as lineage and no longer names it; drop the entry"),
                ));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canon_rows::testkit::view;

    fn findings(rows: &[(&str, &[(&str, &str)])]) -> Vec<String> {
        check(&view(rows)).into_iter().map(|e| e.message).collect()
    }

    const OLD: (&str, &[(&str, &str)]) = ("tenet::old", &[("what", "x"), ("superseded_by", "mid")]);
    const MID: (&str, &[(&str, &str)]) = ("tenet::mid", &[("what", "y"), ("superseded_by", "new")]);
    const NEW: (&str, &[(&str, &str)]) = ("tenet::new", &[("what", "z")]);

    #[test]
    fn a_live_row_naming_a_replaced_row_is_pointed_at_the_end_of_its_chain() {
        let f = findings(&[
            OLD,
            MID,
            NEW,
            ("tenet::live", &[("what", "As old says."), ("cites", "old")]),
        ]);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(
            f[0].contains("tenet::live")
                && f[0].contains("`old`")
                && f[0].contains("point it at `new`"),
            "{f:?}"
        );
    }

    #[test]
    fn a_cite_in_cites_or_a_link_alone_is_refused_too() {
        let f =
            findings(&[OLD, MID, NEW, ("tenet::a", &[("what", "w"), ("related", "tenet::old")])]);
        assert_eq!(f.len(), 1, "{f:?}");
    }

    #[test]
    fn a_marked_cite_stands_while_the_content_is_unchanged() {
        let fp = fingerprint("As old says.", "");
        let lineage = format!("old:{fp}");
        let f = findings(&[
            OLD,
            MID,
            NEW,
            ("tenet::live", &[
                ("what", "As old says."),
                ("lineage", &lineage),
            ]),
        ]);
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn a_marked_cite_lapses_when_the_content_changes() {
        let fp = fingerprint("As old says.", "");
        let lineage = format!("old:{fp}");
        let f = findings(&[
            OLD,
            MID,
            NEW,
            ("tenet::live", &[
                ("what", "As old says, and more."),
                ("lineage", &lineage),
            ]),
        ]);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(
            f[0].contains("content has changed") && f[0].contains("`new`"),
            "{f:?}"
        );
    }

    #[test]
    fn a_change_to_the_because_lapses_it_as_well() {
        let fp = fingerprint("As old says.", "r");
        let lineage = format!("old:{fp}");
        let f = findings(&[
            OLD,
            MID,
            NEW,
            ("tenet::live", &[
                ("what", "As old says."),
                ("because", "r2"),
                ("lineage", &lineage),
            ]),
        ]);
        assert_eq!(f.len(), 1, "{f:?}");
    }

    #[test]
    fn a_mark_on_a_row_that_is_live_or_no_longer_named_is_stale() {
        let fp = fingerprint("Nothing named.", "");
        let live = format!("new:{fp}");
        let gone = format!("old:{fp}");
        let f = findings(&[
            OLD,
            MID,
            NEW,
            ("tenet::a", &[
                ("what", "Nothing named."),
                ("lineage", &live),
            ]),
            ("tenet::b", &[
                ("what", "Nothing named."),
                ("lineage", &gone),
            ]),
        ]);
        assert_eq!(f.len(), 2, "{f:?}");
        assert!(
            f.iter()
                .any(|m| m.contains("no tenet by that id is replaced")),
            "{f:?}"
        );
        assert!(f.iter().any(|m| m.contains("no longer names it")), "{f:?}");
    }

    #[test]
    fn a_replaced_row_is_history_and_may_name_anything() {
        let f = findings(&[
            OLD,
            NEW,
            ("tenet::gone", &[
                ("what", "Names old."),
                ("superseded_by", "new"),
            ]),
        ]);
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn a_malformed_entry_is_reported() {
        let f = findings(&[OLD, NEW, ("tenet::a", &[("what", "w"), ("lineage", "old")])]);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].contains("not `<replaced id>:<fingerprint>`"), "{f:?}");
    }
}
