//! Where a checkout's index is kept, and the refresh that runs beside a question.
//!
//! The index belongs to the checkout: a worktree holds other files under the same paths than its
//! clone does. It is kept in the checkout's own git directory, which git never commits and which a
//! linked worktree has apart from its clone, and a new one starts as a copy of the clone's.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::listing::git;

/// The index's file under a checkout's git directory.
pub const INDEX_FILE: &str = "mockspace/search.sqlite";

/// The index of the checkout at `repo_root`, in its own git directory.
///
/// # Errors
/// Git's words where the checkout has no git directory.
pub fn index_path(repo_root: &Path) -> Result<PathBuf, String> {
    let dir = git(repo_root, &["rev-parse", "--absolute-git-dir"])?;
    Ok(PathBuf::from(dir.trim()).join(INDEX_FILE))
}

/// The index of the clone a linked worktree belongs to, which a new worktree's index starts from;
/// none for the clone's own checkout.
#[must_use]
pub fn sibling_index(repo_root: &Path) -> Option<PathBuf> {
    let own = index_path(repo_root).ok()?;
    let common = git(repo_root, &[
        "rev-parse",
        "--path-format=absolute",
        "--git-common-dir",
    ])
    .ok()?;
    let theirs = PathBuf::from(common.trim()).join(INDEX_FILE);
    (theirs != own).then_some(theirs)
}

/// Start `cargo mock search --refresh` in `repo_root`, in a process group of its own so it outlives
/// the question that started it, and do not wait for it. A refresh already running holds the index,
/// and the one started here sees that and ends.
pub fn background(repo_root: &Path) {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let mut command = Command::new(cargo);
    command
        .args(["mock", "search", "--refresh"])
        .current_dir(repo_root)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    // A refresh that cannot be started leaves the index as it stands, which the question has
    // already said; there is nothing more to tell the reader.
    let _ = command.spawn();
}
