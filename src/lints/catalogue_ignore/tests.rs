use std::collections::BTreeMap;
use std::path::Path;

use mockspace_lint_rules::RegistryView;

use super::*;

fn registry(rows: &[&str]) -> RegistryView {
    let rows = rows
        .iter()
        .map(|q| ((*q).to_string(), BTreeMap::new()))
        .collect::<BTreeMap<_, _>>();
    RegistryView::new(rows, BTreeMap::new())
}

fn configured(ledgers: &str) -> CatalogueIgnore {
    let mut lint = CatalogueIgnore::default();
    lint.configure(&HashMap::from([(LEDGERS_KEY.to_string(), ledgers.to_string())]));
    lint
}

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn run_with(lint: &CatalogueIgnore, reg: &RegistryView, root: &Path) -> Vec<LintError> {
    let crates = BTreeSet::from(["alpha".to_string()]);
    let ctx = RepoContext {
        mock_dir: &root.join("mock"),
        repo_root: root,
        all_crates: &crates,
        src_dirs: &[],
        invocation: None,
        canon_paths: &[],
        open_panels: &[],
        registry: reg,
    };
    lint.check_repo(&ctx)
}

/// A repository holding one test file with `body`, a ledger with rows `C1-001` and `C2-054`, and a
/// registry with the row `tenet::the_first`.
fn run(body: &str) -> Vec<LintError> {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "mock/crates/alpha/src/tests/a.rs", body);
    write(
        d.path(),
        "mock/research/LEDGER.md",
        "# Ledger\n\n- `C1-001` a row\n| C2-054 | another |\n",
    );
    run_with(&configured("mock/research/LEDGER.md"), &registry(&["tenet::the_first"]), d.path())
}

fn kinds(found: &[LintError]) -> Vec<Option<&'static str>> {
    found.iter().map(|e| e.finding_kind).collect()
}

const GOOD: &str = "#[test]\n#[ignore = \"catalogue: the gap; tracked #C1-001\"]\nfn t() {}\n";

#[test]
fn a_catalogue_ignore_that_resolves_passes() {
    assert!(run(GOOD).is_empty());
}

#[test]
fn a_bare_ignore_is_refused_at_its_line_with_its_crate_and_path() {
    let found = run("#[test]\n\n#[ignore]\nfn t() {}\n");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, 3);
    assert_eq!(found[0].crate_name, "alpha");
    assert_eq!(found[0].path.as_deref(), Some("mock/crates/alpha/src/tests/a.rs"));
    assert_eq!(found[0].finding_kind, Some(BARE));
    assert_eq!(found[0].severity, Severity::HARD_ERROR);
}

#[test]
fn a_reason_of_any_other_shape_is_refused_and_each_says_what_is_wrong() {
    let shapes = [
        ("a probe, run by hand", "does not begin"),
        ("catalogue:the gap; tracked #C1-001", "followed by a space"),
        ("catalogue: the gap", "no `; tracked"),
        ("catalogue: ; tracked #C1-001", "gap is empty"),
        ("catalogue: the gap; tracked work/creatures-3d", "not a list of `#<id>`"),
        ("catalogue: the gap; tracked FIXME(stand-in)", "not a list of `#<id>`"),
        ("catalogue: the gap; tracked #", "an id is made of"),
        ("catalogue: the gap; tracked # C1-001", "an id is made of"),
        ("catalogue: the gap; tracked #C1-001 and more", "an id is made of"),
        ("catalogue: the gap; tracked #C1-001,", "not a list of `#<id>`"),
        ("Catalogue: the gap; tracked #C1-001", "does not begin"),
    ];
    for (reason, why) in shapes {
        let found = run(&format!("#[ignore = \"{reason}\"]\nfn t() {{}}\n"));
        assert_eq!(found.len(), 1, "{reason}: {found:?}");
        assert_eq!(found[0].finding_kind, Some(MALFORMED), "{reason}");
        assert!(found[0].message.contains(why), "{reason}: {:?}", found[0].message);
    }
}

#[test]
fn several_ids_all_have_to_resolve_and_the_finding_names_the_ones_that_do_not() {
    let ok = run("#[ignore = \"catalogue: g; tracked #C1-001, #C2-054, #the_first\"]\nfn t() {}\n");
    assert!(ok.is_empty(), "{ok:?}");
    let found = run("#[ignore = \"catalogue: g; tracked #C1-001, #C9-999, #nowhere\"]\nfn t() {}\n");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].finding_kind, Some(UNRESOLVED));
    assert!(found[0].message.contains("`#C9-999`") && found[0].message.contains("`#nowhere`"));
    assert!(!found[0].message.contains("`#C1-001`"), "{:?}", found[0].message);
}

#[test]
fn an_id_resolves_to_a_registry_row_by_its_slug() {
    let by_slug = run("#[ignore = \"catalogue: g; tracked #the_first\"]\nfn t() {}\n");
    assert!(by_slug.is_empty(), "{by_slug:?}");
    let by_namespace = run("#[ignore = \"catalogue: g; tracked #tenet\"]\nfn t() {}\n");
    assert_eq!(kinds(&by_namespace), [Some(UNRESOLVED)], "{by_namespace:?}");
}

#[test]
fn a_ledger_row_is_a_list_item_or_a_table_row_and_nothing_else_is() {
    let rows = |line: &str| row_id(line);
    assert_eq!(rows("- `C1-192` `kaski/src/lib.rs:632` | live | x"), Some("C1-192".into()));
    assert_eq!(rows("* C1-001 a row"), Some("C1-001".into()));
    assert_eq!(rows("  | C2-054 | another |"), Some("C2-054".into()));
    assert_eq!(rows("|`M-3`|x|"), Some("M-3".into()));
    assert_eq!(rows("## Map"), None);
    assert_eq!(rows("The census review turned up"), None);
    assert_eq!(rows("-C1-001 no space after the marker"), None);
    assert_eq!(rows("- "), None);
    assert_eq!(rows("| --- | --- |"), None);
}

#[test]
fn an_id_only_the_prose_of_the_ledger_mentions_does_not_resolve() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "mock/crates/alpha/src/lib.rs", "#[ignore = \"catalogue: g; tracked #C7-007\"]\nfn t() {}\n");
    write(d.path(), "LEDGER.md", "See C7-007 for the history.\n\n- `C1-001` a row\n");
    let found = run_with(&configured("LEDGER.md"), &registry(&[]), d.path());
    assert_eq!(kinds(&found), [Some(UNRESOLVED)], "{found:?}");
}

#[test]
fn the_ledgers_come_from_the_config_and_an_unconfigured_lint_resolves_only_the_registry() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "mock/crates/alpha/src/lib.rs", GOOD);
    write(d.path(), "LEDGER.md", "- `C1-001` a row\n");
    let reg = registry(&[]);
    assert!(run_with(&configured("LEDGER.md"), &reg, d.path()).is_empty());
    let unconfigured = run_with(&CatalogueIgnore::default(), &reg, d.path());
    assert_eq!(kinds(&unconfigured), [Some(UNRESOLVED)], "{unconfigured:?}");
    let elsewhere = run_with(&configured("docs/*.md"), &reg, d.path());
    assert_eq!(kinds(&elsewhere), [Some(UNRESOLVED)], "{elsewhere:?}");
}

#[test]
fn the_ledgers_key_takes_a_list_and_is_the_only_key() {
    let lint = configured(" a.md , b/*.md ,, ");
    assert_eq!(lint.ledgers, ["a.md", "b/*.md"]);
    assert_eq!(lint.config_keys(), &[LEDGERS_KEY]);
}

#[test]
fn a_reason_across_lines_with_a_continuation_is_read_whole() {
    let found = run(
        "#[ignore = \"catalogue: the web target serves no store yet, so the page \\\n            cannot read the pack; tracked #C1-001\"]\nfn t() {}\n",
    );
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_raw_string_reason_is_read() {
    let found = run("#[ignore = r#\"catalogue: a \"quoted\" gap; tracked #C1-001\"#]\nfn t() {}\n");
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn a_reason_that_is_not_a_literal_is_malformed_and_so_is_the_call_form() {
    let by_macro = run("#[ignore = concat!(\"catalogue: g; \", \"tracked #C1-001\")]\nfn t() {}\n");
    assert_eq!(kinds(&by_macro), [Some(MALFORMED)], "{by_macro:?}");
    let by_call = run("#[ignore(\"catalogue: g; tracked #C1-001\")]\nfn t() {}\n");
    assert_eq!(kinds(&by_call), [Some(MALFORMED)], "{by_call:?}");
}

#[test]
fn only_the_attribute_is_an_ignore() {
    let body = concat!(
        "// #[ignore]\n",
        "/// #[ignore = \"x\"]\n",
        "const S: &str = \"#[ignore]\";\n",
        "#[derive(Debug)]\n",
        "#[should_panic]\n",
        "#[cfg_attr(unix, ignore)]\n",
        "#[ignored]\n",
        "fn ignore() {}\n",
        "macro_rules! m { () => { #[ignore] fn x() {} } }\n",
    );
    let found = run(body);
    // The macro body is tokens, not an item, so it is not an attribute either.
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn an_ignore_in_a_module_a_method_and_an_integration_test_is_found() {
    let d = tempfile::tempdir().unwrap();
    write(
        d.path(),
        "mock/crates/alpha/src/lib.rs",
        "mod t {\n    struct S;\n    impl S {\n        #[ignore]\n        fn m(&self) {}\n    }\n}\n",
    );
    write(d.path(), "mock/crates/alpha/tests/it.rs", "#[ignore]\nfn t() {}\n");
    write(d.path(), "mock/tools/x.rs", "#[ignore]\nfn t() {}\n");
    let found = run_with(&CatalogueIgnore::default(), &registry(&[]), d.path());
    let at: Vec<_> = found.iter().map(|e| (e.path.clone().unwrap(), e.line, e.crate_name.clone())).collect();
    assert_eq!(
        at,
        [
            ("mock/crates/alpha/src/lib.rs".to_string(), 4, "alpha".to_string()),
            ("mock/crates/alpha/tests/it.rs".to_string(), 1, "alpha".to_string()),
            ("mock/tools/x.rs".to_string(), 1, "workspace".to_string()),
        ]
    );
}

#[test]
fn build_output_and_vendored_sources_are_not_read() {
    let d = tempfile::tempdir().unwrap();
    write(d.path(), "target/debug/x.rs", "#[ignore]\nfn t() {}\n");
    write(d.path(), "mock/vendor/dep/src/lib.rs", "#[ignore]\nfn t() {}\n");
    let found = run_with(&CatalogueIgnore::default(), &registry(&[]), d.path());
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn the_default_blocks_every_gate_and_the_kinds_are_declared() {
    let lint = CatalogueIgnore::default();
    assert_eq!(lint.default_severity(), Severity::HARD_ERROR);
    assert_eq!(lint.finding_kinds(), &[BARE, MALFORMED, UNRESOLVED]);
    assert_eq!(lint.name(), "catalogue-ignore");
}

#[test]
fn the_ids_are_what_follows_the_last_tracked_so_a_gap_may_say_the_word() {
    let found = run("#[ignore = \"catalogue: it says; tracked #nowhere in prose; tracked #C1-001\"]\nfn t() {}\n");
    assert!(found.is_empty(), "{found:?}");
}
