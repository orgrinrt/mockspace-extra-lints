# mockspace-hilavitkutin-stack-lints

The mockspace lint pack for the `notko` / `arvo` / `hilavitkutin` / `vehje` stack. It exists so the
rules those four repos share are written once and enforced identically, rather than drifting into
four slightly different opinions about the same thing.

The rules it carries are not general Rust style. They are the stack's own discipline: no heap, no
bare standard primitives at an API boundary, no runtime dispatch or registration where a compile-time
shape would do, and every escape hatch tied to a tracked task. Those are choices the stack made
deliberately, and a lint is what keeps them from eroding one convenient exception at a time.

This crate is `publish = false`. It is consumed from git by the repos that use it, and there is no
version of it that makes sense outside them.

## Using it

Each consuming repo names the pack in its `mockspace.toml`:

```toml
[lint-crates]
mockspace-hilavitkutin-stack-lints = { path = "../mockspace-hilavitkutin-stack-lints" }
```

A path dependency is the normal shape here because the consumers sit beside this repo in the same
workspace. Point it at the git remote instead when they do not.

Severity is set per repo in the same file. The pack decides what a lint means; the consuming repo
decides how loudly it says so.

## The lints

Seventeen source lints, plus one that runs across crates.

| Lint | What it refuses |
|---|---|
| `no-alloc` | `alloc::*`, `Vec`, `String`, `Box`. The stack does not allocate. |
| `no-std` | `std::*` where `core` would do |
| `no-bare-option` | `Option<T>` in favour of `notko::Maybe<T>` |
| `no-bare-result` | `Result<T, E>` in favour of `notko::Outcome<T, E>` |
| `no-bare-numeric` | `u8`..`u128`, `i8`..`i128`, `f32`, `f64` in favour of arvo's exact-width types |
| `no-bare-string` | `String` in an API position |
| `no-bare-static-str` | non-`'static` `&str`, including as a const or static |
| `no-dyn-dispatch` | `dyn Trait` where a generic parameter carries the same contract |
| `no-runtime-spawn` | spawning work the scheduler should have planned |
| `no-runtime-registration` | registering at runtime what the type system could have declared |
| `no-public-raw-field` | a raw-typed field, public or not, since a field is part of the perimeter |
| `no-vec-in-trait-sig` | `Vec<T>` in a trait signature, where an iterator or a collector belongs |
| `strategy-marker-required` | a numeric type without its strategy marker |
| `semantic-alias-nudge` | advisory: raw arvo primitives at a public boundary |
| `trait-first-signatures` | a concrete type where a trait bound would let the call site choose |
| `arvo-types-only` | the headline: no bare numerics anywhere, at all |
| `lint-allow-requires-task-id` | a `// lint:allow(...)` with no tracked task behind it |

The cross-crate lint is `writing-style`, which applies the workspace's prose rules to public-facing
documentation.

`semantic-alias-nudge` is the only advisory one. The rest refuse.

## Building it

```bash
cargo test
```

Fifty-two tests over the lint bodies and the pack's entry points. The `mockspace-lint-rules`
dependency is taken from the mockspace repo over git; redirect it with a `[patch]` section in
`.cargo/config.toml` when iterating on both at once.

## A note on coding agents

We do not recommend using coding agents with this codebase.

If you still choose to use one:

- Be aware of the environmental and social impact of large-scale model inference. Minimise agent use
  where it is not needed. Be responsible.
- Only use an agent if you yourself understand the architecture. Do not use an agent because you do
  not understand; you will waste time and energy, both yours and the planet's.
- A lint pack is a poor thing to hand to an agent in particular, because a lint that is subtly wrong
  is worse than no lint: it teaches everyone downstream to write around it.

The recommendation stands: do this work yourself unless you know what you are doing and why.

## License

MPL-2.0. See [LICENSE](LICENSE).
