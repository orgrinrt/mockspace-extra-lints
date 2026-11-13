use std::collections::BTreeSet;
use std::path::PathBuf;

use mockspace::RegistryView;
use mockspace::tool::ToolContext;

use super::*;
use crate::render::citation_base_rate;
use crate::tests_corpus::registry;

#[test]
fn the_cap_keeps_the_keywords_that_discriminate_rather_than_the_early_alphabet() {
    // A one-word subject matches every row on exactly one term, so the row
    // count ties everywhere and the tie-break decided the whole expansion.
    // It was alphabetical, and the cap fell in the c's: a licensing question
    // searched `aggregate`, `bundling` and `copyleft` and never searched
    // `gpl`, `licence`, `mpl` or `spdx`, all of which sit on the same rows.
    let candidates = vec![
        (1, "aggregate".to_string()),
        (1, "bundling".to_string()),
        (1, "copyleft".to_string()),
        (1, "spdx".to_string()),
    ];
    // `aggregate` is everywhere in this corpus, `spdx` is in four files.
    let df = |k: &str| {
        match k {
            "aggregate" => 900,
            "bundling" => 700,
            "copyleft" => 400,
            "spdx" => 4,
            _ => 0,
        }
    };

    let kept = rank_keywords(candidates, df, 2);
    assert!(
        kept.contains(&"spdx".to_string()),
        "the rare one carries the search and must survive the cap: {kept:?}"
    );
    assert!(
        !kept.contains(&"aggregate".to_string()),
        "the one in most of the corpus discriminates nothing: {kept:?}"
    );
}

#[test]
fn a_keyword_the_corpus_does_not_carry_does_not_take_a_slot() {
    // The control, and without it the fix is worse than the defect: rarest
    // first, taken literally, fills every slot with terms present in no
    // document at all, and the expansion then searches for nothing.
    let candidates = vec![
        (1, "spdx".to_string()),
        (1, "zzznowhere".to_string()),
        (1, "copyleft".to_string()),
    ];
    let df = |k: &str| {
        match k {
            "spdx" => 4,
            "copyleft" => 400,
            _ => 0,
        }
    };

    let kept = rank_keywords(candidates, df, 2);
    assert_eq!(kept, vec!["spdx".to_string(), "copyleft".to_string()]);
}

#[test]
fn a_row_carrying_more_of_the_question_still_wins() {
    // Rarity is the tie-break, not the ranking. A keyword from a row that
    // carries two words of the question is about the subject; a rare word
    // from a row that barely matched is a coincidence, and preferring the
    // second would undo the reason the rows are scored at all.
    let candidates = vec![(2, "common but on point".to_string()), (1, "rare".to_string())];
    let df = |k: &str| if k == "rare" { 2 } else { 800 };

    let kept = rank_keywords(candidates, df, 1);
    assert_eq!(kept, vec!["common but on point".to_string()]);
}

#[test]
fn the_marker_carries_the_share_it_is_measured_against() {
    // `UNCITED` was called the point of the line while marking 92.3% of the
    // corpus, so it named the ordinary state and read as a finding. The
    // informative half is the other one: 306 documents of 3983 have ever
    // been cited, and that is what the reader needs beside the marker.
    let line = citation_base_rate(306, 3983);
    assert!(line.contains("306"), "{line}");
    assert!(line.contains("3983"), "{line}");
    assert!(
        line.contains('%'),
        "the share is stated, not left to arithmetic: {line}"
    );
}

#[test]
fn a_corpus_nothing_has_cited_says_the_marker_decides_nothing() {
    // The control that matters, and it is the limit case of the finding
    // rather than a separate one: where nothing is cited, every hit carries
    // `UNCITED` and the marker has no work left to do at all.
    let line = citation_base_rate(0, 3983);
    assert!(
        line.contains("nothing"),
        "a marker that cannot vary is said to be useless here: {line}"
    );
}

#[test]
fn an_empty_corpus_does_not_divide_by_zero() {
    let line = citation_base_rate(0, 0);
    assert!(!line.contains("NaN"), "{line}");
    assert!(!line.contains("inf"), "{line}");
}

// -----------------------------------------------------------------------
// `run` itself. Everything above tests two of the helpers underneath it,
// and nothing tested what a person invokes: what the trailing word does,
// where the keywords come from, what the marker means, and whether the
// verdict at the foot describes the corpus or describes what the mode
// happened to show. That last one is the whole product of this tool.
// -----------------------------------------------------------------------

/// A directory holding the repository and, beside it, the directory of the maintainer's words.
struct Workspace(PathBuf);

impl Workspace {
    fn new(name: &str) -> Self {
        let ws =
            std::env::temp_dir().join(format!("unearth-ws-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&ws);
        for d in [
            "repo/mock/research",
            "repo/mock/design_rounds",
            "words",
        ] {
            std::fs::create_dir_all(ws.join(d)).unwrap();
        }
        std::fs::write(ws.join("repo/mockspace.toml"), "project_name = \"granary\"\n").unwrap();
        Self(ws)
    }

    fn write(&self, rel: &str, body: &str) -> &Self {
        let p = self.0.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
        self
    }

    fn run(&self, reg: &RegistryView, args: &[&str]) -> ToolReport {
        let mock = self.0.join("repo/mock");
        let repo = self.0.join("repo");
        let words = self.0.join("words");
        let crates = BTreeSet::new();
        let dirs: Vec<PathBuf> = Vec::new();
        run_with(
            &ToolContext {
                mock_dir: &mock,
                repo_root: &repo,
                all_crates: &crates,
                src_dirs: &dirs,
                args,
                stdin: None,
                registry: reg,
            },
            &|| corpus::repo::ready_at(None, &mock, &repo, Some(&words), &[]),
        )
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_maintainer_s_answer_beside_the_repository_is_found_and_ranked_as_theirs() {
    // The planted case: the maintainer answered who directs research in a capture, and
    // the only note on the subject talks about advancement lanes in other words. Asked
    // as a question, the capture is found, ranked first and named as the maintainer's.
    let ws = Workspace::new("words");
    ws.write(
        "repo/mock/research/202610032320_advancement/NOTES.md",
        "# Advancement\n\nResearch moves along the lanes as work is done.\n",
    )
    .write(
        "words/202610040900_on-advancement.md",
        "# granary: advancement\n\nWho directs research.\n> Whoever holds the office\n",
    );
    let reg = registry(&[("tenet::knowledge_is_a_concept", &[
        ("what", "Knowledge is owned through advancement."),
        ("keywords", "knowledge, advancement, research"),
        (
            "provenance",
            "research::202610032320_advancement::NOTES::#advancement",
        ),
    ])]);
    let out = ws.run(&reg, &["who", "directs", "research"]).output;
    let first = out
        .lines()
        .skip_while(|l| !l.starts_with("IN THE RESEARCH"))
        .find(|l| l.trim_start().starts_with(|c: char| c.is_ascii_digit()))
        .unwrap_or_default();
    assert!(
        first.contains("maintainer-words/202610040900_on-advancement.md"),
        "the maintainer's capture ranks first and is named as theirs: {out}"
    );
    assert!(!out.contains("captured words were not read"), "{out}");
}

#[test]
fn a_note_cited_by_heading_counts_as_cited() {
    // A row cites a note by its heading, `research::<dir>::NOTES::#<heading>`,
    // and the note so cited is the absorbed half of the corpus.
    let ws = Workspace::new("cited");
    ws.write(
        "repo/mock/research/202610031300_minds/NOTES.md",
        "# Minds\n\nA passphrase is never kept.\n",
    );
    let reg = registry(&[("tenet::a", &[
        ("what", "about passphrases"),
        ("provenance", "research::202610031300_minds::NOTES::#minds"),
    ])]);
    let out = ws.run(&reg, &["passphrase"]).output;
    assert!(out.contains("cited by 1 row"), "{out}");
}

#[test]
fn a_superseded_tenet_in_the_registry_section_is_marked() {
    let ws = Workspace::new("superseded");
    ws.write(
        "repo/mock/research/202610031300_minds/NOTES.md",
        "# Minds\n\nA scholar chooses.\n",
    );
    let reg = registry(&[
        ("tenet::the_old_one", &[
            ("what", "a scholar picks the subject"),
            ("superseded_by", "the_new_one"),
            ("related", "tenet::the_new_one"),
        ]),
        ("tenet::the_new_one", &[(
            "what",
            "the realm picks the subject and a scholar the method",
        )]),
    ]);
    let out = ws.run(&reg, &["scholar"]).output;
    assert!(
        out.contains("  tenet::the_old_one  superseded by the_new_one\n"),
        "{out}"
    );
    assert!(out.contains("  tenet::the_new_one\n"), "{out}");
}
