# Roadmap

What comes after 1.0.0. These are intentions and leads, **not promises or dates**; the
authority on what is planned is the [issue tracker](https://github.com/micheltlutz/molt/issues).
What 1.0.0 contains is in [`CHANGELOG.md`](CHANGELOG.md).

## The 1.x line

**Compatibility.** Anything in the public API at 1.0.0 keeps working across 1.x. New variants
and fields are possible on the `#[non_exhaustive]` types; new required methods on `Render`
are not. A change to rendered output for valid input needs a fixture diff and a line in
`MIGRATION.md`'s *Output changes* table; a change that only affects input that used to be
unsafe is a security fix and goes in a patch release.

## Likely next

- **A fuzz target for what 2.0 added.** `.github/workflows/fuzz.yml` runs the four targets
  (`scripts/fuzz.sh`: both escapers, the URL policy, the JSON-in-script escaper) every night.
  `Element::csp_hash`'s SHA-256, `copy_dir` and `SitemapUrl::alternate` take input from
  outside and have none yet.
- **Keep the pins current.** Dependabot proposes SHA bumps for the actions; each needs a
  look at the action's changelog.

## Leads, if there is a reason

Each of these was measured or reasoned and **not** done; the evidence is on the issue.
Reopen with a benchmark that shows the cost, not with "it should be faster".

- `Node::Element(Box<Element>)`: `Node` is 104 bytes, so a text node costs 104. Boxing would
  cost one allocation per element child. ([#37](https://github.com/micheltlutz/molt/issues/37))
- An `O(depth)` work stack instead of `O(width)`. ([#36](https://github.com/micheltlutz/molt/issues/36))
- Caching attribute validity for trees that are rendered many times.
  ([#34](https://github.com/micheltlutz/molt/issues/34))

## Beyond the HTML layer

A `Site`/`Page` model, a Markdown parser, a command-line tool, an Atom feed and a `robots.txt`
generator are out of scope for the core crate, and so is any web-framework integration:
Markdown has `pulldown-cmark`, and a server has its own routing and its own response types. If one of them is wanted, it would be a separate crate that depends on this
one, not a feature of it.

## Deliberately not planned

- **`unsafe`.** `unsafe_code = "forbid"` stays; some micro-optimisations are simply
  unavailable, and the benchmarks are read with that in mind.
- **A public `Visitor`, `Escaper` or strategy trait.** One consumer; frozen API.
  ([`design-patterns.md`](.claude/skills/molt/references/design-patterns.md))
- **A global render switch**, **sorting attributes**, **a `Drop` on `Node`**.
- **SIMD in the escaper**, a custom allocator, changing `panic = "abort"`.
- **`console_error_panic_hook`, or any other dependency in the WebAssembly build** beyond
  `wasm-bindgen`. With `panic = "abort"` a panic is a trap without a message, which is
  acceptable for a library that must not panic on valid input. Sitemap and feed bindings
  wait for a JavaScript user who needs them. ([#78](https://github.com/micheltlutz/molt/issues/78))
- **A cargo feature for a web framework** (`axum`, `actix`, `rocket`, …) or an async streaming
  render. Either would tie the crate's release cycle to `tokio`/`http`/`bytes`. The output is a
  `String`, `Html(page.render())` is the integration, and a newtype in the caller's crate gets
  `IntoResponse` (see the README, *Serving the output*). A `molt-axum` crate is welcome as a
  separate project. ([#92](https://github.com/micheltlutz/molt/issues/92),
  [#95](https://github.com/micheltlutz/molt/issues/95))
