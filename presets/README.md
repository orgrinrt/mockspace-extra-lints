# Presets

TOML overlays a consumer extends instead of restating a lint's whole
configuration in its own `mockspace.toml`. Each one carries the tokens, the
severities and the path scopes for one policy, so a project that wants that
policy names it rather than copying twenty lines that then drift.

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

`primitive` names the mockspace catalogue primitive the overlay configures, and
`name` names the lint it lands on. The file name is the overlay's own identity
and is deliberately allowed to differ from `name`, which is what lets one lint
carry several policies: `commit-conventional.toml` and `commit-hiisi.toml` both
target `commit-style` and disagree about parenthesised scopes, and three files
target `message-attribution`, one per tool plus the general one. A project picks
one from each such group.

## What is here

`ls` the directory. Writing the count into this file means one of the two goes
stale the next time somebody adds an overlay.

Not every lint in the pack has one. A preset configures a mockspace primitive,
so a lint that carries its own detection has nothing for an overlay to set, and
`arvo-types-only`, `no-bare-static-str` and `semantic-alias-nudge` are the three
in that shape today. Moving them would mean shipping their detection as
catalogue primitives first.

## Reaching them from a consumer

The host shorthand a consumer would write resolves to a `mock://ext/` URI that
the loader fetches and pins by hash. That loader is not shipped, so nothing in a
consumer's `mockspace.toml` reaches these files yet, and the equivalents
embedded in the mockspace binary are what a project extends today. These
overlays are the content the loader will fetch once it lands, and they are kept
in step meanwhile.

Do note that they are expected to diverge from the embedded ones rather than
stay identical, with tighter token lists, narrower scopes and harder severities,
because the whole reason for a separate pack is that the right answer for one
codebase is noise for another.
