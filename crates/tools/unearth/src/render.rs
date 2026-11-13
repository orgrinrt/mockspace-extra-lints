//! Turning what the passes found into the report a reader gets.
//!
//! Split out because deciding and rendering are two jobs and the file was over
//! the limit carrying both. Nothing here decides anything: every function takes
//! what the passes already worked out and writes it into the buffer.
//!
//! The one thing it reads from the tool is [`super::SHOWN`], which bounds the
//! list rather than the search.

use std::collections::BTreeMap;

use corpus::Query;
use corpus::day::ymd;
use muisti::repo::{RepoHit as IndexHit, RepoReport as IndexReport};

use super::SHOWN;

pub(crate) fn render_question(
    out: &mut String,
    query: &Query,
    keywords: &[String],
    rows: &[String],
) {
    out.push_str("SEARCHED FOR\n\n");
    for t in &query.terms {
        out.push_str(&format!("  {:<28} {}\n", t.text, t.source.label()));
    }
    // What was thrown out, which decides how much of the question the list above
    // is actually made of. A question reduced to one throwaway word plus the
    // keywords that word happened to harvest ranks the corpus confidently and on
    // nothing, and the only place that is visible is here.
    if !query.dropped.is_empty() {
        out.push_str(&format!(
            "\n  Too common to search, and dropped: {}.\n",
            query.dropped.join(", ")
        ));
    }
    // Two silences, and this said the first for both. A row can carry the
    // subject and declare no keywords, and the report then said no registry row
    // carries this subject directly above the list of the rows that do. The
    // fix a reader needs differs: rephrase in the first case, and put keywords
    // on those rows in the second, which is what the field was added for.
    if keywords.is_empty() {
        if rows.is_empty() {
            out.push_str(
                "\n  No registry row carries this subject, so the question was asked only in \
                 its own words. That is the weaker search and it is worth rerunning with a \
                 term the corpus might use.\n",
            );
        } else {
            out.push_str(&format!(
                "\n  The {} row{} below carry this subject and declare no keywords, so the \
                 question was asked only in its own words. That is the weaker search, and \
                 keywords on those rows are what would widen it.\n",
                rows.len(),
                if rows.len() == 1 { "" } else { "s" }
            ));
        }
    }
    out.push('\n');
}

pub(crate) fn render_registry(out: &mut String, rows: &[String]) {
    out.push_str("IN THE REGISTRY\n\n");
    if rows.is_empty() {
        out.push_str("  nothing\n\n");
        return;
    }
    for q in rows.iter().take(20) {
        out.push_str(&format!("  {q}\n"));
    }
    if rows.len() > 20 {
        out.push_str(&format!("  ... and {} more\n", rows.len() - 20));
    }
    out.push('\n');
}

/// One hit, on one line: when it was written, how long ago, by whom, whether the canon has
/// spoken on the subject since, whether a row ever came from it, where to open it, and how many
/// copies of it were folded in.
fn hit_line(h: &IndexHit, cited: &BTreeMap<String, usize>, canon_day: Option<i64>) -> String {
    let when = match (h.day, h.age_days) {
        (Some(d), Some(a)) => format!("{} {:>4}d", ymd(d), a),
        _ => "undated       ".to_string(),
    };
    let stale = match (h.day, canon_day) {
        (Some(d), Some(c)) if d < c => format!(" OLDER THAN THE CANON ({})", ymd(c)),
        _ => String::new(),
    };
    let mark = match cited.get(&h.path) {
        None => "UNCITED".to_string(),
        Some(1) => "cited by 1 row".to_string(),
        Some(n) => format!("cited by {n} rows"),
    };
    let copies = match h.copies.len() {
        0 => String::new(),
        1 => ", 1 copy folded".to_string(),
        n => format!(", {n} copies folded"),
    };
    format!(
        "  {when}  {:<8} {}:{}  [{mark}{copies}{}]{stale}\n      {}\n",
        h.class.label(),
        h.path,
        h.line,
        corpus::repo::found_by(h),
        h.excerpt
    )
}

pub(crate) fn render_corpus(
    out: &mut String,
    found: &IndexReport,
    cited: &BTreeMap<String, usize>,
    filtered: usize,
    canon_day: Option<i64>,
) {
    out.push_str(&format!(
        "IN THE RESEARCH ({} documents indexed)\n\n",
        found.indexed
    ));
    if found.hits.is_empty() {
        if filtered > 0 {
            // Not the same fact as nothing scoring, and the report printed both
            // in one block. Everything that scored is cited already, which
            // means the corpus carries the subject and none of it is
            // unabsorbed, and that is the answer rather than an empty result.
            out.push_str(
                "  nothing uncited. Every document that scored is cited by a row already, so \
                 this mode dropped all of them.\n\n",
            );
        } else {
            out.push_str("  nothing scored\n\n");
        }
    }
    for h in &found.hits {
        out.push_str(&hit_line(h, cited, canon_day));
    }
    // A hit is the best of the copies folded into it, so these are distinct
    // passages and the count is of those.
    let shown = found.hits.len() + filtered;
    if found.matched > shown {
        out.push_str(&format!(
            "\n  {} more documents matched and did not fit the limit of {SHOWN}. Ask something \
             narrower to reach them.\n",
            found.matched - shown
        ));
    }
    if filtered > 0 {
        out.push_str(&format!(
            "  {filtered} more cleared the cut and are cited already, so this mode dropped \
             them.\n"
        ));
    }
    if !found.dead_terms.is_empty() {
        out.push_str(&format!(
            "\n  Matched nothing anywhere: {}.\n",
            found.dead_terms.join(", ")
        ));
    }
    for c in &found.caveats {
        out.push_str(&format!("  Not read: {c}.\n"));
    }
    out.push_str(&citation_base_rate(cited.len(), found.indexed));
    out.push('\n');
}

/// The sentence a reader acts on.
///
/// Four states, and the third is the one this tool was built for: the research
/// carries the subject and the registry does not, which is not a gap in the
/// design and is a transcription that never happened.
/// What share of the corpus any row has ever cited.
///
/// **Printed beside the marker, because a marker is worth what it varies over.**
/// `UNCITED` was described in this file as the point of the line while it marked
/// 92.3% of the corpus, so it named the ordinary state and read as a finding,
/// and `unearth <subject> uncited` read as a shortlist while filtering out about
/// one document in thirteen. The informative half is the other one: 306 of 3983
/// documents have ever been cited by a row.
pub(crate) fn citation_base_rate(cited_docs: usize, corpus: usize) -> String {
    if corpus == 0 {
        return String::new();
    }
    if cited_docs == 0 {
        return format!(
            "  No row anywhere cites any of the {corpus} documents, so nothing here is marked \
             cited and the marker decides nothing.\n"
        );
    }
    #[allow(clippy::cast_precision_loss)]
    let share = cited_docs as f64 * 100.0 / corpus as f64;
    format!(
        "  {cited_docs} of the {corpus} documents have ever been cited by a row, which is \
         {share:.1}%. So UNCITED is the ordinary state and `cited by` is the exception; read \
         the marker that way rather than as a finding.\n"
    )
}

pub(crate) fn render_verdict(
    out: &mut String,
    rows: &[String],
    found: &IndexReport,
    cited: &BTreeMap<String, usize>,
    carried: bool,
) {
    out.push_str("VERDICT\n\n");

    // The uncited hits first, because they are the work. A subject can be
    // present in both the registry and the research and still have most of its
    // material unabsorbed, and that case reads as healthy on every other line
    // of this report.
    let uncited: Vec<&IndexHit> = found
        .hits
        .iter()
        .filter(|h| !cited.contains_key(&h.path))
        .collect();
    if !uncited.is_empty() {
        out.push_str(&format!(
            "  {} of the {} documents above are cited by no row anywhere in the registry:\n",
            uncited.len(),
            found.hits.len()
        ));
        for h in uncited.iter().take(8) {
            out.push_str(&format!("    {}:{}\n", h.path, h.line));
        }
        if uncited.len() > 8 {
            out.push_str(&format!("    ... and {} more\n", uncited.len() - 8));
        }
        out.push('\n');
    }

    // Whether the research carries the subject, taken before the `uncited`
    // filter rather than after it. Read off the shown hits, a subject whose
    // every document is cited came back saying neither the registry nor the
    // research carries it, with the line about a short dead-term list meaning a
    // real absence underneath. That is the exact claim this tool exists to stop
    // anybody making, printed by the tool, over a corpus carrying the subject
    // in the documents it had just dropped.
    let has_corpus = carried;
    match (rows.is_empty(), has_corpus) {
        (true, true) => {
            out.push_str(
                "  The research carries this and the registry does not. That is a transcription \
             that never happened rather than an open question, and the files above are where \
             the answer is. Read them before putting anything to a person.\n",
            )
        },
        (false, true) => {
            out.push_str(
                "  Both carry it, which is the state that most often reads as finished and is \
             not. Check the rows against the documents before adding anything, and start with \
             the uncited ones above: the usual finding is that a row states half of what the \
             research settled, and the half it drops is in a file no row ever cited.\n",
            )
        },
        (false, false) => {
            out.push_str(
                "  The registry carries it and the research does not, which is unusual. Either the \
             rows rest on something outside this tree, or the question is in the registry's \
             words rather than the corpus's.\n",
            )
        },
        (true, false) => {
            out.push_str(
                "  Neither carries it. If the dead-term line above is short, this is a real \
             absence and the question is worth asking. If it is long, the question was asked \
             in words nothing here uses, and rephrasing costs less than a person's attention.\n",
            )
        },
    }
}
