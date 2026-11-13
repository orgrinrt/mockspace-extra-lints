//! Every lint of the pack reads the repository through the view it is handed, so what a unit
//! reads is what its trace says, and a saved finding replays only while its answers are unchanged.
//!
//! The guard reads every directory of the pack that holds source and is no tool's: the root's
//! `src/` and each crate of `crates/` outside `crates/tools/`. A lint declared anywhere the guard
//! does not read, a tool's crate above all, is never held to it, so every lint the pack registers
//! has to be declared where the guard reads.

use std::path::{Path, PathBuf};

const ROOT: &str = env!("CARGO_MANIFEST_DIR");

/// Every source directory the guard is run over: the root's, and each crate's outside the tools.
fn guarded() -> Vec<PathBuf> {
    let mut dirs = vec![Path::new(ROOT).join("src")];
    let mut crates: Vec<PathBuf> = std::fs::read_dir(Path::new(ROOT).join("crates"))
        .expect("the pack has `crates/`")
        .map(|e| e.expect("a listing of `crates/`").path())
        .filter(|p| p.file_name().is_some_and(|n| n != "tools") && p.join("src").is_dir())
        .map(|p| p.join("src"))
        .collect();
    crates.sort();
    dirs.extend(crates);
    dirs
}

/// Whether the guard reads a file as lint source: the rule it states for test code and tools.
fn read_by_the_guard(path: &Path) -> bool {
    let named_out = path.components().any(|c| {
        let c = c.as_os_str().to_string_lossy();
        c == "tests" || c == "tools" || c == "target" || c.starts_with('.')
    });
    let test_file = path
        .file_stem()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s == "tests" || s.ends_with("_tests"));
    path.extension().is_some_and(|e| e == "rs") && !named_out && !test_file
}

fn sources(dir: &Path, out: &mut Vec<(PathBuf, String)>) {
    for entry in std::fs::read_dir(dir).expect("a source directory") {
        let path = entry.expect("an entry").path();
        let rel = path.strip_prefix(ROOT).unwrap_or(&path).to_path_buf();
        if path.is_dir() {
            sources(&path, out);
        } else if read_by_the_guard(&rel) {
            let text = std::fs::read_to_string(&path).expect("a source file");
            out.push((rel, text));
        }
    }
}

/// The name of every lint the pack registers with the workflow on.
fn registered() -> Vec<String> {
    let mut pack = mockspace_lint_rules::LintPack::default();
    mockspace_extra_lints::collect(&mut pack);
    let mut names: Vec<String> = pack
        .crate_lints
        .iter()
        .map(|l| l.name())
        .chain(pack.workspace_lints.iter().map(|l| l.name()))
        .chain(pack.repo_lints.iter().map(|l| l.name()))
        .chain(pack.message_lints.iter().map(|l| l.name()))
        .map(String::from)
        .collect();
    names.sort();
    names
}

#[test]
fn every_guarded_directory_reads_through_the_view_only() {
    for dir in guarded() {
        mockspace_lint_rules::assert_lints_read_through_the_view(&dir);
    }
}

#[test]
fn the_guard_reads_the_registry_lints() {
    assert!(
        guarded().contains(&Path::new(ROOT).join("crates/registry-lints/src")),
        "{:?}",
        guarded()
    );
}

#[test]
fn every_registered_lint_is_declared_where_the_guard_reads() {
    let mut files = Vec::new();
    for dir in guarded() {
        sources(&dir, &mut files);
    }
    let names = registered();
    assert!(names.iter().any(|n| n == "a-ticket-graph-holds"), "{names:?}");
    let unguarded: Vec<&String> = names
        .iter()
        .filter(|name| {
            // Declared where a file both names it and implements a lint's `name`, so a mention in a
            // doc string or a test elsewhere does not count as its declaration.
            let quoted = format!("\"{name}\"");
            !files
                .iter()
                .any(|(_, text)| text.contains(&quoted) && text.contains("fn name(&self)"))
        })
        .collect();
    assert!(
        unguarded.is_empty(),
        "these lints are declared nowhere the guard reads, so nothing holds them to the view: {unguarded:?}"
    );
}

#[test]
fn a_tool_crate_is_not_where_the_guard_reads() {
    let tools = Path::new(ROOT).join("crates/tools/tickets/src");
    assert!(!guarded().contains(&tools));
    assert!(!read_by_the_guard(Path::new("crates/tools/tickets/src/lint.rs")));
    assert!(!read_by_the_guard(Path::new("crates/registry-lints/src/ratchets_tests.rs")));
    assert!(read_by_the_guard(Path::new("crates/registry-lints/src/ratchets.rs")));
}
