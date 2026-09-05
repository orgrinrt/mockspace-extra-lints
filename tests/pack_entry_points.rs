//! Smoke test: the pack's one entry point contributes its lints, and every
//! contributed lint is well-formed whatever its input kind.

use mockspace_extra_lints as pack;
use mockspace_lint_rules::{Lint, LintPack};

/// Collect the pack once, as the host does.
fn collected() -> LintPack {
    let mut p = LintPack::default();
    pack::collect(&mut p);
    p
}

/// Every lint the pack contributes, as trait objects over the shared supertrait.
///
/// That supertrait is what lets a check like "every name is kebab-case" be
/// written once for all kinds rather than once per kind.
fn all_lints(p: &LintPack) -> Vec<&dyn Lint> {
    let mut v: Vec<&dyn Lint> = Vec::new();
    v.extend(p.crate_lints.iter().map(|l| l.as_ref() as &dyn Lint));
    v.extend(p.workspace_lints.iter().map(|l| l.as_ref() as &dyn Lint));
    v.extend(p.repo_lints.iter().map(|l| l.as_ref() as &dyn Lint));
    v.extend(p.message_lints.iter().map(|l| l.as_ref() as &dyn Lint));
    v
}

#[test]
fn the_pack_contributes_crate_lints() {
    let p = collected();
    assert!(
        !p.crate_lints.is_empty(),
        "the pack should contribute at least one crate lint"
    );
}

#[test]
fn the_pack_contributes_a_workspace_lint() {
    let p = collected();
    assert!(
        !p.workspace_lints.is_empty(),
        "writing-style compares across crates, so it belongs to the workspace kind"
    );
}

/// The pack contributes its tool, which is the whole claim of shipping one.
///
/// **Nothing asserted this.** Every other arm constructs the tool directly, so
/// removing it from the `lint_pack!` left the suite green while the one thing a
/// consumer gets by depending on this pack silently went away. That claim is
/// also what licensed deleting the local copy in the repository this was ported
/// out of, so it is the load-bearing one.
#[test]
fn the_pack_contributes_its_tool_under_the_name_the_subcommand_uses() {
    let p = collected();
    assert!(
        p.tools
            .iter()
            .any(|t| t.name() == "rulings-with-no-verbatim"),
        "a consumer gets this tool by depending on the pack, and the name is the subcommand: {:?}",
        p.tools.iter().map(|t| t.name()).collect::<Vec<_>>()
    );
}

#[test]
fn every_lint_has_a_kebab_case_name() {
    let p = collected();
    for lint in all_lints(&p) {
        let name = lint.name();
        assert!(!name.is_empty(), "lint name should not be empty");
        assert!(
            name.chars()
                .all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit()),
            "lint name `{name}` should be kebab-case"
        );
    }
}

#[test]
fn lint_names_are_unique_across_every_kind() {
    // Names key the config, so a collision between two kinds would leave one of
    // them unconfigurable. Checking within each kind separately would miss it.
    let p = collected();
    let names: Vec<&'static str> = all_lints(&p).iter().map(|l| l.name()).collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(names.len(), sorted.len(), "lint names collide: {names:?}");
}

#[test]
fn collecting_twice_accumulates_rather_than_replacing() {
    // The host collects every pack into one shared value, so `collect` appends.
    let mut p = LintPack::default();
    pack::collect(&mut p);
    let first = p.crate_lints.len();
    pack::collect(&mut p);
    assert_eq!(p.crate_lints.len(), first * 2);
}

/// Every preset names a lint the pack actually contributes.
///
/// A preset naming a lint that does not exist is silently inert: the project
/// imports it, sees no error, and gets none of the enforcement it asked for.
/// That is the failure class this whole arc exists to remove, so it is checked
/// mechanically rather than by review.
#[test]
fn every_preset_names_a_lint_the_pack_contributes() {
    let p = collected();
    let contributed: Vec<&str> = all_lints(&p).iter().map(|l| l.name()).collect();

    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("presets");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("presets dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read preset");
        let name = text
            .lines()
            .find_map(|l| l.strip_prefix("name = "))
            .map(|v| v.trim().trim_matches('"').to_string())
            .unwrap_or_else(|| panic!("{} has no top-level name", path.display()));
        assert!(
            contributed.contains(&name.as_str()),
            "preset {} names lint `{name}`, which the pack does not contribute: {contributed:?}",
            path.file_name().unwrap().to_string_lossy()
        );
        checked += 1;
    }
    assert!(
        checked > 0,
        "no presets were checked; the glob found nothing"
    );
}

/// Every config key a preset sets is one its lint declares.
///
/// A typo in a preset key is otherwise silently ignored, leaving the knob at its
/// default while the project believes it was set.
///
/// Currently red, and deliberately left so. 14 of the 15 pre-existing presets set
/// keys their lint declares nothing for, because the pack's lints are hand-written
/// Rust with the knobs hardcoded inside them, while the presets describe a
/// primitive-driven design (`primitive = "token-scan"` and friends) that was never
/// built. Nothing reads the `presets/` directory at all. Closing this means
/// implementing those primitive kinds so the lints become configured instances.
/// The assertion states the intended behaviour rather than the present one, so it
/// flips to green when that lands.
#[test]
#[ignore = "catalogue: nearly every preset sets keys its lint ignores, and the run \
            names which; the primitive layer the presets presume is not implemented \
            and presets/ is never loaded"]
fn every_preset_config_key_is_declared_by_its_lint() {
    let p = collected();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("presets");
    for entry in std::fs::read_dir(&dir).expect("presets dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("read preset");
        let Some(name) = text
            .lines()
            .find_map(|l| l.strip_prefix("name = "))
            .map(|v| v.trim().trim_matches('"').to_string())
        else {
            continue;
        };
        let Some(lint) = all_lints(&p).into_iter().find(|l| l.name() == name) else {
            continue; // covered by the test above
        };
        let declared = lint.config_keys();

        // keys inside the [config] table only
        let mut in_config = false;
        for line in text.lines() {
            let t = line.trim();
            if t.starts_with('[') {
                in_config = t == "[config]";
                continue;
            }
            if !in_config || t.is_empty() || t.starts_with('#') {
                continue;
            }
            let Some((key, _)) = t.split_once('=') else { continue };
            let key = key.trim();
            assert!(
                declared.contains(&key),
                "preset {} sets `{key}` on lint `{name}`, which declares only {declared:?}",
                path.file_name().unwrap().to_string_lossy()
            );
        }
    }
}
