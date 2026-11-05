use std::collections::BTreeMap;
use std::path::Path;

use mockspace_lint_rules::RegistryView;

use super::*;

fn schema(extra_field: &str) -> String {
    format!(
        r#"
[[registry.namespace]]
key = "tenet"

[[registry.namespace]]
key = "bevy"

[[registry.namespace]]
key = "ticket"

[[registry.namespace.field]]
name = "related"
type = "ref[]"

[[registry.namespace.field]]
name = "needs"
type = "ticket[]"

[[registry.namespace.field]]
name = "kind"
type = "tenet"
{extra_field}
"#
    )
}

fn run_with(lint: &OneRelationField, root: &Path) -> Vec<LintError> {
    let crates = BTreeSet::new();
    let registry = RegistryView::new(BTreeMap::new(), BTreeMap::new());
    lint.check_repo(&RepoContext {
        mock_dir: &root.join("mock"),
        repo_root: root,
        all_crates: &crates,
        src_dirs: &[],
        invocation: None,
        canon_paths: &[],
        open_panels: &[],
        registry: &registry,
        verdicts: None,
    })
}

fn run(toml_text: &str) -> Vec<LintError> {
    let d = tempfile::tempdir().unwrap();
    fs::write(d.path().join("mockspace.toml"), toml_text).unwrap();
    run_with(&OneRelationField::default(), d.path())
}

#[test]
fn a_field_per_target_namespace_beside_the_relation_field_is_refused() {
    let found = run(&schema("\n[[registry.namespace.field]]\nname = \"bevy\"\ntype = \"bevy[]\"\n"));
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.contains("`bevy`"), "{}", found[0].message);
    assert!(found[0].message.contains("`ticket`"), "{}", found[0].message);
    assert_eq!(found[0].finding_kind, Some(PER_TARGET));
}

#[test]
fn the_structural_fields_and_single_slugs_are_let_through() {
    // `needs` is a list of another namespace's slugs and `kind` a single slug, and neither is refused
    assert!(run(&schema("")).is_empty());
}

#[test]
fn a_namespace_with_no_relation_field_is_not_read() {
    let text = r#"
[[registry.namespace]]
key = "bevy"

[[registry.namespace]]
key = "tenet"

[[registry.namespace.field]]
name = "bevy"
type = "bevy[]"
"#;
    assert!(run(text).is_empty());
}

#[test]
fn a_list_of_something_that_is_no_namespace_is_not_a_target() {
    let found = run(&schema("\n[[registry.namespace.field]]\nname = \"keywords\"\ntype = \"string[]\"\n"));
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn the_structural_list_is_the_projects_to_replace() {
    let d = tempfile::tempdir().unwrap();
    fs::write(
        d.path().join("mockspace.toml"),
        schema("\n[[registry.namespace.field]]\nname = \"bevy\"\ntype = \"bevy[]\"\n"),
    )
    .unwrap();
    let mut lint = OneRelationField::default();
    lint.configure(&HashMap::from([(STRUCTURAL_KEY.to_string(), "needs, bevy".to_string())]));
    assert!(run_with(&lint, d.path()).is_empty());
    // the control: replacing the list drops the defaults, so `needs` is now refused
    lint.configure(&HashMap::from([(STRUCTURAL_KEY.to_string(), "bevy".to_string())]));
    assert_eq!(run_with(&lint, d.path()).len(), 1);
}

#[test]
fn a_repository_with_no_schema_or_an_unreadable_one_gets_no_finding() {
    let d = tempfile::tempdir().unwrap();
    assert!(run_with(&OneRelationField::default(), d.path()).is_empty());
    assert!(run("this is = not [ toml").is_empty());
}
