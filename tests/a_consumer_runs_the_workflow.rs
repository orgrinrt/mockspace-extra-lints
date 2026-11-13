//! A repository that names the pack with the `workflow` feature, declares the namespaces the pack
//! ships in `registry/namespaces.toml` and copies its vocabulary, gets the workflow tools from the
//! pack alone and nothing of its own: a ticket is added, moved and listed, a tenet is written by
//! hand, and `canon find` and `canon check` answer over it.
//!
//! Every step goes the way a run goes: the engine's own config reader reads the scratch's
//! `mockspace.toml`, the registry is loaded and viewed as the tool runner loads it, and the tool is
//! the one the pack's `collect` registered under that name, so a tool the feature does not register
//! or a declaration the shipped file lacks fails here.

#![cfg(feature = "workflow")]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mockspace::config::Config;
use mockspace_lint_rules::LintPack;
use mockspace_lint_rules::tool::{Outcome, ToolContext, ToolReport};

/// The pack's root, where its shipped `registry/` sits.
fn pack_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

/// A consumer on `dev`: its config is its own root keys, then the shipped declarations as they
/// are; its registry holds the shipped ticket vocabulary and planning roles and no row of its own.
fn consumer() -> PathBuf {
    let root = std::env::temp_dir().join(format!("extra-lints-consumer-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let shipped = pack_root().join("registry");
    let declarations = fs::read_to_string(shipped.join("namespaces.toml")).unwrap();
    write(
        &root,
        "mockspace.toml",
        &format!(
            "project_name = \"scratch\"\ncrate_prefix = \"scratch\"\nmock_dir = \"mock\"\nsrc_dirs = [\"crates\"]\n\n{declarations}"
        ),
    );
    for f in ["ticket/states.toml", "ticket/kinds.toml", "plan/roles.toml"] {
        write(
            &root,
            &format!("mock/registry/{f}"),
            &fs::read_to_string(shipped.join(f)).unwrap(),
        );
    }
    fs::create_dir_all(root.join("mock/crates")).unwrap();
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    };
    git(&["init", "-q", "-b", "dev"]);
    git(&[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "commit",
        "-q",
        "--allow-empty",
        "-m",
        "base",
    ]);
    root
}

/// The pack's tools as the engine gets them.
fn pack() -> LintPack {
    let mut pack = LintPack::default();
    mockspace_extra_lints::collect(&mut pack);
    pack
}

/// Runs the tool registered as `name` over the consumer as it stands now.
fn run(pack: &LintPack, root: &Path, name: &str, args: &[&str]) -> ToolReport {
    let cfg = Config::from_dir(&root.join("mock"));
    let registry = mockspace::registry::load_registry_for(&cfg).expect("the registry loads");
    let view = mockspace::registry::build_view(&registry, &cfg.registry_namespaces);
    let tool = pack
        .tools
        .iter()
        .find(|t| t.name() == name)
        .unwrap_or_else(|| panic!("the pack registers `{name}`"));
    let crates = BTreeSet::new();
    tool.run(&ToolContext {
        mock_dir: &cfg.mock_dir,
        repo_root: &cfg.repo_root,
        all_crates: &crates,
        src_dirs: &cfg.src_dirs,
        args,
        stdin: None,
        registry: &view,
    })
}

fn clean(r: &ToolReport) -> bool {
    matches!(r.outcome, Outcome::Clean { .. })
}

#[test]
fn every_tool_name_is_registered_once() {
    let pack = pack();
    let mut seen = BTreeSet::new();
    for t in &pack.tools {
        assert!(seen.insert(t.name()), "`{}` is registered twice", t.name());
    }
    for name in ["ticket-add", "ticket-move", "tickets", "canon", "owed", "corpus", "unearth"] {
        assert!(seen.contains(name), "`{name}` is not registered");
    }
}

#[test]
fn a_consumer_adds_moves_and_lists_a_ticket_and_asks_its_canon() {
    let root = consumer();
    let pack = pack();

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
