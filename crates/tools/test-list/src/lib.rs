//! Every test of every crate, read from the source.
//!
//! `test-list` prints one line per test: its crate, its file and line, its name, whether the file
//! sits under the crate's `src/`, under its `tests/` or elsewhere, and what its `#[ignore]` says. It
//! builds nothing, so the list is there before any test binary is and costs a read of the tree.
//!
//! A tool because a person and an audit read it. It has no failing case of its own: where a test may
//! sit is a lint's to refuse.
//!
//! What it reads as a test is a function whose attributes, written straight above it, hold `#[test]`
//! or an attribute whose path ends in `test` (`#[tokio::test]`, `#[wasm_bindgen_test]`). So it lists a
//! test written in the tokens of a macro call, and the template of a test in a macro's definition
//! under the name the definition gives it. A test a macro stamps where it is called has no function
//! in the source and is not listed: the template's line stands for every one of them.

use std::fmt::Write;
use std::path::Path;

use mockspace::tool::{ArgSpec, NotALint, Purpose, Tool, ToolContext, ToolReport};

// Public so the items the lints read and this tool does not are not reported unused.
use registry_lints::tree_walk;

pub mod mask;

#[cfg(test)]
mod tests;

/// Where in its crate a test's file sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Place {
    /// Under `src/`, compiled into the crate's own test build.
    Src,
    /// Under `tests/`, a test target of its own.
    Tests,
    /// Anywhere else in the crate: `benches/`, `examples/`, a build script.
    Other,
}

impl Place {
    /// The place of a file by its path within its crate.
    pub fn of(rel: &Path) -> Self {
        match rel.components().next().and_then(|c| c.as_os_str().to_str()) {
            Some("src") => Self::Src,
            Some("tests") => Self::Tests,
            _ => Self::Other,
        }
    }

    const fn word(self) -> &'static str {
        match self {
            Self::Src => "src",
            Self::Tests => "tests",
            Self::Other => "other",
        }
    }
}

/// One test function as its source declares it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    /// The line of its `fn`, from 1.
    pub line:   usize,
    /// Its name as written; `$name` where a macro's definition names it by a fragment.
    pub name:   String,
    /// What its `#[ignore]` says: nothing where it has none, an empty text for a bare one.
    pub ignore: Option<String>,
}

/// One test with the crate and file it is in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The crate's directory name.
    pub crate_name: String,
    /// The file, relative to the crate.
    pub rel:        String,
    /// Where the file sits.
    pub place:      Place,
    /// The test.
    pub found:      Found,
}

/// An attribute's path and the rest of it.
struct Attribute<'a> {
    path: &'a str,
    rest: &'a str,
}

impl<'a> Attribute<'a> {
    /// Read the inside of `#[...]`.
    fn read(inner: &'a str) -> Self {
        let inner = inner.trim();
        let end = inner
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == ':'))
            .unwrap_or(inner.len());
        Self {
            path: &inner[.. end],
            rest: inner[end ..].trim(),
        }
    }

    fn is_test(&self) -> bool {
        self.path
            .rsplit("::")
            .next()
            .is_some_and(|last| last == "test" || last.ends_with("_test"))
    }

    /// What an `ignore` says, with each run of whitespace as one space.
    fn ignore(&self) -> Option<String> {
        if self.path != "ignore" {
            return None;
        }
        let said = self
            .rest
            .strip_prefix('=')
            .map(str::trim)
            .and_then(|v| v.strip_prefix('"'))
            .and_then(|v| v.strip_suffix('"'))
            .unwrap_or("");
        Some(said.split_whitespace().collect::<Vec<_>>().join(" "))
    }
}

/// Every test function of a source, in order.
pub fn tests_in(source: &str) -> Vec<Found> {
    let masked = mask::masked(source);
    let bytes = masked.as_bytes();
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(
            bytes
                .iter()
                .enumerate()
                .filter(|(_, b)| **b == b'\n')
                .map(|(i, _)| i + 1),
        )
        .collect();
    let line_of = |at: usize| line_starts.partition_point(|&s| s <= at);

    let mut out = Vec::new();
    let mut is_test = false;
    let mut ignore: Option<String> = None;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if let Some((open, close)) = attribute_at(bytes, i) {
            let attribute = Attribute::read(&source[open .. close]);
            is_test |= attribute.is_test();
            ignore = attribute.ignore().or(ignore);
            i = close + 1;
            continue;
        }
        if is_test
            && let Some((at, name)) = function_at(&masked, i) {
                out.push(Found {
                    line:   line_of(at),
                    name:   name.to_string(),
                    ignore: ignore.take(),
                });
            }
        // anything that is not an attribute ends the run of attributes above it
        is_test = false;
        ignore = None;
        i = token_end(bytes, i);
    }
    out
}

/// The inside of an outer attribute that opens at `at`, as the offsets after its `[` and of its `]`.
fn attribute_at(bytes: &[u8], at: usize) -> Option<(usize, usize)> {
    if bytes[at] != b'#' {
        return None;
    }
    let bracket = (at + 1 .. bytes.len()).find(|&j| !bytes[j].is_ascii_whitespace())?;
    if bytes[bracket] != b'[' {
        return None;
    }
    let mut depth = 0usize;
    for (j, b) in bytes.iter().enumerate().skip(bracket) {
        match b {
            b'[' => depth += 1,
            b']' => {
                depth -= 1;
                if depth == 0 {
                    return Some((bracket + 1, j));
                }
            },
            _ => {},
        }
    }
    None
}

/// The function whose head opens at `at`: the offset of its `fn` and its name.
fn function_at(masked: &str, at: usize) -> Option<(usize, &str)> {
    let mut rest = &masked[at ..];
    loop {
        let word_end = rest
            .find(|c: char| !(c.is_alphanumeric() || c == '_'))
            .unwrap_or(rest.len());
        let (word, after) = rest.split_at(word_end);
        match word {
            "fn" => {
                let name = after.trim_start();
                let end = name
                    .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '$'))
                    .unwrap_or(name.len());
                let at = masked.len() - rest.len();
                return (end > 0).then(|| (at, &name[.. end]));
            },
            "pub" => {
                let after = after.trim_start();
                // a visibility's scope, `pub(crate)`
                rest = match after.strip_prefix('(') {
                    Some(scope) => scope.split_once(')')?.1.trim_start(),
                    None => after,
                };
            },
            "async" | "const" | "unsafe" => rest = after.trim_start(),
            _ => return None,
        }
    }
}

/// The offset past the token that opens at `at`: a word whole, anything else one byte.
fn token_end(bytes: &[u8], at: usize) -> usize {
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80;
    if !word(bytes[at]) {
        return at + 1;
    }
    (at .. bytes.len())
        .find(|&j| !word(bytes[j]))
        .unwrap_or(bytes.len())
}

/// Every test of the crates under `crates_dir`, narrowed to `only`, in crate, file and line order.
pub fn list(crates_dir: &Path, only: Option<&str>) -> Vec<Row> {
    let mut rows = Vec::new();
    for (crate_name, dir) in tree_walk::subdirs(crates_dir) {
        if only.is_some_and(|o| o != crate_name) {
            continue;
        }
        for file in tree_walk::with_extension(&dir, "rs", &["target"]) {
            let Ok(text) = std::fs::read_to_string(&file) else {
                continue;
            };
            let rel = file.strip_prefix(&dir).unwrap_or(&file);
            let place = Place::of(rel);
            rows.extend(tests_in(&text).into_iter().map(|found| {
                Row {
                    crate_name: crate_name.clone(),
                    rel: rel.to_string_lossy().into_owned(),
                    place,
                    found,
                }
            }));
        }
    }
    rows
}

/// The list as text: a line per test with its fields tab-separated, then the counts.
pub fn render(rows: &[Row]) -> String {
    let mut out = String::new();
    for row in rows {
        let ignore = match row.found.ignore.as_deref() {
            None => "-".to_string(),
            Some("") => "ignored".to_string(),
            Some(said) => format!("ignored: {said}"),
        };
        let _ = writeln!(
            out,
            "{}\t{}:{}\t{}\t{}\t{ignore}",
            row.crate_name,
            row.rel,
            row.found.line,
            row.found.name,
            row.place.word()
        );
    }
    let count = |place: Place| rows.iter().filter(|r| r.place == place).count();
    let ignored = rows.iter().filter(|r| r.found.ignore.is_some()).count();
    let mut crates: Vec<&str> = rows.iter().map(|r| r.crate_name.as_str()).collect();
    crates.dedup();
    let _ = writeln!(
        out,
        "{} tests in {} crates: {} under src, {} under tests, {} elsewhere; {ignored} ignored",
        rows.len(),
        crates.len(),
        count(Place::Src),
        count(Place::Tests),
        count(Place::Other)
    );
    out
}

/// The list of every test.
pub struct TestList;

impl Tool for TestList {
    fn name(&self) -> &'static str {
        "test-list"
    }

    fn description(&self) -> &'static str {
        "every test of every crate with its file, line, name, place and ignore reason, read from the source"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Check(NotALint::NoFailingCase)
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[ArgSpec {
            name:        "crate",
            required:    false,
            description: "a crate to list, by directory name; every crate when none",
        }]
    }

    fn help(&self) -> &'static str {
        "One line per test, tab-separated: the crate, the file with the line of the test's `fn`, its \
         name, `src`, `tests` or `other` for where the file sits in the crate, and `-` or what its \
         `#[ignore]` says. The last line counts them. It reads the Rust files of each crate under the \
         mock directory's `crates` and builds nothing. A test is a function with `#[test]`, or an \
         attribute whose path ends in `test`, straight above it; one a macro stamps where it is called \
         is listed once, at the template in the macro's definition."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let only = ctx.args.iter().copied().find(|a| !a.starts_with('-'));
        if let Some(name) = only.filter(|name| !ctx.all_crates.contains(*name)) {
            return ToolReport::inconclusive(format!(
                "no crate named `{name}` under the mock directory"
            ));
        }
        let rows = list(&ctx.mock_dir.join("crates"), only);
        ToolReport::reported(render(&rows), rows.len())
    }
}

mockspace::lint_pack! {
    tools: [TestList],
}
