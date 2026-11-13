//! An inverted index over the corpus, so a question costs a lookup.
//!
//! Scanning the whole corpus per term is fine for one question and is not fine
//! for fifteen. `tackle` scores every declared topic against the research, which
//! is fifteen questions of about twenty-five terms each: rescanning forty
//! megabytes for every one of them is several hundred passes over the text and
//! turns a tool somebody runs into one they do not.
//!
//! So the text is tokenised once and the postings answer every question after
//! that. Building it costs one pass; a query costs a range lookup.

use std::collections::BTreeMap;

use crate::corpus::Corpus;

/// What one term found in one document.
#[derive(Clone, Copy)]
pub struct Posting {
    pub doc:      usize,
    /// Occurrences, each weighted by where it sat.
    pub weighted: f64,
    /// The offset of the best-placed occurrence, for the excerpt.
    pub best:     usize,
}

/// Every token in the corpus, and where it is.
///
/// Keyed on the whole token rather than on a stem, because a stem is a property
/// of a **question** and the same text is asked different questions. Prefix
/// lookup over an ordered map recovers the stem match: `mediat` reaches
/// `mediation` and `mediator` by walking the range that starts with it, and
/// stops before `medicine` because the map is ordered.
pub struct Index {
    tokens: BTreeMap<String, Vec<Posting>>,
}

impl Index {
    /// Tokenise the whole corpus.
    #[must_use]
    pub fn build(corpus: &Corpus) -> Self {
        let mut tokens: BTreeMap<String, Vec<Posting>> = BTreeMap::new();
        for (i, doc) in corpus.docs.iter().enumerate() {
            // Per document, so a term occurring forty times becomes one posting
            // rather than forty. Aggregating later would mean sorting millions
            // of entries; aggregating here is a small map per document.
            let mut here: BTreeMap<&str, (f64, usize, f64)> = BTreeMap::new();
            let bytes = doc.lower.as_bytes();
            let mut start: Option<usize> = None;
            for at in 0 ..= bytes.len() {
                let is_word =
                    at < bytes.len() && (bytes[at].is_ascii_alphanumeric() || bytes[at] == b'_');
                match (start, is_word) {
                    (None, true) => start = Some(at),
                    (Some(s), false) => {
                        start = None;
                        if at - s < 2 {
                            continue;
                        }
                        let word = &doc.lower[s .. at];
                        let w = doc.position_weight(s);
                        let e = here.entry(word).or_insert((0.0, s, 0.0));
                        e.0 += w;
                        if w > e.2 {
                            e.2 = w;
                            e.1 = s;
                        }
                    },
                    _ => {},
                }
            }
            for (word, (weighted, best, _)) in here {
                tokens.entry(word.to_string()).or_default().push(Posting {
                    doc: i,
                    weighted,
                    best,
                });
            }
        }
        Self {
            tokens,
        }
    }

    /// Every posting for tokens beginning with this stem, merged per document.
    ///
    /// Merged rather than concatenated: `mediat` reaching both `mediation` and
    /// `mediator` in one document is one document carrying the subject twice,
    /// not two documents, and a caller counting postings would double the
    /// document frequency and halve the term's apparent rarity.
    #[must_use]
    pub fn postings(&self, stem: &str) -> Vec<Posting> {
        let mut merged: BTreeMap<usize, (f64, usize)> = BTreeMap::new();
        for (_, list) in self
            .tokens
            .range(stem.to_string() ..)
            .take_while(|(k, _)| k.starts_with(stem))
        {
            for p in list {
                let e = merged.entry(p.doc).or_insert((0.0, p.best));
                e.0 += p.weighted;
            }
        }
        merged
            .into_iter()
            .map(|(doc, (weighted, best))| {
                Posting {
                    doc,
                    weighted,
                    best,
                }
            })
            .collect()
    }

    /// How many distinct tokens were indexed. A zero means every query over this
    /// index is void and the caller says so rather than reporting no hits.
    #[must_use]
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }
}
