# mockspace-extra-lints

> An opt-in lint pack for mockspace. Imported by a project that wants these
> rules, rather than shipped in mockspace core.

Mockspace ships the lints every project needs: changelist discipline, document
and source agreement, file size, dead crates. This pack holds the ones that only
make sense once a project has committed to a particular substrate, where the
right answer for one codebase is noise for another.

A project opts in from its `mockspace.toml`:

```toml
[lint-crates]
mockspace-extra-lints = { git = "ssh://git@github.com/orgrinrt/mockspace-hilavitkutin-stack-lints.git", branch = "dev" }
```

Every lint is then configured, or silenced, per project in the same file. The
pack registers names; it does not decide severities.

## What is in it

**Primitive discipline.** `arvo-types-only`, `no-bare-numeric`,
`no-bare-option`, `no-bare-result`, `no-bare-string`, `no-bare-static-str`,
`no-public-raw-field`, `no-vec-in-trait-sig`. These enforce that a codebase
built on a typed numeric substrate does not leak the primitives that substrate
exists to replace. They read every file in a crate rather than its root alone.

**Environment discipline.** `no-std`, `no-alloc`, `no-runtime-spawn`,
`no-runtime-registration`. What a `no_std`, allocation-free engine may reach
for, and what it may not.

**Shape discipline.** `no-dyn-dispatch`, `strategy-marker-required`,
`trait-first-signatures`, `semantic-alias-nudge`. Advisory in tone, and mostly
about whether a signature says what it means.

**Prose and process.** `writing-style`, `commit-style`, `forge-body`,
`message-attribution`, `lint-allow-requires-task-id`.

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
