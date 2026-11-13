//! kaski has no `mechanism` rows. What moves when its implementation moves is its crates and the
//! types its designs declare, so a statement is matched against those.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use mockspace::{RegistryView, RowFields};

use super::super::*;

/// A repository with a schema, a mock directory, and one crate whose design declares two types.
struct Repo(PathBuf);

const SCHEMA: &str = r#"
[[registry.namespace]]
key = "tenet"

[[registry.namespace.field]]
name = "what"
type = "string"

[[registry.namespace.field]]
name = "subject"
type = "subject"
"#;

const DESIGN: &str = "# kaski-fire\n\n`HeatLattice` carries heat between tiles. A fire is a `Situation` \
                      at a spot.\n\nIt reads `kaski_fire::spread` and nothing else.\n";

impl Repo {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("kaski-iom-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("mock/crates/kaski-fire")).unwrap();
        std::fs::create_dir_all(dir.join("content/kaski")).unwrap();
        std::fs::write(dir.join("mockspace.toml"), SCHEMA).unwrap();
        std::fs::write(dir.join("mock/crates/kaski-fire/DESIGN.md.tmpl"), DESIGN).unwrap();
        Self(dir)
    }

    fn run(&self, rows: &[(&str, &str)]) -> ToolReport {
        let mut map: BTreeMap<String, RowFields> = BTreeMap::new();
        for (q, what) in rows {
            let mut f = RowFields::new();
            f.insert("what".to_string(), (*what).to_string());
            f.insert("subject".to_string(), "fire".to_string());
            map.insert((*q).to_string(), f);
        }
        let reg = RegistryView::new(map, BTreeMap::new());
        let crates: BTreeSet<String> = ["kaski-fire".to_string()].into();
        let dirs: Vec<PathBuf> = vec![self.0.join("mock/crates")];
        IntentOrMechanism.run(&ToolContext {
            mock_dir:   &self.0.join("mock"),
            repo_root:  &self.0,
            all_crates: &crates,
            src_dirs:   &dirs,
            args:       &[],
            stdin:      None,
            registry:   &reg,
        })
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_tenet_naming_a_type_a_design_declares_is_found() {
    let r = Repo::new("type");
    let out = r
        .run(&[
            (
                "tenet::fire_spreads_through_the_heat",
                "Fire spreads through the HeatLattice to its neighbours.",
            ),
            (
                "tenet::fire_spreads_with_heat",
                "Fire spreads to what is hot enough beside it.",
            ),
        ])
        .output;
    assert!(
        out.contains(
            "tenet::fire_spreads_through_the_heat what: names `HeatLattice` (kaski-fire's design)"
        ),
        "{out}"
    );
    assert!(!out.contains("tenet::fire_spreads_with_heat"), "{out}");
}

#[test]
fn a_one_word_type_counts_only_where_the_sentence_did_not_capitalise_it() {
    // `Situation` is a type in kaski-fire's design and a word every sentence
    // may open with. Capitalised mid-sentence it names the type; opening a
    // sentence, or in lower case, it is the word.
    let r = Repo::new("one-word");
    let out = r
        .run(&[
            (
                "tenet::a",
                "Work at a spot is a Situation with its own progress.",
            ),
            (
                "tenet::b",
                "Situation and place decide what a being does. It weighs both.",
            ),
            ("tenet::c", "A fire is a situation, and nothing more."),
        ])
        .output;
    assert!(
        out.contains("tenet::a what: names `Situation` (kaski-fire's design)"),
        "{out}"
    );
    assert!(!out.contains("tenet::b what"), "{out}");
    assert!(!out.contains("tenet::c what"), "{out}");
}

#[test]
fn a_tenet_naming_a_crate_or_a_rust_path_is_found() {
    let r = Repo::new("crate");
    let out = r
        .run(&[
            ("tenet::a", "The spread is decided in kaski-fire alone."),
            (
                "tenet::b",
                "The spread is kaski_fire::spread and nothing else.",
            ),
        ])
        .output;
    assert!(
        out.contains("tenet::a what: names the crate `kaski-fire`"),
        "{out}"
    );
    assert!(out.contains("tenet::b what: a Rust path (`::`)"), "{out}");
}

#[test]
fn a_design_declaring_nothing_is_said_rather_than_reported_clean() {
    let r = Repo::new("blind");
    std::fs::write(
        r.0.join("mock/crates/kaski-fire/DESIGN.md.tmpl"),
        "# kaski-fire\n\nNothing.\n",
    )
    .unwrap();
    let out = r.run(&[("tenet::a", "Fire spreads.")]);
    let mockspace::tool::Outcome::Inconclusive {
        reason,
    } = &out.outcome
    else {
        panic!("a run that could match no type passed: {}", out.output);
    };
    assert!(reason.contains("no design declares a type"), "{reason}");
}

#[test]
fn a_slash_between_alternatives_is_not_a_path_and_a_repository_path_is() {
    // kaski writes alternatives with a slash, `Being/Genome`, `realm/culture/region`,
    // and every slash in its statements when this was ported was one of those.
    let r = Repo::new("slash");
    let out = r
        .run(&[
            ("tenet::a", "It applies at realm/culture/region scope."),
            (
                "tenet::b",
                "The rules are read from mock/registry before play.",
            ),
            ("tenet::c", "The packs under content/kaski hold them."),
        ])
        .output;
    assert!(!out.contains("tenet::a what"), "{out}");
    assert!(
        out.contains("tenet::b what: a path in the repository (`mock/`)"),
        "{out}"
    );
    assert!(
        out.contains("tenet::c what: a path in the repository (`content/`)"),
        "{out}"
    );
}
