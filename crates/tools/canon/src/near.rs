//! The tenets a near word carries, listed after the plain answer of `find`.
//!
//! The index gives the forms the documents hold and the synonyms the registry's keywords give; this
//! says which tenets carry them, leaving out those the plain answer already listed.

use std::fmt::Write as _;

use corpus::repo::Near;

use crate::rows::{Tenet, carries};
use crate::verbs::{line, width};

/// The section to append to a `find` answer, or nothing where no near word adds a tenet.
pub(crate) fn section(all: &[Tenet<'_>], near: &[Near], plain: &str) -> String {
    let listed = |id: &str| {
        plain
            .lines()
            .any(|l| l.starts_with("  ") && l.split_whitespace().next() == Some(id))
    };
    let mut out = String::new();
    let mut seen: Vec<&str> = Vec::new();
    for n in near {
        let needle = n.word.to_lowercase();
        let fresh: Vec<&Tenet<'_>> = all
            .iter()
            .filter(|t| carries(t, &needle) && !listed(t.id) && !seen.contains(&t.id))
            .collect();
        if fresh.is_empty() {
            continue;
        }
        let how = if n.synonym { format!("synonym, {} rows", n.rows) } else { "form".to_string() };
        let w = width(fresh.iter().copied());
        let _ = writeln!(out, "  {}  ({how})", n.word);
        for t in fresh {
            seen.push(t.id);
            out.push_str("  ");
            line(&mut out, t, w);
        }
    }
    if out.is_empty() {
        return out;
    }
    format!("\nnear words, tenets that carry them and the word asked did not:\n{out}")
}
