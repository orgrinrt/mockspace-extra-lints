//! `canon stats`: the shape of the registry's graph, asked of the repository index: the rows most
//! cited, the rows nothing touches, the words of a subject and the files citing a row.

use std::fmt::Write as _;

use corpus::repo::Index;

/// How many lines a table prints.
pub const LIMIT: usize = 20;

/// The tables, in the order the help lists them.
pub const TABLES: [&str; 4] = ["cited", "orphans", "terms", "files"];

/// One table, or a refusal naming what was wrong with the question.
///
/// # Errors
/// When the table is unknown, or one that needs a word has none, or the index could not answer.
pub fn table(index: &Index, which: &str, arg: Option<&str>) -> Result<String, String> {
    let mut out = String::new();
    let need = |what: &str| arg.ok_or_else(|| format!("`stats {which}` wants {what}"));
    let failed = |e: corpus::repo::IndexError| format!("the index could not answer: {e}");
    match which {
        "cited" => {
            index
                .most_cited(LIMIT, |k, n| {
                    let _ = writeln!(out, "{n:>4}  {k}");
                })
                .map_err(failed)?;
        },
        "orphans" => {
            index
                .orphans(arg, |k| {
                    let _ = writeln!(out, "{k}");
                })
                .map_err(failed)?;
        },
        "terms" => {
            let subject = need("a subject")?;
            index
                .terms_of_subject(subject, LIMIT, |w, n| {
                    let _ = writeln!(out, "{n:>4}  {w}");
                })
                .map_err(failed)?;
        },
        "files" => {
            let row = need("a row key such as `tenet::the_id`")?;
            index
                .files_citing(row, |p| {
                    let _ = writeln!(out, "{p}");
                })
                .map_err(failed)?;
        },
        other => {
            return Err(format!(
                "`{other}` is no table of stats; the tables are {}",
                TABLES.join(", ")
            ));
        },
    }
    if out.is_empty() {
        out.push_str("the index holds nothing for this\n");
    }
    Ok(out)
}
