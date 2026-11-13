use std::cell::Cell;
use std::collections::BTreeSet;
use std::path::PathBuf;

use corpus::repo::{Ready, ready_at};
use mockspace::tool::{Outcome, ToolContext};
use mockspace::{RegistryView, RowFields};

use super::*;

/// A mock directory with a research tree under it, removed when the test
/// ends.
///
/// Named per test because the whole suite shares a process id and the
/// threads run at once.
pub(crate) struct Planted(pub(crate) PathBuf);

impl Planted {
    pub(crate) fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("kaski-unearth-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("research")).unwrap();
        Self(dir)
    }

    /// A mock directory with no roots at all on disk.
    fn bare(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("kaski-unearth-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Self(dir)
    }

    /// Every declared root present and every one of them empty, which is
    /// the only way to reach an empty corpus that is missing nothing.
    fn all_roots_empty(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("kaski-unearth-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for root in corpus::corpus::ROOTS {
            std::fs::create_dir_all(dir.join(root)).unwrap();
        }
        Self(dir)
    }

    fn file(&self, rel: &str, body: &str) -> &Self {
        let p = self.0.join("research").join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
        self
    }
}

impl Drop for Planted {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

pub(crate) fn registry(rows: &[(&str, &[(&str, &str)])]) -> RegistryView {
    let mut map: BTreeMap<String, RowFields> = BTreeMap::new();
    for (q, fields) in rows {
        let mut f = RowFields::new();
        for (k, v) in *fields {
            f.insert((*k).to_string(), (*v).to_string());
        }
        map.insert((*q).to_string(), f);
    }
    RegistryView::new(map, BTreeMap::new())
}

/// The index over what is planted, kept in memory so a test never touches the user's cache.
pub(crate) fn planted_index(dir: &Planted) -> Ready {
    ready_at(None, &dir.0, &dir.0, None)
}

pub(crate) fn run(dir: &Planted, reg: &RegistryView, args: &[&str]) -> ToolReport {
    run_on(dir, reg, planted_index(dir), args)
}

pub(crate) fn run_on(dir: &Planted, reg: &RegistryView, index: Ready, args: &[&str]) -> ToolReport {
    let crates = BTreeSet::new();
    let dirs: Vec<PathBuf> = Vec::new();
    let once = Cell::new(Some(index));
    run_with(
        &ToolContext {
            mock_dir: &dir.0,
            repo_root: &dir.0,
            all_crates: &crates,
            src_dirs: &dirs,
            args,
            stdin: None,
            registry: reg,
        },
        &|| once.take().expect("the index is made once"),
    )
}

/// A document carrying one distinctive word the registry rows below do not.
pub(crate) const DOC: &str = "# holding a passphrase\n\nThe passphrase never leaves the person, and \
                       nothing on the machine keeps a copy of it.\n";

#[test]
fn a_trailing_uncited_is_the_mode_and_not_a_word_of_the_question() {
    let dir = Planted::new("mode");
    dir.file("01.md", DOC);
    let reg = registry(&[]);

    // Taken off the end, so the subject is what is left.
    let out = run(&dir, &reg, &["passphrase", "uncited"]).output;
    assert!(
        out.contains("research/01.md"),
        "the subject was searched: {out}"
    );
    assert!(
        !out.contains("uncited                "),
        "and the mode word was not searched for as a term: {out}"
    );

    // The control: without it the same subject is searched and the
    // document, which nothing cites, is shown either way.
    assert!(
        run(&dir, &reg, &["passphrase"])
            .output
            .contains("research/01.md")
    );
}

#[test]
fn a_question_that_is_only_the_mode_word_or_nothing_at_all_is_inconclusive() {
    let dir = Planted::new("nosubject");
    dir.file("01.md", DOC);
    let reg = registry(&[]);

    // Two states with two reasons, because a reader who typed the mode and
    // forgot the subject is in a different position from one who typed
    // nothing.
    let Outcome::Inconclusive {
        reason,
    } = run(&dir, &reg, &["uncited"]).outcome
    else {
        panic!("the mode is not a subject");
    };
    assert!(
        reason.contains("the mode rather than the subject"),
        "{reason}"
    );

    let Outcome::Inconclusive {
        reason,
    } = run(&dir, &reg, &[]).outcome
    else {
        panic!("nothing was searched");
    };
    assert!(reason.contains("no subject given"), "{reason}");
}

#[test]
fn a_question_of_nothing_but_common_words_is_reported_as_never_asked() {
    // The most confident absence this tool can print came out of a question
    // that expanded into no terms at all, so no document was opened and the
    // silence was about the question. It is reported rather than answered,
    // and the words that were thrown away are named so the reader can
    // rephrase.
    let dir = Planted::new("unasked");
    dir.file("01.md", DOC);
    let reg = registry(&[]);

    let Outcome::Inconclusive {
        reason,
    } = run(&dir, &reg, &[
        "what", "is", "it", "that", "this", "one", "has",
    ])
    .outcome
    else {
        panic!("a question of stop words was never asked");
    };
    assert!(reason.contains("was never asked"), "{reason}");
    assert!(reason.contains("too common to carry a subject"), "{reason}");
    assert!(
        reason.contains("what"),
        "the dropped words are named: {reason}"
    );

    // The control: one word that carries a subject and the question is
    // asked, however many common ones sit beside it.
    assert!(matches!(
        run(&dir, &reg, &["what", "is", "the", "passphrase"]).outcome,
        Outcome::Clean { .. }
    ));
}

#[test]
fn a_corpus_that_was_not_read_is_inconclusive_and_says_which_roots_were_missing() {
    let reg = registry(&[]);

    let bare = Planted::bare("noroots");
    let Outcome::Inconclusive {
        reason,
    } = run(&bare, &reg, &["passphrase"]).outcome
    else {
        panic!("nothing was read, so nothing was established");
    };
    assert!(
        reason.contains("research is not on disk and was not read"),
        "the index's own caveat is carried: {reason}"
    );

    // The other empty corpus, and it is a different fact: every root is
    // there and every one of them is empty, so nothing is missing and the
    // sentence must not name a root that exists.
    let empty = Planted::all_roots_empty("emptyroots");
    let Outcome::Inconclusive {
        reason,
    } = run(&empty, &reg, &["passphrase"]).outcome
    else {
        panic!("an empty corpus establishes nothing either");
    };
    assert!(
        !reason.contains("is not on disk"),
        "and nothing is said to be missing when nothing is: {reason}"
    );
}

#[test]
fn the_registry_expands_the_question_into_words_the_corpus_uses() {
    // The one thing a grep structurally cannot do, and the reason the
    // keyword field exists. The document says `passphrase` and never
    // `secret`, so the question only reaches it through a row that carries
    // the subject and declares the other word.
    let dir = Planted::new("expand");
    dir.file("01.md", DOC);

    let without = registry(&[("ruling::r", &[("says", "a secret is the person's own")])]);
    assert!(
        !run(&dir, &without, &["secret"])
            .output
            .contains("research/01.md"),
        "the control: with no keyword on the row, the corpus is out of reach"
    );

    let with = registry(&[("ruling::r", &[
        ("says", "a secret is the person's own"),
        ("keywords", "passphrase"),
    ])]);
    let out = run(&dir, &with, &["secret"]).output;
    assert!(out.contains("research/01.md"), "{out}");
    assert!(
        out.contains("passphrase") && out.contains("registry"),
        "and the term is shown with where it came from: {out}"
    );
}

#[test]
fn a_citation_field_is_read_for_what_it_cites_and_not_as_prose() {
    // A hit in a provenance entry says a document was cited, not that a
    // subject was discussed, so a row is never listed for carrying the
    // subject inside a path it points at.
    let dir = Planted::new("notprose");
    dir.file("01.md", DOC);

    let cite_only = registry(&[("ruling::r", &[
        ("provenance", "panel::passphrase_notes::#a"),
        ("related", "ruling::passphrase_rules"),
    ])]);
    let out = run(&dir, &cite_only, &["passphrase"]).output;
    assert!(
        out.contains("IN THE REGISTRY\n\n  nothing"),
        "the word is in a citation rather than in a statement: {out}"
    );

    // The control, and it is what makes the exclusion the finding rather
    // than the tool being unable to match a row at all.
    let says = registry(&[("ruling::r", &[("says", "the passphrase is theirs")])]);
    assert!(
        run(&dir, &says, &["passphrase"])
            .output
            .contains("  ruling::r")
    );
}

#[test]
fn the_marker_says_which_documents_a_row_has_ever_come_from() {
    // `UNCITED` is the ordinary state and `cited by` is the rare one, which
    // is why the share is printed beside them: read one way round the
    // marker is a finding, and read the other it names the base rate.
    let dir = Planted::new("marker");
    dir.file("01.md", DOC);
    dir.file("02.md", "a second passphrase note, uncited by anything\n");
    let reg = registry(&[("ruling::r", &[("provenance", "panel::01::#a")])]);

    let out = run(&dir, &reg, &["passphrase"]).output;
    assert!(out.contains("cited by 1 row"), "{out}");
    assert!(out.contains("UNCITED"), "{out}");
    assert!(
        out.contains("1 of the 2 documents have ever been cited by a row"),
        "and the share the marker is measured against: {out}"
    );
}

#[test]
fn the_verdict_names_which_of_the_two_carries_the_subject() {
    // The sentence a reader acts on, over its whole matrix rather than the
    // one corner that is easy to plant. The third is what this tool was
    // built for: the research carries it and the registry does not, which
    // is a transcription that never happened rather than an open question.
    let dir = Planted::new("verdict");
    dir.file("01.md", DOC);

    let none = registry(&[]);
    let only_registry = registry(&[("ruling::r", &[("says", "a quaternion is not used here")])]);

    for (subject, reg, expected) in [
        (
            "passphrase",
            &none,
            "The research carries this and the registry does not",
        ),
        (
            "quaternion",
            &only_registry,
            "The registry carries it and the research does not",
        ),
        ("quaternion", &none, "Neither carries it"),
    ] {
        let out = run(&dir, reg, &[subject]).output;
        assert!(out.contains(expected), "`{subject}`: {out}");
    }

    let both = registry(&[("ruling::r", &[("says", "the passphrase is theirs")])]);
    let out = run(&dir, &both, &["passphrase"]).output;
    assert!(out.contains("Both carry it"), "{out}");
}

#[test]
fn the_uncited_mode_narrows_what_is_shown_and_not_what_the_verdict_says() {
    // The defect, and it is the worst one this tool can have. The verdict
    // was read off the hits after the mode had filtered them, so a subject
    // whose every document is cited came back saying neither the registry
    // nor the research carries it, with the line about a short dead-term
    // list meaning a real absence underneath. That is the exact claim this
    // tool exists to stop anybody making, printed by the tool, over a
    // corpus that carries the subject in the documents it had just dropped.
    let dir = Planted::new("modeverdict");
    dir.file("01.md", DOC);
    let reg = registry(&[("ruling::r", &[("provenance", "panel::01::#a")])]);

    // Unfiltered, the research carries it and the row is a citation rather
    // than a statement, so the registry does not.
    let plain = run(&dir, &reg, &["passphrase"]).output;
    assert!(
        plain.contains("The research carries this and the registry does not"),
        "{plain}"
    );

    let filtered = run(&dir, &reg, &["passphrase", "uncited"]).output;
    assert!(
        filtered.contains("The research carries this and the registry does not"),
        "the mode showed nothing and the corpus is unchanged: {filtered}"
    );
    assert!(!filtered.contains("Neither carries it"), "{filtered}");
    assert!(
        filtered.contains("cleared the cut and are cited already"),
        "and it says what it dropped: {filtered}"
    );
    assert!(
        !filtered.contains("nothing scored"),
        "which is a different fact and is false here: {filtered}"
    );
}

#[test]
fn a_row_carrying_the_subject_with_no_keywords_is_not_reported_as_no_row_at_all() {
    // Two silences and the sentence said the first for both. A row can
    // carry the subject and declare no `keywords`, and the report then said
    // no registry row carries this subject, directly above the list of the
    // rows that do. The fix a reader needs is different in the two cases:
    // rephrase in the first, put keywords on those rows in the second.
    let dir = Planted::new("nokeywords");
    dir.file("01.md", DOC);
    let reg = registry(&[("ruling::r", &[("says", "the passphrase is theirs")])]);

    let out = run(&dir, &reg, &["passphrase"]).output;
    assert!(out.contains("  ruling::r"), "the row is listed: {out}");
    assert!(
        !out.contains("No registry row carries this subject"),
        "and the report does not then say there is none: {out}"
    );
    assert!(
        out.contains("declare no keywords"),
        "it says which of the two silences it is: {out}"
    );

    // The control: with no row at all, the original sentence is the right
    // one and still prints.
    let none = registry(&[]);
    assert!(
        run(&dir, &none, &["passphrase"])
            .output
            .contains("No registry row carries this subject")
    );
}

// -----------------------------------------------------------------------
// kaski's shape: op's captured words beside the repository, a citation by
// heading into a research note, and a superseded tenet.
// -----------------------------------------------------------------------
