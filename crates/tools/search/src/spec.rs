//! What a search asks: the question's words, the filters that narrow it, and the mode.
//!
//! Every narrowing is a flag, and the flags compose: each one given is one more filter every answer
//! has to pass. A flag that takes a value takes it as the next word; a list is comma separated.

use muisti::repo::DocClass;

/// The flags that take the word after them.
pub const VALUE_FLAGS: &[&str] = &[
    "--class",
    "--under",
    "--since",
    "--until",
    "--days",
    "--namespace",
    "--subject",
    "--linked",
    "--limit",
];

/// What a run does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Answer from the index as it stands.
    Ask,
    /// Bring the index up to date: the one writer, which a question starts in the background.
    Refresh,
    /// Say what the index holds and how far behind it is.
    Status,
}

/// A parsed search.
#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    pub mode:      Mode,
    /// The question, its words joined by a space; empty for none.
    pub question:  String,
    pub classes:   Vec<DocClass>,
    pub under:     Option<String>,
    pub since:     Option<i64>,
    pub until:     Option<i64>,
    /// Newest first, by the last day a document changed, rather than by the score.
    pub newest:    bool,
    pub spoken:    bool,
    pub phrase:    bool,
    pub meaning:   bool,
    /// Only documents no registry row cites.
    pub uncited:   bool,
    /// Only rows, no documents.
    pub rows:      bool,
    pub namespace: Vec<String>,
    pub subject:   Option<String>,
    pub linked:    Option<String>,
    pub limit:     usize,
}

/// How many documents a report shows unless `--limit` says otherwise. A reader given sixty files
/// has been handed the problem back.
pub const SHOWN: usize = 5;

impl Spec {
    /// Whether a window of days was given, which a search with no words lists, newest first.
    #[must_use]
    pub const fn windowed(&self) -> bool {
        self.since.is_some() || self.until.is_some()
    }

    /// Whether any flag narrows the registry's rows, which makes a search answer rows.
    #[must_use]
    pub fn narrows_rows(&self) -> bool {
        !self.namespace.is_empty() || self.subject.is_some() || self.linked.is_some()
    }

    /// Read a command line.
    ///
    /// # Errors
    /// One sentence naming the flag and what was wrong with it.
    pub fn parse(args: &[&str]) -> Result<Self, String> {
        let mut spec = Self {
            mode:      Mode::Ask,
            question:  String::new(),
            classes:   Vec::new(),
            under:     None,
            since:     None,
            until:     None,
            newest:    false,
            spoken:    false,
            phrase:    false,
            meaning:   false,
            uncited:   false,
            rows:      false,
            namespace: Vec::new(),
            subject:   None,
            linked:    None,
            limit:     SHOWN,
        };
        let mut words: Vec<&str> = Vec::new();
        let mut it = args.iter().copied();
        while let Some(a) = it.next() {
            if VALUE_FLAGS.contains(&a) {
                let v = it
                    .next()
                    .filter(|v| !v.starts_with("--"))
                    .ok_or_else(|| format!("`{a}` wants a value after it"))?;
                spec.take(a, v)?;
                continue;
            }
            match a {
                "--refresh" => spec.mode = Mode::Refresh,
                "--status" => spec.mode = Mode::Status,
                "--spoken" => spec.spoken = true,
                "--newest" => spec.newest = true,
                "--phrase" => spec.phrase = true,
                "--meaning" => spec.meaning = true,
                "--uncited" => spec.uncited = true,
                "--rows" => spec.rows = true,
                flag if flag.starts_with("--") => {
                    return Err(format!(
                        "`{flag}` is no filter of search; the filters are {}, --spoken, --newest, \
                         --phrase, --meaning, --uncited and --rows",
                        VALUE_FLAGS.join(", ")
                    ));
                },
                word => words.push(word),
            }
        }
        spec.question = words.join(" ");
        Ok(spec)
    }

    fn take(&mut self, flag: &str, v: &str) -> Result<(), String> {
        let list = || v.split(',').map(str::trim).filter(|s| !s.is_empty());
        match flag {
            "--class" => {
                for label in list() {
                    let class = DocClass::from_label(label).ok_or_else(|| {
                        let known: Vec<&str> = DocClass::ALL.iter().map(|c| c.label()).collect();
                        format!(
                            "`{label}` is no class; the classes are {}",
                            known.join(", ")
                        )
                    })?;
                    self.classes.push(class);
                }
            },
            "--under" => self.under = Some(v.trim_matches('/').to_string()),
            "--since" | "--until" => {
                let day = corpus::day::day_of(v)
                    .ok_or_else(|| format!("`{flag}` wants a date as YYYY-MM-DD, not `{v}`"))?;
                if flag == "--since" {
                    if self.since.is_some() {
                        return Err(
                            "`--days` and `--since` both say where the window opens; give \
                                    one"
                            .to_string(),
                        );
                    }
                    self.since = Some(day);
                } else {
                    self.until = Some(day);
                }
            },
            "--days" => {
                if self.since.is_some() {
                    return Err(
                        "`--days` and `--since` both say where the window opens; give one"
                            .to_string(),
                    );
                }
                let n: i64 = v
                    .parse()
                    .ok()
                    .filter(|n| *n >= 0)
                    .ok_or_else(|| format!("`--days` wants a count of days, not `{v}`"))?;
                self.since = Some(corpus::day::today() - n);
            },
            "--namespace" => self.namespace.extend(list().map(str::to_string)),
            "--subject" => self.subject = Some(v.to_string()),
            "--linked" => self.linked = Some(v.to_string()),
            "--limit" => {
                self.limit =
                    v.parse().ok().filter(|n| *n > 0).ok_or_else(|| {
                        format!("`--limit` wants a count above nothing, not `{v}`")
                    })?;
            },
            _ => {},
        }
        Ok(())
    }
}
