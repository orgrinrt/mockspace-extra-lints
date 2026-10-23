//! No lint in this pack reads a configuration key the engine keeps for itself.
//!
//! The engine deserialises a `[lints.<name>]` table into a struct with these fields named, and
//! only what is left over reaches a lint's `configure`. A lint reading one of them is configurable
//! from a hand-built map and from nowhere a project can write: an array under a reserved name is
//! taken as the engine's own filter, and a string under one fails the whole table, which the
//! engine then drops without a word.
//!
//! The list is the fields of `LintTableConfig` in mockspace's `src/config.rs`. Two checks: the
//! keys each lint declares through `config_keys`, which is the only place a lint states them, and
//! the keys its source reads from `params`, because the engine never consults `config_keys` and a
//! read that was not declared would pass the first check untouched.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use mockspace_extra_lints::collect;
use mockspace_lint_rules::{Lint, LintPack};

/// The fields of the engine's lint-table struct, which no lint's own key may share.
const RESERVED: &[&str] = &[
    "include",
    "exclude",
    "crates",
    "exempt_crates",
    "severity",
    "commit",
    "build",
    "push",
    "findings",
    "rule",
    "rules",
];

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

#[test]
fn no_declared_config_key_is_one_the_engine_reserves() {
    let all = declared();
    assert!(all.len() > 10, "the enumeration found too little to mean anything: {all:?}");
    let bad: Vec<_> = all.iter().filter(|(_, k)| RESERVED.contains(&k.as_str())).collect();
    assert!(bad.is_empty(), "lints declaring an engine-reserved key: {bad:?}");
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

/// The keys one source file reads with `params.get(<arg>)`: a literal as it stands, and a
/// constant through the `const NAME: &str = "literal";` that gives it, in the same file.
fn read_keys(text: &str) -> BTreeSet<String> {
    let constant = |name: &str| -> Option<String> {
        let head = format!("const {name}: &str = \"");
        let at = text.find(&head)? + head.len();
        Some(text[at..].split('"').next()?.to_string())
    };
    let mut out = BTreeSet::new();
    for (at, _) in text.match_indices("params.get(") {
        let rest = &text[at + "params.get(".len()..];
        let arg = rest.split(')').next().unwrap().trim();
        match arg.strip_prefix('"') {
            Some(lit) => {
                out.insert(lit.trim_end_matches('"').to_string());
            }
            None => out.extend(constant(arg)),
        }
    }
    out
}

#[test]
fn no_key_a_lint_reads_from_params_is_one_the_engine_reserves() {
    let mut files = Vec::new();
    sources(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lints"), &mut files);
    let mut read = BTreeSet::new();
    for f in &files {
        for k in read_keys(&fs::read_to_string(f).unwrap()) {
            read.insert((f.file_name().unwrap().to_string_lossy().to_string(), k));
        }
    }
    assert!(read.len() > 10, "the scan found too little to mean anything: {read:?}");
    let bad: Vec<_> = read.iter().filter(|(_, k)| RESERVED.contains(&k.as_str())).collect();
    assert!(bad.is_empty(), "sources reading an engine-reserved key: {bad:?}");

    let declared: BTreeSet<String> = declared().into_iter().map(|(_, k)| k).collect();
    let undeclared: Vec<_> = read.iter().filter(|(_, k)| !declared.contains(k)).collect();
    assert!(undeclared.is_empty(), "keys read from params and declared by no lint: {undeclared:?}");
}
