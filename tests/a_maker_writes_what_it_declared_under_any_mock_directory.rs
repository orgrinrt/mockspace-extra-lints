//! A maker run the way `cargo mock <tool>` runs one: the engine checks its arguments and what it
//! wrote against the writes it declared, through `maker_faults`, after the run. The pack's makers
//! declare their writes as fixed patterns under `mock/` and `content/`, and the engine has no way yet
//! to say "under the mock directory" in a declaration, so in a consumer whose mock directory or
//! content directory has another name the engine refuses what the maker rightly wrote. Those cases
//! are catalogued here, ignored until the engine carries the placeholder; the control beside them is
//! the same audit holding in a consumer that keeps `mock/`, so the harness is shown to pass where the
//! declaration is right.

use std::fs;
use std::path::Path;
use std::process::Command;

use mockspace_lint_rules::tool::maker_faults;

mod consumer;
use consumer::{consumer, git, pack_for, run, write};

const MARK: &str = concat!("FIX", "ME");

/// Runs the maker `name` over `root` and returns what the engine's audit says of the run: the
/// consumer is committed first, so what `git status` names afterwards is what the run wrote.
fn audited(root: &Path, name: &str, args: &[&str]) -> Vec<String> {
    git(root, &["add", "."]);
    git(root, &[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "-c",
        "commit.gpgsign=false",
        "-c",
        "core.hooksPath=/dev/null",
        "commit",
        "-q",
        "--allow-empty",
        "-m",
        "before the run",
    ]);
    let pack = pack_for(Some(root));
    let report = run(&pack, root, name, args);
    let status = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain", "--untracked-files=all"])
        .output()
        .unwrap();
    let written: Vec<String> = String::from_utf8_lossy(&status.stdout)
        .lines()
        .filter_map(|l| l.get(3..))
        .map(str::to_owned)
        .collect();
    assert!(!written.is_empty(), "the run wrote nothing: {:?} {}", report.outcome, report.output);
    let tool = pack.tools.iter().find(|t| t.name() == name).unwrap();
    maker_faults(tool.as_ref(), &report, args, &written)
}

#[test]
fn the_control_a_maker_in_a_consumer_keeping_mock_writes_what_it_declared() {
    let root = consumer("audit-mock-unbundle", "mock", "");
    write(&root, "bundle.txt", "=== FILE: NOTES.md ===\n# Fire\n");
    let faults = audited(&root, "research-unbundle", &["bundle.txt", "mock/research/202611130000_fire"]);
    assert!(faults.is_empty(), "{faults:?}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
#[ignore = "catalogue: a maker's writes cannot name the mock directory; tracked #a_maker_tool_s_writes_cannot_name_the_mock_directory"]
fn research_unbundle_under_another_mock_directory_writes_what_it_declared() {
    let root = consumer("audit-linux-unbundle", "linux", "");
    write(&root, "bundle.txt", "=== FILE: NOTES.md ===\n# Fire\n");
    let faults = audited(&root, "research-unbundle", &["bundle.txt", "linux/research/202611130000_fire"]);
    assert!(faults.is_empty(), "{faults:?}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
#[ignore = "catalogue: a maker's writes cannot name the mock directory; tracked #a_maker_tool_s_writes_cannot_name_the_mock_directory"]
fn owed_under_another_mock_directory_writes_what_it_declared() {
    let root = consumer("audit-linux-owed", "linux", "[tools.owed]\nholder = \"coord\"\n");
    write(&root, "linux/crates/alpha/src/lib.rs", &format!("// {MARK}: stand-in roof\n"));
    let faults = audited(&root, "owed", &["--no-fetch"]);
    assert!(faults.is_empty(), "{faults:?}");
    let _ = fs::remove_dir_all(&root);
}

#[test]
#[ignore = "catalogue: a maker's writes cannot name a content directory; tracked #a_maker_tool_s_writes_cannot_name_the_mock_directory"]
fn owed_rewriting_a_marker_in_another_content_directory_writes_what_it_declared() {
    let root = consumer("audit-packs-owed", "mock", "[tools.owed]\ncontent_dirs = [\"packs\"]\n");
    write(&root, "packs/fire.toml", &format!("# {MARK}: stand-in fire\nkind = \"fire\"\n"));
    let faults = audited(&root, "owed", &["--no-fetch"]);
    assert!(faults.is_empty(), "{faults:?}");
    let _ = fs::remove_dir_all(&root);
}
