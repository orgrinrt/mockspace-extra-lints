# mockspace-extra-lints

> An opt-in lint pack for mockspace. Imported by a project that wants these
> rules, rather than shipped in mockspace core.

Mockspace ships the lints every project needs, changelist discipline, document
and source agreement, file size, dead crates. This pack holds the ones that only
make sense once a project has committed to a particular set of foundations,
where the right answer for one codebase is noise for another.

A project opts in from its `mockspace.toml`:

```toml
[lint-crates]
mockspace-extra-lints = { git = "ssh://git@github.com/orgrinrt/mockspace-extra-lints.git", branch = "dev" }
```

Every lint is then configured, or silenced, per project in the same file. The
pack registers names; it does not decide severities.

## What is in it

### Primitive discipline

`arvo-types-only`, `no-bare-numeric`, `no-bare-option`, `no-bare-result`,
`no-bare-string`, `no-bare-static-str`, `no-public-raw-field`,
`no-vec-in-trait-sig`. These enforce that a codebase built on a typed numeric
layer does not leak the primitives that layer exists to replace. They read every
file in a crate rather than its root alone.

The type of a const generic parameter is excepted, and it is the only excepted
position. `<const BITS: u32>` is permitted, while `const BITS: u32 = 32;`, an
associated constant, a field, a cast and a literal suffix are all still
reported, even though a line scan cannot tell any of them apart. The exception
comes out of the parse rather than the line, in
`src/const_generic_parameters.rs`, which blanks the type of every const
generic parameter before the scan runs and keeps the byte length so the line
numbers stay the file's own.

### Environment discipline

`no-std`, `no-alloc`, `no-runtime-spawn`, `no-runtime-registration`. What a
`no_std`, allocation-free engine may reach for, and what it may not.

### Shape discipline

`no-dyn-dispatch`, `strategy-marker-required`, `trait-first-signatures`,
`semantic-alias-nudge`. Advisory in tone, and mostly about whether a signature
says what it means.

### Prose and process

`writing-style`, `commit-style`, `forge-body`, `message-attribution`,
`lint-allow-requires-task-id`.

## Per-file and per-crate dispatch

Worth knowing before adding a lint here, because getting it wrong is invisible
in the lint's own output.

Mockspace hands a lint one crate at a time and, by default, calls it once per
file with the file swapped into `source`. That default is what lets a line scan
see module files rather than only the crate root.

A lint that walks `ctx.all_sources` itself must therefore declare
`per_file(false)`, or it reports its whole crate once per file. A lint making a
claim about the crate root must check that `source` is the root before making
it. Both defects report a plausible-looking number and neither fails a test that
only calls the lint once, so `tests/per_file_dispatch.rs` asserts the dispatch
axis directly.

## Presets

`presets/` holds shareable severity sets a consumer can extend rather than
restate. See `presets/README.md`.

## Building

```
cargo build --release
cargo test
```

Requires the pinned nightly in `rust-toolchain.toml`.

The `mockspace-lint-rules` dependency comes from the mockspace repo over git.
Iterating on both at once, redirect it with a `[patch]` section in
`.cargo/config.toml` rather than editing the manifest, so nothing local reaches
a commit. A path dependency works the same way where the two sit beside each
other in one workspace.

## A note on coding agents

We do not recommend using coding agents with this codebase.

If you still choose to use a coding agent:

- Be aware of the environmental and social impact of large-scale model
  inference. Minimise agent use where it is not needed. Be responsible.
- Only use an agent if you yourself understand the architecture. Do not use an
  agent because you do not understand; you will waste time and energy, both
  yours and the planet's.
- A lint is a rule about a codebase that a reader will trust without rereading
  it. Getting one subtly wrong is worse than not having it, because the gate
  then reports clean over the thing it was added to catch.

The recommendation stands: do this work yourself unless you know what you are
doing and why.

## License

MPL-2.0. See `LICENSE`.
