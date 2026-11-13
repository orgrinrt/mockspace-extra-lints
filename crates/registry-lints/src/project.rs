//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The consuming repository's own particulars, as the engine's keys and its context say them.
//!
//! A pack's lint or tool is handed the mock directory and the repository root and nothing of the
//! configuration, and a pack's tool takes no parameters of its own yet (mockspace ticket
//! `a_lint_pack_s_tool_takes_no_parameters_from_mockspace_toml`). So what differs between two
//! repositories is read here, once, from what they already declare: `project_name` and
//! `crate_prefix` at the root of `mockspace.toml`, defaulted as the engine defaults them, and the mock
//! directory's place under the root. This file declares no lint.

use std::path::Path;

use crate::gate_config::quoted;

/// What a repository is called and where its mock tree sits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// `project_name`, `project` where the config says none, as the engine has it.
    pub name:   String,
    /// `crate_prefix`, the name where the config says none, as the engine has it.
    pub prefix: String,
    /// The mock directory relative to the repository root, `/` separated, empty where it is the root.
    pub mock:   String,
}

impl Project {
    /// The particulars of the repository whose mock directory this is, from the config the engine
    /// reads there, found the way the registry finds it.
    #[must_use]
    pub fn of(mock_dir: &Path) -> Self {
        let located = mockspace_registry::locate(mock_dir);
        let text = std::fs::read_to_string(&located.config).unwrap_or_default();
        Self::from_config(&text, &mock_rel(mock_dir, &located.repo_root))
    }

    /// The particulars a `mockspace.toml` text names, the mock directory given.
    #[must_use]
    pub fn from_config(config: &str, mock: &str) -> Self {
        let name = root_key(config, "project_name").unwrap_or_else(|| "project".to_owned());
        let prefix = root_key(config, "crate_prefix").unwrap_or_else(|| name.clone());
        Self {
            name,
            prefix,
            mock: mock.to_owned(),
        }
    }

    /// `rel` under the mock directory, `/` separated.
    #[must_use]
    pub fn in_mock(&self, rel: &str) -> String {
        if self.mock.is_empty() {
            rel.to_owned()
        } else {
            format!("{}/{rel}", self.mock)
        }
    }

    /// The crate `name` names without the prefix and its dash, where it carries them.
    #[must_use]
    pub fn unprefixed<'a>(&self, name: &'a str) -> Option<&'a str> {
        name.strip_prefix(self.prefix.as_str())?.strip_prefix('-')
    }
}

/// The mock directory relative to the root, empty where it is the root or not under it.
#[must_use]
pub fn mock_rel(mock_dir: &Path, repo_root: &Path) -> String {
    mock_dir
        .strip_prefix(repo_root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default()
}

/// A string at the root of a `mockspace.toml` text, before its first table.
fn root_key(config: &str, key: &str) -> Option<String> {
    for line in config.lines() {
        let line = line.split('#').next().unwrap_or_default().trim();
        if line.starts_with('[') {
            return None;
        }
        if let Some((k, rest)) = line.split_once('=')
            && k.trim() == key
        {
            return quoted(rest).into_iter().next();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_are_read_from_the_root_of_the_config() {
        let p = Project::from_config(
            "# a comment\nproject_name = \"arvo\"\ncrate_prefix = \"arvo\"\n\n[lints.x]\nproject_name = \"no\"\n",
            "mock",
        );
        assert_eq!((p.name.as_str(), p.prefix.as_str()), ("arvo", "arvo"));
    }

    #[test]
    fn the_prefix_defaults_to_the_name_and_the_name_to_project() {
        assert_eq!(
            Project::from_config("project_name = \"kaski\"\n", "mock").prefix,
            "kaski"
        );
        assert_eq!(Project::from_config("", "mock").name, "project");
    }

    #[test]
    fn a_key_inside_a_table_is_not_the_root_key() {
        assert_eq!(
            Project::from_config("[a]\ncrate_prefix = \"x\"\n", "mock").prefix,
            "project"
        );
    }

    #[test]
    fn a_crate_is_unprefixed_only_with_its_dash() {
        let p = Project::from_config("crate_prefix = \"kaski\"\n", "mock");
        assert_eq!(p.unprefixed("kaski-sim"), Some("sim"));
        assert_eq!(p.unprefixed("kaskisim"), None);
        assert_eq!(p.unprefixed("arvo-sim"), None);
    }

    #[test]
    fn the_mock_directory_is_read_from_its_place_under_the_root() {
        assert_eq!(mock_rel(Path::new("/r/linux"), Path::new("/r")), "linux");
        assert_eq!(mock_rel(Path::new("/r"), Path::new("/r")), "");
        assert_eq!(
            Project::from_config("", "linux").in_mock("registry"),
            "linux/registry"
        );
        assert_eq!(Project::from_config("", "").in_mock("registry"), "registry");
    }
}
