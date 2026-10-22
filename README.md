<p align="center">
  <img src="https://raw.githubusercontent.com/micheltlutz/molt/main/molt-logo.png" alt="Molt" width="220">
</p>

# Molt

> **A safe, dependency-free HTML DSL for Rust and WebAssembly.**

[![crates.io](https://img.shields.io/crates/v/molt-framework.svg?logo=rust)](https://crates.io/crates/molt-framework)
[![docs.rs](https://img.shields.io/docsrs/molt-framework?logo=docsdotrs)](https://docs.rs/molt-framework)
[![CI](https://github.com/micheltlutz/molt/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/micheltlutz/molt/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-dea584.svg)](https://www.rust-lang.org)
[![WebAssembly](https://img.shields.io/badge/wasm32-supported-654ff0.svg)](#webassembly)
[![Sponsor](https://img.shields.io/github/sponsors/micheltlutz?logo=githubsponsors&label=sponsor)](https://github.com/sponsors/micheltlutz)

📦 **[molt-framework on crates.io](https://crates.io/crates/molt-framework)** · 📚 [docs.rs](https://docs.rs/molt-framework) · 🚀 [Getting started](GETTING_STARTED.md) · 🇧🇷 [Português](docs/pt-BR/README.md)

Molt is an embedded DSL: you describe the page in Rust, with a fluent builder or the `html!`
macro, and the compiler checks it. The result is a tree of plain values rendered once as a
**string**, compact or pretty. There is no DOM, no runtime, no template language and no
reflection, so there is nothing to parse at run time and nothing to keep in sync.

It is the HTML layer and only that: the element tree, the renderer, metadata, an
accessibility audit, sitemaps, feeds, static site output and a WebAssembly build. Routing,
servers and Markdown stay in the crates that already do them well, and plug in through a
`String`.

```rust
use molt::prelude::*;
use molt::Document;

let page = Document::new(Some("pt-BR"))
    .head_children([
        meta().attr("charset", "UTF-8"),
        title().text("RideKeeper — track every service"),
        stylesheet("/css/style.css"),
    ])
    .body_children([
        header().child(nav().child(link_to("/").add_class("logo").text("RideKeeper"))),
        main_tag().child(
            article()
                .add_class("card p-4")
                .child(h1().text("Track every service"))
                .child(p().text("Fuel, tyres & chain — all in one place.")),
        ),
    ]);

println!("{}", page.render());
```

Or with the `html!` macro, which expands to the same builder calls, never a second code path:

```rust
use molt::{html, prelude::*};

let items = ["Fuel", "Tyres", "Chain"];
let markup = html! {
    section(class = "log") {
        h2 { "Service log" }
        ul { @for item in items { li { (item) } } }
    }
};
```

## Why Molt

**Secure by construction.** The one guarantee is that text and attribute values you pass in
cannot become markup, and it is enforced where the data enters, not where you remember to.

- Text and attribute values are escaped once, on entry. There is no way to forget and no way
  to double-escape.
- Tag and attribute names are validated at write time. An invalid one is dropped, never
  repaired.
- URL attributes (`href`, `src`, `action`, `srcset`, …) that resolve to `javascript:`,
  `vbscript:` or `data:text/html` are written as `#`, including through character references.
  `srcdoc` is never written with a value you did not vouch for.
- JSON inside `<script>` goes through its own escaper, so a `</script>` in a value cannot end
  the element.
- `raw_text`, `Node::raw`, `Attribute::raw` and `trusted_url` are the only ways around any of
  this. They are named so an audit is one `grep`.
- `unsafe_code = "forbid"`, `cargo deny` on the dependency graph, four fuzz targets, and
  [`SECURITY.md`](SECURITY.md) maps every guarantee onto the OWASP Top 10.

```rust
# use molt::prelude::*;
assert_eq!(p().text("<script>alert(1)</script>").render(),
           "<p>&lt;script&gt;alert(1)&lt;/script&gt;</p>");
assert_eq!(a().attr("href", "javascript:alert(1)").render(), r##"<a href="#"></a>"##);
```

**Robust under hostile input.** Rendering, cloning, comparing, auditing and dropping a tree
all walk an explicit stack, never the call stack, so nesting depth cannot be turned into a
process abort. A 100,000-level tree renders in both modes in the test suite, and
`RenderOptions::with_max_depth` bounds depth that comes from outside. Output is pinned
byte for byte by golden fixtures: a change in markup is a failing test, never a surprise in
production. The crate is 1.x, so what compiles today keeps compiling, and
[`MIGRATION.md`](MIGRATION.md) shows every change that ever altered output.

**Light.** Zero mandatory dependencies: `cargo tree` on the default features shows one crate,
this one. `wasm-bindgen` and `rayon` are opt-in features. The WebAssembly build is about
25 KB gzipped. The renderer writes the whole tree into one buffer, allocation counts are
asserted in CI, and `Node` is `Send + Sync`, so bulk generation parallelises without
ceremony.

**Fast enough that it is not the bottleneck.** Measured on the development machine with
`cargo bench --bench render` (Criterion, `opt-level = 3`):

| bench | median |
| --- | --- |
| compact render, 10,000-item list | ≈ 400 µs |
| full 50-card document, pretty / compact | ≈ 8 µs / 6.5 µs |

Building the tree costs more than rendering it; both are far below the cost of the request
that asked for the page.

## Install

The crate is on [crates.io](https://crates.io/crates/molt-framework) and its API
documentation on [docs.rs](https://docs.rs/molt-framework). It is published as
`molt-framework` — `molt` on crates.io is an unrelated project — and imported as `molt`.

```bash
cargo add molt-framework
```

or, by hand, in `Cargo.toml`:

```toml
[dependencies]
molt-framework = "1.1"
```

New to Rust? [`docs/new-to-rust.md`](docs/new-to-rust.md) goes from installing Rust to a page
open in your browser, and says how to run it.

```rust
use molt::prelude::*;
```

Features:

| feature | default | what it does |
| --- | :-: | --- |
| `ssg` | ✓ | `StaticSiteGenerator` — writes pages and assets to disk |
| `wasm` | | `wasm-bindgen` bindings for JavaScript |
| `parallel` | | `rayon`-backed bulk page generation |

### Serving the output

Molt renders a `String`, and that is the whole integration contract with a web server: there
is no `axum`, `actix` or `tokio` feature, and none is planned, so the core keeps zero
mandatory dependencies. With [Axum](https://crates.io/crates/axum), `Html` already accepts a
`String`, so a complete program, from an empty `cargo new`, looks like this:

```bash
cargo add molt-framework axum
cargo add tokio --features macros,rt-multi-thread
```

```rust,ignore
use axum::{Router, response::Html, routing::get};
use molt::Document;
use molt::prelude::*;

async fn home() -> Html<String> {
    let page = Document::new(Some("en"))
        .head_children([meta().attr("charset", "UTF-8"), title().text("Molt + Axum")])
        .body_children([h1().text("Track every service")]);
    Html(page.render())
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/", get(home));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

`cargo run`, then open <http://localhost:3000>. `Document` is not in the prelude; import it
from `molt` as above. Molt does not depend on `axum` or `tokio`, so your project adds them
itself.

If you want your own type to be a response, wrap the document in a newtype in your crate and
implement the framework's trait there; the orphan rule allows it because the newtype is yours:

```rust,ignore
use axum::response::{Html, IntoResponse, Response};

struct Page(molt::Document);

impl IntoResponse for Page {
    fn into_response(self) -> Response {
        Html(self.0.render()).into_response()
    }
}
```

The same pattern works for any framework. A framework-specific crate (`molt-axum`, say) is
welcome as a separate project that depends on this one; see [`ROADMAP.md`](ROADMAP.md).

## What is in it

- **93 HTML elements**, generated from one table — see [`docs/tag-catalog.md`](docs/tag-catalog.md)
- **Fluent builder API** — `set_id`, `add_class`, `add_classes`, `set_style`, `attr`,
  `data_attr(s)`, `aria_attr(s)`, `set_role`, `child`, `text`, `raw_text`
- **`html!` macro** — nesting, attributes, `@if` / `@else`, `@for`
- **`Document`** — owns the doctype, renders pretty by default
- **`Layout` trait** — reusable page shells
- **SEO** — Open Graph, Twitter Cards, the common `<meta>` block, and a `SeoBuilder`
- **Accessibility** — ARIA helpers plus an audit that finds images without `alt`, buttons
  with no accessible name, iframes without a title
- **Sitemaps and RSS 2.0**
- **Static site generation** — atomic writes, asset copying, optional parallelism
- **WebAssembly** — the same API from JavaScript

## WebAssembly

```bash
wasm-pack build --target web --features wasm
```

```js
import init, { element, WDocument } from "./pkg/molt.js";
await init();

const page = new WDocument("pt-BR")
  .addBody(element("h1").text("Track every service"))
  .addBody(element("p").addClass("lead").text("Fuel, tyres & chain."));

document.body.innerHTML = page.render();
```

The bindings wrap the real tree rather than accumulating pre-rendered strings, so
`renderPretty()` works and JavaScript drives exactly the renderer Rust does.

**Handles are consumed.** Builder methods and element arguments move their value into Rust,
so a handle you have already passed on (or chained from) is gone: using it throws
`null pointer passed to rust`. Chain calls, or keep the handle each call returns.

## Known limitations

**Pretty output is quadratic in nesting depth.** Every line carries one indent string per
level above it, so a very deep tree renders a very large string. Rendering itself has no
depth limit — the writer walks an explicit stack rather than recursing — but the size is
worth knowing about when depth comes from untrusted input. See
[`SECURITY.md`](SECURITY.md#rendering-depth-is-no-longer-bounded-by-the-stack).

## Documentation

- [`docs/new-to-rust.md`](docs/new-to-rust.md): never used Rust? install, create a project, add Molt, run it.
- [`GETTING_STARTED.md`](GETTING_STARTED.md): a first page, escaping, `html!`, SEO, a site on disk.
- [`MIGRATION.md`](MIGRATION.md): from 0.1.0 to 1.0.0, change by change.
- [`SECURITY.md`](SECURITY.md): what is guaranteed, the limits, how to report a problem.
- [`ROADMAP.md`](ROADMAP.md) and [`CHANGELOG.md`](CHANGELOG.md).
- API reference on [docs.rs](https://docs.rs/molt-framework).

## Development

```bash
./scripts/verify.sh        # build, test, clippy, fmt, catalog, fixtures, smoke test
cargo test                 # unit, integration and doc tests
cargo bench                # render performance
```

`verify.sh` is a superset of CI. Run it before reporting that a change is done.

## Contributing

Molt is open source and contributions are welcome — a bug report with the input that
triggered it is as useful as a patch.

- **Found a bug or want a feature?** [Open an issue](https://github.com/micheltlutz/molt/issues).
  [`ROADMAP.md`](ROADMAP.md) says what is planned, what is a lead, and what is deliberately
  not.
- **Found a security problem?** Please do not open a public issue; use
  [a private security advisory](https://github.com/micheltlutz/molt/security/advisories/new).
  See [`SECURITY.md`](SECURITY.md).
- **Sending code?** Branch from `develop`, add a test that asserts on the rendered markup, and
  run `./scripts/verify.sh` before you open the pull request. The rules and the reasons behind
  them are in [`CONTRIBUTING.md`](CONTRIBUTING.md).
- **Using an AI agent?** Point it at [`AGENTS.md`](AGENTS.md), the map over the rules in
  [`.claude/rules`](.claude/rules), the slash commands, the review agents and the knowledge base
  in [`.claude/skills/molt`](.claude/skills/molt/SKILL.md): security rules, Rust practices and
  the lessons learned getting to 1.0. The code style every contributor follows is
  [`.claude/rules/code-style.md`](.claude/rules/code-style.md).

### Support the project

If Molt saves you time, you can help keep it maintained by
**[sponsoring on GitHub](https://github.com/sponsors/micheltlutz)**. Starring the repository,
sharing it, and telling me what you built with it help just as much.

## The name

*Molt* is the shedding of an exoskeleton, the way a crustacean grows: a nod to Ferris, and to
where this project came from. It started as a port of a Swift HTML library, built to run
commercial SaaS products on small cloud footprints, and moved to Rust for the efficiency and
the guarantees. The shell it kept is the API; what it shed is the overhead.

## License

MIT — see [`LICENSE`](LICENSE).

Created by [Michel Anderson Lutz Teixeira](https://micheltlutz.me).
