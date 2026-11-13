//! A repository that names the pack with the `workflow` feature, declares the namespaces the pack
//! ships in `registry/namespaces.toml` and copies its vocabulary, gets the workflow tools from the
//! pack alone and nothing of its own: a ticket is added, moved and listed, a tenet is written by
//! hand, and `canon find` and `canon check` answer over it.
//!
//! Every step goes the way a run goes: the engine's own config reader reads the scratch's
//! `mockspace.toml`, the registry is loaded and viewed as the tool runner loads it, and the tool is
//! the one the pack's `collect` registered under that name, so a tool the feature does not register
//! or a declaration the shipped file lacks fails here. The test build turns the
//! feature on through the pack's own dev-dependency on itself, so a plain `cargo test` runs this.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use mockspace_lint_rules::tool::{Outcome, ToolReport};

mod consumer;
use consumer::{consumer, pack_for, pack_root, run, write};

fn clean(r: &ToolReport) -> bool {
    matches!(r.outcome, Outcome::Clean { .. })
}

#[test]
fn every_tool_name_is_registered_once() {
    let pack = pack_for(None);
    let mut seen = BTreeSet::new();
    for t in &pack.tools {
        assert!(seen.insert(t.name()), "`{}` is registered twice", t.name());
    }
    // Every tool of `crates/tools`, by its directory, so a crate the feature or `collect` leaves out
    // is named here.
    let dirs = fs::read_dir(pack_root().join("crates/tools")).unwrap();
    let mut expected: Vec<String> = dirs
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    expected.sort();
    assert_eq!(expected.len(), 18, "{expected:?}");
    for name in &expected {
        assert!(seen.contains(name.as_str()), "`{name}` is not registered");
    }
    // THE CONTROL: a name no crate carries is not found, so the loop above is not passing on a set
    // that holds everything.
    assert!(
        !seen.contains("bevy-quote-check"),
        "the bevy arm ships with the pack"
    );
}

#[test]
fn a_consumer_adds_moves_and_lists_a_ticket_and_asks_its_canon() {
    let root = consumer("ticket", "mock", "");
    let pack = pack_for(None);

    let added = run(&pack, &root, "ticket-add", &[
        "Wet the ground",
        "--by",
        "tester",
        "--at",
        "2026-11-13T10:00:00Z",
    ]);
    assert!(clean(&added), "{:?} {}", added.outcome, added.output);
    assert!(
        root.join("mock/registry/ticket/items")
            .read_dir()
            .unwrap()
            .next()
            .is_some(),
        "the ticket was written under the consumer's registry"
    );

    let moved = run(&pack, &root, "ticket-move", &[
        "wet_the_ground",
        "ready",
        "--by",
        "tester",
        "--at",
        "2026-11-13T11:00:00Z",
    ]);
    assert!(clean(&moved), "{:?} {}", moved.outcome, moved.output);

    let listed = run(&pack, &root, "tickets", &["list", "--tree"]);
    let line = listed
        .output
        .lines()
        .find(|l| l.contains("wet_the_ground"))
        .unwrap_or_else(|| panic!("the ticket is listed: {}", listed.output));
    assert!(
        line.contains("ready") && line.contains("Wet the ground"),
        "{line}"
    );
    // THE CONTROL: a state the ticket is not in finds nothing.
    let none = run(&pack, &root, "tickets", &[
        "list", "--tree", "--state", "doing",
    ]);
    assert!(!none.output.contains("wet_the_ground"), "{}", none.output);

    write(
        &root,
        "mock/research/202611130000_ground/NOTES.md",
        "# Ground\n\n## Rain\n\nRain wets the ground it falls on.\n",
    );
    write(
        &root,
        "mock/registry/tenet/ground.toml",
        "[[subject]]\nid = \"ground\"\nwhat = \"The ground and what lies on it.\"\n\n\
         [[tenet]]\nid = \"rain_wets_the_ground\"\nwhat = \"Rain wets the ground it falls on, and the ground stays wet a while.\"\n\
         subject = \"ground\"\nstanding = \"ratified\"\nkeywords = [\"rain\", \"wet\", \"puddle\"]\n\
         provenance = [\"research::202611130000_ground::NOTES::#rain\"]\n",
    );

    let found = run(&pack, &root, "canon", &["find", "puddle"]);
    assert!(
        found.output.contains("rain_wets_the_ground"),
        "{:?} {}",
        found.outcome,
        found.output
    );

    // THE CONTROL: a word no row carries finds nothing, so the hit above is the tenet's keyword and
    // not a listing of every row.
    let missed = run(&pack, &root, "canon", &["find", "glacier"]);
    assert!(
        !missed.output.contains("rain_wets_the_ground"),
        "{:?} {}",
        missed.outcome,
        missed.output
    );

    let checked = run(&pack, &root, "canon", &["check", "rain_wets_the_ground"]);
    assert!(clean(&checked), "{:?} {}", checked.outcome, checked.output);
    // THE CONTROL: an id that is no tenet is a finding.
    let refused = run(&pack, &root, "canon", &["check", "snow_wets_the_ground"]);
    match refused.outcome {
        Outcome::Findings(f) => {
            assert!(
                f.iter().any(|e| e.message.contains("snow_wets_the_ground")),
                "{f:?}"
            )
        },
        other => panic!("{other:?}"),
    }

    let _ = fs::remove_dir_all(&root);
}

/// Two subjects with a tenet each, so `tackle` has something to rank and every subject is named.
fn two_subjects(root: &Path) {
    write(
        root,
        "mock/registry/tenet/ground.toml",
        "[[subject]]\nid = \"ground\"\nwhat = \"The ground and what lies on it.\"\n\n\
         [[subject]]\nid = \"tooling\"\nwhat = \"How the project is built.\"\n\n\
         [[tenet]]\nid = \"rain_wets_the_ground\"\nwhat = \"Rain wets the ground it falls on.\"\n\
         subject = \"ground\"\nstanding = \"ratified\"\n\n\
         [[tenet]]\nid = \"a_build_is_one_command\"\nwhat = \"A build is one command.\"\n\
         subject = \"tooling\"\nstanding = \"ratified\"\n",
    );
}

#[test]
fn a_consumer_names_the_subjects_about_the_work_in_its_tools_table() {
    let root = consumer(
        "tackle",
        "mock",
        "[tools.tackle]\nabout_the_work = [\"ground\", \"tooling\"]\n",
    );
    two_subjects(&root);
    let out = run(&pack_for(Some(&root)), &root, "tackle", &["next"]);
    match &out.outcome {
        Outcome::Inconclusive {
            reason,
        } => {
            assert!(reason.contains("about the work"), "{reason}")
        },
        other => panic!("both subjects are named about the work, so none is picked: {other:?}"),
    }

    // THE CONTROL: the same consumer with no table has a subject picked, so the refusal above is the
    // table's doing.
    let bare = consumer("tackle-bare", "mock", "");
    two_subjects(&bare);
    let picked = run(&pack_for(Some(&bare)), &bare, "tackle", &["next"]);
    assert!(
        picked.output.contains("TACKLE NEXT:"),
        "{:?} {}",
        picked.outcome,
        picked.output
    );

    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&bare);
}
