# `mockspace-extra-lints`

> An opt-in pack of lints and tools for mockspace, for the rules and the workflow that only make sense once a project has picked its foundations.

Mockspace ships the lints every project needs, changelist discipline, document and source agreement, file size and dead crates. This pack holds the ones that only make sense once a project has committed to a particular set of foundations, where the right answer for one codebase is noise for another. It isn't on crates.io and isn't meant to be, since a lint pack is reached through mockspace and not through a dependency graph. It's pre-1.0, so the lints and their keys may move, and it builds on the nightly pinned in `rust-toolchain.toml`.

## Usage

A project opts in from its `mockspace.toml`:

```toml
[lint-crates]
mockspace-extra-lints = { git = "ssh://git@github.com/orgrinrt/mockspace-extra-lints.git", branch = "dev" }
```

Every lint is then configured, or silenced, per project in the same file. The pack registers names and doesn't decide severities, which is what lets one codebase run `no-alloc` at `error` on every gate while another never turns it on. A lint nobody names does whatever its own default says.

```toml
[lints.no-alloc]
commit = "error"
build = "error"
push = "error"

[lints.no-bare-string]
commit = "off"
```

## What is in it

The primitive discipline lints, `arvo-types-only`, `no-bare-numeric`, `no-bare-option`, `no-bare-result`, `no-bare-string`, `no-bare-static-str`, `no-public-raw-field` and `no-vec-in-trait-sig`, keep a codebase built on a typed numeric layer from leaking the primitives that layer replaces. The environment ones, `no-std`, `no-alloc`, `no-runtime-spawn` and `no-runtime-registration`, say what a `no_std`, allocation-free engine may reach for. The shape ones, `no-dyn-dispatch`, `strategy-marker-required`, `trait-first-signatures`, `semantic-alias-nudge` and `re-export-foreign-names`, are about whether a signature says what it means, and the last has teeth: a foreign type in a public signature wants a `pub use` in the crate that names it.

For tests there's `catalogue-ignore`, which wants every `#[ignore]` to say which tracked row it waits on, and `no-source-include-in-test`, which refuses a test that reads a source file as text to search it. `no-superseded-citation` fails on a design or source file naming a canon row that a newer one replaced. `one-relation-field` reads the registry schema in `mockspace.toml` and refuses a field per target namespace beside the one `ref[]` relation field, with `relation` and `structural` for what a project calls them. The prose and process ones are `writing-style`, `commit-style`, `forge-body`, `message-attribution` and `lint-allow-requires-task-id`, and `message-attribution` also refuses an author or committer identity that names an agent, with `agent_names`, `extra_agent_identities` and `not_agents` for the cases it gets wrong.

A repo lint reads the manifests of the consuming repository. `a-git-dependency-is-not-pinned` refuses a git dependency that names a fixed `rev` or `tag`, in a `Cargo.toml` or a `mockspace.toml`, wherever the table sits. It knows no repository of its own: `accounts` lists the `host/account` pairs it reads (`github.com/acme`), `branch` is the branch a dependency on one of them has to track (a missing or other branch is a finding only when it is set), and `exempt` lists path globs from the repository root that are not read, such as records of earlier work. Without `accounts` it finds nothing, and the severity is the project's at each gate.

The pack carries three tools along with the lints, `rulings-with-no-verbatim`, `coverage` and `near-copies`, which `mock tools` lists beside the builtins. All three report and none gates. `near-copies` lists types in different crates that are close without being the same, which `single-source` leaves alone because a gate cannot tell a reinvention from two concepts sharing a word, and with no arguments it reads the crates the branch touched against `origin/dev`. Path globs in the pack are matched from the repository root with `/` as the separator, and the rest of each lint's keys are in its rustdoc.

Behind the `workflow` feature there's also a ticket and canon workflow kept in the registry itself. `ticket-add`, `ticket-move` and `tickets` keep tickets as registry rows with their moves, planning and the board, `canon` and `canon-links` answer what the canon holds and how well it's joined, and `corpus`, `already-said` and `unearth` search the research notes, the design rounds and the maintainer's captured words. `owed`, `finish`, `known-red`, `tackle`, `test-list`, `build-weigh`, `parallel-statements`, `intent-or-mechanism`, `quote-check` and `research-unbundle` come with them, and six repo lints hold the tenet, subject and ticket rows and every `FIXME` to naming a ticket. It's off by default, because a repository keeping its own copy of one of these tools would register the name twice and the engine refuses that. A repository turns it on with `features = ["workflow"]` on its `[lint-crates]` entry, copies the namespace declarations of `registry/namespaces.toml` into its `mockspace.toml` and the states, kinds and roles beside them into its own registry. The tools read `project_name` and `crate_prefix` from the same file, and the tools that read the maintainer's captured words take the directory from `maintainer_words` in their own `[tools.<name>]` table, or from `MOCKSPACE_MAINTAINER_WORDS` where that is unset, and read none when neither names one. `maintainer_names` lists whose quoted lines in a note count as those words. For now they take the mock directory to be `mock`.

To build it, run `cargo build --release` and `cargo test` on the pinned nightly. The `mockspace-lint-rules` dependency comes from the mockspace repo over git, and when iterating on both at once a `[patch]` section in `.cargo/config.toml` redirects it without touching the manifest.

## License

> The project is licensed under the **Mozilla Public License 2.0**.

`SPDX-License-Identifier: MPL-2.0`

> You can check out the full license [here](https://github.com/orgrinrt/mockspace-extra-lints/blob/dev/LICENSE)
