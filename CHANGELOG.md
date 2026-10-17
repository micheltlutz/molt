# Changelog

All notable changes to this project are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

`molt-framework` starts its own version line at 0.1.0. The 1.0.0 and 1.0.1 entries below
were released as `winged-rust`.

## [Unreleased]

The 1.0.0 hardening release. Everything under **Breaking** needs a change in code that
calls the API; [`MIGRATION.md`](MIGRATION.md) shows each one, before and after. Issue
numbers are in [micheltlutz/molt](https://github.com/micheltlutz/molt/issues).

### Breaking

- **`Node` is `#[non_exhaustive]`**, and `Node::Text`, `Node::Raw` and `Node::Comment` hold a
  `Markup`, a string with no public constructor. `Node::Text("<script>".into())` no longer
  compiles; build nodes with `Node::text` (escapes), `Node::comment` (neutralises) and
  `Node::raw` (the greppable hatch). A `match` on `Node` needs a `_` arm.
  ([#13](https://github.com/micheltlutz/molt/issues/13),
  [#14](https://github.com/micheltlutz/molt/issues/14))
- **`#[non_exhaustive]`** on `Role`, `A11yIssue`, `RenderOptions`, `Document`, `SitemapUrl`,
  `RssItem` and `RssGenerator`: build them with their constructors, not struct literals.
- **`RenderOptions::indent` is private.** Read it with `indent()`; `with_indent` keeps only
  spaces and tabs, at most 16. ([#28](https://github.com/micheltlutz/molt/issues/28))
- **`Layout::render` and `Layout::render_many` are `wrap` and `wrap_many`**, because they
  collided with `Render::render`. ([#18](https://github.com/micheltlutz/molt/issues/18))
- **`escape_text`, `escape_text_with_slashes`, `escape_attribute` and `escape_xml` return
  `Cow<str>`**, borrowed when nothing needed escaping.
  ([#21](https://github.com/micheltlutz/molt/issues/21))
- **`VOID_TAGS` and `WHITESPACE_SENSITIVE_TAGS` are `&[&str]`**, not arrays with a length.
  ([#16](https://github.com/micheltlutz/molt/issues/16))
- **`SitemapUrl::changefreq` takes a `ChangeFreq`** instead of text; `priority` is clamped to
  `0.0..=1.0`. ([#20](https://github.com/micheltlutz/molt/issues/20))
- **`StaticSiteGenerator` returns `SsgError`**, not `io::Error`. It converts into
  `io::Error`, so `?` in a function returning `io::Result` still works.
  ([#19](https://github.com/micheltlutz/molt/issues/19))
- **WASM:** `child`, `addHead` and `addBody` take their element **by value**, which consumes
  the JavaScript handle; using it afterwards throws `null pointer passed to rust`.
  ([#23](https://github.com/micheltlutz/molt/issues/23))
- The `parallel` feature implies `ssg`. ([#22](https://github.com/micheltlutz/molt/issues/22))

### Security

Output now differs from Winged-Swift 3.0.1 in the cases below, on purpose; each is listed
in [`PORTING.md`](PORTING.md).

- **Comments can no longer close early.** `Node::comment("x--->")` used to leave a `-->`
  that ended the comment and let the rest through as markup. ([#2](https://github.com/micheltlutz/molt/issues/2))
- **Tag names are validated.** `Element::new("img src=x onerror=alert(1)")` was written
  verbatim, also from JavaScript through `new WElement(str)`. An element whose tag is not a
  letter followed by letters, digits, `-`, `_`, `.` or `:` is not rendered.
  ([#3](https://github.com/micheltlutz/molt/issues/3))
- **`<` and `>` in attribute values are escaped under raw-text ancestors** (`noscript`,
  `iframe`, `xmp`, `noembed`, `noframes`, `script`, `style`, `plaintext`), where
  `</noscript>` inside an `alt` ended the parent. Elsewhere the byte-for-byte output is
  unchanged. ([#5](https://github.com/micheltlutz/molt/issues/5))
- **`meta_name`, `meta_property`, `meta_charset` and `meta_http_equiv` escape their
  values.** ([#4](https://github.com/micheltlutz/molt/issues/4))
- **`StaticSiteGenerator`:** refuses symlinks on the way, paths that name no file (`""`,
  `.`, `a/`) and Windows drive prefixes; writes through unique temporary files (two pages
  with the same stem used to share one); `copy_asset` no longer deletes before it copies;
  `clean` refuses `.`, the working directory and its ancestors, `$HOME` and top-level
  directories. ([#6](https://github.com/micheltlutz/molt/issues/6),
  [#7](https://github.com/micheltlutz/molt/issues/7),
  [#8](https://github.com/micheltlutz/molt/issues/8))
- **URL policy:** also covers `ping`, `manifest`, `longdesc` and, per candidate, `srcset`;
  `srcdoc` is written empty unless `trusted_url`; on `iframe`, `frame`, `embed` and `object`
  a `data:` URL passes only if it is a raster image. `is_blocked_in(tag, …)` is the new
  tag-aware form. ([#25](https://github.com/micheltlutz/molt/issues/25))
- **XML:** `escape_xml` drops the characters XML 1.0 cannot hold; a sitemap `<loc>` or feed
  `<link>` that is not an absolute `http(s)` URL is left out; `isPermaLink="true"` is written
  only for an `http(s)` guid. ([#27](https://github.com/micheltlutz/molt/issues/27))
- **`RenderOptions::with_max_depth`** drops elements nested deeper than a limit, which is what
  bounds pretty output (quadratic in depth). ([#28](https://github.com/micheltlutz/molt/issues/28))
- **CI and release:** every action pinned to a commit SHA, read-only default token,
  `persist-credentials: false`, job timeouts, and a release that refuses a tag that is not
  on `main` or has no green CI run. ([#29](https://github.com/micheltlutz/molt/issues/29))

### Fixed

- `Document::render`, `accessibility::audit`, `Node::is_empty`, `Clone`, `PartialEq` and
  `Hash` recursed once per level and aborted the process on a deep tree that the renderer
  handled. All are iterative now, tested at 100,000 levels.
  ([#9](https://github.com/micheltlutz/molt/issues/9),
  [#10](https://github.com/micheltlutz/molt/issues/10),
  [#11](https://github.com/micheltlutz/molt/issues/11)) `Debug` is still recursive.
- `SECURITY.md` and the WASM docs described a 256-level limit and a function name that did
  not exist. ([#12](https://github.com/micheltlutz/molt/issues/12))

### Added

- `Markup`, `SsgError`, `ChangeFreq` and `ParseChangeFreqError`.
- `seo::json_ld`, `seo::script_json` and `escape_json_script`: JSON that is safe inside a
  `<script>`, where HTML escaping would corrupt it and no escaping would let `</script>`
  through. WASM `jsonLd`. ([#26](https://github.com/micheltlutz/molt/issues/26))
- `Element::from_static`, `RenderOptions::{indent, with_max_depth}`,
  `core::tags::is_valid_tag_name`, `core::url_policy::{is_http_url, is_blocked_in}`,
  `sitemap::{MAX_URLS, MAX_BYTES, SitemapGenerator::chunks}`.
- Trait impls: `Display` for `Node`, `Element` and `Document` (`{:#}` pretty-prints);
  `Hash`; `Default` for `Document`; `FromIterator` for `Node`; `Extend` for `Element`;
  `From<Vec<Node>>`, `From<Option<T>>` and `From<char>` for `Node`; `Render` for `&T`,
  `Box<T>` and `Option<T>`. ([#17](https://github.com/micheltlutz/molt/issues/17))
- WASM: `WDocument.addSeo`, `WDocument.renderWithOptions`, and on `WElement` `addClasses`,
  `rawChild`, `selfClosing`, `renderWithOptions` and `renderBytes`.
  ([#24](https://github.com/micheltlutz/molt/issues/24))
- `.claude/skills/molt`: a knowledge base for agents (security, Rust practice, performance,
  design patterns, governance, a decision log) whose lessons name the tests that guard them;
  `scripts/check-knowledge-base.sh` checks that those still exist.
- `tests/allocations.rs` pins allocation counts and type sizes; the benchmarks run at
  `opt-level = 3` and no longer time the `Drop`.

### Changed

- Tag names and attribute keys are `Cow<'static, str>` internally; `div()` makes no
  allocation. `Element::new` and `Attribute::new` still take `impl Into<String>`.
  ([#15](https://github.com/micheltlutz/molt/issues/15))
- `html!` formats interpolated values straight into their escaped form: 40% faster to
  build, with the same tree.
- Escapers copy the clean stretches between special characters in one step: prose-like text
  is about twice as fast, and text where every other character needs escaping about 2.8×
  slower (still ~400 MB/s).
- `Attribute::raw` is documented as one of the three escape hatches, with the reason.

### Not changed, deliberately

`panic = "abort"`, the three HTML/XML escapers, `escape_attribute` leaving `<` and `>` alone
outside raw-text ancestors, and the golden fixtures. Four tests were added after the fact
because they were claimed but missing ([#57](https://github.com/micheltlutz/molt/pull/57)).

## [0.1.0] - 2026-10-17

### Renamed

- **winged-rust is now Molt.** The crate is published as `molt-framework` (`molt` on
  crates.io is an unrelated project) and imported as `molt::`. The WebAssembly files are
  `molt.js` / `molt_bg.wasm`, and the repository moved to
  [micheltlutz/molt](https://github.com/micheltlutz/molt). `WINGED_UPDATE_FIXTURES` is now
  `MOLT_UPDATE_FIXTURES` and `WINGED_WASM_BUDGET_KB` is `MOLT_WASM_BUDGET_KB`. Issue links
  below still point to the winged-rust repository, where those issues live.

### Removed

- The npm package. The `wasm` feature and `scripts/build-wasm.sh` stay; the build now goes
  to `pkg/{web,node,bundler}` and is not published.

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

- The release workflow verifies (including `cargo publish --dry-run`) in a job with no
  registry token, then publishes from a fresh build with `cargo publish --locked
  --no-verify`, reading the `CARGO_REGISTRY_TOKEN` repository secret. CI runs the same
  dry run on every push.

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
