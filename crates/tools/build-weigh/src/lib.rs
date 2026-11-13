//--------------------------------------------------------------------------------------------------
// Copyright (c) 2026                   orgrinrt                 ort@hiisi.digital
// SPDX-License-Identifier: MPL-2.0     https://mozilla.org/MPL/2.0        contact@hiisi.digital
//--------------------------------------------------------------------------------------------------

//! `mock build-weigh <crate> [dev|test|release|wasm] [--build] [--limit <bytes>]`: what building one
//! crate costs in bytes, minutes and memory, read from the files the build leaves in the workspace's
//! target directory.
//!
//! Without `--build` it weighs what is there, so the same directory gives the same numbers every time.
//! With `--build` it first builds the crate the way the profile builds it, times the build and samples
//! the machine's memory in use while it runs, and reports what the build added. The numbers are the
//! files' lengths and not disk blocks. The memory is the machine's, so another process running beside
//! the build is in it; measure on a quiet container.
//!
//! A tool that reports and never gates: a limit is asked for on the command line and the report says
//! whether the crate's own files are over it, and nothing blocks on that.

mod measure;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use mockspace::tool::{ArgSpec, Purpose, Tool, ToolContext, ToolReport};

use crate::measure::{Own, Sizes, dir_bytes, json_string, mem_used_kib, own_sizes, profile_sizes};

/// How the build is run, and where cargo leaves what it builds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// `cargo build`, the dev profile.
    Dev,
    /// `cargo test --no-run`, the test binaries a suite runs.
    Test,
    /// `cargo build --release`.
    Release,
    /// The page build's module: release, for `wasm32-unknown-unknown`.
    Wasm,
}

impl Profile {
    const ALL: &'static str = "dev, test, release, wasm";

    fn named(name: &str) -> Option<Self> {
        match name {
            "dev" => Some(Self::Dev),
            "test" => Some(Self::Test),
            "release" => Some(Self::Release),
            "wasm" => Some(Self::Wasm),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Test => "test",
            Self::Release => "release",
            Self::Wasm => "wasm",
        }
    }

    /// The profile's directory under the target directory.
    pub fn dir(self, target: &Path) -> PathBuf {
        match self {
            Self::Dev | Self::Test => target.join("debug"),
            Self::Release => target.join("release"),
            Self::Wasm => target.join("wasm32-unknown-unknown").join("release"),
        }
    }

    fn cargo_args(self, crate_name: &str) -> Vec<String> {
        let mut a: Vec<&str> = match self {
            Self::Dev => vec!["build"],
            Self::Test => vec!["test", "--no-run"],
            Self::Release => vec!["build", "--release"],
            Self::Wasm => vec!["build", "--release", "--target", "wasm32-unknown-unknown"],
        };
        a.extend(["-p", crate_name]);
        a.into_iter().map(String::from).collect()
    }
}

/// What was asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub crate_name: String,
    pub profile:    Profile,
    pub build:      bool,
    pub limit:      Option<u64>,
}

/// What a build cost, measured around it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Built {
    pub wall:           Duration,
    pub added:          u64,
    pub peak_mem_kib:   Option<u64>,
    pub mem_before_kib: Option<u64>,
}

/// Everything a run found.
#[derive(Clone, Debug)]
pub struct Report {
    pub request: Request,
    pub sizes:   Sizes,
    pub own:     Own,
    pub built:   Option<Built>,
}

pub fn parse(args: &[String]) -> Result<Request, String> {
    let mut crate_name = None;
    let mut profile = Profile::Dev;
    let mut build = false;
    let mut limit = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--build" => build = true,
            "--limit" => {
                let v = it.next().ok_or("`--limit` takes a number of bytes")?;
                limit = Some(
                    v.parse::<u64>()
                        .map_err(|_| format!("`--limit {v}`: not a number of bytes"))?,
                );
            },
            flag if flag.starts_with('-') => {
                return Err(format!(
                    "`{flag}` is not a flag here: --build, --limit <bytes>"
                ));
            },
            word if crate_name.is_none() => crate_name = Some(word.to_string()),
            word => {
                profile = Profile::named(word)
                    .ok_or_else(|| format!("`{word}` is not a profile: {}", Profile::ALL))?
            },
        }
    }
    let crate_name = crate_name.ok_or("name a crate: `mock build-weigh <crate> [profile]`")?;
    Ok(Request {
        crate_name,
        profile,
        build,
        limit,
    })
}

/// The crate has to be one the workspace holds, and a near miss names the nearest.
pub fn check_crate(name: &str, have: &BTreeSet<String>) -> Result<(), String> {
    if have.contains(name) {
        return Ok(());
    }
    let nearest = have
        .iter()
        .filter(|c| c.starts_with(&name[.. name.len().saturating_sub(1)]))
        .take(3);
    let near: Vec<&str> = nearest.map(String::as_str).collect();
    Err(format!(
        "`{name}` is not a crate of this workspace{}",
        if near.is_empty() {
            String::new()
        } else {
            format!("; nearest: {}", near.join(", "))
        }
    ))
}

pub fn render(r: &Report) -> String {
    let mut o = String::new();
    let line = |o: &mut String, k: &str, v: String| o.push_str(&format!("{k}={v}\n"));
    line(&mut o, "crate", r.request.crate_name.clone());
    line(&mut o, "profile", r.request.profile.name().to_string());
    if let Some(b) = r.built {
        line(&mut o, "wall_ms", b.wall.as_millis().to_string());
        line(&mut o, "added_bytes", b.added.to_string());
        if let (Some(peak), Some(before)) = (b.peak_mem_kib, b.mem_before_kib) {
            line(
                &mut o,
                "peak_mem_over_before_kib",
                peak.saturating_sub(before).to_string(),
            );
        }
    }
    line(&mut o, "own_artefact_bytes", r.own.artefacts.to_string());
    line(&mut o, "own_artefact_files", r.own.files.to_string());
    line(
        &mut o,
        "own_incremental_bytes",
        r.own.incremental.to_string(),
    );
    line(&mut o, "dir_deps_bytes", r.sizes.deps.to_string());
    line(&mut o, "dir_build_bytes", r.sizes.build.to_string());
    line(
        &mut o,
        "dir_incremental_bytes",
        r.sizes.incremental.to_string(),
    );
    line(
        &mut o,
        "dir_fingerprint_bytes",
        r.sizes.fingerprint.to_string(),
    );
    line(&mut o, "dir_other_bytes", r.sizes.other.to_string());
    line(&mut o, "dir_total_bytes", r.sizes.total().to_string());
    if let Some(limit) = r.request.limit
        && r.own.artefacts + r.own.incremental > limit
    {
        o.push_str(&format!(
            "over the limit of {limit}: the crate's own files are {} bytes\n",
            r.own.artefacts + r.own.incremental
        ));
    }
    o
}

fn target_directory(mock_dir: &Path) -> Result<PathBuf, String> {
    let out = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(mock_dir)
        .output()
        .map_err(|e| format!("could not run `cargo metadata`: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "`cargo metadata` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    json_string(&text, "target_directory")
        .map(PathBuf::from)
        .ok_or_else(|| "`cargo metadata` named no target directory this tool can read".to_string())
}

fn current_mem_used_kib() -> Option<u64> {
    mem_used_kib(&std::fs::read_to_string("/proc/meminfo").ok()?)
}

/// Builds the crate and measures the build: wall time, the bytes it added under the profile directory
/// and the most memory in use the machine showed while it ran.
fn build(mock_dir: &Path, req: &Request, profile_dir: &Path) -> Result<Built, String> {
    let before_bytes = dir_bytes(profile_dir);
    let mem_before = current_mem_used_kib();
    let done = Arc::new(AtomicBool::new(false));
    let sampler = {
        let done = Arc::clone(&done);
        std::thread::spawn(move || {
            let mut peak = None;
            while !done.load(Ordering::Relaxed) {
                peak = peak.max(current_mem_used_kib());
                std::thread::sleep(Duration::from_millis(250));
            }
            peak
        })
    };
    let started = Instant::now();
    let status = Command::new("cargo")
        .args(req.profile.cargo_args(&req.crate_name))
        .current_dir(mock_dir)
        .status();
    let wall = started.elapsed();
    done.store(true, Ordering::Relaxed);
    let peak = sampler.join().ok().flatten();
    let status = status.map_err(|e| format!("could not run cargo: {e}"))?;
    if !status.success() {
        return Err(format!(
            "the build of `{}` failed, so nothing here measures it",
            req.crate_name
        ));
    }
    let after_bytes = dir_bytes(profile_dir);
    Ok(Built {
        wall,
        added: after_bytes.saturating_sub(before_bytes),
        peak_mem_kib: peak,
        mem_before_kib: mem_before,
    })
}

pub struct BuildWeigh;

impl Tool for BuildWeigh {
    fn name(&self) -> &'static str {
        "build-weigh"
    }

    fn description(&self) -> &'static str {
        "what building one crate costs in bytes, minutes and memory, per profile, from the files the build leaves"
    }

    fn purpose(&self) -> Purpose {
        Purpose::Make {
            writes: &["mock/target/**"],
            roots:  &[],
        }
    }

    fn args(&self) -> &'static [ArgSpec] {
        &[
            ArgSpec {
                name:        "crate",
                required:    true,
                description: "the workspace crate to weigh",
            },
            ArgSpec {
                name:        "profile",
                required:    false,
                description: "dev (the default), test, release or wasm",
            },
        ]
    }

    fn value_flags(&self) -> &[&'static str] {
        &["--limit"]
    }

    fn help(&self) -> &'static str {
        "Weighs the files the build of one crate leaves in the target directory: the crate's own artefacts \
         and incremental directories, and the profile directory by kind. With --build it builds the crate \
         first and reports the wall time, the bytes the build added and the most memory in use the machine \
         showed meanwhile. --limit <bytes> says whether the crate's own files are over a size. It reports \
         and never gates. Run it a crate at a time: a whole-workspace build fills a container's disk."
    }

    fn run(&self, ctx: &ToolContext<'_>) -> ToolReport {
        let args: Vec<String> = ctx.args.iter().map(|a| (*a).to_string()).collect();
        let request = match parse(&args) {
            Ok(r) => r,
            Err(e) => return ToolReport::inconclusive(e),
        };
        if let Err(e) = check_crate(&request.crate_name, ctx.all_crates) {
            return ToolReport::inconclusive(e);
        }
        let target = match target_directory(ctx.mock_dir) {
            Ok(t) => t,
            Err(e) => return ToolReport::inconclusive(e),
        };
        let dir = request.profile.dir(&target);
        let built = if request.build {
            match build(ctx.mock_dir, &request, &dir) {
                Ok(b) => Some(b),
                Err(e) => return ToolReport::inconclusive(e),
            }
        } else {
            None
        };
        let own = own_sizes(&dir, &request.crate_name);
        if own.files == 0 {
            return ToolReport::inconclusive(format!(
                "no artefact of `{}` under {}; build it first, or pass --build",
                request.crate_name,
                dir.display()
            ));
        }
        let report = Report {
            sizes: profile_sizes(&dir),
            own,
            built,
            request,
        };
        ToolReport::reported(render(&report), 1)
    }
}

mockspace::lint_pack! {
    tools: [BuildWeigh],
}

#[cfg(test)]
mod tests;
