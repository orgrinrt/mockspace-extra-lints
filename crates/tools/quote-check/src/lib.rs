//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `mock quote-check <namespace> <source>`: whether the rows of a namespace that quote another
//! repository quote the lines they cite as those lines are, and what of that repository no row holds.
//!
//! A port that copies a document's words into rows keeps a locator beside each quote, the file and the
//! lines and the revision they were read at, and the copy is only as good as its agreement with them.
//! This reads each row's quote and locator, reads the cited lines out of a clone of the source at the
//! row's revision, and compares the two by the ladder in [`compare`], so a quote that differs only in
//! space at a line's end, a blank line closing the range, the separator a port writes between two
//! ranges or a run of tabs is told apart from one whose words differ. A row whose words differ, whose
//! file is gone or whose range runs past its file's end is a finding; the quirks are listed so they can
//! be fixed, and are not.
//!
//! A tool because it takes a question, the clone to read: the source is another repository, which no
//! gate can count on being beside this one.
//!
//! The comparison, the locator and the reading of a source are public, so a project whose rows cite
//! some other kind of place, files of a package registry or one forge's links, writes its own tool on
//! them and registers it in its own pack, and the ladder stays the one this tool reads by.

pub mod compare;
mod coverage;
pub mod locator;
pub mod source;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

use mockspace::LintError;
use mockspace::tool::{ArgSpec, NotALint, Outcome, Purpose, Tool, ToolContext, ToolReport};

use crate::compare::Verdict;
use crate::locator::Cited;
use crate::source::Git;

/// How a run reads the rows: which fields hold the quote, the locator and the revision, the base a
/// bare file name sits under, the line between two joined ranges, and whether coverage is measured.
struct Options<'a> {
    namespace: &'a str,
    source:    &'a str,
    quote:     &'a str,
    doc:       &'a str,
    rev:       &'a str,
    base:      &'a str,
    separator: &'a str,
    coverage:  bool,
    within:    &'a str,
}

impl<'a> Options<'a> {
    fn read(args: &[&'a str]) -> Result<Self, String> {
        let mut o = Self {
            namespace: "",
            source:    "",
            quote:     "quote",
            doc:       "doc",
            rev:       "rev",
            base:      "",
            separator: "---",
            coverage:  false,
            within:    "",
        };
        let mut positional = Vec::new();
        let mut it = args.iter().copied();
        while let Some(a) = it.next() {
            let mut value = |name: &str| it.next().ok_or_else(|| format!("`{name}` wants a value"));
            match a {
                "--quote" => o.quote = value(a)?,
                "--doc" => o.doc = value(a)?,
                "--rev" => o.rev = value(a)?,
                "--base" => o.base = value(a)?,
                "--separator" => o.separator = value(a)?,
                "--within" => o.within = value(a)?,
                "--coverage" => o.coverage = true,
                flag if flag.starts_with("--") => {
                    return Err(format!("`{flag}` is not a flag of `quote-check`"));
                },
                plain => positional.push(plain),
            }
        }
        match positional.as_slice() {
            [ns, src] => {
                o.namespace = ns;
                o.source = src;
                Ok(o)
            },
            _ => {
                Err("`quote-check` wants a namespace and the path of the source's clone".to_owned())
            },
        }
    }
}

pub struct QuoteCheck;

impl Tool for QuoteCheck {
    fn name(&self) -> &'static str {
        "quote-check"
    }

    fn description(&self) -> &'static str {
        "whether rows quoting another repository quote the lines they cite as they are, and what no row holds"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Check(NotALint::TakesAQuestion)
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[
            ArgSpec {
                name:        "namespace",
                required:    true,
                description: "the namespace whose rows quote",
            },
            ArgSpec {
                name:        "source",
                required:    true,
                description: "the path of a clone of the repository they quote",
            },
        ]
    }

    fn help(&self) -> &'static str {
        "Each row of the namespace carrying a quote is compared with the lines its locator cites, read out of the \
         clone at the row's revision: `<path>:<first>-<last>`, several joined by `, `, a path led by `<branch>:` \
         read at the row's revision all the same, and a bare file name under `--base <dir>`. The verdicts, least \
         difference first: exact; trailing, equal once space and a carriage return at line ends are dropped; \
         edge-blank, once blank lines opening and closing the range are dropped too; joined, several ranges \
         equal once the `--separator` line (`---` by default) and blank lines are dropped; spacing, once every \
         run of white space is one space; and the findings, differs (with the first line whose words differ), \
         no-file, past-end and locator. `--quote`, `--doc` and `--rev` name the fields when they are not \
         `quote`, `doc` and `rev`; a row with no revision is read at its locator's branch, else at HEAD. \
         `--coverage` adds which cited files have lines carrying words that no range covers, and which text \
         files of the cited revisions no row cites, under `--within <prefix>` where one is given."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let o = match Options::read(ctx.args) {
            Ok(o) => o,
            Err(why) => return ToolReport::inconclusive(why),
        };
        let root = Path::new(o.source);
        let root = if root.is_absolute() { root.to_path_buf() } else { ctx.repo_root.join(root) };
        if !Git::readable(&root) {
            return ToolReport::inconclusive(format!(
                "`{}` is not a clone git can read",
                root.display()
            ));
        }
        let mut source = Git::at(&root);
        let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
        let mut quirks = String::new();
        let mut findings = Vec::new();
        let mut cited_all = Vec::new();
        let mut examined = 0;
        let mut bare_missing = 0;
        for q in ctx.registry.rows_in(o.namespace) {
            let Some(row) = ctx.registry.row(q) else { continue };
            let Some(quote) = row.get(o.quote) else { continue };
            examined += 1;
            let doc = row.get(o.doc).map_or("", String::as_str);
            let verdict = match locator::parse(doc, o.base) {
                Err(e) => Verdict::Locator(e),
                Ok(cited) => {
                    let rev = row
                        .get(o.rev)
                        .map(String::as_str)
                        .or_else(|| cited.first().and_then(|c| c.branch.as_deref()))
                        .unwrap_or("HEAD")
                        .to_owned();
                    cited_all.extend(cited.iter().map(|c| (rev.clone(), c.path.clone(), c.lines)));
                    let v = compare::compare(&mut source, &rev, &cited, quote, o.separator);
                    if o.base.is_empty()
                        && matches!(v, Verdict::NoFile(_))
                        && cited.iter().any(Cited::is_bare)
                    {
                        bare_missing += 1;
                    }
                    v
                },
            };
            *counts.entry(verdict.name()).or_default() += 1;
            if verdict == Verdict::Exact {
                continue;
            }
            if verdict.faithful() {
                let _ = writeln!(quirks, "  {q}  {}  {doc}", verdict.name());
            } else {
                findings.push(LintError::error(
                    "quote-check".to_owned(),
                    0,
                    "quote-check",
                    finding(q, doc, &verdict),
                ));
            }
        }
        if examined == 0 {
            return ToolReport::inconclusive(format!(
                "no row of `{}` carries a `{}` field",
                o.namespace, o.quote
            ));
        }
        if let Some(why) = unbased(o.base, bare_missing) {
            return ToolReport::inconclusive(why);
        }
        let mut output = format!("{examined} rows of `{}` quote:", o.namespace);
        for (name, n) in &counts {
            let _ = write!(output, " {n} {name},");
        }
        output.pop();
        output.push('\n');
        if !quirks.is_empty() {
            let _ = write!(output, "faithful, with a quirk of space to fix:\n{quirks}");
        }
        if o.coverage {
            output.push_str(&coverage_report(&coverage::measure(
                &mut source,
                &cited_all,
                o.within,
            )));
        }
        if findings.is_empty() {
            ToolReport::reported(output, examined)
        } else {
            ToolReport {
                outcome: Outcome::Findings(findings),
                output,
            }
        }
    }
}

/// The refusal of a run given no `--base` in which `missing` rows cite a bare file name the clone's
/// root does not hold: their documents sit under a directory the run was not told, so each would read as
/// a false no-file finding.
pub(crate) fn unbased(base: &str, missing: usize) -> Option<String> {
    (base.is_empty() && missing > 0).then(|| {
        format!(
            "{missing} rows cite a bare file name the clone's root does not hold; name the directory those \
             documents sit under with `--base <dir>`"
        )
    })
}

fn finding(q: &str, doc: &str, verdict: &Verdict) -> String {
    match verdict {
        Verdict::Differs {
            at,
            cited,
            quoted,
        } => {
            format!(
                "`{q}` quotes `{doc}` with different words at its line {at} that carries words: the source has `{cited}`, the quote `{quoted}`"
            )
        },
        Verdict::NoFile(path) => format!("`{q}` cites `{path}`, which its revision does not hold"),
        Verdict::PastEnd {
            path,
            last,
            lines,
        } => {
            format!("`{q}` cites line {last} of `{path}`, which has {lines} lines at its revision")
        },
        Verdict::Locator(e) => format!("`{q}` has a locator that cannot be read: {e:?}"),
        other => format!("`{q}` is {}", other.name()),
    }
}

fn coverage_report(c: &coverage::Coverage) -> String {
    let mut out = format!(
        "coverage: {} cited files held whole, {} with lines no row holds, {} uncited\n",
        c.whole,
        c.partial.len(),
        c.uncited.len()
    );
    for (path, open, of) in &c.partial {
        let _ = writeln!(
            out,
            "  partial  {path}  {open} of {of} lines that carry words"
        );
    }
    for (path, n) in &c.uncited {
        let _ = writeln!(out, "  uncited  {path}  {n} lines");
    }
    out
}

mockspace::lint_pack! {
    tools: [QuoteCheck],
}
