//! Running a repo lint over a directory a test built on disk, the way the runner would hand it over.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use mockspace_lint_rules::src_layout::NO_REF_FILES;
use mockspace_lint_rules::testkit::check_repo;
use mockspace_lint_rules::{LintError, RegistryView, RepoInput, RepoLint};

/// What `lint` finds over the repository at `root`, whose mock directory is `root/mock` and whose
/// crates are `crates`, with no registry, no canon paths and no open panels.
pub(crate) fn run_repo_lint(lint: &dyn RepoLint, root: &Path, crates: &BTreeSet<String>) -> Vec<LintError> {
    run_repo_lint_in(lint, root, crates, &RegistryView::new(BTreeMap::new(), BTreeMap::new()), &[])
}

/// [`run_repo_lint`] with the registry and the canon paths the repository declares.
pub(crate) fn run_repo_lint_in(
    lint: &dyn RepoLint,
    root: &Path,
    crates: &BTreeSet<String>,
    registry: &RegistryView,
    canon_paths: &[String],
) -> Vec<LintError> {
    check_repo(
        lint,
        RepoInput {
            mock_dir: &root.join("mock"),
            repo_root: root,
            all_crates: crates,
            src_dirs: &[],
            invocation: None,
            canon_paths,
            open_panels: &[],
            ref_files: &NO_REF_FILES,
            registry,
        },
    )
}
