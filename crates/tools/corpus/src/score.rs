//! Ranking documents against an expanded question.
//!
//! Four things decide a score and every one of them is reported per hit, so a
//! ranking can be argued with rather than believed. A number nobody can take
//! apart is the same defect as a grep nobody can trust, one rung up.

use crate::corpus::Corpus;
use crate::doc::{Doc, Kind};
use crate::index::Index;
use crate::terms::{self, Query};

/// One document that matched.
pub struct Hit {
    pub path:        String,
    pub kind:        Kind,
    pub score:       f64,
    /// Which terms landed, for the reader to see what the match rests on.
    pub matched:     Vec<String>,
    /// The share of the question's terms this document carries.
    pub coverage:    f64,
    /// The same share, weighted by what the corpus says each term is worth.
    ///
    /// **[`Hit::coverage`] counts `file` and `paper` the same**, and on this
    /// corpus `file` is in 59% of the documents and `paper` in 3%. A slot whose
    /// `use` is written in plain English then draws a high share out of any
    /// document also written in plain English, which is most of them.
    ///
    /// Measured on the seventeen slot leads: twenty-five of the seventy-four
    /// terms that placed them sit in more than a fifth of the corpus, and the
    /// commonest is `file` at 59%, just under the [`UBIQUITOUS`] cut, so not one
    /// of them was ever reported as carrying nothing.
    ///
    /// **Neither number may be sorted on, and that was measured over four leads
    /// before it was measured over all of them.** The claim that used to sit
    /// here was that the two move together and separate nothing, which is
    /// harmless. Over the whole set of seventeen it is worse than that: the one
    /// lead a reader followed and called good ranks **third**, and the two above
    /// it are leads nobody wanted, the top one pointing at a line that is
    /// somebody's note to cross-check an audit. Coverage does the same, putting
    /// that lead at 92% against the good one's 90%.
    ///
    /// So a floor over either keeps the two bad ones to keep the good one, and a
    /// reader who sorts on the column is led away from the answer rather than
    /// merely not toward it. The four leads the earlier claim was measured over
    /// were the four a previous pass had labelled, which is choosing the sample
    /// and then reporting what it says.
    ///
    /// **Both are kept, printed, and used for nothing.** What a share says is
    /// how much of the question a document carries, which is true and worth
    /// showing beside a hit, and it is not a ranking of how much of the answer
    /// it carries. The line itself is what decides a lead, which is why it is
    /// printed.
    pub informative: f64,
    /// The strongest single occurrence: where to open, and what is there.
    pub line:        usize,
    pub excerpt:     String,
}

/// A search, with everything needed to disbelieve it.
pub struct Search {
    pub hits:       Vec<Hit>,
    /// Documents actually read. A zero makes every finding void, and the caller
    /// says so rather than reporting no hits.
    pub searched:   usize,
    /// Terms that matched nothing anywhere. **The most useful line in the
    /// output.** A question whose asked word is dead and whose registry
    /// expansions all landed found the subject under other words, and a
    /// question where everything is dead has established a real absence.
    pub dead:       Vec<String>,
    /// Terms that matched so widely they carry no information.
    pub ubiquitous: Vec<String>,
    /// Scored below the confidence cut, and how many.
    ///
    /// **Counted before the limit is applied**, because a document's score does
    /// not know what limit it was ranked under. This once counted both this and
    /// the field below, and the sentence naming it said the whole lot had scored
    /// too low: on one real question it covered 501 documents that had cleared
    /// the cut comfortably, which a reader takes as the corpus having little to
    /// say.
    pub trimmed:    usize,
    /// Cleared the cut and did not fit the limit.
    ///
    /// A different fact and a different action: these are worth reading, and a
    /// narrower question or a deeper limit reaches them, where nothing reaches
    /// what `trimmed` counts.
    pub capped:     usize,
    /// Words of the question that carry no subject and were not searched for.
    ///
    /// **Non-empty beside a `searched` of zero means the question was never
    /// asked**, and nothing else here says anything about the corpus. That pair
    /// is what separates a question thrown away from a subject genuinely
    /// absent, and a caller reporting the second when it had the first is the
    /// failure this field exists to make visible.
    pub dropped:    Vec<String>,
}

impl Search {
    /// How many documents matched at all, whatever became of them afterwards.
    ///
    /// **The number a reader is asking for**, and the one that was unavailable
    /// while the cut and the limit were a single count. `already-said` printed
    /// "in at least 3 of 3970 documents" for every term the corpus carried,
    /// because 3 was the limit it had passed in; `boot` is in 1284.
    #[must_use]
    pub fn scored(&self) -> usize {
        self.hits.len() + self.capped + self.trimmed
    }
}

/// Documents scoring below this share of the top score are not shown.
///
/// A share rather than an absolute, because scores are not comparable between
/// questions: a rare subject scores in the tens and a common one in the
/// hundreds, and a fixed floor would show everything for one and nothing for the
/// other. A judgement, and the number is here rather than buried so it can be
/// argued with.
const CONFIDENCE_CUT: f64 = 0.12;

/// A term appearing in more than this share of the corpus is reported as
/// carrying no information rather than silently contributing nothing.
///
/// It already contributes nearly nothing, because the inverse frequency below
/// takes it to about zero. What this adds is saying so: a reader whose question
/// expanded into five ubiquitous words and one real one should see that, since
/// the ranking is then resting on one term.
const UBIQUITOUS: f64 = 0.6;

/// How many of a document's matching terms count toward its score.
///
/// **Breadth has to stop being unbounded or the longest file always wins.** A
/// question expanded through the registry can carry ninety terms, and a document
/// long enough to touch fifty of them outscores the one that answers the
/// question, however weakly each of the fifty lands. Measured: the intent
/// catalogue took the top of a data-store search on forty-eight terms, over the
/// file that states the design.
///
/// So a document is scored on the best it has to offer rather than on how much
/// of the language it contains, which is also what a reader does when they open
/// it. Eight is a judgement and it is here rather than buried.
const BEST_TERMS: usize = 8;

/// How many matched terms a report names per document.
///
/// Not a scoring parameter. Forty term names on one line is a list nobody reads,
/// and a report nobody reads cannot be argued with, which is the whole point of
/// reporting the axes at all.
const NAMED_TERMS: usize = 6;

/// How hard a long document is discounted for its length.
///
/// **Nothing normalised for length at all until a real question exposed it.** A
/// slot asking about getting a secret wrong a few times was handed a
/// display-pipeline document, because `wrong`, `few` and `times` are ordinary
/// words and a file of a hundred thousand words contains most ordinary words.
/// Capping the terms that count and flooring the coverage both failed on it: the
/// coverage was genuinely high, which is the point. A long document really does
/// carry more of any question, and that is length rather than relevance.
///
/// Zero would ignore length and one would divide it out entirely, which
/// overcorrects and buries the long consolidations that are often the answer
/// here. A half is the usual middle and the corpus agrees with it: at a half the
/// display-pipeline document leaves the top of that question and the demand-side
/// deliverable takes it.
const LENGTH_PULL: f64 = 0.5;

/// Where a term's repetition stops adding anything.
///
/// **The first version damped repetition with a logarithm, which is unbounded**,
/// so four hundred occurrences of an ordinary word still outscored one
/// occurrence in a heading by four to one and no amount of length discount
/// caught up. Saturating instead means the fortieth mention is worth almost
/// exactly what the four-hundredth is, which is what a reader thinks too: a
/// document either is about the subject or is not, and saying the word again
/// does not make it more so.
///
/// The pairing with [`LENGTH_PULL`] is the standard one and they only work
/// together: saturation decides how fast repetition stops counting, and the
/// length term decides how much of the repetition was the document being long
/// rather than being relevant.
const SATURATION: f64 = 1.2;

/// Rank the corpus against the question.
///
/// The shape, and each part earns its place:
///
/// - **Inverse document frequency.** A term in six hundred of six hundred and
///   eighty files says nothing about which one to open. A term in four is the
///   whole answer. Without this the ranking is a word-count and the longest file
///   always wins, which is what a naive grep over this corpus does.
/// - **Position.** A hit in a heading is the document declaring its subject; a
///   hit in a blockquote is somebody being quoted, and in this corpus that is
///   usually the maintainer. Both outrank a mention in passing.
/// - **Coverage.** How many of the question's distinct terms land, which
///   separates a document about the subject from one that says one of its words
///   forty times. This is the part a term-frequency ranking cannot express and
///   it is what finds the file that discusses the thing under other words.
/// - **Kind.** Where a settled answer lives. A consolidation and a probe may say
///   the same words and only one of them is the answer.
#[must_use]
pub fn rank(corpus: &Corpus, index: &Index, query: &Query, limit: usize) -> Search {
    let n = corpus.docs.len();
    // Two ways to consult nothing: an empty corpus, and a question that expanded
    // into no terms because every word of it was too common to search. Both
    // report zero read, which is literally what happened, and it is the number a
    // caller weighs before calling an empty result an absence.
    //
    // **Reporting `n` here was the defect.** A question of ordinary English left
    // no terms, the corpus was never opened, and the report said the whole of it
    // had been read and had nothing, with an empty dead-term line underneath.
    // The verdict reads a short dead-term line as evidence that the absence is
    // real, and no line at all is as short as it gets, so the tool answered its
    // most confident absence to the questions it had understood least.
    if n == 0 || query.terms.is_empty() {
        return Search {
            hits:       Vec::new(),
            searched:   0,
            dead:       query.terms.iter().map(|t| t.text.clone()).collect(),
            ubiquitous: Vec::new(),
            trimmed:    0,
            capped:     0,
            dropped:    query.dropped.clone(),
        };
    }

    // Pass one: where each term lands, per document, term by term rather than
    // document by document. The inverse frequency is not knowable until every
    // document has been consulted, so this is kept rather than recomputed.
    //
    // A single-token term is a range lookup on the index. Anything spanning
    // several tokens is scanned, because adjacency is exactly what an index over
    // single tokens throws away, and no key can ever carry the separator.
    let mut per_doc: Vec<Vec<(usize, f64, usize)>> = vec![Vec::new(); n];
    let mut df = vec![0usize; query.terms.len()];
    for (i, term) in query.terms.iter().enumerate() {
        if terms::is_multi_token(&term.stem) {
            for (d, doc) in corpus.docs.iter().enumerate() {
                let at = Query::find(&term.stem, &doc.lower);
                if at.is_empty() {
                    continue;
                }
                df[i] += 1;
                let mut weighted = 0.0f64;
                let mut best = at[0];
                let mut best_w = 0.0f64;
                for offset in at {
                    let w = doc.position_weight(offset);
                    weighted += w;
                    if w > best_w {
                        best_w = w;
                        best = offset;
                    }
                }
                per_doc[d].push((i, weighted, best));
            }
            continue;
        }
        for p in index.postings(&term.stem) {
            df[i] += 1;
            per_doc[p.doc].push((i, p.weighted, p.best));
        }
    }

    #[allow(clippy::cast_precision_loss)]
    let total = n as f64;
    // Smoothed so it never reaches zero. The unsmoothed form takes a term
    // present in every document to exactly nothing, which is right about its
    // discriminating power and wrong as behaviour: a two-document corpus where
    // both carry the asked word then returns no hits at all, and a tool that
    // answers a question it can answer with silence is the defect this whole
    // crate exists to remove. A term in every file still contributes about a
    // tenth of what a term in one does, and `ubiquitous` below names it, so the
    // reader is told rather than shown an empty list.
    let idf: Vec<f64> = df
        .iter()
        .map(|&d| {
            #[allow(clippy::cast_precision_loss)]
            let d = d as f64;
            (1.0 + total / d.max(1.0)).ln()
        })
        .collect();

    let dead: Vec<String> = query
        .terms
        .iter()
        .zip(&df)
        .filter(|&(_, &d)| d == 0)
        .map(|(t, _)| t.text.clone())
        .collect();
    let ubiquitous: Vec<String> = query
        .terms
        .iter()
        .zip(&df)
        .filter(|&(_, &d)| {
            #[allow(clippy::cast_precision_loss)]
            let d = d as f64;
            d / total > UBIQUITOUS
        })
        .map(|(t, _)| t.text.clone())
        .collect();

    #[allow(clippy::cast_precision_loss)]
    let term_count = query.terms.len() as f64;
    // What the whole question is worth, so a document's share of it can be
    // stated. The same product the scoring below uses per term, summed over
    // every term rather than over the ones that landed.
    let asked_worth: f64 = query
        .terms
        .iter()
        .zip(&idf)
        .map(|(t, i)| t.weight * i)
        .sum();
    // The corpus's own mean, so the discount is relative to what a document here
    // usually looks like rather than to a number somebody picked. A corpus of
    // uniformly long files then discounts nothing, which is right.
    #[allow(clippy::cast_precision_loss)]
    let mean_length =
        (corpus.docs.iter().map(|d| d.lower.len()).sum::<usize>() as f64 / total).max(1.0);
    let mut hits: Vec<Hit> = Vec::new();
    for (doc, here) in corpus.docs.iter().zip(&per_doc) {
        if here.is_empty() {
            continue;
        }
        // Every term's contribution, kept so the strongest can be picked out.
        // Summing as we go was the first version and it could not do either of
        // the two things below: cap the breadth, or name the terms that
        // actually decided the placement.
        #[allow(clippy::cast_precision_loss)]
        let length = doc.lower.len() as f64;
        let norm = 1.0 - LENGTH_PULL + LENGTH_PULL * (length / mean_length);
        let mut contributions: Vec<(f64, usize, usize)> = here
            .iter()
            .map(|&(i, weighted, at)| {
                let tf = weighted * (SATURATION + 1.0) / (weighted + SATURATION * norm);
                (query.terms[i].weight * idf[i] * tf, i, at)
            })
            .collect();
        contributions.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        let raw: f64 = contributions.iter().take(BEST_TERMS).map(|(c, ..)| c).sum();
        if raw <= 0.0 {
            continue;
        }
        // The excerpt comes from the strongest term rather than the first, so a
        // reader lands on the line that placed the document rather than on its
        // first mention of a common word.
        let best_at = contributions[0].2;
        let matched: Vec<String> = contributions
            .iter()
            .take(NAMED_TERMS)
            .map(|&(_, i, _)| query.terms[i].text.clone())
            .collect();

        // Coverage stays over the whole question rather than over the capped
        // set: it is the answer to "how much of what I asked does this carry",
        // and capping it would make every document that clears the cap look
        // identical.
        #[allow(clippy::cast_precision_loss)]
        let coverage = here.len() as f64 / term_count;
        let carried: f64 = here
            .iter()
            .map(|&(i, ..)| query.terms[i].weight * idf[i])
            .sum();
        let informative = if asked_worth > 0.0 { carried / asked_worth } else { 0.0 };
        let score = raw * (0.25 + 0.75 * coverage) * doc.kind.weight();
        hits.push(Hit {
            path: doc.path.display().to_string(),
            kind: doc.kind,
            score,
            matched,
            coverage,
            informative,
            line: doc.line_of(best_at),
            excerpt: excerpt(doc, best_at),
        });
    }

    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // Two separate reductions, counted separately. The cut is about what the
    // documents scored and the limit is about how long a report a person will
    // read, and folding them into one number turned the second into a claim
    // about the first.
    let scored = hits.len();
    if let Some(top) = hits.first().map(|h| h.score) {
        let floor = top * CONFIDENCE_CUT;
        hits.retain(|h| h.score >= floor);
    }
    let above_cut = hits.len();
    hits.truncate(limit);
    Search {
        trimmed: scored - above_cut,
        capped: above_cut - hits.len(),
        hits,
        searched: n,
        dead,
        ubiquitous,
        dropped: query.dropped.clone(),
    }
}

/// The line a hit is on, in the case it was written in, shortened so a report
/// stays readable.
///
/// **Read back from the file rather than taken from `lower`.** The corpus holds
/// one lowercased copy of five million words and that is the right trade for
/// searching, but a line shown to a person is not being searched: `SANE`, `OCR`
/// and `K72` all come back as words nobody wrote, and a heading arrives
/// indistinguishable from a sentence.
///
/// A line number survives lowercasing exactly, since nothing about it moves a
/// newline, so the number is what carries across rather than a byte offset,
/// which would not survive a non-ASCII lowercasing at all.
///
/// The lowercased line is the fallback where the file has since moved or cannot
/// be read. Wrong case is a worse excerpt and no excerpt is a worse report.
fn excerpt(doc: &Doc, at: usize) -> String {
    let n = doc.line_of(at);
    let raw = std::fs::read_to_string(&doc.file)
        .ok()
        .and_then(|text| text.lines().nth(n.saturating_sub(1)).map(str::to_string));
    let owned = raw.unwrap_or_else(|| doc.line_text(at).to_string());
    let line = owned.trim().trim_start_matches(['#', '>', '-', '*', ' ']);
    let mut s: String = line.chars().take(140).collect();
    if line.chars().count() > 140 {
        s.push('…');
    }
    s
}

#[cfg(test)]
mod tests;
