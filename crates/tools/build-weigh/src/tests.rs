use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::measure::*;

/// A scratch directory per test, under the system's temp directory, removed when it drops.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("build-weigh-{}-{n}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Scratch(p)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn put(root: &Path, rel: &str, bytes: usize) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, vec![b'x'; bytes]).unwrap();
}

/// A profile directory shaped like cargo's: two crates' artefacts in `deps`, a build script output, an
/// incremental directory per crate, a fingerprint, and a final binary beside them.
fn fixture(name: &str) -> Scratch {
    let s = Scratch::new(name);
    put(&s.0, "deps/libkaski_sim-0123456789abcdef.rlib", 1000);
    put(&s.0, "deps/libkaski_sim-0123456789abcdef.rmeta", 200);
    put(&s.0, "deps/kaski_sim-0123456789abcdef.d", 30);
    put(&s.0, "deps/libkaski_sim_extra-fedcba9876543210.rlib", 5000);
    put(&s.0, "deps/libserde-aaaaaaaaaaaaaaaa.rlib", 7000);
    put(&s.0, "build/serde-bbbbbbbbbbbbbbbb/out", 400);
    put(&s.0, "incremental/kaski_sim-1abc/s-x/dep-graph.bin", 600);
    put(&s.0, "incremental/serde-2def/s-y/dep-graph.bin", 800);
    put(&s.0, ".fingerprint/kaski-sim-0123/lib", 50);
    put(&s.0, "kaski-sim", 90);
    s
}

#[test]
fn a_profile_directory_is_split_by_what_cargo_keeps_in_each_entry() {
    let s = fixture("split");
    let z = profile_sizes(&s.0);
    assert_eq!(z.deps, 1000 + 200 + 30 + 5000 + 7000);
    assert_eq!(z.build, 400);
    assert_eq!(z.incremental, 600 + 800);
    assert_eq!(z.fingerprint, 50);
    assert_eq!(z.other, 90);
    assert_eq!(z.total(), 13_230 + 400 + 1_400 + 50 + 90);
}

#[test]
fn the_same_directory_weighed_twice_gives_the_same_numbers() {
    let s = fixture("twice");
    assert_eq!(profile_sizes(&s.0), profile_sizes(&s.0));
    assert_eq!(own_sizes(&s.0, "kaski-sim"), own_sizes(&s.0, "kaski-sim"));
}

#[test]
fn a_crate_owns_its_own_artefacts_and_not_a_longer_named_one() {
    let s = fixture("own");
    let own = own_sizes(&s.0, "kaski-sim");
    assert_eq!(own.artefacts, 1000 + 200 + 30);
    assert_eq!(own.files, 3);
    assert_eq!(own.incremental, 600);
    let extra = own_sizes(&s.0, "kaski-sim-extra");
    assert_eq!(extra.artefacts, 5000);
}

#[test]
fn an_artefact_name_needs_the_hash_after_the_crate_name() {
    assert!(is_own_artefact(
        "libkaski_sim-0123456789abcdef.rlib",
        "kaski-sim"
    ));
    assert!(is_own_artefact("kaski_sim-0123456789abcdef", "kaski-sim"));
    assert!(is_own_artefact("kaski_sim-0123456789abcdef.d", "kaski-sim"));
    assert!(!is_own_artefact(
        "libkaski_sim_extra-0123456789abcdef.rlib",
        "kaski-sim"
    ));
    assert!(!is_own_artefact("libkaski_sim-xyz.rlib", "kaski-sim"));
    assert!(!is_own_artefact("libkaski_sim.rlib", "kaski-sim"));
}

#[test]
fn a_missing_directory_weighs_nothing_rather_than_failing() {
    let s = Scratch::new("missing");
    let gone = s.0.join("nope");
    assert_eq!(dir_bytes(&gone), 0);
    assert_eq!(profile_sizes(&gone), Sizes::default());
}

#[test]
fn memory_in_use_is_the_total_less_what_is_available() {
    let m =
        "MemTotal:       16000000 kB\nMemFree:         1000000 kB\nMemAvailable:    6000000 kB\n";
    assert_eq!(mem_used_kib(m), Some(10_000_000));
    assert_eq!(mem_used_kib("MemTotal: 5 kB\n"), None);
    assert_eq!(mem_used_kib(""), None);
}

#[test]
fn a_string_is_read_out_of_cargos_metadata() {
    let j = r#"{"packages":[],"target_directory":"/home/user/target","version":1}"#;
    assert_eq!(
        json_string(j, "target_directory").as_deref(),
        Some("/home/user/target")
    );
    assert_eq!(json_string(j, "missing"), None);
}

// -----------------------------------------------------------------------
// The request, the refusal and the report
// -----------------------------------------------------------------------

use std::collections::BTreeSet;

use crate::{Profile, Report, Request, check_crate, parse, render};

fn args(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn a_request_names_a_crate_and_defaults_to_the_dev_profile() {
    let r = parse(&args(&["kaski-sim"])).unwrap();
    assert_eq!(r.crate_name, "kaski-sim");
    assert_eq!(r.profile, Profile::Dev);
    assert!(!r.build);
    assert_eq!(r.limit, None);
}

#[test]
fn flags_and_a_profile_are_read_in_any_order() {
    let r = parse(&args(&[
        "--limit",
        "5000",
        "kaski-sim",
        "--build",
        "release",
    ]))
    .unwrap();
    assert_eq!(r.crate_name, "kaski-sim");
    assert_eq!(r.profile, Profile::Release);
    assert!(r.build);
    assert_eq!(r.limit, Some(5000));
}

#[test]
fn a_profile_that_is_not_one_is_refused_by_name() {
    let e = parse(&args(&["kaski-sim", "turbo"])).unwrap_err();
    assert!(e.contains("turbo"), "{e}");
    assert!(e.contains("wasm"), "the profiles are listed: {e}");
}

#[test]
fn a_limit_that_is_not_a_number_is_refused() {
    let e = parse(&args(&["kaski-sim", "--limit", "big"])).unwrap_err();
    assert!(e.contains("big"), "{e}");
    assert!(parse(&args(&["--limit"])).is_err());
    assert!(parse(&args(&[])).is_err());
}

#[test]
fn a_crate_that_is_not_in_the_workspace_is_refused() {
    let have: BTreeSet<String> = ["kaski-sim", "kaski-noise"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    assert!(check_crate("kaski-sim", &have).is_ok());
    let e = check_crate("kaski-simm", &have).unwrap_err();
    assert!(e.contains("kaski-simm"), "{e}");
    assert!(e.contains("kaski-sim"), "it names the nearest: {e}");
}

#[test]
fn each_profile_reads_the_directory_cargo_writes_it_to() {
    let t = Path::new("/t");
    assert_eq!(Profile::Dev.dir(t), Path::new("/t/debug"));
    assert_eq!(Profile::Test.dir(t), Path::new("/t/debug"));
    assert_eq!(Profile::Release.dir(t), Path::new("/t/release"));
    assert_eq!(
        Profile::Wasm.dir(t),
        Path::new("/t/wasm32-unknown-unknown/release")
    );
}

fn report(own_bytes: u64) -> Report {
    let s = fixture("report");
    Report {
        request: Request {
            crate_name: "kaski-sim".into(),
            profile:    Profile::Dev,
            build:      false,
            limit:      None,
        },
        sizes:   profile_sizes(&s.0),
        own:     Own {
            artefacts:   own_bytes,
            files:       3,
            incremental: 0,
        },
        built:   None,
    }
}

#[test]
fn a_crate_over_its_limit_is_reported_as_over_and_one_under_is_not() {
    let mut over = report(2_000);
    over.request.limit = Some(1_000);
    let text = render(&over);
    assert!(text.contains("over the limit of 1000"), "{text}");
    let mut under = report(500);
    under.request.limit = Some(1_000);
    assert!(
        !render(&under).contains("over the limit"),
        "{}",
        render(&under)
    );
    assert!(
        !render(&report(2_000)).contains("limit"),
        "no limit asked, none reported"
    );
}

#[test]
fn the_report_prints_the_same_text_for_the_same_numbers() {
    assert_eq!(render(&report(1_230)), render(&report(1_230)));
    let text = render(&report(1_230));
    assert!(text.contains("own_artefact_bytes=1230"), "{text}");
    assert!(text.contains("profile=dev"), "{text}");
    assert!(
        !text.contains("wall_ms"),
        "a run that built nothing has no time: {text}"
    );
}
