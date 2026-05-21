# Stack-lints preset pack

TOML preset overlays consumed via the mockspace v2 preset infrastructure
(see `mock/research/202605220500_lint-preset-infrastructure.md` and
`mock/research/202605221700_directive-vocabulary-reconciled.md` in the
mockspace repo for the design).

## What this is

Once mockspace v2 Phase 4 lands (external preset host loading via the
`mock://ext/<pkg>/export/lint-preset/<name>` URI scheme and
SHA-pinned lockfile), consumers extend a preset from this pack by
adding to their `mockspace.toml`:

```toml
[lints.no-heap]
extends = "stack-lints::no-alloc"
```

The `extends` shorthand resolves to
`mock://ext/stack-lints/export/lint-preset/no-alloc`, the loader
fetches the TOML, and the cascade applies it between catalog defaults
and the consumer's workspace defaults.

Until Phase 4 ships, these files are inert: they exist as the future
content the loader will fetch. Consumers who want the same overlay
behaviour today use the first-party form `extends = "mockspace::no-alloc"`
which resolves to embedded content in the mockspace binary (shipped in
mockspace PR #69).

## What ships here

15 preset overlays mirroring the corresponding policy bundles in
mockspace-rs's first-party preset table:

- `no-alloc.toml`, `no-std.toml`, `no-dyn-dispatch.toml`,
  `no-runtime-spawn.toml`, `no-runtime-registration.toml`
- `no-bare-numeric.toml`, `no-bare-string.toml`, `no-bare-option.toml`,
  `no-bare-result.toml`, `no-public-raw-field.toml`,
  `no-vec-in-trait-sig.toml`, `strategy-marker-required.toml`,
  `trait-first-signatures.toml`
- `writing-style.toml`
- `lint-allow-requires-task-id.toml`

Each preset's content is byte-equivalent to the corresponding
mockspace::* preset at this snapshot. They duplicate intentionally:

- Consumers using `mockspace::<name>` get the first-party (embedded)
  shape today.
- Consumers using `stack-lints::<name>` will get the external
  (lockfile-pinned) shape once Phase 4 lands.

Future divergence is expected: stack-lints presets will tighten as
the stack matures, adding stricter forbidden tokens, narrower
scopes, and harder severities than the mockspace::* baseline. The
identical-content snapshot today is the starting point.

## What stays as Rust code

Three of the 18 lints in `src/lints/` do not have a corresponding
mockspace-rs primitive that a preset could configure; they ship
detection logic, not just policy:

- `arvo_types_only` (arvo-specific newtype-at-boundaries enforcement)
- `no_bare_static_str` (const/static string interning gate)
- `semantic_alias_nudge` (soft warn on raw arvo primitives at API
  boundary; carries the populated arvo-primitive-to-alias table)

These stay as Rust impls in the crate. Migrating them to presets
would require shipping the underlying detection logic as new
catalog primitives in mockspace-rs first; that work has not been
scoped.

## Conventions

- File name (minus `.toml`) is the preset name. It must match the
  `name` field; the mockspace v2 schema validator hard-fails on
  mismatch.
- The `primitive` field names the mockspace-rs catalog primitive the
  preset configures.
- The `extends` field chains to another preset via the
  `<host>::<name>` shorthand. The stack-lints presets currently
  carry no `extends` (they are full overlays); future severity
  profiles will chain off the mockspace::* base.

## Cross-references

- Reconciled directive vocabulary memo: `mockspace/mock/research/202605221700_directive-vocabulary-reconciled.md`.
- Preset infrastructure memo: `mockspace/mock/research/202605220500_lint-preset-infrastructure.md`.
- Migration guide (consumer-facing): `mockspace/docs/MIGRATION-v1-to-v2-lints.md`.
- mockspace-rs's parallel first-party preset table: `mockspace/mock/crates/mockspace-rs/presets/`.
