---
description: Prepare a Molt release locally (version bump, changelog section, verification); never tags or publishes
argument-hint: <version, e.g. 1.0.2>
disable-model-invocation: true
---

Prepare release $ARGUMENTS. Preparing is local and reversible; tagging and publishing are
not, and this command never does them.

1. Check the preconditions and stop if one fails: the working tree is clean, the branch is
   `develop` or `release/$ARGUMENTS`, `## [Unreleased]` in `CHANGELOG.md` is not empty, and
   the version is greater than the one in `Cargo.toml`.
2. Derive the bump from the changelog: a *Breaking* entry needs a major, an *Added* entry a
   minor, only *Fixed*, *Security* or *Changed* a patch. If the requested version
   disagrees, say so and stop.
3. Set `version` in `Cargo.toml`, then run `cargo check` so `Cargo.lock` records the new
   version.
4. In `CHANGELOG.md`, insert `## [$ARGUMENTS] - <today, YYYY-MM-DD>` directly below
   `## [Unreleased]`, so the unreleased section is left empty above it, and add one line
   under the new heading saying whether the public API or the rendered output changed. If
   `MIGRATION.md` has rows for this version, confirm they name it.
5. Run `./scripts/verify.sh` and `cargo publish --locked --dry-run`. Both must pass; report
   the `cargo-deny` caveat if it applies.
6. Show the diff and list the next steps, which are the user's to take: commit as
   `Update: versão $ARGUMENTS`, open a pull request from `release/$ARGUMENTS` to `develop`,
   merge `develop` into `main`, wait for CI on `main`, then tag `v$ARGUMENTS` on that
   commit. The release workflow refuses a tag that is not on `main` with a green CI run,
   checks that `Cargo.toml` and the tag agree, and publishes through Trusted Publishing.
