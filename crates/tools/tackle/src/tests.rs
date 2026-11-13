use super::*;

fn blind(pairs: &[(&str, usize)]) -> BTreeMap<String, usize> {
    pairs.iter().map(|(n, c)| ((*n).to_string(), *c)).collect()
}

#[test]
fn the_denominator_is_what_the_ranking_could_see() {
    // The brief said "10 of 359 rows in the whole registry". 359 is
    // `proposal` plus `ruling`, the two namespaces carrying a `topic`, and
    // the registry held 639. So the sentence claimed the ranking had weighed
    // every row when it had weighed 56% of them, and the shortfall is not
    // small enough to round away.
    let line = coverage_line(359, &blind(&[("mechanism", 96), ("slot", 97)]));
    assert!(
        !line.contains("whole registry"),
        "the ranking sees rows carrying a topic, and nothing else: {line}"
    );
    assert!(line.contains("359"), "{line}");
    assert!(
        line.contains("193"),
        "the rows it could not see are counted: {line}"
    );
    assert!(line.contains("mechanism"), "and named by namespace: {line}");
    assert!(line.contains("slot"), "{line}");
}

#[test]
fn a_registry_where_every_row_carries_a_topic_says_so() {
    // The control. A line that always warns about blind rows carries no
    // information, and a registry that closes the schema hole should stop
    // seeing the warning rather than reading it forever as noise.
    let line = coverage_line(639, &BTreeMap::new());
    assert!(line.contains("639"), "{line}");
    assert!(
        !line.contains("could not"),
        "nothing was invisible, so nothing is reported as invisible: {line}"
    );
}

#[test]
fn a_namespace_with_no_blind_rows_is_not_named() {
    // A namespace whose rows all carry a topic is not a hole and naming it
    // would send somebody to fix a thing that is already right.
    let line = coverage_line(359, &blind(&[("mechanism", 96), ("ruling", 0)]));
    assert!(line.contains("mechanism"), "{line}");
    assert!(!line.contains("ruling"), "{line}");
}

// -----------------------------------------------------------------------
// `run` itself. Everything above is over the scoring and the two rendering
// helpers. Nothing exercised the thing `mock tackle` calls, and it has four
// ways to return early, three of them saying different things about the
// same emptiness.
// -----------------------------------------------------------------------

use std::collections::BTreeMap as Map;
use std::path::{Path, PathBuf};

use mockspace::tool::Outcome;
use mockspace::{RegistryView, RowFields};

/// A corpus on disk. Cleared on construction and again on drop, so a run
/// that panics does not leave the next one reading its files.
struct Planted(PathBuf);

impl Planted {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("kaski-tackle-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("research")).unwrap();
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

fn registry(rows: &[(&str, &[(&str, &str)])]) -> RegistryView {
    let mut map: Map<String, RowFields> = Map::new();
    for (q, fields) in rows {
        let mut f = RowFields::new();
        for (k, v) in *fields {
            f.insert((*k).to_string(), (*v).to_string());
        }
        map.insert((*q).to_string(), f);
    }
    RegistryView::new(map, Map::new())
}

/// The subject the fixtures name as about the work, as a project's `[tools.tackle]` would.
fn about() -> Vec<String> {
    vec!["workflow".to_owned()]
}

fn run_in(mock: &Path, reg: &RegistryView, args: &[&str]) -> ToolReport {
    run_about(mock, reg, args, &about())
}

fn run_about(mock: &Path, reg: &RegistryView, args: &[&str], about: &[String]) -> ToolReport {
    let crates = std::collections::BTreeSet::new();
    let dirs: Vec<PathBuf> = Vec::new();
    let cache = std::env::temp_dir().join(format!("tackle-cache-{}", std::process::id()));
    crate::run_with(
        &ToolContext {
            mock_dir: mock,
            repo_root: mock,
            all_crates: &crates,
            src_dirs: &dirs,
            args,
            stdin: None,
            registry: reg,
        },
        about,
        &|| corpus::repo::ready_at(Some(&cache), mock, mock, None),
    )
}

/// With no corpus on disk, which is the empty-corpus arm.
fn run(reg: &RegistryView, args: &[&str]) -> ToolReport {
    run_in(Path::new("/nonexistent-mock-dir"), reg, args)
}

fn reason(r: &ToolReport) -> String {
    match &r.outcome {
        Outcome::Inconclusive {
            reason,
        } => reason.clone(),
        _ => String::new(),
    }
}

#[test]
fn a_verb_that_is_not_next_is_refused_and_named() {
    let reg = registry(&[("subject::boot", &[("what", "the boot chain")])]);
    let r = run(&reg, &["previous"]);
    let why = reason(&r);
    assert!(why.contains("`previous`"), "the verb is quoted back: {why}");

    // The control. `next` is the one verb, and it must not take this arm,
    // or the refusal is refusing everything and says nothing.
    assert!(
        !reason(&run(&reg, &["next"])).contains("is not a verb here"),
        "next is the verb"
    );
}

#[test]
fn no_arguments_runs_although_the_declaration_says_the_verb_is_required() {
    // Pinned because the two disagree. `args()` declares `next` with
    // `required: true`, and `run` only refuses a first argument that is
    // some other word, so an empty argument list reaches the ranking.
    // Whether the engine refuses it first is the engine's business and not
    // observable here; what is observable is that this tool does not.
    //
    // **If this starts failing, `run` grew the check and the declaration is
    // now true**, which is the better state and wants this test inverted
    // rather than deleted.
    let reg = registry(&[("subject::boot", &[("what", "the boot chain")])]);
    assert!(
        !reason(&run(&reg, &[])).contains("is not a verb here"),
        "no argument is not a wrong argument"
    );
    assert_eq!(
        Tackle::default().args()[0].required,
        true,
        "and the declaration still says it is required"
    );
}

#[test]
fn the_three_emptinesses_say_three_different_things() {
    // Each is a different fact about the registry and a reader has to be
    // able to tell them apart. Reporting one for another is how "nothing is
    // written about this" gets read off a tool that was never asked.

    // No topic rows at all.
    let none = registry(&[("tenet::k1", &[("what", "a thing")])]);
    assert!(reason(&run(&none, &["next"])).contains("no subject rows"));

    // Topics exist and nothing names one.
    let unnamed = registry(&[
        ("subject::boot", &[("what", "the boot chain")]),
        ("tenet::k1", &[("what", "a thing")]),
    ]);
    let why = reason(&run(&unnamed, &["next"]));
    assert!(why.contains("no row anywhere names a subject"), "{why}");

    // The control: one row naming a topic takes it out of that arm, so the
    // arm is about the naming and not about the row count.
    let named = registry(&[
        ("subject::boot", &[("what", "the boot chain")]),
        ("tenet::k1", &[("what", "a thing"), ("subject", "boot")]),
    ]);
    assert!(
        !reason(&run(&named, &["next"])).contains("no row anywhere names a subject"),
        "one namer is enough"
    );
}

#[test]
fn a_topic_nothing_could_be_asked_about_is_named_rather_than_scored_zero() {
    // Its `unrd` is zero because no document was opened, not because no
    // research sits behind it, and the table cannot tell those apart. So
    // the topic is named under the table instead.
    let p = Planted::new("unasked");
    p.file(
        "a.md",
        "the boot chain hands over to the init and seals the slot",
    );

    let reg = registry(&[
        // Every word of the slug AND of its `what` is on the stop list,
        // which is what it takes: the topic's own words seed the question
        // before its keywords do, so a `what` with one content word in it
        // makes the topic askable and this arm unreachable.
        ("subject::the_and_of", &[("what", "the and of it")]),
        ("subject::boot", &[("what", "the boot chain")]),
        ("tenet::k1", &[("what", "boot"), ("subject", "boot")]),
        ("tenet::k2", &[("what", "stop"), ("subject", "the_and_of")]),
    ]);
    let out = run_in(&p.0, &reg, &["next"]).output;
    assert!(
        out.contains("The research was not asked about the_and_of"),
        "{out}"
    );

    // The control, and it is the half that makes the claim mean anything: a
    // topic whose words are searchable is not named here, so the line is
    // about the question rather than about the result.
    assert!(
        !out.contains("not asked about boot"),
        "boot is askable: {out}"
    );
}

#[test]
fn an_absent_corpus_is_not_an_empty_one() {
    // With no corpus the index is never built and every `unrd` is zero. The
    // report has to say the corpus was not read, or a reader takes the
    // zeroes as a measurement over research that is simply not there.
    let reg = registry(&[
        ("subject::boot", &[("what", "the boot chain")]),
        ("tenet::k1", &[("what", "boot"), ("subject", "boot")]),
    ]);
    let out = run(&reg, &["next"]).output;
    assert!(
        out.contains('0'),
        "a zero appears somewhere in the table: {out}"
    );

    // And with one document planted, the same registry reads something.
    // Holding the registry fixed and varying only the disk is what makes
    // this a test of the corpus arm rather than of the ranking.
    let p = Planted::new("absent-vs-empty");
    p.file(
        "a.md",
        "the boot chain hands over to the init and seals the slot",
    );
    let with = run_in(&p.0, &reg, &["next"]).output;
    assert_ne!(out, with, "the corpus changes the report");
}

// -----------------------------------------------------------------------
// kaski's shape: subjects are `subject::` rows, a tenet names one in its
// `subject` field, a superseded tenet no longer binds, and `workflow` is
// about how kaski is built rather than about the game.
// -----------------------------------------------------------------------

/// A registry with its reverse edges, built the way the engine builds them from the typed fields
/// kaski declares: `subject`, `cites` and `superseded_by`.
fn linked(rows: &[(&str, &[(&str, &str)])]) -> RegistryView {
    let mut map: Map<String, RowFields> = Map::new();
    let mut back: Map<String, Vec<String>> = Map::new();
    for (q, fields) in rows {
        let mut f = RowFields::new();
        for (k, v) in *fields {
            f.insert((*k).to_string(), (*v).to_string());
            let target = match *k {
                "subject" => Some("subject"),
                "cites" | "superseded_by" => Some("tenet"),
                _ => None,
            };
            if let Some(ns) = target {
                for id in v.split(',').map(str::trim).filter(|s| !s.is_empty()) {
                    back.entry(format!("{ns}::{id}"))
                        .or_default()
                        .push((*q).to_string());
                }
            }
        }
        map.insert((*q).to_string(), f);
    }
    RegistryView::new(map, back)
}

fn kaski_canon() -> RegistryView {
    linked(&[
        ("subject::sky", &[
            ("what", "The sky and its light."),
            ("keywords", "sun, dawn"),
        ]),
        ("subject::weather", &[
            ("what", "Weather fronts and rain."),
            ("keywords", "front, rain, moisture"),
        ]),
        ("subject::workflow", &[(
            "what",
            "How kaski is built and checked.",
        )]),
        ("tenet::the_sun_rises", &[
            ("subject", "sky"),
            ("what", "The sun rises."),
        ]),
        ("tenet::the_sun_sets", &[
            ("subject", "sky"),
            ("what", "The sun sets."),
            ("cites", "the_sun_rises"),
        ]),
        ("tenet::dawn_is_grey", &[
            ("subject", "sky"),
            ("what", "Dawn is grey."),
            ("cites", "the_sun_rises"),
        ]),
        ("tenet::fronts_bring_rain", &[
            ("subject", "weather"),
            ("what", "A front brings rain."),
        ]),
    ])
}

#[test]
fn a_subject_with_research_behind_it_and_little_written_is_sent_to_read() {
    let p = Planted::new("kaski-unread");
    p.file(
        "202610031000_weather/NOTES.md",
        "# Weather\n\nA front carries moisture and rain across the planet. A front stalls on \
         mountains. Rain falls where a front meets cold air. Moisture rises from the sea.\n",
    );
    let out = run_in(&p.0, &kaski_canon(), &["next"]).output;
    assert!(out.contains("TACKLE NEXT: weather"), "{out}");
    assert!(out.contains("GO AND READ"), "{out}");
}

#[test]
fn the_workflow_subject_is_weighed_and_never_picked() {
    // `workflow` holds nothing, so it is the thinnest subject there is, and it
    // is about how kaski is built rather than the game.
    let out = run(&kaski_canon(), &["next"]).output;
    let line = out.lines().find(|l| {
        l.trim_end()
            .ends_with("workflow  (about the work, never picked)")
    });
    assert!(line.is_some(), "{out}");
    assert!(!out.contains("TACKLE NEXT: workflow"), "{out}");
}

#[test]
fn with_no_subject_named_about_the_work_the_thinnest_is_picked_whatever_it_is() {
    // THE CONTROL for the test above: `workflow` is held back because the project named it, not
    // because of its name, so with nothing named it is the thinnest subject and is picked.
    let out = run_about(
        Path::new("/nonexistent-mock-dir"),
        &kaski_canon(),
        &["next"],
        &[],
    )
    .output;
    assert!(out.contains("TACKLE NEXT: workflow"), "{out}");
    assert!(!out.contains("about the work, never picked"), "{out}");
}

#[test]
fn the_subjects_about_the_work_come_from_the_tools_table() {
    let mut t = Tackle::default();
    assert!(
        t.config_keys().iter().any(|k| k.name == "about_the_work"),
        "the key is declared, so the engine hands it over"
    );
    let mut params = mockspace::tool::ToolParams::new();
    params.set("about_the_work", vec!["workflow".to_owned(), "tools".to_owned()]);
    t.configure(&params);
    assert_eq!(t.about_the_work, vec!["workflow".to_owned(), "tools".to_owned()]);
    // A table that sets the key to an empty array names no subject.
    params.set("about_the_work", Vec::new());
    t.configure(&params);
    assert!(t.about_the_work.is_empty());
}

#[test]
fn a_superseded_tenet_counts_as_moving_and_not_as_written() {
    let reg = linked(&[
        ("subject::sky", &[("what", "The sky.")]),
        ("subject::time", &[("what", "The clock.")]),
        ("tenet::a", &[("subject", "sky"), ("what", "One.")]),
        ("tenet::b", &[("subject", "sky"), ("what", "Two.")]),
        ("tenet::c", &[
            ("subject", "sky"),
            ("what", "Three."),
            ("superseded_by", "a"),
        ]),
        ("tenet::d", &[
            ("subject", "sky"),
            ("what", "Four."),
            ("superseded_by", "b"),
        ]),
        ("tenet::e", &[
            ("subject", "sky"),
            ("what", "Five."),
            ("superseded_by", "b"),
        ]),
        ("tenet::f", &[("subject", "time"), ("what", "Six.")]),
        ("tenet::g", &[("subject", "time"), ("what", "Seven.")]),
    ]);
    let out = run(&reg, &["next"]).output;
    let sky: Vec<&str> = out
        .lines()
        .find(|l| l.trim_end().ends_with(" sky"))
        .unwrap_or_default()
        .split_whitespace()
        .collect();
    // ... rows  sups  open  subject
    assert_eq!(sky[sky.len() - 4 ..], ["2", "3", "0", "sky"], "{out}");
}

#[test]
fn a_tenet_marking_something_left_open_counts_as_a_named_hole() {
    let reg = linked(&[
        ("subject::sky", &[("what", "The sky.")]),
        ("subject::time", &[("what", "The clock.")]),
        ("tenet::a", &[
            ("subject", "sky"),
            ("what", "The colour of dusk is left open."),
        ]),
        ("tenet::b", &[
            ("subject", "sky"),
            ("what", "The sun rises."),
        ]),
        ("tenet::f", &[
            ("subject", "time"),
            ("what", "A day is long."),
        ]),
    ]);
    let out = run(&reg, &["next"]).output;
    let sky: Vec<&str> = out
        .lines()
        .find(|l| l.trim_end().ends_with(" sky"))
        .unwrap_or_default()
        .split_whitespace()
        .collect();
    assert_eq!(sky[sky.len() - 4 ..], ["2", "0", "1", "sky"], "{out}");
}

#[test]
fn the_subject_a_tenet_names_is_not_a_relation_between_tenets() {
    // Every tenet names its subject, so counting that edge makes every row look
    // connected. An island is a tenet citing no tenet and cited by none.
    let reg = kaski_canon();
    let crates = std::collections::BTreeSet::new();
    let dirs: Vec<PathBuf> = Vec::new();
    let ctx = ToolContext {
        mock_dir:   Path::new("/nonexistent-mock-dir"),
        repo_root:  Path::new("/"),
        all_crates: &crates,
        src_dirs:   &dirs,
        args:       &[],
        stdin:      None,
        registry:   &reg,
    };
    let subjects: Vec<String> = ["sky", "weather", "workflow"].map(String::from).to_vec();
    let (raws, ..) = gather(&ctx, &subjects, &about());
    assert_eq!(
        raws["weather"].edges, 0,
        "a lone tenet naming its subject is an island"
    );
    assert_eq!(
        raws["sky"].edges, 4,
        "two citations, each counted from both ends"
    );
}
