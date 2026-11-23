//! A search answered in registry rows: the rows the filters keep, those carrying the question's
//! words first where there is a question.

use corpus::Query;
use mockspace::tool::ToolReport;
use muisti::repo::{Filter, RepoIndex, RowInfo};

use crate::spec::Spec;

/// How many rows a report lists before it counts the rest.
const LISTED: usize = 200;

/// How much of a row's words a report shows under its key.
const GLIMPSE: usize = 140;

/// The rows the filters keep, narrowed to those carrying the question where there is one.
#[must_use]
pub fn answer(
    mut out: String,
    spec: &Spec,
    index: &RepoIndex,
    filters: &impl Filter,
) -> ToolReport {
    let query = Query::new(&spec.question);
    let asked = !spec.question.trim().is_empty();
    let mut kept: Vec<(usize, RowInfo)> = Vec::new();
    let read = index.rows(filters, |r| {
        let words = format!("{} {}", r.key.replace('_', " "), r.words).to_lowercase();
        let carried = query
            .terms
            .iter()
            .filter(|t| !Query::find(&t.stem, &words).is_empty())
            .count();
        if !asked || carried > 0 {
            kept.push((carried, r));
        }
    });
    if let Err(e) = read {
        return ToolReport::inconclusive(format!("the index's rows could not be read: {e}"));
    }
    if asked && query.terms.is_empty() {
        return ToolReport::inconclusive(format!(
            "`{}` was never asked: no word of it is long and uncommon enough to search",
            spec.question
        ));
    }
    kept.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.key.cmp(&b.1.key)));
    out.push_str(&format!("IN THE REGISTRY ({} rows)\n\n", kept.len()));
    if kept.is_empty() {
        out.push_str("  nothing the filters keep\n");
    }
    for (_, r) in kept.iter().take(LISTED) {
        out.push_str(&row_line(r));
    }
    if kept.len() > LISTED {
        out.push_str(&format!("  ... and {} more\n", kept.len() - LISTED));
    }
    let n = kept.len();
    ToolReport::reported(out, n)
}

/// One row: its key, where it sits, its subject, and the start of what it says.
fn row_line(r: &RowInfo) -> String {
    let subject = r
        .subject
        .as_deref()
        .map_or_else(String::new, |s| format!("  [{s}]"));
    let mut glimpse: String = r.words.split_whitespace().collect::<Vec<_>>().join(" ");
    if glimpse.chars().count() > GLIMPSE {
        glimpse = glimpse.chars().take(GLIMPSE).collect::<String>() + "...";
    }
    format!(
        "  {}  {}:{}{subject}\n      {glimpse}\n",
        r.key, r.path, r.line
    )
}
