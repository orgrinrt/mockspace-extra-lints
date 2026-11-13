//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `a-relation-adds-what-the-words-do-not`: the tenets a tenet links in `related` are only edges the row's words do
//! not make, each once, and none the tenet itself.
//!
//! `cites` is read off the words (`a-tenet-cites-what-its-words-name`), so an edge a reader needs that
//! the words do not carry has nowhere to go but a `tenet::<slug>` link in `related`. A tenet the words already name belongs in
//! `cites`, where this lint keeps it, and a link repeating it would count one edge twice in every
//! tool that reads both.

use mockspace::{Lint, LintError, RegistryView, RepoContext, RepoLint, Severity};

use crate::canon_rows::{finding, linked, named, slug, text};

pub fn repo_lint() -> Box<dyn RepoLint> {
    Box::new(ARelationAddsWhatTheWordsDoNot)
}

const NAME: &str = "a-relation-adds-what-the-words-do-not";

struct ARelationAddsWhatTheWordsDoNot;

impl Lint for ARelationAddsWhatTheWordsDoNot {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

impl RepoLint for ARelationAddsWhatTheWordsDoNot {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        check(ctx.registry)
    }
}

fn check(reg: &RegistryView) -> Vec<LintError> {
    let is_tenet = |w: &str| reg.row(&format!("tenet::{w}")).is_some();
    let mut out = Vec::new();
    for q in reg.rows_in("tenet") {
        let id = slug(q);
        let links = linked(reg, q, "related", "tenet");
        if links.is_empty() {
            continue;
        }
        let mut in_words: Vec<&str> = Vec::new();
        for w in [text(reg, q, "what"), text(reg, q, "because")]
            .into_iter()
            .flatten()
        {
            in_words.extend(named(w, is_tenet));
        }
        let mut seen: Vec<&str> = Vec::new();
        let mut twice: Vec<String> = Vec::new();
        let mut named_already: Vec<String> = Vec::new();
        for r in &links {
            if *r == id {
                out.push(finding(
                    NAME,
                    format!("`{q}` links itself in `related`; remove `tenet::{id}`"),
                ));
            } else if in_words.contains(r) {
                named_already.push(format!("`{r}`"));
            }
            if seen.contains(r) {
                twice.push(format!("`{r}`"));
            }
            seen.push(r);
        }
        if !named_already.is_empty() {
            out.push(finding(
                NAME,
                format!(
                    "`{q}` links {} in `related` and its words name it; a tenet its words name is in `cites`, so remove the link",
                    named_already.join(", ")
                ),
            ));
        }
        if !twice.is_empty() {
            out.push(finding(
                NAME,
                format!("`{q}` links {} twice in `related`", twice.join(", ")),
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
    fn a_relation_the_words_do_not_make_passes() {
        let f = findings(&[
            ("tenet::a", &[
                ("what", "Nothing named."),
                ("related", "tenet::b, tenet::c"),
            ]),
            ("tenet::b", &[("what", "x")]),
            ("tenet::c", &[("what", "y")]),
        ]);
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn a_tenet_the_words_name_is_cites_not_a_link() {
        let f = findings(&[
            ("tenet::a", &[
                ("what", "As b says."),
                ("cites", "b"),
                ("related", "tenet::b"),
            ]),
            ("tenet::b", &[("what", "x")]),
        ]);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(
            f[0].contains("tenet::a") && f[0].contains("`b`") && f[0].contains("its words name"),
            "{f:?}"
        );
    }

    #[test]
    fn a_tenet_named_only_in_because_is_caught_too() {
        let f = findings(&[
            ("tenet::a", &[
                ("what", "x"),
                ("because", "b"),
                ("related", "tenet::b"),
            ]),
            ("tenet::b", &[("what", "x")]),
        ]);
        assert_eq!(f.len(), 1, "{f:?}");
    }

    #[test]
    fn a_tenet_related_twice_is_reported() {
        let f = findings(&[
            ("tenet::a", &[
                ("what", "x"),
                ("related", "tenet::b, tenet::b"),
            ]),
            ("tenet::b", &[("what", "y")]),
        ]);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].contains("twice"), "{f:?}");
    }

    #[test]
    fn a_tenet_relating_to_itself_is_reported() {
        let f = findings(&[("tenet::a", &[("what", "x"), ("related", "tenet::a")])]);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].contains("itself"), "{f:?}");
    }

    #[test]
    fn a_link_into_another_namespace_is_not_a_tenet_edge() {
        // The slug is a tenet the words name, the namespace is `bevy`, so the link is to another row.
        // The control is `a_tenet_the_words_name_is_cites_not_a_link`: the same words with `tenet::b`
        // are refused.
        let f = findings(&[
            ("tenet::a", &[
                ("what", "As b says."),
                ("cites", "b"),
                ("related", "bevy::b"),
            ]),
            ("tenet::b", &[("what", "x")]),
            ("bevy::b", &[("what", "a fact")]),
        ]);
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn a_citation_in_related_is_not_a_tenet_edge() {
        let f = findings(&[
            ("tenet::a", &[
                ("what", "As b says."),
                ("related", "research::202611::NOTES::#tenet::b"),
            ]),
            ("tenet::b", &[("what", "x")]),
        ]);
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn a_row_with_no_related_is_not_asked_anything() {
        let f =
            findings(&[("tenet::a", &[("what", "As b says.")]), ("tenet::b", &[("what", "x")])]);
        assert!(f.is_empty(), "{f:?}");
    }
}
