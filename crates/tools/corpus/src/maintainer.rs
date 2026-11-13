//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Where the maintainer's captured words are, and whose quoted lines count as theirs, as a project
//! names them.
//!
//! Every tool reading the corpus declares the same two keys and keeps a [`Maintainer`]. Nothing here
//! searches for a directory: where the project names none, the words are not read, and every report
//! over the corpus says so and says how to name them.

use std::path::{Path, PathBuf};

use mockspace::tool::{ConfigKey, ToolParams};

/// The key naming the directory of the maintainer's captured words.
pub const WORDS: &str = "maintainer_words";

/// The key naming whose quoted lines in a note are read as the maintainer's words.
pub const NAMES: &str = "maintainer_names";

/// The environment variable naming the directory of the maintainer's captured words, read where
/// [`WORDS`] is unset.
pub const WORDS_VAR: &str = "MOCKSPACE_MAINTAINER_WORDS";

/// [`WORDS`] as a tool declares it.
pub const WORDS_KEY: ConfigKey = ConfigKey {
    name:        WORDS,
    description: "the directory of the maintainer's captured words, relative to the repository root or absolute; MOCKSPACE_MAINTAINER_WORDS where unset, and not read where neither names one",
};

/// [`NAMES`] as a tool declares it.
pub const NAMES_KEY: ConfigKey = ConfigKey {
    name:        NAMES,
    description: "the names whose quoted lines in a note are read as the maintainer's words, matched ignoring case; none when unset",
};

/// Both keys, for a tool that reads nothing else from its table.
pub const KEYS: &[ConfigKey] = &[WORDS_KEY, NAMES_KEY];

/// What a project set for the maintainer's words in one tool's table.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Maintainer {
    words: Option<String>,
    names: Vec<String>,
}

impl Maintainer {
    /// Reads [`WORDS`] and [`NAMES`] from a tool's table, leaving what is unset as it was.
    pub fn configure(&mut self, params: &ToolParams) {
        if let Some(w) = params.one(WORDS) {
            self.words = Some(w.to_string());
        }
        if let Some(n) = params.list(NAMES) {
            self.names = n.to_vec();
        }
    }

    /// The directory of the captured words: [`WORDS`] where set, else [`WORDS_VAR`].
    #[must_use]
    pub fn words_dir(&self, repo_root: &Path) -> Option<PathBuf> {
        let var = std::env::var(WORDS_VAR).ok();
        maintainer_words_dir(repo_root, self.words.as_deref(), var.as_deref())
    }

    /// Whose quoted lines are read as the maintainer's words.
    #[must_use]
    pub fn names(&self) -> &[String] {
        &self.names
    }
}

/// The directory of the maintainer's captured words, from the key's value and the variable's.
///
/// The key wins over the variable, and a blank value names nothing. A relative path is read from
/// `repo_root`. Whether the directory exists is left to the reader, so a name pointing nowhere is
/// reported as such rather than mistaken for no name at all.
#[must_use]
pub fn maintainer_words_dir(
    repo_root: &Path,
    configured: Option<&str>,
    var: Option<&str>,
) -> Option<PathBuf> {
    let named = [configured, var]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|n| !n.is_empty())?;
    Some(repo_root.join(named))
}

/// Why the maintainer's words were not read, given the directory that was named for them, if any.
#[must_use]
pub fn unread(named: Option<&Path>) -> String {
    match named {
        Some(dir) => {
            format!(
                "the maintainer's captured words were not read: `{}`, named for them, is not a \
                 directory",
                dir.display()
            )
        },
        None => {
            format!(
                "the maintainer's captured words were not read: name their directory with \
                 `{WORDS}` in the tool's table or with `{WORDS_VAR}`"
            )
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use mockspace::tool::ToolParams;

    use super::*;

    fn params(words: Option<&str>) -> ToolParams {
        let mut p = ToolParams::new();
        if let Some(w) = words {
            p.set(WORDS, vec![w.to_string()]);
        }
        p
    }

    #[test]
    fn the_key_names_the_directory_from_the_repository_root_and_wins_over_the_variable() {
        let root = Path::new("/repo");
        assert_eq!(
            maintainer_words_dir(root, Some("notes/words"), Some("/from/var")),
            Some(PathBuf::from("/repo/notes/words"))
        );
        assert_eq!(
            maintainer_words_dir(root, Some("/abs/words"), None),
            Some(PathBuf::from("/abs/words"))
        );
        // THE CONTROL: the configured value reaches the lookup through `configure`.
        let mut m = Maintainer::default();
        m.configure(&params(Some("/abs/words")));
        assert_eq!(m.words.as_deref(), Some("/abs/words"));
        m.configure(&params(None));
        assert_eq!(m.words.as_deref(), Some("/abs/words"), "an unset key leaves the value");
    }

    #[test]
    fn the_variable_names_the_directory_where_the_key_is_unset_or_blank() {
        let root = Path::new("/repo");
        assert_eq!(
            maintainer_words_dir(root, None, Some("/from/var")),
            Some(PathBuf::from("/from/var"))
        );
        assert_eq!(
            maintainer_words_dir(root, Some("  "), Some("words")),
            Some(PathBuf::from("/repo/words"))
        );
    }

    #[test]
    fn with_neither_set_the_words_are_not_read_and_the_note_says_how_to_name_them() {
        let root = Path::new("/repo");
        assert_eq!(maintainer_words_dir(root, None, None), None);
        assert_eq!(maintainer_words_dir(root, Some(""), Some(" ")), None);
        let note = unread(None);
        assert!(note.contains(&format!("`{WORDS}`")), "{note}");
        assert!(note.contains(&format!("`{WORDS_VAR}`")), "{note}");
        // THE CONTROL: either one set names a directory.
        assert!(maintainer_words_dir(root, Some("w"), None).is_some());
        assert!(maintainer_words_dir(root, None, Some("w")).is_some());
    }

    #[test]
    fn a_named_directory_that_does_not_exist_is_still_named_and_the_note_says_it_is_missing() {
        let root = Path::new("/no/such/repository");
        let named = maintainer_words_dir(root, Some("words"), None);
        assert_eq!(named, Some(PathBuf::from("/no/such/repository/words")));
        let note = unread(named.as_deref());
        assert!(
            note.contains("`/no/such/repository/words`, named for them, is not a directory"),
            "{note}"
        );
        // THE CONTROL: the note for no name at all names no directory.
        assert!(!unread(None).contains("is not a directory"));
    }

    #[test]
    fn the_names_are_read_as_a_list_and_none_when_unset() {
        let mut m = Maintainer::default();
        assert!(m.names().is_empty());
        let mut p = ToolParams::new();
        p.set(NAMES, vec!["Lead".to_string(), "lead designer".to_string()]);
        m.configure(&p);
        assert_eq!(m.names(), ["Lead", "lead designer"]);
    }
}
