//! What a question expands into before it is asked of the corpus.
//!
//! A grep for one word is why the two-surface data store was reported absent
//! from a corpus that describes it in detail: the corpus calls it a broker with
//! two backends, and the question arrived saying surfaces. So a question here is
//! a set of terms rather than a string, and the set comes from three places: the
//! words asked, the forms those words take, and the words the registry already
//! associates with the subject.

/// Where a term came from, which decides what it is worth and lets a report say
/// why it was searched. A reader who cannot see that `conveyance` was searched
/// because they asked about `convey` cannot tell a real hit from a coincidence.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    /// The whole phrase as asked.
    Phrase,
    /// One content word of the phrase.
    Word,
    /// A `keywords` value from a registry row that carries the subject.
    Keyword,
}

impl Source {
    fn weight(self) -> f64 {
        match self {
            Self::Phrase => 3.0,
            Self::Word => 1.5,
            Self::Keyword => 1.0,
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Phrase => "asked",
            Self::Word => "word",
            Self::Keyword => "registry",
        }
    }
}

/// One thing to look for.
pub struct Term {
    /// As it will be shown to a reader.
    pub text:   String,
    /// What is actually matched: the text with a suffix stripped, so one term
    /// finds every form of itself.
    pub stem:   String,
    pub source: Source,
    pub weight: f64,
}

/// Suffixes stripped to reach a stem, longest first.
///
/// Longest first matters: `conveyance` must lose `ance` rather than `e`, or the
/// stem is `conveyanc` and matches nothing else. Ordering by length is the whole
/// of the rule and there is no exception list, because an exception list is a
/// dictionary and this is not one.
///
/// This is deliberately not a real stemmer. A real one conflates `mediate` and
/// `media`, which in this corpus are two subjects, and the cost of being wrong
/// there is a tool that confidently returns the wrong files. Stripping a suffix
/// and matching the remainder as a **prefix** is weaker, explainable, and
/// cannot conflate two words that do not already share a beginning.
/// `ation` is deliberately absent, and its absence is the rule working. It
/// strips cleanly and takes `mediation` to `medi`, while `mediator` loses `or`
/// and reaches `mediat`, so the two forms of one word stop matching each other
/// and the longest-first rule produces the opposite of what it is for. `ion`
/// alone takes both to `mediat`. Every suffix here earns its place by unifying
/// forms rather than by being a suffix.
const SUFFIXES: &[&str] = &[
    "iveness", "ances", "ence", "ance", "ing", "ers", "ors", "ion", "ies", "ed", "es", "er", "or",
    "s",
];

/// The shortest stem worth matching as a prefix.
///
/// Below this a stem matches most of the language. `is` would match `island`
/// and `issue`; `dat` matches `date`, `data` and `datum`, which is fine, and
/// `da` would match `day` and `damage`, which is not. Four is where that turns
/// over, and it is a judgement stated here rather than buried.
const MIN_STEM: usize = 4;

/// Reduce a word to what will be matched as a prefix.
///
/// Returns the word unchanged where no suffix strips cleanly or where stripping
/// would leave too little. **Never returns something shorter than [`MIN_STEM`]
/// unless the word itself is**, so a short word is matched whole rather than
/// becoming a wildcard.
#[must_use]
pub fn stem(word: &str) -> String {
    for suf in SUFFIXES {
        if let Some(base) = word.strip_suffix(suf)
            && base.len() >= MIN_STEM
        {
            return base.to_string();
        }
    }
    word.to_string()
}

/// Words that carry no subject.
///
/// Short on purpose. A long list starts deciding what a subject is, and a word
/// wrongly on it removes that subject from every search silently. These are the
/// ones that appear in most sentences in any corpus.
const STOP: &[&str] = &[
    "the", "a", "an", "and", "or", "of", "to", "in", "is", "it", "that", "this", "for", "on",
    "with", "as", "at", "by", "be", "are", "was", "not", "no", "but", "from", "what", "which",
    "how", "does", "do", "can", "may", "has", "have", "its", "one", "two", "any", "all",
];

/// Lowercase content words. Punctuation breaks, digits and single characters go.
#[must_use]
pub fn words(text: &str) -> Vec<String> {
    split_words(text).0
}

/// The words worth searching, and the words thrown away, in one pass.
///
/// One function returning both halves rather than two applying the same filter
/// in opposite directions, because the second would drift from the first and
/// nothing would report it: a word added to [`STOP`] would silently stop being
/// searched and silently stop being named as dropped.
///
/// **The dropped half is what a caller needs to say a question was never
/// asked.** A question of nothing but these words expands into no terms at all,
/// and a search that asked nothing must not be read as a corpus that answered
/// nothing.
#[must_use]
pub fn split_words(text: &str) -> (Vec<String>, Vec<String>) {
    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    for w in text
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
    {
        if w.len() < 2 || w.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        if STOP.contains(&w) {
            // Once each, in the order they were written. Nothing counts these
            // and a reader deciding how to rephrase wants the words rather than
            // the tally: `how do we do this` naming `do` twice reads as a bug in
            // the report.
            if !dropped.iter().any(|d| d == w) {
                dropped.push(w.to_string());
            }
        } else {
            kept.push(w.to_string());
        }
    }
    (kept, dropped)
}

/// The expanded question.
pub struct Query {
    pub terms:   Vec<Term>,
    /// Words of the question that carry no subject, so nothing was searched for
    /// them.
    ///
    /// **With `terms` empty this is the whole question**, and the search that
    /// follows establishes nothing whatever about the corpus. Carried rather
    /// than discarded so a caller can say which words to replace, which is the
    /// only thing a reader can act on when a question comes back unasked.
    pub dropped: Vec<String>,
}

impl Query {
    /// Start from what was asked.
    ///
    /// The whole phrase is one term and each content word is another. Both,
    /// rather than either: the phrase alone misses a document that discusses the
    /// subject in other words, and the words alone rank a document mentioning
    /// each of them separately above one that names the thing.
    #[must_use]
    pub fn new(phrase: &str) -> Self {
        let (w, dropped) = split_words(phrase);
        let mut q = Self {
            terms: Vec::new(),
            dropped,
        };
        if w.len() > 1 {
            q.push(phrase.trim().to_lowercase(), Source::Phrase);
        }
        for word in w {
            q.push(word, Source::Word);
        }
        q
    }

    /// The phrase and nothing else, for a caller asking whether the corpus
    /// carries this exact thing.
    ///
    /// **[`Query::new`] answers a different question**: it adds every content
    /// word as its own term, so a document carrying one of them scores. That is
    /// right for a subject search, where the point is finding material discussed
    /// under other words, and wrong for a caller whose other half is a substring
    /// match. `already-said quaternion-nonsense-zz` reported the research
    /// carrying it in 306 documents over a corpus holding that string zero
    /// times, because `zz` alone is in 596 files.
    ///
    /// A single word is still stemmed, so `convey` reaches `conveyance`. Only
    /// the splitting goes.
    /// A phrase whose every word is too common still expands to nothing, the
    /// same as through [`Query::new`]. [`push`](Self::push) does not consult the
    /// stop list, so pushing the phrase unconditionally searched for `the` and
    /// answered that the research carries it in 3430 documents, undoing the
    /// unasked-question fix through a second door.
    #[must_use]
    pub fn phrase_only(phrase: &str) -> Self {
        let (kept, dropped) = split_words(phrase);
        let mut q = Self {
            terms: Vec::new(),
            dropped,
        };
        if !kept.is_empty() {
            q.push(phrase.trim().to_lowercase(), Source::Phrase);
        }
        q
    }

    /// Add a term the registry associates with the subject.
    ///
    /// Silently ignored where it is already present under a stronger source, so
    /// a keyword repeating the asked word does not double that word's weight.
    pub fn push(&mut self, text: String, source: Source) {
        let text = text.trim().to_lowercase();
        if text.len() < 2 {
            return;
        }
        let stem = if text.contains(' ') { text.clone() } else { stem(&text) };
        if let Some(existing) = self.terms.iter_mut().find(|t| t.stem == stem) {
            // Keep the stronger provenance and the stronger weight. A term
            // reached two ways is not twice as good; it is the better of the
            // two, and summing here was what made a keyword list of thirty
            // outweigh the question in the first version.
            if source.weight() > existing.weight {
                existing.source = source;
                existing.weight = source.weight();
                existing.text = text;
            }
            return;
        }
        self.terms.push(Term {
            text,
            stem,
            source,
            weight: source.weight(),
        });
    }

    /// Every occurrence of one term in a text, as byte offsets.
    ///
    /// A stem matches at a word boundary and then runs as a prefix, so `mediat`
    /// finds `mediates` and `mediator` and does not find `intermediate`. A term
    /// spanning several tokens is matched literally, because adjacency is its
    /// whole value and is exactly what an index over single tokens throws away.
    ///
    /// The leading boundary is checked for every term, compound ones included:
    /// without it `dm-verity` matches inside `xdm-verity` and the tool reports a
    /// hit on something the corpus does not say.
    #[must_use]
    pub fn find(stem: &str, hay: &str) -> Vec<usize> {
        let mut out = Vec::new();
        let bytes = hay.as_bytes();
        let mut from = 0usize;
        while let Some(rel) = hay[from ..].find(stem) {
            let at = from + rel;
            from = at + 1;
            // A word boundary before it, or the start of the text.
            if at == 0 || !is_word_byte(bytes[at - 1]) {
                out.push(at);
            }
        }
        out
    }
}

/// Whether a term spans more than one of the tokens the index is keyed on.
///
/// The index stores what [`words`] produces, which splits on everything that is
/// not alphanumeric or an underscore. So a term carrying a hyphen, a dot or a
/// space can never equal a key, and looking one up returns nothing however often
/// the corpus says it. Such a term is scanned against the text instead.
///
/// This existed as `stem.contains(' ')` and covered only the space, which made
/// every compound name this project argues about unsearchable: `dm-verity`,
/// `systemd-boot`, `read-only`, `cargo.toml`. Each came back dead, and a dead
/// term is what the verdict reads as evidence that the corpus is silent.
#[must_use]
pub fn is_multi_token(stem: &str) -> bool {
    stem.chars().any(|c| !c.is_alphanumeric() && c != '_')
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}
