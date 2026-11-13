//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! What the repository ships, read as it ships: the vocabularies and the tickets in `mock/registry/`
//! hold to the rules and to the schema `mockspace.toml` declares, and the lint gates the same rows.

use std::collections::BTreeSet;

use kaski_tickets::model::Store;
use kaski_tickets::rules;
use mockspace::{RegistryView, RepoContext, RepoLint};

use crate::lint::ATicketGraphHolds;
use crate::testkit::{Scratch, committed_registry, repo_root};

fn shipped() -> Store {
    crate::load(&committed_registry()).expect("the committed registry reads")
}

/// The field names `mockspace.toml` declares for a namespace.
fn declared_fields(namespace: &str) -> BTreeSet<String> {
    let src = std::fs::read_to_string(repo_root().join("mockspace.toml")).expect("mockspace.toml");
    let doc: toml::Table = src.parse().expect("mockspace.toml parses");
    let spaces = doc["registry"]["namespace"].as_array().expect("namespaces");
    let space = spaces
        .iter()
        .filter_map(toml::Value::as_table)
        .find(|t| t.get("key").and_then(toml::Value::as_str) == Some(namespace))
        .unwrap_or_else(|| panic!("`{namespace}` is declared"));
    space["field"]
        .as_array()
        .expect("fields")
        .iter()
        .filter_map(|f| f.get("name").and_then(toml::Value::as_str))
        .map(String::from)
        .collect()
}

/// The skill's "Link by reference" table claims a flag writes a field: each row names flags that
/// `ticket-add` or `ticket-move` take (or the planning layer's `--span` and `--core`) and a field the
/// schema declares for a ticket (`--pr` and `pr` were claimed for weeks with neither, 8 November).
#[test]
fn every_flag_the_use_tickets_skill_links_by_is_a_flag_of_the_tool_and_a_field_of_the_schema() {
    let skill =
        std::fs::read_to_string(repo_root().join("mock/agent/skills/use-tickets/SKILL.md.tmpl"))
            .expect("the skill template");
    let table = skill
        .split("## Link by reference")
        .nth(1)
        .and_then(|s| s.split("\n## ").next())
        .expect("the skill has a link table");
    let fields = declared_fields("ticket");
    let mut rows = 0;
    for line in table.lines().filter(|l| l.starts_with("| `--")) {
        rows += 1;
        let mut cells = line.split('|').skip(1);
        let (flags, field) = (cells.next().unwrap_or(""), cells.next().unwrap_or(""));
        if field.contains("member row") {
            continue;
        }
        let field = field.trim().trim_matches('`');
        assert!(
            fields.contains(field),
            "the skill names the field `{field}`, which the ticket schema lacks"
        );
        for flag in flags.split(',').map(|f| f.trim().trim_matches('`')) {
            assert!(
                crate::args::ADD_FLAGS.contains(&flag) || crate::args::MOVE_FLAGS.contains(&flag),
                "the skill names `{flag}`, which neither ticket-add nor ticket-move takes"
            );
        }
    }
    assert!(rows >= 10, "the table was read: {rows} rows");
}

#[test]
fn the_committed_tickets_pass_the_rules() {
    let found = rules::check_store(&shipped());
    assert!(found.is_empty(), "{found:?}");
}

#[test]
fn every_kind_lists_states_that_exist_and_starts_in_one_of_them() {
    let store = shipped();
    assert!(!store.kinds.is_empty());
    for k in store.kinds.values() {
        for s in &k.states {
            assert!(
                store.states.contains_key(s),
                "kind `{}` lists `{s}`, which is no state",
                k.id
            );
        }
        assert!(
            k.states.contains(&k.initial),
            "kind `{}` starts in `{}`, which it does not list",
            k.id,
            k.initial
        );
    }
}

#[test]
fn every_required_field_of_a_kind_is_a_field_of_a_ticket() {
    let declared = declared_fields("ticket");
    for k in shipped().kinds.values() {
        for r in &k.requires {
            assert!(
                declared.contains(r.split_once(':').map_or(r.as_str(), |(f, _)| f)),
                "kind `{}` requires `{r}`, which a ticket does not declare",
                k.id
            );
        }
    }
}

#[test]
fn the_board_columns_run_from_the_backlog_to_the_closed_states() {
    let store = shipped();
    let mut states: Vec<_> = store.states.values().collect();
    states.sort_by_key(|s| s.column);
    let ids: Vec<&str> = states.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids, [
        "backlog", "ready", "doing", "waiting", "held", "review", "done", "dropped"
    ]);
    let columns: BTreeSet<i64> = states.iter().map(|s| s.column).collect();
    assert_eq!(columns.len(), states.len(), "two states share a column");
}

#[test]
fn the_lint_gates_the_rows_the_makers_refuse_by() {
    let crates = BTreeSet::new();
    let registry = RegistryView::default();
    let check = |s: &Scratch| {
        let ctx = RepoContext {
            mock_dir:    &s.mock,
            repo_root:   &s.mock,
            all_crates:  &crates,
            src_dirs:    &[],
            invocation:  None,
            canon_paths: &[],
            open_panels: &[],
            registry:    &registry,
            verdicts:    None,
        };
        ATicketGraphHolds.check_repo(&ctx)
    };
    let s = Scratch::new();
    let a = s.add("Chicken", &[]).unwrap();
    s.add("Egg", &[("needs", &[&a])]).unwrap();
    assert!(check(&s).is_empty());
    let text = s
        .read("ticket/items/chicken.toml")
        .replace("kind = \"task\"\n", "kind = \"task\"\nneeds = [\"egg\"]\n");
    std::fs::write(s.root.join("ticket/items/chicken.toml"), text).unwrap();
    let found = check(&s);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].message.contains("goes round"),
        "{}",
        found[0].message
    );
}

#[test]
fn the_fields_a_question_and_a_comment_need_are_declared() {
    assert!(declared_fields("ticket").contains("options"));
    assert!(
        declared_fields("ticket").contains("preview"),
        "a question names the page it depends on in `preview`"
    );
    let note = declared_fields("note");
    for f in ["quote", "where"] {
        assert!(note.contains(f), "`note` does not declare `{f}`");
    }
}

#[test]
fn every_field_the_tool_writes_to_a_ticket_row_is_declared() {
    let declared = declared_fields("ticket");
    for f in crate::write::written_fields() {
        assert!(
            declared.contains(f),
            "the tool writes `{f}` to a ticket row, and `mockspace.toml` does not declare it"
        );
    }
    assert!(
        crate::write::written_fields().contains(&"tags"),
        "the tags a ticket is marked with are among the fields written"
    );
}
