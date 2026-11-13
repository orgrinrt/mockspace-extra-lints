// -----------------------------------------------------------------------
// `run` itself. Everything above tests the two functions underneath it, and
// nothing tested what a person invokes: which rows are read, what the
// argument narrows, which namespaces are skipped, and whether the
// accounting under the count is over the population the count is over.
// Every field of `ToolContext` is public and `RegistryView::new` is too, so
// this was never a matter of not being able to.
// -----------------------------------------------------------------------
use std::path::PathBuf;

use mockspace::tool::Outcome;
use mockspace::{RegistryView, RowFields};

use super::super::*;

/// A repo root holding one `mockspace.toml`, removed when the test ends.
///
/// The schema is read off disk rather than handed over, so a test of `run`
/// has to plant one. Named per test because the whole suite shares a
/// process id and the threads run at once.
struct Rooted(PathBuf);

impl Rooted {
    fn new(name: &str, schema: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("kaski-iom-run-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("mockspace.toml"), schema).unwrap();
        // One design declaring one type, so the run is not refused as blind: kaski matches
        // statements against the types its designs declare rather than kamu's mechanism rows.
        std::fs::create_dir_all(dir.join("crates/kaski-x")).unwrap();
        std::fs::write(
            dir.join("crates/kaski-x/DESIGN.md.tmpl"),
            "`PlantedType` is planted.\n",
        )
        .unwrap();
        Self(dir)
    }
}

impl Drop for Rooted {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A registry from rows given as `("ns::slug", &[("field", "value")])`.
///
/// No reverse edges, which this tool never asks for.
fn registry(rows: &[(&str, &[(&str, &str)])]) -> RegistryView {
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

fn run(root: &Rooted, reg: &RegistryView, args: &[&str]) -> ToolReport {
    let crates = BTreeSet::new();
    let dirs: Vec<PathBuf> = Vec::new();
    IntentOrMechanism.run(&ToolContext {
        mock_dir: &root.0,
        repo_root: &root.0,
        all_crates: &crates,
        src_dirs: &dirs,
        args,
        stdin: None,
        registry: reg,
    })
}

/// Two namespaces, one statement field each. `mechanism::name` is a plain
/// string in the real schema too, so it is examined like any other and the
/// counts below include it.
const TWO_NS: &str = r#"
[[registry.namespace]]
key = "ruling"

[[registry.namespace.field]]
name = "says"
type = "string"

[[registry.namespace]]
key = "mechanism"

[[registry.namespace.field]]
name = "name"
type = "string"
"#;

#[test]
fn every_shape_that_names_a_spelling_is_found_and_named() {
    // The whole table rather than the two a test author would pick. Each is
    // a substring test on purpose, so one statement per shape and the same
    // control text under all of them.
    for (i, (shape, what)) in SHAPES.iter().enumerate() {
        let root = Rooted::new(&format!("shape{i}"), TWO_NS);
        let says = format!("it lives at boot{shape}thing");
        let reg = registry(&[
            ("mechanism::seal", &[("name", "the seal")]),
            ("ruling::a", &[("says", says.as_str())]),
            ("ruling::b", &[(
                "says",
                "it lives wherever the design puts it",
            )]),
        ]);

        let out = run(&root, &reg, &[]).output;
        assert!(
            out.contains(&format!("ruling::a says: {what} (`{shape}`)")),
            "`{shape}` is reported as {what}: {out}"
        );
        assert!(
            !out.contains("ruling::b"),
            "and prose carrying no spelling is not: {out}"
        );
    }
}

/// One namespace declaring two statement fields, for the counting tests.
const TWO_FIELDS: &str = r#"
[[registry.namespace]]
key = "ruling"

[[registry.namespace.field]]
name = "says"
type = "string"

[[registry.namespace.field]]
name = "note"
type = "string"

[[registry.namespace]]
key = "mechanism"

[[registry.namespace.field]]
name = "name"
type = "string"
"#;

#[test]
fn each_declared_field_of_a_row_is_examined_and_flagged_on_its_own() {
    let root = Rooted::new("perfield", TWO_FIELDS);
    let reg = registry(&[
        ("mechanism::seal", &[("name", "the seal")]),
        ("ruling::a", &[
            ("says", "the PlantedType is opened once"),
            ("note", "the PlantedType is not the volume"),
        ]),
    ]);

    let out = run(&root, &reg, &[]).output;
    assert!(
        out.contains("ruling::a says: names `PlantedType` (kaski-x's design)"),
        "{out}"
    );
    assert!(
        out.contains("ruling::a note: names `PlantedType` (kaski-x's design)"),
        "{out}"
    );
    assert!(
        out.contains("2 of 3 statements"),
        "a statement is a field rather than a row: {out}"
    );
}

#[test]
fn a_declared_field_the_row_leaves_out_is_counted_rather_than_ignored() {
    // The count above is over the fields carrying text, so the fields that
    // do not have to be counted somewhere or the total is over a corner and
    // reads as a total.
    let root = Rooted::new("absent", TWO_FIELDS);
    let reg = registry(&[
        ("mechanism::seal", &[("name", "the seal")]),
        ("ruling::a", &[("says", "the seal is opened once")]),
    ]);

    let out = run(&root, &reg, &[]).output;
    assert!(
        out.contains("Empty on their row: 1 declared field."),
        "{out}"
    );

    // The control: fill it and the count goes to zero rather than the line
    // going away, since an absent line and a zero read the same.
    let full = registry(&[
        ("mechanism::seal", &[("name", "the seal")]),
        ("ruling::a", &[
            ("says", "the seal is opened once"),
            ("note", "nothing further"),
        ]),
    ]);
    let out = run(&root, &full, &[]).output;
    assert!(
        out.contains("Empty on their row: 0 declared fields."),
        "{out}"
    );
}

#[test]
fn the_argument_narrows_the_accounting_as_well_as_the_run() {
    // The defect. The summary under the count was computed over the whole
    // schema whatever the argument said, so `mock intent-or-mechanism
    // ruling` reported a field count and a namespace count over rows it
    // never opened, beside a hit count over the one namespace it did. That
    // is the total over a corner every report line in this file is written
    // against, in the line whose whole job is saying what the total is over.
    let root = Rooted::new("narrow", TWO_NS);
    let reg = registry(&[
        ("mechanism::seal", &[("name", "the seal")]),
        ("ruling::a", &[("says", "the PlantedType is opened once")]),
    ]);

    let all = run(&root, &reg, &[]).output;
    assert!(all.contains("1 of 2 statements"), "{all}");
    assert!(
        all.contains("2 statement fields across 2 namespaces"),
        "unnarrowed, the schema and the run are the same population: {all}"
    );

    let one = run(&root, &reg, &["ruling"]).output;
    assert!(
        one.contains("1 of 1 statements"),
        "one row was opened: {one}"
    );
    assert!(
        one.contains("1 statement field across 1 namespace"),
        "and the summary is over that same one: {one}"
    );
}

#[test]
fn a_namespace_the_schema_does_not_declare_is_named_with_how_much_it_holds() {
    // `reference` is a mockspace builtin, so it has rows and no
    // `[[registry.namespace]]` block. Skipping it silently would put a
    // total over eight namespaces under the word total.
    for (extra, expected) in [(1usize, "reference (1 row)"), (2, "reference (2 rows)")] {
        let root = Rooted::new(&format!("undeclared{extra}"), TWO_NS);
        let mut spec: Vec<(&str, &[(&str, &str)])> = vec![
            ("mechanism::seal", &[("name", "the seal")]),
            ("ruling::a", &[("says", "the seal is opened once")]),
        ];
        for q in ["reference::one", "reference::two"].iter().take(extra) {
            spec.push((q, &[("what", "a document")]));
        }

        let out = run(&root, &registry(&spec), &[]).output;
        assert!(out.contains("Not examined at all:"), "{out}");
        assert!(out.contains(expected), "{out}");
    }
}

#[test]
fn every_namespace_declared_leaves_no_not_examined_line() {
    // The control for the one above. A line appearing whatever happened
    // says nothing, so it has to be absent when nothing was skipped.
    let root = Rooted::new("alldeclared", TWO_NS);
    let reg = registry(&[
        ("mechanism::seal", &[("name", "the seal")]),
        ("ruling::a", &[("says", "the seal is opened once")]),
    ]);

    assert!(!run(&root, &reg, &[]).output.contains("Not examined at all"));
}

#[test]
fn no_schema_on_disk_is_inconclusive_rather_than_clean() {
    let root = Rooted::new("noschema", TWO_NS);
    std::fs::remove_file(root.0.join("mockspace.toml")).unwrap();
    let reg = registry(&[
        ("mechanism::seal", &[("name", "the seal")]),
        ("ruling::a", &[("says", "the seal is opened once")]),
    ]);

    assert!(matches!(
        run(&root, &reg, &[]).outcome,
        Outcome::Inconclusive { .. }
    ));
}

#[test]
fn a_schema_declaring_no_statement_field_is_inconclusive() {
    // Every field closed, internal or a list, which is a schema this tool
    // can read and has nothing to look at. Distinguished from a corpus that
    // says nothing, which is the same empty output.
    let root = Rooted::new(
        "nofields",
        r#"
[[registry.namespace]]
key = "ruling"

[[registry.namespace.field]]
name = "kind"
type = "string"
values = ["intent", "ruling"]

[[registry.namespace.field]]
name = "quote"
type = "string"
visibility = "internal"
"#,
    );
    let reg = registry(&[("ruling::a", &[("kind", "ruling")])]);

    assert!(matches!(
        run(&root, &reg, &[]).outcome,
        Outcome::Inconclusive { .. }
    ));
}

#[test]
fn a_registry_filling_no_declared_field_is_inconclusive() {
    let root = Rooted::new("nothingfilled", TWO_NS);
    let reg = registry(&[("ruling::a", &[("topic", "t")])]);

    assert!(matches!(
        run(&root, &reg, &[]).outcome,
        Outcome::Inconclusive { .. }
    ));
}

#[test]
fn narrowing_to_a_namespace_the_schema_does_not_declare_says_which_it_is() {
    // The instrument never looked, and the reason printed said the registry
    // was empty or filled nothing, which are both false and both about the
    // corpus. A reader is told the corpus is quiet when the schema is what
    // is silent.
    let root = Rooted::new("narrowundeclared", TWO_NS);
    let reg = registry(&[
        ("mechanism::seal", &[("name", "the seal")]),
        ("ruling::a", &[("says", "the seal is opened once")]),
        ("reference::one", &[("what", "a document")]),
    ]);

    let Outcome::Inconclusive {
        reason,
    } = run(&root, &reg, &["reference"]).outcome
    else {
        panic!("a namespace with no declared field cannot be examined");
    };
    assert!(reason.contains("reference"), "{reason}");
    assert!(
        reason.contains("declares no"),
        "the schema is what is silent, not the registry: {reason}"
    );
}
