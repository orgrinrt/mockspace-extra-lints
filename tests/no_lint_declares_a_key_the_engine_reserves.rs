//! No lint in this pack reads a configuration key the engine keeps for itself.
//!
//! The engine deserialises a `[lints.<name>]` table into a struct with some fields named, and
//! only what is left over reaches a lint's `configure`. A lint reading one of those is
//! configurable from a hand-built map and from nowhere a project can write: an array under a
//! reserved name is taken as the engine's own filter, and a string under one either becomes the
//! engine's field or makes the whole table unreadable, which the engine then drops or refuses
//! depending on its version.
//!
//! No list of the reserved names is kept here, since one goes stale the day the engine gains a
//! field. The engine is asked instead: each key a lint declares is written under its lint in a
//! `mockspace.toml`, read through the engine's own config reader, and has to come out the other
//! side in the lint's parameters. Two checks lead to it: the keys each lint declares through
//! `config_keys`, which is the only place a lint states them, and the keys its source reads from
//! `params`, because the engine never consults `config_keys` and a read that was not declared
//! would pass the first check untouched.

use std::collections::BTreeSet;
use std::fs;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};

use mockspace::config::Config;
use mockspace_extra_lints::collect;
use mockspace_lint_rules::{Lint, LintPack};

const PROBE: &str = "probe";

fn declared() -> Vec<(String, String)> {
    let mut pack = LintPack::default();
    collect(&mut pack);
    let mut out = Vec::new();
    let mut take = |l: &dyn Lint| {
        for k in l.config_keys() {
            out.push((l.name().to_string(), (*k).to_string()));
        }
    };
    pack.crate_lints.iter().for_each(|l| take(l.as_ref()));
    pack.workspace_lints.iter().for_each(|l| take(l.as_ref()));
    pack.repo_lints.iter().for_each(|l| take(l.as_ref()));
    pack.message_lints.iter().for_each(|l| take(l.as_ref()));
    out
}

/// What the engine does with `key` set to a string under `[lints.<lint>]`, or why it did not
/// reach the lint's parameters.
fn probe(lint: &str, key: &str) -> Result<(), String> {
    let d = tempfile::tempdir().unwrap();
    fs::write(d.path().join("mockspace.toml"), format!("[lints.\"{lint}\"]\n{key} = \"{PROBE}\"\n"))
        .unwrap();
    let read = catch_unwind(AssertUnwindSafe(|| Config::from_dir(d.path())));
    let Ok(cfg) = read else {
        return Err("the engine refused the table".to_string());
    };
    match cfg.lint_overrides.params.get(lint).and_then(|p| p.get(key)) {
        Some(v) if v == PROBE => Ok(()),
        Some(v) => Err(format!("the engine changed the value to {v:?}")),
        None => Err("the engine dropped the table or took the key for its own".to_string()),
    }
}

#[test]
fn every_declared_config_key_reaches_the_lint_through_the_engines_reader() {
    let all = declared();
    assert!(all.len() > 10, "the enumeration found too little to mean anything: {all:?}");
    let bad: Vec<_> =
        all.iter().filter_map(|(l, k)| probe(l, k).err().map(|why| (l, k, why))).collect();
    assert!(bad.is_empty(), "keys that never reach their lint from `mockspace.toml`: {bad:?}");
}

#[test]
fn the_probe_tells_a_reserved_key_from_a_free_one() {
    // The controls: these are fields of the engine's own table on every version, a string under
    // `include` or `exclude` makes the table unreadable, and a key no engine has a field for is a
    // plain parameter. A probe that passed both kinds, or failed both, would say nothing.
    assert!(probe("any-lint", "a_key_nobody_reserves").is_ok());
    for reserved in ["severity", "include", "exclude"] {
        let got = probe("any-lint", reserved);
        assert!(got.is_err(), "the probe let `{reserved}` through: {got:?}");
    }
}

fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let p = entry.unwrap().path();
        if p.is_dir() {
            sources(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            let name = p.file_name().unwrap().to_string_lossy().to_string();
            if name != "tests.rs" && !name.ends_with("_tests.rs") {
                out.push(p);
            }
        }
    }
}

/// What a scan of source text found: the keys read from `params`, and every `params.get`
/// argument it could not turn into one.
#[derive(Default, Debug)]
struct Scan {
    /// `(file, key)`.
    read: BTreeSet<(String, String)>,
    /// `(file, argument)`, as written in the source.
    unresolved: Vec<(String, String)>,
}

/// The string a `const NAME: &str = "literal";` or `const NAME: &'static str = "literal";` in
/// `text` gives, or `None` for any other shape, an escaped literal included.
fn constant(text: &str, name: &str) -> Option<String> {
    let head = format!("const {name}:");
    for (at, _) in text.match_indices(&head) {
        let rest = text[at + head.len()..].trim_start();
        let Some(rest) = rest.strip_prefix('&') else { continue };
        let rest = rest.trim_start();
        let rest = rest.strip_prefix("'static").map_or(rest, str::trim_start);
        let Some(rest) = rest.strip_prefix("str") else { continue };
        let Some(rest) = rest.trim_start().strip_prefix('=') else { continue };
        let Some(rest) = rest.trim_start().strip_prefix('"') else { continue };
        let value = rest.split('"').next().unwrap_or_default();
        if !value.contains('\\') {
            return Some(value.to_string());
        }
    }
    None
}

/// The keys each source reads with `params.get(<arg>)`: a literal as it stands, and a constant
/// through the `const` that gives it, in the same file. An argument that is neither is recorded
/// as unresolved, never skipped, so a read the scan cannot follow is a failure of the test.
fn scan(files: &[(String, String)]) -> Scan {
    let mut out = Scan::default();
    for (file, text) in files {
        for (at, _) in text.match_indices("params.get(") {
            let rest = &text[at + "params.get(".len()..];
            let arg = rest.split(')').next().unwrap().trim();
            match arg.strip_prefix('"') {
                Some(lit) => {
                    out.read.insert((file.clone(), lit.trim_end_matches('"').to_string()));
                }
                None => match constant(text, arg) {
                    Some(k) => {
                        out.read.insert((file.clone(), k));
                    }
                    None => out.unresolved.push((file.clone(), arg.to_string())),
                },
            }
        }
    }
    out
}

fn fixture(text: &str) -> Vec<(String, String)> {
    vec![("fixture.rs".to_string(), text.to_string())]
}

#[test]
fn a_static_str_constant_is_followed_to_its_literal() {
    let found = scan(&fixture(
        "const K: &'static str = \"exclude\";\nfn c(params: &M) { params.get(K); }\n",
    ));
    assert!(found.unresolved.is_empty(), "{found:?}");
    assert_eq!(
        found.read,
        BTreeSet::from([("fixture.rs".to_string(), "exclude".to_string())]),
        "{found:?}"
    );
    let plain = scan(&fixture("pub const K:  &  str  =  \"x\";\nfn c() { params.get(K); }\n"));
    assert_eq!(plain.read.len(), 1, "{plain:?}");
}

#[test]
fn an_argument_the_scan_cannot_resolve_is_reported_with_its_file() {
    for source in [
        "fn c(params: &M) { params.get(name_from_elsewhere); }\n",
        "fn c(params: &M) { params.get(KEY); }\n",
        "const KEY: String = String::new();\nfn c(params: &M) { params.get(KEY); }\n",
    ] {
        let found = scan(&fixture(source));
        assert!(found.read.is_empty(), "{source}: {found:?}");
        assert_eq!(found.unresolved.len(), 1, "{source}: {found:?}");
        assert_eq!(found.unresolved[0].0, "fixture.rs", "{found:?}");
    }
    let found = scan(&fixture("fn c(params: &M) { params.get(some_name); }\n"));
    assert_eq!(found.unresolved, [("fixture.rs".to_string(), "some_name".to_string())]);
}

#[test]
fn no_key_a_lint_reads_from_params_goes_unchecked_or_is_the_engines() {
    let mut paths = Vec::new();
    sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lints"), &mut paths);
    let files: Vec<(String, String)> = paths
        .iter()
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().to_string(),
                fs::read_to_string(p).unwrap(),
            )
        })
        .collect();
    let found = scan(&files);
    assert!(found.read.len() > 10, "the scan found too little to mean anything: {:?}", found.read);
    assert!(
        found.unresolved.is_empty(),
        "`params.get` arguments the scan cannot resolve to a literal (file, argument): {:?}",
        found.unresolved
    );

    let bad: Vec<_> = found
        .read
        .iter()
        .filter_map(|(f, k)| probe("scanned-lint", k).err().map(|why| (f, k, why)))
        .collect();
    assert!(bad.is_empty(), "sources reading a key that never reaches a lint: {bad:?}");

    let declared: BTreeSet<String> = declared().into_iter().map(|(_, k)| k).collect();
    let undeclared: Vec<_> = found.read.iter().filter(|(_, k)| !declared.contains(k)).collect();
    assert!(undeclared.is_empty(), "keys read from params and declared by no lint: {undeclared:?}");
}
