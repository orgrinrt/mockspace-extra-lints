//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `mock research-unbundle <bundle> <dir>`: a research bundle written out as the files of a research note.
//!
//! A helper that has no push of its own posts its research as one attachment: every file in turn, each
//! under a line `=== FILE: <name> ===`. This writes the files under `dir`, `mock/research/<stamp>_<slug>/`,
//! and splits any markdown file over [`LIMIT`] lines along its `## ` headings, so a note lands in the
//! shape the research directory keeps without being cut by hand.

use std::path::{Path, PathBuf};

use mockspace::tool::{ArgRoot, ArgSpec, Purpose, Tool, ToolContext, ToolReport};

mod bundle;
mod split;

#[cfg(test)]
mod tests;

/// The longest a file of a note may be.
pub const LIMIT: usize = 500;

/// The maker.
pub struct ResearchUnbundle;

impl Tool for ResearchUnbundle {
    fn name(&self) -> &'static str {
        "research-unbundle"
    }

    fn description(&self) -> &'static str {
        "writes a research bundle into a research note's directory, long files split along their headings"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Make {
            writes: &["{dir}/**"],
            roots:  &[ArgRoot {
                arg:   "dir",
                under: "mock/research/*",
            }],
        }
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[
            ArgSpec {
                name:        "bundle",
                required:    true,
                description: "the file holding the bundle, as `msg <channel> show --from <name> --attachment` prints it",
            },
            ArgSpec {
                name:        "dir",
                required:    true,
                description: "the note's directory, `mock/research/<YYYYMMDDHHMM>_<slug>`, which must not hold any of the files yet",
            },
        ]
    }

    fn help(&self) -> &'static str {
        "The bundle is every file in turn, each under a line `=== FILE: <name> ===`. A name is a relative path of letters, digits, `.`, `_` and `-`. A markdown file over 500 lines is written as `<stem>.md`, `<stem>-2.md`, and so on, cut before a `## ` heading, and each part after the first opens at a heading."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let (Some(bundle), Some(dir)) = (ctx.args.first(), ctx.args.get(1)) else {
            return ToolReport::inconclusive(
                "research-unbundle wants a bundle file and the note's directory",
            );
        };
        match unbundle(ctx.repo_root, bundle, dir) {
            Ok(out) => ToolReport::reported(out.report, out.files),
            Err(e) => ToolReport::inconclusive(e),
        }
    }
}

/// What a run wrote.
pub struct Written {
    /// How many files, after splitting.
    pub files:  usize,
    /// One line for each file that was split.
    pub report: String,
}

/// `<12 digits>_<slug>`, the name a note's directory has.
fn note_name(dir: &str) -> bool {
    let last = dir.rsplit('/').next().unwrap_or(dir);
    let Some((stamp, slug)) = last.split_once('_') else {
        return false;
    };
    stamp.len() == 12
        && stamp.bytes().all(|b| b.is_ascii_digit())
        && !slug.is_empty()
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// The bundle at `bundle` written under `dir`, both relative to the repository unless absolute.
///
/// # Errors
///
/// When the directory is not `mock/research/<stamp>_<slug>`, the bundle cannot be read or holds
/// no file, a name is unsafe or given twice, or a file to write is already there.
pub fn unbundle(repo: &Path, bundle: &str, dir: &str) -> Result<Written, String> {
    let rel = dir.trim_end_matches('/');
    if !rel.starts_with("mock/research/") || rel.matches('/').count() != 2 || !note_name(rel) {
        return Err(format!(
            "`{dir}` is not a note's directory: `mock/research/<12 digits>_<slug>`, the slug lowercase letters, digits and `-`"
        ));
    }
    let path = if Path::new(bundle).is_absolute() || Path::new(bundle).exists() {
        PathBuf::from(bundle)
    } else {
        repo.join(bundle)
    };
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read the bundle {}: {e}", path.display()))?;
    let files = bundle::parse(&text)?;
    let mut planned: Vec<(String, String)> = Vec::new();
    let mut report = String::new();
    for (name, body) in files {
        let parts = split::parts(&name, &body, LIMIT);
        if parts.len() > 1 {
            let names: Vec<&str> = parts.iter().map(|(n, _)| n.as_str()).collect();
            report.push_str(&format!("split {name} into {}\n", names.join(", ")));
        }
        planned.extend(parts);
    }
    let root = repo.join(rel);
    for (name, _) in &planned {
        if root.join(name).exists() {
            return Err(format!(
                "{rel}/{name} is already there, and nothing is overwritten"
            ));
        }
    }
    for (name, body) in &planned {
        let target = root.join(name);
        std::fs::create_dir_all(target.parent().unwrap_or(&root))
            .map_err(|e| format!("cannot make the directory of {rel}/{name}: {e}"))?;
        std::fs::write(&target, body).map_err(|e| format!("cannot write {rel}/{name}: {e}"))?;
    }
    Ok(Written {
        files: planned.len(),
        report,
    })
}

mockspace::lint_pack! {
    tools: [ResearchUnbundle],
}
