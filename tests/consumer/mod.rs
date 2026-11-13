//! A scratch consumer of the pack, shared by the tests that run the workflow over one: a repository on
//! `dev` whose `mockspace.toml` names the mock directory it keeps, the pack's tools and lints as the
//! engine gets them, configured from that file, and a tool run the way the tool runner runs one.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use mockspace::config::Config;
use mockspace_lint_rules::LintPack;
use mockspace_lint_rules::tool::{ToolContext, ToolReport};

/// The pack's root, where its shipped `registry/` sits.
pub fn pack_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn write(root: &Path, rel: &str, text: &str) {
    let p = root.join(rel);
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, text).unwrap();
}

/// Runs git in `root`, asserting it succeeds.
pub fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git").arg("-C").arg(root).args(args).output().unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A consumer on `dev`, in a directory of its own for `tag`, whose mock directory is `mock`: its config
/// is its own root keys, the shipped declarations as they are and then `tables`, its `[tools]` and
/// `[lints]` tables; its registry holds the shipped ticket vocabulary and planning roles and no row of
/// its own.
pub fn consumer(tag: &str, mock: &str, tables: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("extra-lints-consumer-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let shipped = pack_root().join("registry");
    let declarations = fs::read_to_string(shipped.join("namespaces.toml")).unwrap();
    write(
        &root,
        "mockspace.toml",
        &format!(
            "project_name = \"scratch\"\ncrate_prefix = \"scratch\"\nmock_dir = \"{mock}\"\nsrc_dirs = [\"crates\"]\n\n{declarations}\n{tables}"
        ),
    );
    for f in ["ticket/states.toml", "ticket/kinds.toml", "plan/roles.toml"] {
        write(
            &root,
            &format!("{mock}/registry/{f}"),
            &fs::read_to_string(shipped.join(f)).unwrap(),
        );
    }
    fs::create_dir_all(root.join(mock).join("crates")).unwrap();
    git(&root, &["init", "-q", "-b", "dev"]);
    git(&root, &[
        "-c",
        "user.name=t",
        "-c",
        "user.email=t@t",
        "commit",
        "-q",
        "--allow-empty",
        "-m",
        "base",
    ]);
    root
}

/// The consumer's config, read by the engine's own reader from the mock directory its
/// `mockspace.toml` names.
pub fn config(root: &Path) -> Config {
    let text = fs::read_to_string(root.join("mockspace.toml")).unwrap();
    let mock = text
        .lines()
        .find_map(|l| l.strip_prefix("mock_dir = \""))
        .and_then(|l| l.strip_suffix('"'))
        .expect("the consumer names its mock directory");
    Config::from_dir(&root.join(mock))
}

/// The pack as the engine gets it, its tools and lints configured from `root`'s `[tools]` and
/// `[lints]` the way the engine configures a loaded pack, or unconfigured with no `root`.
pub fn pack_for(root: Option<&Path>) -> LintPack {
    let mut pack = LintPack::default();
    mockspace_extra_lints::collect(&mut pack);
    if let Some(root) = root {
        let cfg = config(root);
        pack.configure_tools(&cfg.tool_params);
        pack.configure_from(&cfg.lint_overrides);
    }
    pack
}

/// Runs the tool registered as `name` over the consumer as it stands now.
pub fn run(pack: &LintPack, root: &Path, name: &str, args: &[&str]) -> ToolReport {
    let cfg = config(root);
    let registry = mockspace::registry::load_registry_for(&cfg).expect("the registry loads");
    let view = mockspace::registry::build_view(&registry, &cfg.registry_namespaces);
    let tool = pack
        .tools
        .iter()
        .find(|t| t.name() == name)
        .unwrap_or_else(|| panic!("the pack registers `{name}`"));
    let crates = BTreeSet::new();
    tool.run(&ToolContext {
        mock_dir: &cfg.mock_dir,
        repo_root: &cfg.repo_root,
        all_crates: &crates,
        src_dirs: &cfg.src_dirs,
        args,
        stdin: None,
        registry: &view,
    })
}
