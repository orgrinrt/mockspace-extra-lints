//! The lint over a repository whose mock directory is not `mock`: it reads the registry under the
//! directory the context names, through the view it is handed.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use mockspace::testkit::check_repo;
use mockspace::{LintError, RegistryView, RepoInput};

use crate::canon_rows::testkit::scratch;

const SHIPPED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../registry/ticket");

fn ticket(id: &str, needs: &[&str]) -> String {
    let needs: Vec<String> = needs.iter().map(|n| format!("\"{n}\"")).collect();
    format!(
        "[[ticket]]\nid = \"{id}\"\ntitle = \"{id}\"\nkind = \"task\"\nneeds = [{}]\n\n[[ticket_move]]\nid = \"m_{id}\"\nticket = \"{id}\"\nto = \"backlog\"\nby = \"tester\"\nat = \"2026-10-23T10:00:00Z\"\n",
        needs.join(", ")
    )
}

/// A repository whose mock directory is `linux`, holding the shipped vocabularies and `tickets`.
fn repo(tickets: &[(&str, &[&str])]) -> PathBuf {
    let d = scratch("ticket-graph");
    let dir = d.join("linux/registry/ticket");
    std::fs::create_dir_all(dir.join("items")).unwrap();
    for f in ["states.toml", "kinds.toml"] {
        std::fs::copy(Path::new(SHIPPED).join(f), dir.join(f)).unwrap();
    }
    for (id, needs) in tickets {
        std::fs::write(dir.join("items").join(format!("{id}.toml")), ticket(id, needs)).unwrap();
    }
    d
}

fn check(root: &Path) -> Vec<LintError> {
    let crates = BTreeSet::new();
    let registry = RegistryView::default();
    let mock: PathBuf = root.join("linux");
    check_repo(super::repo_lint().as_ref(), RepoInput {
        mock_dir:    &mock,
        repo_root:   root,
        all_crates:  &crates,
        src_dirs:    &[],
        invocation:  None,
        canon_paths: &[],
        open_panels: &[],
        ref_files:   &mockspace::src_layout::NO_REF_FILES,
        registry:    &registry,
    })
}

#[test]
fn a_graph_that_holds_under_another_mock_directory_finds_nothing() {
    let d = repo(&[("chicken", &[]), ("egg", &["chicken"])]);
    let found = check(&d);
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_cycle_in_needs_under_another_mock_directory_is_found() {
    let d = repo(&[("chicken", &["egg"]), ("egg", &["chicken"])]);
    let found = check(&d);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.contains("goes round"), "{}", found[0].message);
}

#[test]
fn a_registry_read_through_the_view_holds_every_toml_file_under_it() {
    let d = repo(&[("chicken", &[])]);
    let reading = mockspace::reading::Reading::new(&d);
    let registry = d.join("linux/registry");
    let texts = super::registry_texts(&reading.view(), &registry).unwrap();
    let mut names: Vec<String> = texts
        .iter()
        .map(|(p, _)| Path::new(p).file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["chicken.toml", "kinds.toml", "states.toml"]);
    assert!(texts.iter().all(|(_, t)| !t.is_empty()));
}
