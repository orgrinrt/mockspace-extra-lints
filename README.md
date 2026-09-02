# `mockspace-extra-lints`

<div align="center" style="text-align: center;">

[![GitHub Stars](https://img.shields.io/github/stars/orgrinrt/mockspace-extra-lints.svg)](https://github.com/orgrinrt/mockspace-extra-lints/stargazers)
[![GitHub Issues](https://img.shields.io/github/issues/orgrinrt/mockspace-extra-lints.svg)](https://github.com/orgrinrt/mockspace-extra-lints/issues)
![License](https://img.shields.io/github/license/orgrinrt/mockspace-extra-lints?color=%23009689)

> An opt-in lint pack for mockspace, for the rules that only make sense once a
> project has picked its foundations.

</div>

Mockspace ships the lints every project needs, changelist discipline, document
and source agreement, file size, dead crates. This pack holds the ones that only
make sense once a project has committed to a particular set of foundations,
where the right answer for one codebase is noise for another.

## Installation

Not on crates.io, and it is not meant to be: a lint pack is reached through
mockspace rather than through a dependency graph, so a project opts in from its
`mockspace.toml`.

```toml
[lint-crates]
mockspace-extra-lints = { git = "ssh://git@github.com/orgrinrt/mockspace-extra-lints.git", branch = "dev" }
```

## Usage

Every lint is then configured, or silenced, per project in the same file. The
pack registers names; it does not decide severities, which is what lets one
codebase run `no-alloc` at `error` on every gate while another never turns it on.

```toml
[lints.no-alloc]
commit = "error"
build = "error"
push = "error"

[lints.semantic-alias-nudge]
commit = "warn"

[lints.no-bare-string]
commit = "off"
```

A lint nobody names does whatever its own default says, so the file is a set of
deliberate departures rather than a full listing.

## What is in it

### Primitive discipline

`arvo-types-only`, `no-bare-numeric`, `no-bare-option`, `no-bare-result`,
`no-bare-string`, `no-bare-static-str`, `no-public-raw-field`,
`no-vec-in-trait-sig`. These enforce that a codebase built on a typed numeric
layer does not leak the primitives that layer exists to replace: `u8` through
`u128`, `i8` through `i128`, `f32`, `f64`, `String`, a non-`'static` `&str`, and
`Option` and `Result` in an API position. They read every file in a crate rather
than its root alone.

The type of a const generic parameter is excepted, and it is the only excepted
position. `<const BITS: u32>` is permitted, while `const BITS: u32 = 32;`, an
associated constant, a field and a cast are all still reported, even though a
line scan cannot tell any of them apart. The exception comes out of the parse
rather than the line, in `src/const_generic_parameters.rs`, which blanks the
type of every const generic parameter before the scan runs and keeps the byte
length so the line numbers stay the file's own.

A literal suffix is reported by neither lint, and the scan is why: it wants a
non-identifier byte on each side of the name, and a suffix always carries a
digit or an underscore in front of it, so `0u32`, `1_usize` and `0.0_f32` go
past. Reaching them wants the parse as well, which is a change nobody has made
yet, and the gap is catalogued as a test rather than left to be rediscovered.

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

### Tools

A tool rides the same cdylib a lint does, so depending on this pack gives you
its tools too and your repo needs no `mock/tools/` of its own. `mock tools`
lists them beside the builtins and your own, and does not distinguish where one
came from, because at the point of running one it does not matter.

`rulings-with-no-verbatim` reports every `ruling` row that sets `says` and
carries no `quote`, so what stands behind it is somebody's restatement rather
than the words themselves. Nothing gates on it: a row it names is sometimes the
best available record of a real call, and the list is for knowing where the
holes are. Where a corpus records a `ratified_by` of `experts` those rows are
out of scope, and a corpus declaring no such field makes no such distinction, so
all of its rulings are in scope.

What belongs here rather than in one repository is the same question a lint
answers. A check only one repo will ever want stays in that repo. A check
several want is one implementation here instead of one per consumer, drifting
apart. Two corpora had independently grown tools for the provenance of a ruling
and for coverage of a namespace, under different names, neither citing the
other, and a fix to either was a fix to one.

Two things make a pair worth folding into one tool, and a resemblance in the
name is neither of them. The check has to ask the same question, and the thing
it differs on has to be expressible as an argument. Coverage of a namespace by
what reaches it is one question wherever it is asked, so the namespace is an
argument and one implementation serves. A check that takes no argument and reads
one namespace from a constant is not the same shape as one that takes a required
term and sweeps every namespace declared, however alike the two read, and
merging them would change what one of them does.

So the test is the contract rather than the noun. Where a check is genuinely one
repository's, it stays there.

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

## Support

Whether you use this project, have learned something from it, or just like it, please consider supporting it by buying me a coffee, so I can dedicate more time on open-source projects like this :)

<a href="https://buymeacoffee.com/orgrinrt" target="_blank"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" alt="Buy Me A Coffee" style="height: auto !important;width: auto !important;" ></a>

## License

> The project is licensed under the **Mozilla Public License 2.0**.

`SPDX-License-Identifier: MPL-2.0`

> You can check out the full license [here](https://github.com/orgrinrt/mockspace-extra-lints/blob/dev/LICENSE)
