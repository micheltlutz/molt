# Decision log

Decisions a future agent could reasonably undo without knowing why. Newest first within each
batch. Format: **decision** — reason — what would change it.

Superseded or rejected options are kept, because "we considered it" is the cheapest thing to
lose.

## Standing constraints (from `docs/plans/2026-10-17-seguranca-e-performance.md`)

- **Three escapers, never one.** Fixtures depend on the differences; the original spec
  (`docs/history/MOLT_SPEC.md` §3.3) is wrong. *Changes if:* the fixtures are regenerated on
  purpose, with a `MIGRATION.md` row.
- **`panic = "abort"` stays in the release profile.** The WASM budget (120 KB gzip) depends on
  it. *Changes if:* the budget is renegotiated; server unwind is the engine's decision.
- **No Strategy/Visitor/Escaper as public traits.** One consumer, frozen API.
- **No attribute sorting, no `Drop` on `Node`.** `Drop` on `Node` would forbid the partial move
  `match node { Node::Element(e) => e }` used by `examples/static_site.rs` and is a 2.0 topic.
- **Rendering configured by value (`RenderOptions`), never globally.**
- **No SIMD, no `mimalloc`, no Markdown rewrite.**

## Batch A — security and correctness

- **Contextual `<` `>` escaping in attributes, not a wider `escape_attribute`** (S4). *Why:*
  keeps the golden fixtures byte-identical everywhere except under raw-text ancestors, where
  the unescaped form is exploitable. *Changes if:* the fixtures are regenerated with the
  stricter escape on purpose, then drop the contextual branch and widen the function.
- **Invalid tag → whole element dropped before the indent** (S2). *Why:* no safe guess for what
  `div><script>` meant; dropping before the indent leaves no blank line in pretty mode. Rust
  API unchanged (no `Result` from `Element::new`), JS API unchanged.
- **WASM did not get a fallible constructor** for S2. *Why:* the writer-level guard already
  covers every path, and a `Result` constructor would be a JS-facing break for no extra safety.
- **Symlink in a path component is refused, not followed** (S5). *Residual:* a link created
  during a build is a race this crate does not close; documented in `SECURITY.md`.
- **`Debug` is hand-written and bounded at 128 levels** (#79, 2.0.0; B4 had left it a recursive
  derive because a deep `{:?}` is a diagnostic nobody reads). *Why:* `assert_eq!` formats both
  trees when it fails, so a diagnostic is exactly where a deep tree arrives, and an abort there
  hides the failure. Recursion is kept, not an explicit stack, because it is bounded by the
  limit and prints the same text as the derive. *What would change it:* a need to see below
  level 128.
- **PR stacking over independent branches** (process). *Why:* two branches editing `node.rs` and
  `SECURITY.md` conflict; a chain tests every tip.

## Batch B — public API

- **`Element::new` / `Attribute::new` keep `impl Into<String>`; the field is `Cow<'static,
  str>`** (A3 diverged from issue #15). *Why:* `Into<Cow<'static, str>>` rejects non-`'static`
  `&str`, a common pattern (`.attr(&key, v)` in loops). Savings come from internal literals
  via `from_static`/`new_static`. *Changes if:* the ergonomics cost is judged acceptable, in
  which case it is a breaking change to take on purpose.
- **`Attribute::raw` stays public** (A2). *Why:* it is one of the three declared hatches, a
  fuzz target uses it, `SECURITY.md` documents it. The docs now say why not to use it.
- **`Render` is not implemented for `Document`** (A5). *Why:* inherent `render()` is pretty,
  the trait's is compact; one name, two meanings.
- **`Hash` is implemented iteratively, not derived** (A5), same reason as `Clone`/`PartialEq`.
- **`Node` is `#[non_exhaustive]`** (A1). *Cost:* every external `match` needs a `_` arm.
  *Why:* `Comment` was added after the port; more variants are plausible.
- **`ChangeFreq` is an enum; `priority` is clamped, not rejected** (A8). *Why:* a clamp keeps
  the builder chainable and the protocol has no value outside 0..1.
- **`SsgError` is hand-written, not `thiserror`** (A7). *Why:* zero runtime dependencies is a
  property of this crate.
- **WASM arguments are by value** (W1), which consumes the JS handle. *Cost:* callers must not
  reuse a handle; documented in the module and README.
- **`parallel` implies `ssg`; `cdylib` stays** (A10). *Why:* `parallel` only parallelises
  `generate_multiple`; wasm-pack needs a `cdylib`.
- **`stats_alloc` as a dev-dependency** (P0). *Why:* `unsafe_code = "forbid"` also covers
  tests, so a counting allocator cannot live in the repo. *Needs:* a licence check by
  `cargo-deny` in CI.

## Batch C — performance

- **Escapers copy clean runs (`push_str`)** (P4, PR #56) — accepted despite a 2.8× slowdown on
  adversarial (50% special) input, because prose-like input is 2× faster and the worst case is
  still ~400 MB/s. *Reverts cleanly:* the change is one commit. *Changes if:* the owner
  prefers a flat worst case.
- **Generic `impl Fn` entity tables, not `fn` pointers** — the pointer form is not inlined.
- **`html!` formats `Display` through a `fmt::Write` that escapes** (`TextWriter`,
  `AttributeWriter`), the key is borrowed, and a text-only element moves its text (PR #59).
  *Why:* rule 9 says `html!` is sugar over the builder, yet it cost 1.8× because of three
  intermediate copies. The tree it produces is unchanged. *Invariant to keep:* escaping each
  piece a `Display` writes equals escaping the whole, because escaping is per character.
- **`[profile.bench] opt-level = 3`**, release stays `z` for WASM.
- **`Node::Element(Box<Element>)` was not done** (P8, #37). *Measured:* `Node` and `Element` are
  104 bytes, `Attribute` 56. *Why not:* a text node would shrink to ~32 bytes but every element
  child costs one more allocation, and the enum's public shape changes. *Guard:* `size_of`
  ceilings in `tests/allocations.rs`. *Changes if:* a benchmark shows memory pressure from
  text-heavy trees.
- **No `DropIfEmpty` skipping for `Element`** (P6, #35). *Why:* since S2 an element with an
  invalid tag renders empty, and the rollback keeps pretty mode free of blank lines.
- **No cached attribute-validity flags** (P5, #34), **no O(depth) work stack** (P7, #36),
  **no larger initial stack** (P9, #38): each analysed or measured with no win, recorded on its
  issue.

## Standing policy: unsafe output is fixed, not preserved

(Owner's decision, after batch D.) Molt began as a port of a Swift library that is to be
discontinued after Molt 1.0.0, and it is now the reference for its own behaviour. The four
golden fixtures pin the output of valid input. Where behaviour is unsafe or wrong, fix it
instead of keeping an old output for its own sake. A change that alters output is complete
with a test asserting the rendered string, an entry in `CHANGELOG.md` (*Security* or
*Breaking*) and a row in `MIGRATION.md`'s *Output changes* table. The history of what the
port found, and where it was reported, is in `docs/history/`. *Examples:* `isPermaLink`,
`data:` in frames, `srcdoc`, contextual `<` `>` escaping.

## Batch D — security hardening

- **The URL policy takes the element's tag** (S7). `is_blocked_in(tag, key, value)`, with
  `is_blocked` kept tag-free. *Why:* `data:image/svg+xml` is inert on `<img>` and runs script in
  `iframe`/`frame`/`embed`/`object`. Before this, only `data:text/html` was blocked (`CHANGELOG.md`, *Security*).
- **`srcdoc` is written empty unless `trusted_url`** (S7), not replaced by `#`.
- **`meta refresh` is not checked** (S7): the HTML spec makes browsers ignore `javascript:`
  there. *Changes if:* a browser is found to honour it.
- **`escape_json_script` is a fourth escaper and does not validate** (S8). *Why:* HTML escaping
  corrupts JSON; validating needs a parser dependency this crate does not have.
- **A non-`http(s)` sitemap `loc` or feed `link` is omitted, not repaired** (S9), and
  `isPermaLink` is `true` only for an `http(s)` guid. (`CHANGELOG.md`, *Security*).
  Sitemap limits are exposed (`MAX_URLS`, `MAX_BYTES`, `chunks`) rather
  than enforced by silent truncation.
- **`RenderOptions::indent` is private; `max_depth` is opt-in** (S10). *Why:* the indent is
  written on every line; a depth bound is what limits O(depth²) output. The depth counter
  exists only in the `LIMITED` monomorphisation (zero cost otherwise).
- **Actions pinned by SHA; the release gates on `main` and green CI; publishing uses
  Trusted Publishing** (S11). *Why:* a long-lived registry token in a repository secret can
  publish any version from anywhere it leaks to; an OIDC exchange yields a token bound to this
  repository, workflow file and environment, alive for one job. The crates.io registration
  (owner-only) had to exist before the workflow changed, or the next release would have failed;
  it was done first, then the workflow (#77). 1.0.1 was the first release through that path;
  the old secret was then deleted and crates.io set to require Trusted Publishing. Required
  reviewers on the `release` environment were tried on 1.0.1 and removed: a solo maintainer
  approving their own deployment is a click, not a review.

## Repository layout for agents (2026-10-20)

- **`AGENTS.md` is the map; the rules live in `.claude/rules/`, one file per topic, scoped
  with `paths:`.** *Why:* `AGENTS.md` loads in every session, so everything in it costs
  context whether or not the task needs it, while a rule scoped to `src/wasm.rs` is read only
  when that file is touched. The twelve one-line rules stay in `AGENTS.md`, numbered as
  before, because the lessons, `src/macros.rs` and the governance notes cite them by number.
  *Rejected:* the full text in both places (it would drift) and dropping the numbered list
  (it would orphan every "rule 11" reference). *Changes if:* Claude Code stops loading
  `.claude/rules/`; the files then move back under `AGENTS.md` headings.
- **Commands are thin procedures over the rules; agents are read-only reviewers.** `/verify`,
  `/add-element`, `/fix-bug`, `/review`, `/security-audit`, `/golden`, `/lesson` and
  `/release` encode recipes that used to be prose in `AGENTS.md`, and none performs an
  outward-facing action: `/release` stops before the tag, `/golden` never regenerates. The
  three agents (`code-reviewer`, `security-auditor`, `api-reviewer`) cannot write, so a
  finding is always relayed and verified, never applied unseen. *Why:* the autonomy table in
  [`agent-governance.md`](agent-governance.md) says those actions need a yes each time.
- **`check-knowledge-base.sh` guards the layout too:** links under `.claude/`, every rule's
  `paths:` against `git ls-files`, and the tables in `AGENTS.md`. *Why:* a rule whose glob
  matches nothing never loads, and a command missing from the map is one nobody is told
  about. Both are the quiet rot the guard lines already prevent for lessons.

## Open decisions (not taken yet)

- Whether `Box<Element>` inside `Node` is worth a visible change to the public enum (needs a
  `size_of` measurement first).
- A safe JSON-LD helper (#26) versus documenting `raw_text` as unsafe for `<script>`.
