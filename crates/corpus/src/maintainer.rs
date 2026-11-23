//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Where the maintainer's captured words are, and whose quoted lines count as theirs, as a project
//! names them.
//!
//! Every tool reading the corpus declares the same two keys and keeps a [`Maintainer`]. Nothing here
//! searches for a directory: where the project names none, the words are not read, and every report
//! over the corpus says so and says how to name them. The directory is the repository's own: a path
//! under its root. One that is absolute or climbs out of the repository names nothing the repository
//! holds, and is reported as such rather than read.

use std::path::{Component, Path, PathBuf};

use mockspace::tool::{ConfigKey, ToolParams};

/// The key naming the directory of the maintainer's captured words.
pub const WORDS: &str = "maintainer_words";

/// The key naming whose quoted lines in a note are read as the maintainer's words.
pub const NAMES: &str = "maintainer_names";

/// [`WORDS`] as a tool declares it.
pub const WORDS_KEY: ConfigKey = ConfigKey {
    name:        WORDS,
    description: "the directory of the maintainer's captured words, a path under the repository's root; not read where unset, and refused where it is absolute or leaves the repository",
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

    /// The directory of the captured words, where [`WORDS`] names one inside the repository.
    #[must_use]
    pub fn words_dir(&self, repo_root: &Path) -> Option<PathBuf> {
        maintainer_words_dir(repo_root, self.words.as_deref())
    }

    /// What [`WORDS`] was set to, as written, for a report that says why nothing was read.
    #[must_use]
    pub fn named(&self) -> Option<&str> {
        self.words
            .as_deref()
            .map(str::trim)
            .filter(|w| !w.is_empty())
    }

    /// Whose quoted lines are read as the maintainer's words.
    #[must_use]
    pub fn names(&self) -> &[String] {
        &self.names
    }
}

/// The directory of the maintainer's captured words, from the key's value: a relative path read from
/// the repository's root. A blank value, an absolute path and one with a `..` component name
/// nothing, since the repository holds nothing outside itself. Whether the directory exists is left to
/// the reader, so a name pointing nowhere is reported as such rather than mistaken for no name at all.
#[must_use]
pub fn maintainer_words_dir(repo_root: &Path, configured: Option<&str>) -> Option<PathBuf> {
    let named = Path::new(configured.map(str::trim).filter(|n| !n.is_empty())?);
    let inside = named
        .components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir));
    inside.then(|| repo_root.join(named))
}

/// Why the maintainer's words were not read: what was named for them, if anything, and the
/// directory it was read as, where it named one inside the repository.
#[must_use]
pub fn unread(named: Option<&str>, dir: Option<&Path>) -> String {
    match (named, dir) {
        (Some(_), Some(dir)) => {
            format!(
                "the maintainer's captured words were not read: `{}`, named for them, is not a \
                 directory",
                dir.display()
            )
        },
        (Some(named), None) => {
            format!(
                "the maintainer's captured words were not read: `{named}`, named for them, is not \
                 a path inside the repository"
            )
        },
        (None, _) => {
            format!(
                "the maintainer's captured words were not read: name their directory under the \
                 repository with `{WORDS}` in the tool's table"
            )
        },
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use mockspace::tool::ToolParams;

    use super::*;

    #[test]
    fn the_key_names_the_directory_from_the_repository_root() {
        let root = Path::new("/repo");
        assert_eq!(
            maintainer_words_dir(root, Some("notes/words")),
            Some(PathBuf::from("/repo/notes/words"))
        );
        assert_eq!(
            maintainer_words_dir(root, Some("./words")),
            Some(PathBuf::from("/repo/./words"))
        );
    }

    #[test]
    fn a_path_that_leaves_the_repository_names_nothing() {
        let root = Path::new("/repo");
        assert_eq!(
            maintainer_words_dir(root, Some("../.data/op-responses")),
            None
        );
        assert_eq!(maintainer_words_dir(root, Some("words/../../out")), None);
        assert_eq!(maintainer_words_dir(root, Some("/abs/words")), None);
        // THE CONTROL: the same names kept inside are read.
        assert!(maintainer_words_dir(root, Some("data/op-responses")).is_some());
    }

    #[test]
    fn with_nothing_set_the_words_are_not_read_and_the_note_says_how_to_name_them() {
        assert_eq!(maintainer_words_dir(Path::new("/repo"), None), None);
        assert_eq!(maintainer_words_dir(Path::new("/repo"), Some("  ")), None);
        let note = unread(None, None);
        assert!(note.contains(&format!("`{WORDS}`")), "{note}");
    }

    #[test]
    fn each_reason_for_not_reading_is_its_own_note() {
        let missing = unread(Some("words"), Some(Path::new("/repo/words")));
        assert!(
            missing.contains("`/repo/words`, named for them, is not a directory"),
            "{missing}"
        );
        let outside = unread(Some("../words"), None);
        assert!(
            outside.contains("is not a path inside the repository"),
            "{outside}"
        );
        // THE CONTROL: the note for no name at all names no directory.
        assert!(!unread(None, None).contains("is not a directory"));
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
