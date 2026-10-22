use std::collections::BTreeSet;

use super::*;

fn registry(rows: &[(&str, &[(&str, &str)])]) -> RegistryView {
    let rows = rows
        .iter()
        .map(|(q, f)| {
            let f = f
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect::<BTreeMap<_, _>>();
            ((*q).to_string(), f)
        })
        .collect::<BTreeMap<_, _>>();
    RegistryView::new(rows, BTreeMap::new())
}

/// Two replaced rows, one of them replaced twice over, and a live one.
fn canon() -> RegistryView {
    registry(&[
        ("tenet::old", &[("superseded_by", "mid")]),
        ("tenet::mid", &[("superseded_by", "new")]),
        ("tenet::new", &[("what", "x")]),
        ("tenet::split", &[("superseded_by", "left, right")]),
        ("tenet::left", &[]),
        ("tenet::right", &[]),
        ("tenet::old-rule", &[("superseded_by", "tenet::new-rule")]),
        ("tenet::new-rule", &[]),
    ])
}

fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

fn configured(field: &str) -> NoSupersededCitation {
    let mut lint = NoSupersededCitation::default();
    lint.configure(&HashMap::from([(FIELD_KEY.to_string(), field.to_string())]));
    lint
}

fn run_with(
    lint: &NoSupersededCitation,
    reg: &RegistryView,
    root: &Path,
    canon_paths: &[String],
) -> Vec<LintError> {
    let crates = BTreeSet::from(["alpha".to_string()]);
    let ctx = RepoContext {
        mock_dir: &root.join("mock"),
        repo_root: root,
        all_crates: &crates,
        src_dirs: &[],
        invocation: None,
        canon_paths,
        open_panels: &[],
        registry: reg,
    };
    lint.check_repo(&ctx)
}

fn run(root: &Path, canon_paths: &[String]) -> Vec<LintError> {
    run_with(&configured("superseded_by"), &canon(), root, canon_paths)
}

fn messages(root: &Path) -> Vec<String> {
    run(root, &[]).into_iter().map(|e| e.message).collect()
}

fn dir(tag: &str) -> tempfile::TempDir {
    tempfile::Builder::new().prefix(tag).tempdir().unwrap()
}

#[test]
fn the_default_blocks_every_gate() {
    assert_eq!(NoSupersededCitation::default().default_severity(), Severity::HARD_ERROR);
}

#[test]
fn a_design_citing_a_superseded_row_is_reported_at_its_line_with_the_live_successor() {
    let d = dir("design");
    write(
        d.path(),
        "mock/crates/alpha/DESIGN.md.tmpl",
        "# alpha\n\nIt stands after `old`, and nothing else.\n",
    );
    let found = run(d.path(), &[]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, 3);
    assert_eq!(found[0].crate_name, "alpha");
    assert_eq!(
        found[0].path.as_deref(),
        Some("mock/crates/alpha/DESIGN.md.tmpl")
    );
    assert!(found[0].message.contains("`old`"), "{:?}", found[0].message);
    assert!(found[0].message.contains("`new`"), "{:?}", found[0].message);
    assert!(!found[0].message.contains("`mid`"), "{:?}", found[0].message);
    assert_eq!(found[0].severity, Severity::HARD_ERROR);
}

#[test]
fn source_citing_a_superseded_row_is_reported_bare_or_backticked() {
    let d = dir("source");
    write(
        d.path(),
        "mock/crates/alpha/src/lib.rs",
        "//! After old-rule.\nfn f() {} // `old` again\n",
    );
    let found = run(d.path(), &[]);
    assert_eq!(found.len(), 2, "{found:?}");
    assert_eq!(found[0].line, 1);
    assert_eq!(found[1].line, 2);
}

#[test]
fn a_row_split_into_two_names_both_successors() {
    let d = dir("split");
    write(d.path(), "mock/crates/alpha/src/lib.rs", "//! After `split`.\n");
    let m = messages(d.path());
    assert_eq!(m.len(), 1, "{m:?}");
    assert!(m[0].contains("`left`") && m[0].contains("`right`"), "{m:?}");
}

#[test]
fn a_live_row_and_a_longer_word_that_holds_the_slug_are_not_citations() {
    let d = dir("live");
    write(
        d.path(),
        "mock/crates/alpha/DESIGN.md.tmpl",
        "After `new`, and `older`, and `not_old`, and `old_enough`.\n",
    );
    assert!(messages(d.path()).is_empty());
}

#[test]
fn a_sentence_saying_the_row_is_superseded_may_name_it() {
    let d = dir("said");
    write(
        d.path(),
        "mock/crates/alpha/DESIGN.md.tmpl",
        "This replaces what `old` said, which is superseded by `new`. It also leans on\n`old` here.\n",
    );
    let found = run(d.path(), &[]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, 2);
}

#[test]
fn the_canon_the_rounds_the_research_and_generated_files_are_not_read() {
    let d = dir("skipped");
    // `old-rule` is a compound slug, so a bare mention is a citation wherever the file is read.
    write(d.path(), "mock/registry/tenet/a.toml", "what = \"old-rule\"\n");
    write(d.path(), "mock/design_rounds/1_topic.md", "old-rule\n");
    write(d.path(), "mock/research/1_x/NOTES.md", "old-rule\n");
    write(d.path(), "mock/target/debug/x.rs", "// old-rule\n");
    write(d.path(), "vendor/x/lib.rs", "// old-rule\n");
    write(
        d.path(),
        "docs/ALPHA.md",
        "<!--\n  AUTO-GENERATED: DO NOT EDIT\n-->\nold-rule\n",
    );
    let canon_paths = vec!["mock/registry/tenet/*.toml".to_string()];
    let found = run(d.path(), &canon_paths);
    assert!(found.is_empty(), "{found:?}");
    // The controls: the canon file is read when it is not declared canon, and the generated file
    // once the marker is gone, so the silence above is the skip and not a blind lint.
    let undeclared = run(d.path(), &[]);
    assert_eq!(undeclared.len(), 1, "{undeclared:?}");
    assert_eq!(undeclared[0].path.as_deref(), Some("mock/registry/tenet/a.toml"));
    write(d.path(), "docs/ALPHA.md", "old-rule\n");
    let found = run(d.path(), &canon_paths);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].path.as_deref(), Some("docs/ALPHA.md"));
}

#[test]
fn content_and_root_files_outside_the_crates_are_read() {
    let d = dir("content");
    write(d.path(), "content/pack/items.toml", "# after old-rule\n");
    write(d.path(), "mock/DESIGN.md.tmpl", "After `old`.\n");
    let found = run(d.path(), &[]);
    assert_eq!(found.len(), 2, "{found:?}");
}

#[test]
fn a_registry_with_nothing_superseded_finds_nothing() {
    let d = dir("none");
    write(d.path(), "mock/DESIGN.md.tmpl", "After `old`.\n");
    let reg = registry(&[("tenet::old", &[("what", "x")])]);
    assert!(run_with(&configured("superseded_by"), &reg, d.path(), &[]).is_empty());
}

#[test]
fn the_field_is_the_one_the_project_names_and_no_other() {
    let d = dir("named");
    write(d.path(), "mock/DESIGN.md.tmpl", "After `old` and `gone`.\n");
    let reg = registry(&[
        ("tenet::old", &[("superseded_by", "new")]),
        ("tenet::gone", &[("replaced_by", "new")]),
        ("tenet::new", &[]),
    ]);
    let first = run_with(&configured("superseded_by"), &reg, d.path(), &[]);
    assert_eq!(first.len(), 1, "{first:?}");
    assert!(first[0].message.contains("`old`"), "{first:?}");
    let second = run_with(&configured("replaced_by"), &reg, d.path(), &[]);
    assert_eq!(second.len(), 1, "{second:?}");
    assert!(second[0].message.contains("`gone`"), "{second:?}");
}

#[test]
fn a_project_with_rows_and_no_field_named_gets_one_finding_at_its_config() {
    let d = dir("unnamed");
    write(d.path(), "mock/DESIGN.md.tmpl", "After `old`.\n");
    for lint in [NoSupersededCitation::default(), configured("  ")] {
        let found = run_with(&lint, &canon(), d.path(), &[]);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].path.as_deref(), Some("mockspace.toml"));
        assert!(found[0].message.contains("`field`"), "{found:?}");
    }
    let empty = registry(&[]);
    assert!(run_with(&NoSupersededCitation::default(), &empty, d.path(), &[]).is_empty());
}

#[test]
fn the_field_is_a_declared_config_key() {
    assert_eq!(FIELD_KEY, "field");
    assert_eq!(NoSupersededCitation::default().config_keys(), &["field", "supersedes_field"]);
}

#[test]
fn the_configured_field_is_trimmed() {
    let d = dir("trim");
    write(d.path(), "mock/DESIGN.md.tmpl", "After `old`.\n");
    let found = run_with(&configured("  superseded_by \n"), &canon(), d.path(), &[]);
    assert_eq!(found.len(), 1, "{found:?}");
}

#[test]
fn a_slug_inside_a_longer_kebab_slug_is_not_a_citation() {
    let d = dir("kebab");
    let reg = registry(&[
        ("tenet::a-b", &[("superseded_by", "c")]),
        ("tenet::c", &[]),
    ]);
    write(d.path(), "mock/DESIGN.md.tmpl", "See a-b-c and `a-b`, then a-b-.\nAnd x-a-b.\n");
    let found = run_with(&configured("superseded_by"), &reg, d.path(), &[]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].line, 1);
}

#[test]
fn a_plain_word_slug_counts_only_backticked_and_a_compound_one_either_way() {
    let d = dir("plain");
    write(d.path(), "mock/A.md.tmpl", "The old way, and old-rule here.\n");
    let m = messages(d.path());
    assert_eq!(m.len(), 1, "{m:?}");
    assert!(m[0].contains("`old-rule`"), "{m:?}");
    write(d.path(), "mock/A.md.tmpl", "The `old` way.\n");
    assert_eq!(messages(d.path()).len(), 1);
    write(d.path(), "mock/A.md.tmpl", "The 'old' way and (old).\n");
    assert!(messages(d.path()).is_empty());
    // One backtick only is not a backticked word.
    for text in ["The `old way.\n", "The old` way.\n"] {
        write(d.path(), "mock/A.md.tmpl", text);
        let m = messages(d.path());
        assert!(m.is_empty(), "{text:?}: {m:?}");
    }
}

fn reverse(field: &str) -> NoSupersededCitation {
    let mut lint = NoSupersededCitation::default();
    lint.configure(&HashMap::from([(SUPERSEDES_KEY.to_string(), field.to_string())]));
    lint
}

/// The same canon as `canon()`, written the other way round: the new row names what it replaces.
fn canon_reversed() -> RegistryView {
    registry(&[
        ("tenet::old", &[]),
        ("tenet::mid", &[("supersedes", "old")]),
        ("tenet::new", &[("supersedes", "mid")]),
        ("tenet::split", &[]),
        ("tenet::left", &[("supersedes", "split")]),
        ("tenet::right", &[("supersedes", "tenet::split")]),
        ("tenet::old-rule", &[]),
        ("tenet::new-rule", &[("supersedes", "old-rule")]),
    ])
}

#[test]
fn a_supersedes_edge_alone_flags_a_citation_of_the_replaced_row() {
    let d = dir("reverse");
    write(d.path(), "mock/A.md.tmpl", "After `old`.\n");
    let found = run_with(&reverse("supersedes"), &canon_reversed(), d.path(), &[]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.contains("`old`") && found[0].message.contains("`new`"), "{found:?}");
    assert!(!found[0].message.contains("`mid`"), "{found:?}");
    write(d.path(), "mock/A.md.tmpl", "After `new`, `mid`.\n");
    let found = run_with(&reverse("supersedes"), &canon_reversed(), d.path(), &[]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.contains("`mid`"), "{found:?}");
    write(d.path(), "mock/A.md.tmpl", "After `split`.\n");
    let found = run_with(&reverse("supersedes"), &canon_reversed(), d.path(), &[]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.contains("`left`") && found[0].message.contains("`right`"), "{found:?}");
}

#[test]
fn the_two_directions_agree_and_a_forward_field_alone_ignores_the_reverse_edge() {
    let d = dir("agree");
    write(d.path(), "mock/A.md.tmpl", "After `old-rule`.\n");
    let reg = canon_reversed();
    // The reverse edge is not read by a lint that named only the forward field.
    assert!(run_with(&configured("superseded_by"), &reg, d.path(), &[]).is_empty());
    // And the forward edge is not read by a lint that named only the reverse field.
    assert!(run_with(&reverse("supersedes"), &canon(), d.path(), &[]).is_empty());
    let a = run_with(&reverse("supersedes"), &reg, d.path(), &[]);
    let b = run(d.path(), &[]);
    assert_eq!(a.len(), 1, "{a:?}");
    assert_eq!(a[0].message, b[0].message);
}

#[test]
fn both_fields_named_read_both_edges_without_doubling_a_finding() {
    let d = dir("both");
    write(d.path(), "mock/A.md.tmpl", "After `old-rule` and `gone-rule`.\n");
    let reg = registry(&[
        ("tenet::old-rule", &[("superseded_by", "new-rule")]),
        ("tenet::new-rule", &[("supersedes", "old-rule")]),
        ("tenet::gone-rule", &[]),
        ("tenet::next-rule", &[("supersedes", "gone-rule")]),
    ]);
    let mut lint = configured("superseded_by");
    lint.configure(&HashMap::from([(SUPERSEDES_KEY.to_string(), "supersedes".to_string())]));
    let found = run_with(&lint, &reg, d.path(), &[]);
    assert_eq!(found.len(), 2, "{found:?}");
    // Sorted by slug: `gone-rule` is replaced by `next-rule` alone, `old-rule` by `new-rule` alone
    // although both of its edges are named.
    assert!(found[0].message.contains("`next-rule`") && !found[0].message.contains("`new-rule`"), "{found:?}");
    assert!(found[1].message.contains("`new-rule`") && !found[1].message.contains("`next-rule`"), "{found:?}");
}

#[test]
fn a_reverse_edge_does_not_hide_the_replacing_row_or_a_namesake_in_another_namespace() {
    let d = dir("reverse-namespaces");
    write(d.path(), "mock/A.md.tmpl", "After `dup-row` and `kept-row`.\n");
    let reg = registry(&[
        ("tenet::dup-row", &[]),
        ("term::dup-row", &[]),
        ("tenet::next-row", &[("supersedes", "dup-row")]),
        ("tenet::kept-row", &[("supersedes", "dup-row")]),
    ]);
    let found = run_with(&reverse("supersedes"), &reg, d.path(), &[]);
    // `dup-row` in `tenet` is replaced and in `term` it is live, so a text cannot tell them apart.
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn neither_field_named_is_still_the_one_finding_and_both_keys_are_declared() {
    let d = dir("neither");
    write(d.path(), "mock/A.md.tmpl", "After `old`.\n");
    let found = run_with(&NoSupersededCitation::default(), &canon_reversed(), d.path(), &[]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.contains("`field`") && found[0].message.contains("`supersedes_field`"), "{found:?}");
    assert_eq!(SUPERSEDES_KEY, "supersedes_field");
    assert_eq!(NoSupersededCitation::default().config_keys(), &["field", "supersedes_field"]);
    assert!(run_with(&reverse("  "), &canon_reversed(), d.path(), &[]).len() == 1);
}

#[test]
fn a_cycle_of_replacements_is_reported_as_one_not_as_an_empty_successor_list() {
    let d = dir("cycle");
    let reg = registry(&[
        ("tenet::xx-a", &[("superseded_by", "xx-b")]),
        ("tenet::xx-b", &[("superseded_by", "xx-a")]),
    ]);
    write(d.path(), "mock/DESIGN.md.tmpl", "After `xx-a`.\n");
    let found = run_with(&configured("superseded_by"), &reg, d.path(), &[]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.contains("lead back"), "{found:?}");
    assert!(!found[0].message.contains("which  supersede"), "{found:?}");
}

#[test]
fn a_slug_a_live_row_in_another_namespace_also_carries_is_not_read_as_replaced() {
    let d = dir("namespaces");
    let reg = registry(&[
        ("tenet::dup-row", &[("superseded_by", "other")]),
        ("tenet::other", &[]),
        ("term::dup-row", &[("what", "x")]),
    ]);
    write(d.path(), "mock/DESIGN.md.tmpl", "After `dup-row`.\n");
    assert!(run_with(&configured("superseded_by"), &reg, d.path(), &[]).is_empty());
}

#[test]
fn a_qualified_successor_is_named_by_its_slug() {
    let d = dir("qualified");
    write(d.path(), "mock/DESIGN.md.tmpl", "After `old-rule`.\n");
    let m = messages(d.path());
    assert!(m[0].contains("`new-rule`") && !m[0].contains("tenet::"), "{m:?}");
}

#[test]
fn the_excuse_stops_at_the_sentence_the_list_item_and_the_line_of_a_source_file() {
    let d = dir("excuse");
    let cases = [
        ("It is superseded. After `old`.\n", 1),
        ("It is v1.2 superseded by `new`, `old` too.\n", 0),
        ("- it is superseded\n- after `old`\n", 1),
        ("# superseded\nAfter `old`\n", 1),
        ("superseded\n\nAfter `old`\n", 1),
        ("1. it is superseded\n2. after `old`\n", 1),
        ("# After `old`\nit is superseded\n", 1),
        ("Superseded since v1.2 for `old`.\n", 0),
    ];
    for (text, want) in cases {
        write(d.path(), "mock/A.md.tmpl", text);
        let m = messages(d.path());
        assert_eq!(m.len(), want, "{text:?}: {m:?}");
    }
    std::fs::remove_file(d.path().join("mock/A.md.tmpl")).unwrap();
    write(d.path(), "mock/crates/alpha/src/lib.rs", "// superseded\nfn f() { g(\"`old`\"); }\n");
    assert_eq!(messages(d.path()).len(), 1);
}

#[test]
fn hidden_directories_and_files_of_other_extensions_are_not_read() {
    let d = dir("walk");
    write(d.path(), ".hidden/x.md", "After `old`.\n");
    write(d.path(), "mock/notes.txt", "After `old`.\n");
    write(d.path(), "mock/x.slang", "// After `old`.\n");
    let found = run(d.path(), &[]);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].path.as_deref(), Some("mock/x.slang"));
    assert_eq!(found[0].crate_name, "workspace");
}
