//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! A lint's table in `mockspace.toml`, read as text: the three gate severities and the `crates` and
//! `exempt_crates` arrays. The engine reads the same keys for the crate and workspace lints, and not for a
//! repo lint, so a repo lint of this pack and the tests over the gates both come here for them.

use mockspace::PathFilter;

/// What a `[lints.<name>]` table says.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Table {
    pub commit: Option<String>,
    pub build: Option<String>,
    pub push: Option<String>,
    pub crates: Vec<String>,
    pub exempt_crates: Vec<String>,
}

impl Table {
    /// The crates the lint binds. Neither key means every crate.
    pub fn filter(&self) -> PathFilter {
        PathFilter {
            include: self.crates.clone(),
            exclude: self.exempt_crates.clone(),
        }
    }

    /// Whether the lint is `error` at the commit, the build and the push.
    pub fn blocks_at_every_gate(&self) -> bool {
        [&self.commit, &self.build, &self.push]
            .iter()
            .all(|g| g.as_deref() == Some("error"))
    }
}

/// The strings between double quotes in `text`, in order.
pub fn quoted(text: &str) -> Vec<String> {
    text.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// The table of `lint` in a `mockspace.toml` text; an empty one where there is none.
pub fn table(config: &str, lint: &str) -> Table {
    let header = format!("[lints.{lint}]");
    let mut in_table = false;
    let mut key = "";
    let mut out = Table::default();
    let mut crates = String::new();
    let mut exempt = String::new();
    for line in config.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.starts_with('[') && !line.starts_with("[\"") {
            in_table = line == header;
            key = "";
            continue;
        }
        if !in_table {
            continue;
        }
        if let Some((k, rest)) = line.split_once('=') {
            key = k.trim();
            let value = quoted(rest).into_iter().next();
            match key {
                "commit" => out.commit = value,
                "build" => out.build = value,
                "push" => out.push = value,
                "crates" => crates.push_str(rest),
                "exempt_crates" => exempt.push_str(rest),
                _ => {}
            }
        } else if key == "crates" {
            crates.push_str(line);
        } else if key == "exempt_crates" {
            exempt.push_str(line);
        }
    }
    out.crates = quoted(&crates);
    out.exempt_crates = quoted(&exempt);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = "[lints.no-todo]\ncrates = [\"kaski-x\"]\n\n[lints.one-concept-one-owner]\ncommit = \"error\"\nbuild = \"warn\"\ncrates = [\n    \"kaski-clock\",\n    \"kaski-work\", # a comment\n]\nexempt_crates = [\"kaski-clock\"]\n\n[lints.other]\ncrates = [\"kaski-ui\"]\n";

    #[test]
    fn a_table_reads_its_own_keys_and_nothing_beside_it() {
        let t = table(CONFIG, "one-concept-one-owner");
        assert_eq!(t.commit.as_deref(), Some("error"));
        assert_eq!(t.build.as_deref(), Some("warn"));
        assert_eq!(t.push, None);
        assert_eq!(t.crates, ["kaski-clock", "kaski-work"]);
        assert_eq!(t.exempt_crates, ["kaski-clock"]);
        assert!(!t.blocks_at_every_gate());
    }

    #[test]
    fn the_filter_admits_the_listed_and_refuses_the_exempt_and_the_rest() {
        let f = table(CONFIG, "one-concept-one-owner").filter();
        assert!(f.allows_str("kaski-work"));
        assert!(!f.allows_str("kaski-clock"), "exempt wins over listed");
        assert!(
            !f.allows_str("kaski-ui"),
            "another lint's list is not this one's"
        );
        assert!(
            !f.allows_str("kaski-x"),
            "nor is a crate only another lint lists"
        );
    }

    #[test]
    fn a_table_with_neither_key_binds_every_crate_and_a_missing_one_is_empty() {
        let t = table("[lints.a]\ncommit = \"error\"\n", "a");
        assert!(t.filter().allows_str("kaski-anything"));
        assert_eq!(table("", "a"), Table::default());
        assert!(!table("", "a").blocks_at_every_gate());
    }

    #[test]
    fn all_three_gates_at_error_is_what_blocks() {
        let t = table(
            "[lints.a]\ncommit = \"error\"\nbuild = \"error\"\npush = \"error\"\n",
            "a",
        );
        assert!(t.blocks_at_every_gate());
    }
}
