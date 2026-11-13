//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! Reading what a build left in a target directory.
//!
//! Everything here reads files and takes a decision already made, so a run twice over the same
//! directory gives the same numbers. Sizes are the files' lengths and not the disk blocks they
//! occupy, which a filesystem rounds up and a copy-on-write one shares.

use std::path::Path;

/// The bytes under each top-level entry of a profile directory.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Sizes {
    pub deps:        u64,
    pub build:       u64,
    pub incremental: u64,
    pub fingerprint: u64,
    pub other:       u64,
}

impl Sizes {
    pub fn total(&self) -> u64 {
        self.deps + self.build + self.incremental + self.fingerprint + self.other
    }
}

/// What one crate's own files weigh.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Own {
    pub artefacts:   u64,
    pub files:       usize,
    pub incremental: u64,
}

/// The bytes in every regular file under `path`, symlinks not followed. A path that is not there
/// weighs nothing, since a profile that was never built has no directory.
pub fn dir_bytes(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|e| dir_bytes(&e.path()))
        .sum()
}

/// A profile directory split by the entry cargo keeps each kind of file in.
pub fn profile_sizes(profile_dir: &Path) -> Sizes {
    let mut z = Sizes::default();
    let Ok(entries) = std::fs::read_dir(profile_dir) else {
        return z;
    };
    for e in entries.filter_map(Result::ok) {
        let bytes = dir_bytes(&e.path());
        match e.file_name().to_string_lossy().as_ref() {
            "deps" => z.deps += bytes,
            "build" => z.build += bytes,
            "incremental" => z.incremental += bytes,
            ".fingerprint" => z.fingerprint += bytes,
            _ => z.other += bytes,
        }
    }
    z
}

/// The files in `deps` that are the crate's own, and its incremental directories.
pub fn own_sizes(profile_dir: &Path, crate_name: &str) -> Own {
    let mut own = Own::default();
    if let Ok(entries) = std::fs::read_dir(profile_dir.join("deps")) {
        for e in entries.filter_map(Result::ok) {
            if is_own_artefact(&e.file_name().to_string_lossy(), crate_name) {
                own.artefacts += dir_bytes(&e.path());
                own.files += 1;
            }
        }
    }
    let stem = crate_name.replace('-', "_");
    if let Ok(entries) = std::fs::read_dir(profile_dir.join("incremental")) {
        for e in entries.filter_map(Result::ok) {
            let name = e.file_name().to_string_lossy().into_owned();
            if name
                .strip_prefix(&stem)
                .is_some_and(|rest| rest.starts_with('-'))
            {
                own.incremental += dir_bytes(&e.path());
            }
        }
    }
    own
}

/// Whether a file in `deps` is `crate_name`'s: `[lib]<crate_name with underscores>-<16 hex>[.ext]`.
/// The hash after the name is what tells `kaski-sim` from `kaski-sim-extra`, whose own name continues
/// where this one's hash would start.
pub fn is_own_artefact(file_name: &str, crate_name: &str) -> bool {
    let stem = crate_name.replace('-', "_");
    let rest = file_name.strip_prefix("lib").unwrap_or(file_name);
    let Some(rest) = rest
        .strip_prefix(stem.as_str())
        .and_then(|r| r.strip_prefix('-'))
    else {
        return false;
    };
    let (hash, tail) = rest.split_at(rest.len().min(16));
    hash.len() == 16
        && hash.bytes().all(|b| b.is_ascii_hexdigit())
        && (tail.is_empty() || tail.starts_with('.'))
}

/// Memory in use, in KiB, from the text of `/proc/meminfo`: the total less what is available.
pub fn mem_used_kib(meminfo: &str) -> Option<u64> {
    let field = |name: &str| {
        meminfo
            .lines()
            .find_map(|l| l.strip_prefix(name)?.strip_prefix(':'))?
            .split_whitespace()
            .next()?
            .parse::<u64>()
            .ok()
    };
    field("MemTotal")?.checked_sub(field("MemAvailable")?)
}

/// The string value of `"key":"..."` in a JSON text, without unescaping: enough for a path that has
/// none, and `None` for a value that does.
pub fn json_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":\"");
    let start = json.find(&needle)? + needle.len();
    let end = start + json[start ..].find('"')?;
    let value = &json[start .. end];
    (!value.contains('\\')).then(|| value.to_string())
}
