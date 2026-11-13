//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `a-tenet-cites-what-its-words-name`: a tenet's `cites` holds every other tenet its `what` and
//! `because` name by id, and nothing they do not.
//!
//! `cites` is the typed edge the registry's graph reads, so `refsto(tenet::<id>)` and `mock canon
//! related` answer from it. Read off the words rather than written separately, it cannot be allowed to
//! drift from them: a row written later that names an older one in its words gains the edge here or
//! is refused.

use mockspace::{Lint, LintError, RegistryView, RepoContext, RepoLint, Severity};

use crate::canon_rows::{finding, list, named, slug, text};

pub fn repo_lint() -> Box<dyn RepoLint> {
    Box::new(ATenetCitesWhatItsWordsName)
}

const NAME: &str = "a-tenet-cites-what-its-words-name";

struct ATenetCitesWhatItsWordsName;

impl Lint for ATenetCitesWhatItsWordsName {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

impl RepoLint for ATenetCitesWhatItsWordsName {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        check(ctx.registry())
    }
}

fn check(reg: &RegistryView) -> Vec<LintError> {
    let is_tenet = |w: &str| reg.row(&format!("tenet::{w}")).is_some();
    let mut out = Vec::new();
    for q in reg.rows_in("tenet") {
        let id = slug(q);
        let words = [text(reg, q, "what"), text(reg, q, "because")];
        let mut names: Vec<&str> = Vec::new();
        for w in words.into_iter().flatten() {
            for n in named(w, is_tenet) {
                if n != id && !names.contains(&n) {
                    names.push(n);
                }
            }
        }
        let cites = list(reg, q, "cites");
        let missing: Vec<String> = names
            .iter()
            .filter(|n| !cites.contains(n))
            .map(|n| format!("`{n}`"))
            .collect();
        if !missing.is_empty() {
            out.push(finding(
                NAME,
                format!(
                    "`{q}` names {} in its words and does not cite them; add them to its `cites`",
                    missing.join(", ")
                ),
            ));
        }
        let extra: Vec<String> = cites
            .iter()
            .filter(|c| !names.contains(c))
            .map(|c| format!("`{c}`"))
            .collect();
        if !extra.is_empty() {
            out.push(finding(
                NAME,
                format!("`{q}` cites {} where its words do not name them; `cites` holds only the tenets its `what` and `because` name", extra.join(", ")),
            ));
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

    #[test]
    fn cites_matching_the_words_passes_in_any_order() {
        let f = findings(&[
            ("tenet::a", &[
                ("what", "Unlike b, and as c says."),
                ("because", "b again"),
                ("cites", "c, b"),
            ]),
            ("tenet::b", &[("what", "Names a itself: a.")]),
            ("tenet::c", &[("what", "Nothing.")]),
        ]);
        // `b` names `a` and carries no `cites`, which is the one finding
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].contains("tenet::b") && f[0].contains("`a`"), "{f:?}");
    }

    #[test]
    fn a_tenet_named_in_the_words_and_missing_from_cites_is_reported() {
        let f = findings(&[
            ("tenet::a", &[("what", "As b and c."), ("cites", "b")]),
            ("tenet::b", &[("what", "x")]),
            ("tenet::c", &[("what", "y")]),
        ]);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(
            f[0].contains("tenet::a") && f[0].contains("`c`") && f[0].contains("does not cite"),
            "{f:?}"
        );
    }

    #[test]
    fn a_citation_the_words_do_not_make_is_reported() {
        let f = findings(&[
            ("tenet::a", &[("what", "Nothing named."), ("cites", "b")]),
            ("tenet::b", &[("what", "x")]),
        ]);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(
            f[0].contains("tenet::a") && f[0].contains("`b`") && f[0].contains("do not name"),
            "{f:?}"
        );
    }

    #[test]
    fn a_row_naming_itself_or_a_word_that_is_no_tenet_needs_no_citation() {
        let f = findings(&[
            ("tenet::a", &[("what", "a is not_a_tenet and subject::a.")]),
            ("subject::a", &[("what", "x")]),
        ]);
        assert!(f.is_empty(), "{f:?}");
    }
}
