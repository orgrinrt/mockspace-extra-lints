//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `a-tenet-names-a-subject`: every tenet names a subject that is a row.
//!
//! A tenet is `tenet::<id>` and what it is about is its `subject` field, which is what the registry's
//! graph reads. The file a tenet is written in is no part of that: any number of files, under any
//! names, may hold tenets, and moving a tenet to another file changes nothing about it.

use std::path::Path;

use mockspace::{Lint, LintError, RepoContext, RepoLint, Severity};

use crate::canon_rows::{finding, written_tenets};

pub fn repo_lint() -> Box<dyn RepoLint> {
    Box::new(ATenetNamesASubject)
}

const NAME: &str = "a-tenet-names-a-subject";

struct ATenetNamesASubject;

impl Lint for ATenetNamesASubject {
    fn name(&self) -> &'static str {
        NAME
    }

    fn default_severity(&self) -> Severity {
        Severity::HARD_ERROR
    }
}

impl RepoLint for ATenetNamesASubject {
    fn check_repo(&self, ctx: &RepoContext) -> Vec<LintError> {
        check(ctx.mock_dir, ctx.registry())
    }
}

fn check(mock_dir: &Path, reg: &mockspace::RegistryView) -> Vec<LintError> {
    let registry = mock_dir.join("registry");
    let is_subject = |s: &str| reg.row(&format!("subject::{s}")).is_some();
    let mut out = Vec::new();
    for w in written_tenets(&registry) {
        // where to look for it, and nothing more
        let shown = w
            .file
            .strip_prefix(mock_dir)
            .unwrap_or(&w.file)
            .display()
            .to_string();
        let Some(subject) = w.subject.as_deref() else {
            out.push(finding(
                NAME,
                format!("`tenet::{}` (in `{shown}`) names no subject; give it `subject = \"<subject>\"`", w.id),
            ));
            continue;
        };
        if !is_subject(subject) {
            out.push(finding(
                NAME,
                format!("`tenet::{}` (in `{shown}`) names `{subject}`, which is not a subject; name a `subject` row, or add the subject", w.id),
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canon_rows::testkit::{ctx, scratch, view, write};

    fn reg() -> mockspace::RegistryView {
        view(&[
            ("subject::sky", &[("what", "The sky.")]),
            ("subject::time", &[("what", "The clock.")]),
        ])
    }

    fn findings(d: &Path) -> Vec<String> {
        let r = reg();
        ATenetNamesASubject
            .check_repo(&ctx(d, &r))
            .into_iter()
            .map(|e| e.message)
            .collect()
    }

    #[test]
    fn a_tenet_passes_in_any_file_under_any_name_at_any_depth() {
        let d = scratch("names-anywhere");
        write(
            &d,
            "registry/tenet/sky.toml",
            "[[tenet]]\nid = \"a\"\nsubject = \"time\"\n",
        );
        write(
            &d,
            "registry/tenet.toml",
            "[[tenet]]\nid = \"b\"\nsubject = \"sky\"\n",
        );
        write(
            &d,
            "registry/tenet/weather.toml",
            "[[tenet]]\nid = \"c\"\nsubject = \"sky\"\n",
        );
        write(
            &d,
            "registry/a/b/c/anything.toml",
            "[[tenet]]\nid = \"d\"\nsubject = \"time\"\n",
        );
        assert!(findings(&d).is_empty(), "{:?}", findings(&d));
    }

    #[test]
    fn a_file_holding_no_tenet_is_no_finding() {
        let d = scratch("names-empty");
        write(&d, "registry/tenet/weather.toml", "# nothing yet\n");
        assert!(findings(&d).is_empty(), "{:?}", findings(&d));
    }

    #[test]
    fn a_tenet_naming_no_subject_is_reported() {
        let d = scratch("names-none");
        write(&d, "registry/tenet/sky.toml", "[[tenet]]\nid = \"a\"\n");
        let f = findings(&d);
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(
            f[0].contains("tenet::a") && f[0].contains("names no subject"),
            "{f:?}"
        );
    }

    #[test]
    fn a_subject_that_is_not_a_row_is_reported() {
        let d = scratch("names-unknown");
        write(
            &d,
            "registry/tenet/sky.toml",
            "[[tenet]]\nid = \"a\"\nsubject = \"weather\"\n",
        );
        let f = findings(&d);
        assert!(
            f.iter().any(|m| {
                m.contains("tenet::a") && m.contains("`weather`") && m.contains("not a subject")
            }),
            "{f:?}"
        );
    }
}
