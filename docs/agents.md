# Using Molt with an AI agent

An agent working in a project that depends on `molt-framework` usually has the crate's source
and nothing else: the development notes of the Molt repository (`.claude/`, `AGENTS.md`) are not
published with the crate. What is published is enough to avoid the usual mistakes, if the agent
knows where to look.

- [`llms.txt`](../llms.txt) at the root of the crate is the index, in the
  [llmstxt.org](https://llmstxt.org) format.
- [`capabilities.md`](capabilities.md) says whether an API exists in the version the project
  uses, and what is left out on purpose.
- [`pitfalls.md`](pitfalls.md) maps a symptom to its cause and fix.

The unpacked crate is in Cargo's registry, at
`~/.cargo/registry/src/<index>/molt-framework-<version>/`, where `<version>` is the one in the
project's `Cargo.lock`. Paste this into the project's `AGENTS.md` (or `CLAUDE.md`) so the agent
reads those files before it searches the source:

```markdown
## Molt (HTML generation)

This project builds HTML with `molt-framework`. Before reading its source or guessing an API,
read these files from the unpacked crate (`~/.cargo/registry/src/*/molt-framework-<version>/`,
the version is in `Cargo.lock`): `llms.txt`, `docs/capabilities.md`, `docs/pitfalls.md`.

- Escape once: pass raw values to `text()` and `attr()`. Never put external data in
  `raw_text`, `Node::raw`, `Attribute::raw` or `trusted_url`.
- `use molt::prelude::*;`, plus `molt::Document` and `molt::Layout` from the crate root.
- Check the version column in `docs/capabilities.md`; if the API is missing in this version,
  say so instead of searching the source for it.
- Assert on the exact rendered string, and run `accessibility::audit` on trees that matter.
```
