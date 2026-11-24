//! What the index holds and how far behind its files it is, and the refresh that brings it up to
//! date.

use std::collections::BTreeMap;

use corpus::Maintainer;
use corpus::repo::Opened;
use mockspace::tool::{ToolContext, ToolReport};

use crate::registry::mock_prefix;

/// Bring the index of this checkout up to date. This is what a question starts in the background,
/// and what a hook runs; a person need not.
#[must_use]
pub fn refresh(ctx: &ToolContext<'_>, maintainer: &Maintainer) -> ToolReport {
    match corpus::repo::refresh(ctx.mock_dir, ctx.repo_root, Some(maintainer)) {
        Ok(Some(done)) => {
            let mut out = format!("{}\n", counts(&done));
            for c in &done.caveats {
                out.push_str(&format!("NOTE: {c}\n"));
            }
            ToolReport::reported(out, done.added + done.changed + done.removed + done.commits)
        },
        Ok(None) => {
            ToolReport::reported(
                "another refresh holds the index and is bringing it up to date\n",
                0,
            )
        },
        Err(e) => ToolReport::inconclusive(e),
    }
}

/// What a refresh read, as one line.
fn counts(done: &muisti::repo::Refreshed) -> String {
    format!(
        "refreshed: {} added, {} changed, {} removed, {} commits read",
        done.added, done.changed, done.removed, done.commits
    )
}

/// The line a question opens with, saying what the refresh before it read, or why it read nothing.
#[must_use]
pub fn refreshed_note(r: &Result<Option<muisti::repo::Refreshed>, String>) -> String {
    match r {
        Ok(Some(done)) => {
            let mut s = format!("the index was brought current first; {}", counts(done));
            for c in &done.caveats {
                s.push_str(&format!("; {c}"));
            }
            s
        },
        Ok(None) => {
            "another refresh holds the index, so this answer is from the index as it stood"
                .to_string()
        },
        Err(e) => format!("the index could not be brought current: {e}"),
    }
}

/// The index of this checkout: where it is, what it holds by class, and how far behind it is.
#[must_use]
pub fn report(ctx: &ToolContext<'_>, opened: &Opened) -> ToolReport {
    let mut out = String::new();
    if let Ok(path) = corpus::repo::index_path(ctx.repo_root) {
        out.push_str(&format!("INDEX  {}\n\n", path.display()));
    }
    let Some(index) = &opened.index else {
        return ToolReport::inconclusive(format!("{out}{}", opened.notes.join(" ")));
    };
    let mock = mock_prefix(ctx.mock_dir, ctx.repo_root);
    let words = None;
    let mut by_class: BTreeMap<&'static str, usize> = BTreeMap::new();
    let read = index.held(|p| {
        *by_class
            .entry(corpus::repo::class_of(p, &mock, words).label())
            .or_insert(0) += 1;
    });
    if let Err(e) = read {
        return ToolReport::inconclusive(format!(
            "{out}what the index holds could not be read: {e}"
        ));
    }
    let total: usize = by_class.values().sum();
    out.push_str("WHAT IT HOLDS\n\n");
    for (class, n) in &by_class {
        out.push_str(&format!("  {n:>6} {class}\n"));
    }
    out.push_str(&format!("  {total:>6} documents\n\nHOW FAR BEHIND\n\n"));
    let b = &opened.behind;
    if b.is_empty() {
        out.push_str("  current with the checkout's files\n");
    } else {
        out.push_str(&format!(
            "  {} new, {} changed, {} gone, {} commits\n",
            b.missing.len(),
            b.moved.len(),
            b.gone.len(),
            b.commits
        ));
        for why in b.definition.iter().chain(b.history.iter()) {
            out.push_str(&format!("  {why}\n"));
        }
    }
    if !opened.notes.is_empty() {
        out.push('\n');
        for n in &opened.notes {
            out.push_str(&format!("NOTE: {n}\n"));
        }
    }
    ToolReport::reported(out, total)
}
