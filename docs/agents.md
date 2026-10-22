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

## The official skill

The same material is packaged as a Claude Code skill, `using-molt-framework`, in its own
repository, [molt-framework-skill](https://github.com/micheltlutz/molt-framework-skill). It is a
plugin (`molt-framework`, from a marketplace named `molt`) and has been submitted to Anthropic's
plugin directory, where it is waiting for review; until it is listed there, install it from the
repository.

In Claude Code:

```text
/plugin marketplace add micheltlutz/molt-framework-skill
/plugin install molt-framework@molt
```

From a terminal, with the [`skills`](https://github.com/vercel-labs/skills) installer, which
also reaches other agents (`-a` names the agent; add `-g` to install for every project instead
of the current one):

```bash
npx skills add micheltlutz/molt-framework-skill --skill using-molt-framework -a claude-code
```

The skill reads the version from `Cargo.lock`, looks an API up in a table with one column per
version, and sends the agent to the pages in this directory. It bundles snapshots of
`capabilities.md` and `pitfalls.md` so it works offline. If you kept a personal copy in
`~/.claude/skills/using-molt-framework/`, remove it after installing, or the skill is listed
twice.

## Without the skill

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
