//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The reading and writing of `mock/registry/known_red/reds.toml`, the one file of the catalogue.
//!
//! The file is rendered whole from the rows, sorted by id, under a fixed header, because this
//! module is its only writer. A row is the crate, the test filter exactly as `cargo mock test -p
//! <crate> -- <filter>` takes it, the `dev` sha it was seen red at, who found it, when, and why.

use std::path::{Path, PathBuf};

/// The file of the catalogue under the registry directory.
pub const FILE: &str = "known_red/reds.toml";

const HEADER: &str = "# The tests red on `dev`, one row each. Written by `mock known-red-set`, asked by `mock known-red`.\n# A row is a claim about `dev` at its `sha`, cleared by whoever fixes the test.\n";

/// One test red on `dev`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Red {
    pub id:    String,
    pub krate: String,
    pub test:  String,
    pub sha:   String,
    pub by:    String,
    pub at:    String,
    pub why:   String,
}

/// Where the catalogue sits under the mock directory.
pub fn path(mock_dir: &Path) -> PathBuf {
    tickets::registry_dir(mock_dir).join(FILE)
}

/// The rows of the catalogue; an absent file is an empty catalogue.
///
/// # Errors
///
/// When the file cannot be read or a row lacks a field.
pub fn load(mock_dir: &Path) -> Result<Vec<Red>, String> {
    let p = path(mock_dir);
    let text = match std::fs::read_to_string(&p) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("cannot read {}: {e}", p.display())),
    };
    let doc: toml::Table = text.parse().map_err(|e| format!("{}: {e}", p.display()))?;
    let Some(rows) = doc.get("known_red").and_then(toml::Value::as_array) else {
        return Ok(Vec::new());
    };
    rows.iter()
        .map(|r| {
            let field = |k: &str| {
                r.get(k)
                    .and_then(toml::Value::as_str)
                    .map(str::to_string)
                    .ok_or_else(|| format!("{}: a known_red row lacks `{k}`", p.display()))
            };
            Ok(Red {
                id:    field("id")?,
                krate: field("crate")?,
                test:  field("test")?,
                sha:   field("sha")?,
                by:    field("by")?,
                at:    field("at")?,
                why:   field("why")?,
            })
        })
        .collect()
}

/// The id a test gets: a slug of the crate and the filter, with a short hash of both when the slug
/// was cut, so two long filters never share one.
pub fn id_of(krate: &str, test: &str) -> String {
    let slug = kaski_tickets::rules::slugify(&format!("{krate} {test}"));
    let full = format!("{krate}\u{0}{test}");
    if slug.len() >= 55 {
        let h = full.bytes().fold(0xCBF2_9CE4_8422_2325_u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01B3)
        });
        format!("{slug}_{:08x}", h as u32)
    } else {
        slug
    }
}

fn quote(s: &str) -> String {
    toml::Value::String(s.to_string()).to_string()
}

/// Writes the rows, sorted by id.
///
/// # Errors
///
/// When the file cannot be written.
pub fn save(mock_dir: &Path, mut rows: Vec<Red>) -> Result<(), String> {
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    let mut out = String::from(HEADER);
    for r in &rows {
        out.push_str(&format!(
            "\n[[known_red]]\nid = {}\ncrate = {}\ntest = {}\nsha = {}\nby = {}\nat = {}\nwhy = {}\n",
            quote(&r.id),
            quote(&r.krate),
            quote(&r.test),
            quote(&r.sha),
            quote(&r.by),
            quote(&r.at),
            quote(&r.why),
        ));
    }
    let p = path(mock_dir);
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot make {}: {e}", dir.display()))?;
    }
    std::fs::write(&p, out).map_err(|e| format!("cannot write {}: {e}", p.display()))
}
