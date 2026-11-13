//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The committed registry, read the way the engine reads it, for a test that asks the real canon.
//!
//! Behind the `testkit` feature so a tool that tests against the canon (`canon-links`) reads it from
//! its one owner rather than carrying a second reader of the same files.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use mockspace::RegistryView;

/// The repository root, from this crate's manifest.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

/// The committed registry: every `[[<namespace>]]` row of every `*.toml` under `mock/registry/`, list
/// fields joined with `", "`.
pub fn committed() -> RegistryView {
    let files = crate::tree_walk::with_extension(&repo_root().join("mock/registry"), "toml", &[]);
    let mut rows: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for f in files {
        let doc: toml::Table = std::fs::read_to_string(&f)
            .expect("readable")
            .parse()
            .expect("valid TOML");
        for (ns, tables) in doc {
            let Some(tables) = tables.as_array() else {
                continue;
            };
            for t in tables {
                let t = t.as_table().expect("a row");
                let id = t["id"].as_str().expect("an id");
                let fields = t
                    .iter()
                    .map(|(k, v)| {
                        let s = match v {
                            toml::Value::String(s) => s.clone(),
                            toml::Value::Array(a) => {
                                a.iter()
                                    .map(|e| e.as_str().unwrap_or_default().to_string())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            },
                            other => other.to_string(),
                        };
                        (k.clone(), s)
                    })
                    .collect();
                let prev = rows.insert(format!("{ns}::{id}"), fields);
                assert!(prev.is_none(), "`{ns}::{id}` is declared twice");
            }
        }
    }
    RegistryView::new(rows, BTreeMap::new())
}
