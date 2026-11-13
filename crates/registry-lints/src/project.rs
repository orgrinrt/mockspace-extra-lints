//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! The consuming repository's name and crate prefix, as the root of its `mockspace.toml` says them.
//!
//! The engine hands a lint and a tool the mock directory and the repository root, and a tool its own
//! `[tools.<name>]` table, but neither the root keys of the config. So `project_name` and
//! `crate_prefix` are read here, once, defaulted as the engine defaults them. Where the mock tree and
//! the content sit is `layout.rs`. This file declares no lint.

use std::path::Path;

use mockspace::View;

use crate::gate_config::quoted;

/// What a repository is called.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    /// `project_name`, `project` where the config says none, as the engine has it.
    pub name:   String,
    /// `crate_prefix`, the name where the config says none, as the engine has it.
    pub prefix: String,
}

impl Project {
    /// The particulars of the repository whose mock directory this is, from the config the engine
    /// reads there, found the way the registry finds it and read through `view`.
    #[must_use]
    pub fn of(view: &View<'_>, mock_dir: &Path) -> Self {
        let located = mockspace_registry::locate(mock_dir);
        let text = view.text(&located.config).unwrap_or_default();
        Self::from_config(text)
    }

    /// The particulars a `mockspace.toml` text names.
    #[must_use]
    pub fn from_config(config: &str) -> Self {
        let name = root_key(config, "project_name").unwrap_or_else(|| "project".to_owned());
        let prefix = root_key(config, "crate_prefix").unwrap_or_else(|| name.clone());
        Self {
            name,
            prefix,
        }
    }

    /// The crate `name` names without the prefix and its dash, where it carries them.
    #[must_use]
    pub fn unprefixed<'a>(&self, name: &'a str) -> Option<&'a str> {
        name.strip_prefix(self.prefix.as_str())?.strip_prefix('-')
    }
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
#[path = "project_tests.rs"]
mod tests;
