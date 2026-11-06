# Presets

TOML overlays naming one policy for a lint: its tokens, severities and path scopes. Nothing loads them yet. The mockspace that consumers run reads no `presets/` directory, and the loader for external hosts (`mock://ext/`) is not shipped, so a consumer configures a lint under `[lints.<name>]` in its own `mockspace.toml`. The only reader is a test that takes the glob out of `attribution-claude.toml` and `attribution-copilot.toml`.

## What a preset file holds

```toml
schema_version = "1.0"
name = "no-alloc"
primitive = "token-scan"
description = "no heap allocation in stack code"

[config]
tokens = ["Vec<", "Box<", "Rc<", "Arc<", "String", "vec!", "HashMap<", "BTreeMap<"]
word_boundary = false

[severity]
commit = "error"
build = "error"
push = "error"

[scope]
paths = ["**/*.rs"]
exempt_paths = ["**/build.rs", "**/benches/**"]
```

`primitive` names the catalogue primitive the overlay configures, `name` the lint it lands on. The file name may differ from `name`, so one lint can carry several policies: `commit-conventional.toml` and `commit-hiisi.toml` both target `commit-style`, and three files target `message-attribution`. A project picks one from each group.

`ls` the directory for the list. `arvo-types-only`, `no-bare-static-str` and `semantic-alias-nudge` have no preset, because their detection is the lint's own and no primitive exists to configure.

Most presets set keys their lint does not read, since the lints hold their knobs in Rust. `every_preset_config_key_is_declared_by_its_lint` in `tests/pack_entry_points.rs` states the intended behaviour and is ignored until the primitive layer exists.
