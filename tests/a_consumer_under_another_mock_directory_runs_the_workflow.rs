//! A consumer whose `mockspace.toml` declares `mock_dir = "linux"` and keeps its content packs in
//! `packs/`, named by `content_dirs` in the tables of the lints and tools that read content: every
//! path the workflow reads under the mock directory is read under `linux/`, and the packs are the
//! content tier. A lint or tool that still reads `mock/` or `content/` by name finds the wrong files
//! here, or none.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use mockspace_lint_rules::src_layout::NO_REF_FILES;
use mockspace_lint_rules::testkit::check_repo;
use mockspace_lint_rules::{LintError, RepoInput};

mod consumer;
use consumer::{config, consumer, pack_for, run, write};

/// The marker word, put together so this file holds no marker of its own.
const MARK: &str = concat!("FIX", "ME");

/// The lint and tool tables naming the content directory.
const TABLES: &str = "[lints.a-retired-word-stays-out]\ncontent_dirs = \"packs\"\n\n\
                      [lints.every-fixme-names-a-ticket]\ncontent_dirs = \"packs\"\n\n\
                      [tools.canon]\ncontent_dirs = [\"packs\"]\n\n\
                      [tools.owed]\nholder = \"coord\"\ncontent_dirs = [\"packs\"]\n";

/// A tenet that rules out `phlogiston`, and the subject it is about.
fn fire(root: &Path) {
    write(
        root,
        "linux/registry/tenet/fire.toml",
        "[[subject]]\nid = \"fire\"\nwhat = \"What burns and how.\"\n\n\
         [[tenet]]\nid = \"burning_is_oxidation\"\nwhat = \"Burning is oxidation.\"\n\
         subject = \"fire\"\nstanding = \"ratified\"\nrules_out = [\"phlogiston\"]\n",
    );
}

/// The findings of the repository lint registered as `name`, configured from the consumer's
/// `[lints]` and run over it the way the engine runs one.
fn lint(root: &Path, name: &str) -> Vec<LintError> {
    let pack = pack_for(Some(root));
    let cfg = config(root);
    let registry = mockspace::registry::load_registry_for(&cfg).expect("the registry loads");
    let view = mockspace::registry::build_view(&registry, &cfg.registry_namespaces);
    let lint = pack
        .repo_lints
        .iter()
        .find(|l| l.name() == name)
        .unwrap_or_else(|| panic!("the pack registers `{name}`"));
    let crates = BTreeSet::new();
    check_repo(lint.as_ref(), RepoInput {
        mock_dir:    &cfg.mock_dir,
        repo_root:   &cfg.repo_root,
        all_crates:  &crates,
        src_dirs:    &cfg.src_dirs,
        invocation:  None,
        canon_paths: &[],
        open_panels: &[],
        ref_files:   &NO_REF_FILES,
        registry:    &view,
    })
}

fn paths(found: &[LintError]) -> Vec<String> {
    let mut p: Vec<String> = found.iter().filter_map(|e| e.path.clone()).collect();
    p.sort();
    p.dedup();
    p
}

#[test]
fn a_marker_in_a_research_note_under_the_mock_directory_is_a_record_and_not_refused() {
    let root = consumer("linux-fixme", "linux", TABLES);
    write(
        &root,
        "linux/research/202611130000_fire/NOTES.md",
        &format!("# Fire\n\nThe note quotes `// {MARK}: stand-in` as it stood.\n"),
    );
    write(&root, "linux/crates/alpha/src/lib.rs", &format!("// {MARK}: stand-in\n"));
    write(&root, "linux/vendor/beta/lib.rs", &format!("// {MARK}: vendored\n"));
    let found = lint(&root, "every-fixme-names-a-ticket");
    assert_eq!(paths(&found), ["linux/crates/alpha/src/lib.rs"], "{found:?}");
    assert_eq!(found[0].crate_name, "alpha", "{found:?}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_retired_word_in_a_content_directory_the_table_names_is_found() {
    let root = consumer("linux-retired", "linux", TABLES);
    fire(&root);
    write(&root, "packs/fire.toml", "kind = \"phlogiston\"\n");
    write(&root, "linux/crates/alpha/DESIGN.md.tmpl", "# Alpha\n\nIt burns phlogiston.\n");
    let found: Vec<LintError> = lint(&root, "a-retired-word-stays-out")
        .into_iter()
        .filter(|e| e.path.as_deref() != Some("mockspace.toml"))
        .collect();
    assert_eq!(paths(&found), ["linux/crates/alpha/DESIGN.md.tmpl", "packs/fire.toml"], "{found:?}");
    let units: BTreeSet<&str> = found.iter().map(|e| e.crate_name.as_str()).collect();
    assert_eq!(units, BTreeSet::from(["alpha", "packs"]), "{found:?}");

    // THE CONTROL: with no table naming it, the content directory is not read.
    let bare = consumer("linux-retired-bare", "linux", "");
    fire(&bare);
    write(&bare, "packs/fire.toml", "kind = \"phlogiston\"\n");
    let none = lint(&bare, "a-retired-word-stays-out");
    assert!(!paths(&none).contains(&"packs/fire.toml".to_owned()), "{none:?}");
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&bare);
}

#[test]
fn canon_contradicts_reads_the_content_directory_the_table_names() {
    let root = consumer("linux-canon", "linux", TABLES);
    fire(&root);
    write(&root, "packs/fire.toml", "kind = \"phlogiston\"\n");
    let pack = pack_for(Some(&root));
    let out = run(&pack, &root, "canon", &["contradicts", "packs/fire.toml"]);
    assert!(
        out.output.contains("packs/fire.toml") && out.output.contains("burning_is_oxidation"),
        "{:?} {}",
        out.outcome,
        out.output
    );
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn owed_files_a_live_marker_under_the_mock_directory_for_the_holder_the_table_names() {
    let root = consumer("linux-owed", "linux", TABLES);
    write(&root, "linux/crates/alpha/src/lib.rs", &format!("// {MARK}: stand-in roof\n"));
    write(
        &root,
        "linux/research/202611130000_roof/NOTES.md",
        &format!("# Roof\n\nQuoted: `// {MARK}: a flat roof`.\n"),
    );
    let pack = pack_for(Some(&root));
    let dry = run(&pack, &root, "owed", &["--dry", "--no-fetch"]);
    assert!(dry.output.contains("linux/crates/alpha/src/lib.rs:1"), "{}", dry.output);
    assert!(!dry.output.contains("202611130000_roof"), "{}", dry.output);
    let done = run(&pack, &root, "owed", &["--no-fetch"]);
    assert!(done.output.contains("filed 1 tasks"), "{:?} {}", done.outcome, done.output);
    let items = root.join("linux/registry/ticket/items");
    let filed: Vec<String> = fs::read_dir(&items)
        .unwrap_or_else(|e| panic!("{}: {e}", items.display()))
        .map(|e| fs::read_to_string(e.unwrap().path()).unwrap())
        .collect();
    assert_eq!(filed.len(), 1, "{filed:?}");
    assert!(filed[0].contains("holder = \"coord\""), "{}", filed[0]);
    let _ = fs::remove_dir_all(&root);
}

#[test]
fn a_research_bundle_is_written_under_the_mock_directory() {
    let root = consumer("linux-unbundle", "linux", "");
    write(&root, "bundle.txt", "=== FILE: NOTES.md ===\n# Fire\n");
    let pack = pack_for(Some(&root));
    let refused = run(&pack, &root, "research-unbundle", &[
        "bundle.txt",
        "mock/research/202611130000_fire",
    ]);
    assert!(
        refused.output.contains("linux/research/") || format!("{:?}", refused.outcome).contains("linux/research/"),
        "{:?} {}",
        refused.outcome,
        refused.output
    );
    run(&pack, &root, "research-unbundle", &["bundle.txt", "linux/research/202611130000_fire"]);
    assert!(root.join("linux/research/202611130000_fire/NOTES.md").exists());
    let _ = fs::remove_dir_all(&root);
}
