# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Renamed

- **winged-rust is now Molt.** The crate and the npm package are `molt`, the Rust path is
  `molt::`, the WebAssembly files are `molt.js` / `molt_bg.wasm`, and the repository moved
  to [micheltlutz/molt](https://github.com/micheltlutz/molt). `WINGED_UPDATE_FIXTURES` is
  now `MOLT_UPDATE_FIXTURES` and `WINGED_WASM_BUDGET_KB` is `MOLT_WASM_BUDGET_KB`. Issue
  links below still point to the winged-rust repository, where those issues live.

### Added

- **URL policy, checked when the tree is written.** `href`, `src`, `action`, `formaction`,
  `poster`, `cite`, `background`, `xlink:href` and `data` render as `#` when the value is a
  `javascript:`, `vbscript:` or `data:text/html` URL. The check decodes character references
  once, so `Attribute::raw("href", "javascript&#58;alert(1)")` is blocked, while a value
  passed through `Attribute::new` keeps its escaped `&amp;`. `Attribute::trusted_url`,
  `Element::trusted_url` and `WElement.trustedUrl` opt out. An attribute name that contains
  a space, quote, `=`, `<`, `>`, `/` or a control character is not emitted.
- `Element::self_closing(true)` closes a non-void tag with `>` or ` />` and drops its
  children, so a self-closing tag does not have to be built as `Node::Raw` (which would
  skip the URL policy).
- `Document::render_to` writes the same bytes as `render_with`, one body section at a time.
  `StaticSiteGenerator::generate` uses it, into the existing temporary file.
- `overflow-checks` in the release profile, `cargo deny`, and a libFuzzer script
  (`scripts/fuzz.sh`) that is outside the pull-request gate. `unwrap` and `expect` are
  denied in library code.

### Changed

- The release workflow verifies in a job with no registry token, then publishes from a
  fresh build with `cargo publish --locked --no-verify` via crates.io Trusted Publishing.
  Registering the `release` environment on crates.io and retiring `CARGO_REGISTRY_TOKEN`
  is a manual step.

## [1.0.1] - 2026-09-21

### Added

- **The Winged-Swift test suite is fully ported.** All 183 non-skipped cases across 23
  suites now have a Rust counterpart, each naming the Swift case it came from, and
  `scripts/check-swift-parity.sh` checks those names against the Swift source rather than
  trusting them. It is step 7 of `scripts/verify.sh`, and skips itself when the sibling
  checkout is absent. ([#30](https://github.com/micheltlutz/winged-rust/issues/30))

### Fixed

- `seo::common` emitted `<meta name="keywords" content="">` when given an empty list, where
  Winged-Swift omits the tag. `SeoBuilder` already guarded against it, so the empty tag was
  only reachable by calling `common` directly — the guard now lives in `common`, where both
  entry points get it. Found by porting `SEOTests.testCommonOmitsEmptyKeywords`.

- **Rendering no longer recurses**, so nesting depth is no longer bounded by the stack.
  The writer walks an explicit work stack, and `Element` tears its subtree down the same
  way. A 100,000-level tree renders in both modes and is freed without aborting; output is
  byte-identical to the recursive writer for every test and all four golden fixtures.
  A stack overflow is an abort rather than a catchable panic, so this was a
  denial-of-service vector wherever nesting depth could be influenced by untrusted input.
  ([#33](https://github.com/micheltlutz/winged-rust/issues/33))
- `Render for Element` no longer clones the whole subtree on every render. It built a
  `Node::Element(self.clone())` first, which deep-copied the tree — and `Clone` is itself
  recursive, so a deep tree aborted there before the writer ever saw it.
- `RenderOptions::write_indent` returns immediately for an empty indent instead of looping
  once per level to append nothing. That loop is quadratic in depth: it cost 135 seconds on
  a 100,000-level tree, against 0.08 with the early return.

### Changed

- `Element` implements `Drop`. Its fields are private, so no partial move was possible on
  it; a bare `Node::Fragment` chain with no element in it is still freed recursively,
  because giving `Node` a `Drop` would forbid `match node { Node::Element(e) => e }`.

## [1.0.0] - 2026-09-18

### Added

- **Core engine** — `Node`, `Element`, `Attribute`, the `Render` trait and `RenderOptions`.
  Rendering is buffered: the whole tree writes into one `String`.
- **Three escapers** — `escape_text`, `escape_attribute` and `escape_xml`. Escaping is
  applied once, when content enters the tree.
- **93 HTML elements**, generated from a single `define_elements!` table, plus typed
  constructors for the shapes that carry required arguments (`image`, `iframe_titled`,
  `label_for`, `link_to`, `stylesheet`, `button_typed`, `input_named`, `script_src`).
- **`html!` macro** — nested markup syntax with attributes, `@if` / `@else` and `@for`.
  Expands to the same tree the builder API produces.
- **`Document`** and the **`Layout`** trait.
- **SEO** — `open_graph`, `open_graph_article`, `twitter_card`, `common`, and `SeoBuilder`.
- **Accessibility** — a `Role` enum and `audit`, which reports images without `alt`,
  buttons and links with no accessible name, iframes without a title, and inputs with
  nothing a label can attach to.
- **`sitemap`** and **`feed`** — XML sitemaps, sitemap indexes and RSS 2.0.
- **`ssg`** — `StaticSiteGenerator` with atomic writes, behind `feature = "ssg"`.
- **`parallel`** — `rayon`-backed bulk page generation, behind `feature = "parallel"`.
- **`wasm`** — `wasm-bindgen` bindings wrapping the real tree, behind `feature = "wasm"`.
- **`Node::Comment`** — an HTML comment node, which Winged-Swift does not have. `--` in the
  content is neutralised so a comment cannot close early.
- **Golden-file tests** reproducing all four Winged-Swift fixtures byte for byte, with
  `MOLT_UPDATE_FIXTURES=1` to regenerate.
- **NPM package** — `web`, `nodejs` and `bundler` builds, 20 KB gzipped, with TypeScript
  typings. `npm/smoke.mjs` renders the golden marketing page through WebAssembly and diffs
  it against the same fixture the Rust tests use, so native and WASM output cannot drift
  apart unnoticed.
- **Browser and Node examples**, both runnable with no bundler.
- **`scripts/build-wasm.sh`** — all three targets, with a gzipped size budget enforced in
  CI.
- **`scripts/verify-npm.sh`** — `tsc --noEmit` over the generated typings, then installs a
  packed tarball into a throwaway project rather than linking the workspace, which is what
  catches a wrong `files` allowlist or a broken `exports` map.
- **`scripts/verify.sh`** — one command that is a superset of CI.
- **`scripts/generate-tag-catalog.sh --check`** — documentation drift fails the build.

### Changed from Winged-Swift 2.0.0

Every deliberate difference is listed with its reason in [`PORTING.md`](PORTING.md).
The short version:

- `data_attrs` and `aria_attrs` produce deterministic output; the Swift versions iterate a
  `Dictionary` and do not.
- The node tree is `Send + Sync`. Winged-Swift's `ROADMAP.md` lists its tree not being
  `Sendable` as an unresolved 3.0 problem.
- `seo::common` emits the `<title>` it is given. `SEO.common` accepts a title and drops it.
- `StaticSiteGenerator::clean` refuses empty, root and `..`-containing paths, and page
  paths cannot escape the output directory.
- `generate_multiple` reports every failure rather than the first.
- Writes are atomic.
- The process-wide `HTMLTag.xhtmlSelfClosing` switch is not ported; pass
  `RenderOptions::with_xhtml_self_closing` instead.

### Known limitations

- **Rendering depth is bounded by the stack.** The renderer recurses once per nesting
  level. 256 levels is guaranteed and covered by a test — deeper than browsers themselves
  render; around 2,000 levels aborts the process, and a stack overflow is an abort rather
  than a catchable panic. Only reachable when nesting depth can be influenced by untrusted
  input; documented in `SECURITY.md`, the `README`, and on the `Render` trait. Tracked in
  [#33](https://github.com/micheltlutz/winged-rust/issues/33).

### Fixed relative to Winged-Swift

- `Scripts/generate-tag-catalog.sh` matches `public class [A-Za-z]+: HTMLTag`, which rejects
  digits, so `H1`–`H6` are silently missing from its checked-in catalog. The Rust generator
  covers all 93 tags, and a test asserts the headings are present.
