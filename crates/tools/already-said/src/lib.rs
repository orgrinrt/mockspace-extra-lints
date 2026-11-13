//! Has this already been answered.
//!
//! Run before a question reaches a person. The expensive failure is not asking
//! twice, it is asking for a decision that is already recorded and then acting
//! on the second answer as though it were the first.
//!
//! Two of the deleted checks asked this: one over a term, one over a subject
//! before briefing a dispatch. Against a registry they are one question, and the
//! answer is which rows carry the term anywhere a reader would look.
//!
//! **A tool because it takes a question.** There is no failing case: no row
//! carrying a term is an ordinary and often correct state.

use corpus::day::{today, ymd};
use mockspace::tool::{ArgSpec, ConfigKey, NotALint, Purpose, Tool, ToolContext, ToolParams, ToolReport};
use muisti::repo::Query as AskQuery;

/// Fields that are citations rather than prose, and are not searched.
///
/// **Everything else on a row is searched, whatever it is called.** The first
/// version carried a hand-written table of namespace-to-field, and it was wrong
/// in both directions at once: it named `what` on `term` and `mechanism`, which
/// neither namespace declares, and omitted `gap`, which many rows carry. So a
/// distinctive phrase reachable only through a `gap` reported as absent, and
/// this tool's output is what licenses interrupting a person with a question.
///
/// A table that has to be kept in step with the schema will fall out of step
/// with the schema. Reading the row's own fields cannot, and a field added to a
/// namespace tomorrow is searched without anybody editing this.
///
/// The exclusions below are structural rather than a matter of taste: a
/// provenance entry is `root::path::anchor`, and `cites`, `superseded_by`, `related` and
/// `subject` name other rows by id, so a hit in one says a row or a document
/// was named, not that a subject was discussed. The row named carries its own
/// words and is found by them.
/// They are verified against the corpus at the foot of `run`, because a stale
/// exclusion is the same defect one rung down.
const NOT_PROSE: &[&str] = &["provenance", "cites", "superseded_by", "subject", "related"];

/// The tool, holding where the project keeps the maintainer's captured words.
#[derive(Default)]
pub struct AlreadySaid {
    maintainer: corpus::Maintainer,
}

impl Tool for AlreadySaid {
    fn name(&self) -> &'static str {
        "already-said"
    }

    fn description(&self) -> &'static str {
        "which rows already carry a term, before a question reaches a person"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Check(NotALint::TakesAQuestion)
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[ArgSpec {
            name:        "term",
            required:    true,
            description: "the word or phrase to look for, matched case-insensitively",
        }]
    }

    fn config_keys(&self) -> &[ConfigKey] {
        corpus::maintainer::KEYS
    }

    fn configure(&mut self, params: &ToolParams) {
        self.maintainer.configure(params);
    }

    fn help(&self) -> &'static str {
        "Every row carrying the term, with the field it appears in. An empty \
         result means nothing here takes the subject, which is the answer that \
         licenses asking. The rows are the working tree's, so when it is behind origin/dev, or that could not be fetched, the answer opens with a notice saying how far behind it is."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        // the rows are the working tree's, so the answer says first when that tree is behind `origin/dev`
        // or the trunk could not be fetched: a question answered there reads as open here
        let ready = corpus::repo::ready(ctx.mock_dir, ctx.repo_root, Some(&self.maintainer));
        self.run_over(ctx, &ready)
    }
}

impl AlreadySaid {
    /// The run over an index already made, with the notice of a tree behind the trunk.
    pub(crate) fn run_over(
        &self,
        ctx: &ToolContext<'_>,
        ready: &corpus::repo::Ready,
    ) -> ToolReport {
        let mut report = self.answer(ctx, ready);
        let notice = tickets::trunk::tree_notice(ctx.repo_root, true);
        if !notice.is_empty() {
            report.output.insert_str(0, &format!("{notice}\n"));
        }
        report
    }
}

impl AlreadySaid {
    /// The answer from the rows and the research, without the notice of a tree behind the trunk.
    pub(crate) fn answer(&self, ctx: &ToolContext<'_>, ready: &corpus::repo::Ready) -> ToolReport {
        // Joined rather than taking only the first, so a multi-word subject is
        // one question. Splitting on spaces would search for each word and
        // report a row that has all of them somewhere and the phrase nowhere.
        //
        // Trimmed, so a term of nothing but spaces is the empty question it is.
        // Untrimmed it went through as a search for a run of spaces, answered
        // that no row holds it and then said the research was not asked, and this
        // tool's own help says an empty result is what licenses interrupting a
        // person.
        let needle = ctx.args.join(" ").trim().to_lowercase();
        if needle.is_empty() {
            return ToolReport::inconclusive("no term given, so nothing was searched");
        }

        let mut out = String::new();
        let mut found = 0usize;
        let mut examined = 0usize;

        // Which exclusions were seen at all. An exclusion naming a field no row
        // carries is a stale exclusion, and it is reported rather than ignored.
        let mut excluded_seen = [false; NOT_PROSE.len()];

        let namespaces: Vec<String> = ctx.registry.namespaces().map(str::to_string).collect();
        for ns in &namespaces {
            for q in ctx.registry.rows_in(ns) {
                examined += 1;
                let slug = q.split_once("::").map_or(q.as_str(), |(_, s)| s);
                let Some(row) = ctx.registry.row(q) else {
                    continue;
                };
                for name in row.keys() {
                    if let Some(i) = NOT_PROSE.iter().position(|x| x == name) {
                        excluded_seen[i] = true;
                    }
                }
                let fields: Vec<(&str, &str)> =
                    row.iter().map(|(n, v)| (n.as_str(), v.as_str())).collect();
                let hits = hits_in_row(slug, &fields, &needle);
                if !hits.is_empty() {
                    found += 1;
                    // A superseded tenet stays whole so its replacement is read with it
                    // in view. Listed bare it reads as an answer that still binds.
                    let superseded = row
                        .get("superseded_by")
                        .filter(|s| !s.trim().is_empty())
                        .map(|s| format!("  superseded by {}", s.trim_matches(['[', ']', '"'])))
                        .unwrap_or_default();
                    out.push_str(&format!("{q}  ({}){superseded}\n", hits.join(", ")));
                }
            }
        }

        // Before the exclusion check, because with no rows at all every
        // exclusion is unseen and the stale branch answered for the empty one.
        // It said the search excludes two fields no row carries, which is true
        // and is not what happened: nothing was searched, and the branch below
        // could not be reached at all.
        if examined == 0 {
            return ToolReport::inconclusive(
                "no rows were searched, so this establishes nothing about the term",
            );
        }

        let stale: Vec<&str> = NOT_PROSE
            .iter()
            .zip(excluded_seen)
            .filter(|(_, seen)| !seen)
            .map(|(n, _)| *n)
            .collect();
        if !stale.is_empty() {
            return ToolReport::inconclusive(format!(
                "these fields are excluded from the search and no row carries one: {}. \
                 An exclusion nobody checks is how the hand-written field table this \
                 replaced came to name a field that did not exist.",
                stale.join(", ")
            ));
        }

        out.push_str(&format!("\n{found} of {examined} rows carry it.\n"));

        // And the research, because the registry is about two percent of what
        // a project has written. A `0 of N rows` answer read as an absence
        // licenses asking a question the research already answered, in a file
        // this line would have named.
        // The phrase itself, not every word of it. The registry half above is a
        // substring match on the whole needle, and the corpus half was splitting
        // it and scoring any document carrying any word: `already-said
        // quaternion-nonsense-zz` reported the research carrying it in 306 of
        // 4019 documents over a corpus holding that string zero times, because
        // `zz` alone is in 596 files. Two questions under one sentence saying
        // the term is carried.
        let query = corpus::Query::phrase_only(&needle);
        let mut found_docs = ready.index.search(&AskQuery {
            text:   &needle,
            expand: &[],
            phrase: true,
            limit:  3,
            today:  today(),
        });
        found_docs
            .caveats
            .splice(0 .. 0, ready.notes.iter().cloned());
        out.push_str(&research_line(
            &needle,
            found_docs.indexed,
            query.terms.len(),
            found_docs.matched,
            found_docs.indexed,
            found_docs.hits.len(),
        ));
        if !found_docs.hits.is_empty() {
            for h in &found_docs.hits {
                let when = match (h.day, h.age_days) {
                    (Some(d), Some(a)) => format!("{}, {a}d ago", ymd(d)),
                    _ => "undated".to_string(),
                };
                out.push_str(&format!(
                    "  {}:{}  [{}, {when}{}]\n",
                    h.path,
                    h.line,
                    h.class.label(),
                    corpus::repo::found_by(h)
                ));
            }
            out.push_str(
                "  Run `mock unearth` before treating any of this as settled or missing.\n",
            );
        }
        // Both of the branches above quote a document count, and a count taken
        // over a corpus with a root absent from disk is a smaller claim in the
        // same words. This tool reported "in at least 3 of 3920 documents" with
        // two whole trees unread and nothing saying so.
        for caveat in &found_docs.caveats {
            out.push_str(&format!("  Not read: {caveat}.\n"));
        }
        ToolReport::reported(out, examined)
    }
}

/// What the research half of the answer says, in one sentence.
///
/// **Four states and they are not interchangeable**, which is the whole reason
/// this is a function rather than three inline branches. Two of them were wrong
/// at once, and each read exactly like a measurement.
///
/// A corpus that could not be read says nothing about the term. A question that
/// expanded to no terms was never asked, so the documents were not opened and
/// the registry rows above are the entire answer; this said "nothing in the 3995
/// research documents either" about the word `the`. A question that was asked
/// and found nothing is a real absence and is the one that licenses interrupting
/// a person. And a question that found something reports how many documents
/// carry it, not how many are listed: this read "in at least 3 of 3970" for
/// every term the corpus holds at all, because 3 was the limit passed to the
/// search.
fn research_line(
    needle: &str,
    corpus: usize,
    terms: usize,
    scored: usize,
    searched: usize,
    shown: usize,
) -> String {
    if corpus == 0 {
        return "\nThe research corpus could not be read, so nothing here says whether it carries \
                the term.\n"
            .to_string();
    }
    if terms == 0 {
        return format!(
            "\nThe research was not asked: `{needle}` is too common to search on, so the {corpus} \
             documents were not read and the rows above are the whole of this answer.\n"
        );
    }
    if shown == 0 {
        return format!(
            "\nNothing in the {searched} research documents either, asked in this term's own \
             words. `mock unearth` asks the same question in the registry's words as well, which \
             is the search that finds a subject discussed under another name.\n"
        );
    }
    format!(
        "\nAnd the research carries it, in {scored} of {searched} documents. The nearest {shown}:\n"
    )
}

/// Which of a row's fields carry the term, `id` included.
///
/// Pulled out of the walk so it can be tested against a row built by hand: this
/// tool decides whether a question reaches a person, and until now nothing
/// anywhere pinned what it matches.
///
/// The slug counts as a field named `id` with its underscores read as spaces,
/// because `slot::the_passphrase_is_gone` is a row about a passphrase being gone
/// and somebody asking in those words should find it. The declared `id` field is
/// skipped so the same hit is not reported twice under two names.
///
/// The order is the order it is handed, and `run` hands it a row, which is a map
/// keyed by field name. So what a person sees is alphabetical: `keywords` before
/// `says`, whatever order the row was written in.
fn hits_in_row(slug: &str, fields: &[(&str, &str)], needle: &str) -> Vec<String> {
    let stem = registry_stem(needle);
    let mut hits = Vec::new();
    if !corpus::Query::find(&stem, &slug.replace('_', " ")).is_empty() {
        hits.push("id".to_string());
    }
    for (name, value) in fields {
        if NOT_PROSE.contains(name) || *name == "id" {
            continue;
        }
        if !corpus::Query::find(&stem, &value.to_lowercase()).is_empty() {
            hits.push((*name).to_string());
        }
    }
    hits
}

/// The needle as the corpus half of this tool will match it.
///
/// **The two halves used different matchers and disagreed in both directions.**
/// The registry half was a bare substring test, so `already-said conveyed`
/// returned no row, which its help says licenses interrupting a
/// person, while `unearth conveyed` named five rows including a ruling whose
/// `says` is about conveyance. And `already-said key` returned rows, many of
/// which carry no standalone `key` at all and matched inside `keywords`.
///
/// One matcher now, the corpus's: a stem anchored at a word boundary and run as
/// a prefix. That reaches `conveyance` from `conveyed`, and it stops `key`
/// reaching `monkey`, which is what a bare substring test allowed.
///
/// **It does not stop `key` reaching `keywords`, and that is the design rather
/// than a residue.** The forward run is exactly what carries `conveyed` to
/// `conveyance`; a matcher without it fixes one direction of the disagreement by
/// breaking the other. What the two halves now share is one answer to the
/// question rather than two, which was the defect.
fn registry_stem(needle: &str) -> String {
    corpus::Query::phrase_only(needle)
        .terms
        .first()
        .map_or_else(|| needle.to_string(), |t| t.stem.clone())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod run_tests;

mockspace::lint_pack! {
    tools: [AlreadySaid::default()],
}
