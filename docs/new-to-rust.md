# New to Rust? Start here

🇧🇷 [Versão em português](pt-BR/new-to-rust.md)

This page takes you from an empty machine to a Molt page open in your browser. It assumes you
have never used Rust. If you already have a Rust project, jump to
[Add Molt to a project you already have](#add-molt-to-a-project-you-already-have).

Every command below was run on a clean project before it was written down.

## 1. Install Rust

Rust comes with two tools you will use all the time: `rustc` (the compiler) and `cargo` (the
package manager and build tool, like `npm` or `pip`). The official installer, `rustup`, gets
you both.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

On Windows, download `rustup-init.exe` from <https://rustup.rs> instead. Accept the defaults,
then **open a new terminal** so the `PATH` change takes effect, and check it worked:

```bash
rustc --version
cargo --version
```

Molt needs Rust 1.85 or newer. A fresh `rustup` install is newer than that; if yours is old,
`rustup update` brings it up to date.

## 2. Create a project

```bash
cargo new hello-molt
cd hello-molt
```

`cargo new` makes a folder with two things that matter:

| file | what it is |
| --- | --- |
| `Cargo.toml` | the project's manifest: its name, and the libraries it depends on |
| `src/main.rs` | where your program starts, in a function called `main` |

## 3. Add Molt

```bash
cargo add molt-framework
```

This adds one line under `[dependencies]` in `Cargo.toml`. You could also write it by hand:

```toml
[dependencies]
molt-framework = "1.1"
```

The package is called `molt-framework` (the name `molt` on crates.io belongs to an unrelated
project), but in your code you import it as `molt`.

## 4. Write your first page

Replace everything in `src/main.rs` with:

```rust,ignore
use molt::Document;
use molt::prelude::*;

fn main() {
    let page = Document::new(Some("en"))
        .head_children([
            meta().attr("charset", "UTF-8"),
            title().text("Hello, Molt"),
        ])
        .body_children([
            h1().text("Hello, Molt!"),
            p().text("This page was built with Rust."),
        ]);

    println!("{}", page.render());
}
```

## 5. Run it

```bash
cargo run
```

The first run downloads and compiles Molt, so it takes a moment. Then your program runs and
prints the page:

```html
<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8">
    <title>Hello, Molt</title>
  </head>
  <body>
    <h1>Hello, Molt!</h1>
    <p>This page was built with Rust.</p>
  </body>
</html>
```

That is all Molt does: it builds a string of HTML. What you do with the string is up to you.
The two common choices are a file on disk (next step) and a response from a web server (see
[Serving the output](../README.md#serving-the-output)).

## 6. Write it to a file and open it in the browser

Change `src/main.rs` so it saves the page instead of printing it:

```rust,ignore
use molt::Document;
use molt::prelude::*;
use molt::ssg::{SsgError, StaticSiteGenerator};

fn main() -> Result<(), SsgError> {
    let page = Document::new(Some("en"))
        .head_children([
            meta().attr("charset", "UTF-8"),
            title().text("Hello, Molt"),
        ])
        .body_children([
            h1().text("Hello, Molt!"),
            p().text("This page was built with Rust."),
        ]);

    let site = StaticSiteGenerator::new("dist");
    site.generate(&page, "index.html", &RenderOptions::pretty())?;
    println!("Wrote dist/index.html");
    Ok(())
}
```

Run it again:

```bash
cargo run
```

`dist/index.html` now exists, and you can open it straight in your browser (double-click it,
or `open dist/index.html` on macOS, `xdg-open dist/index.html` on Linux,
`start dist/index.html` on Windows).

To see it the way a real site is served, start a local server in that folder:

```bash
python3 -m http.server --directory dist
```

and visit <http://localhost:8000>. Press `Ctrl+C` to stop it.

Molt does not watch your files. After you edit `src/main.rs`, run `cargo run` again and
refresh the browser.

## Serve it from a web server (Axum)

Molt does not include a web server. [Axum](https://crates.io/crates/axum) is a popular one, and
it needs [Tokio](https://crates.io/crates/tokio), Rust's async runtime. Your project must add
all three; forgetting `axum` or `tokio` gives `cannot find module or crate`.

```bash
cargo add molt-framework axum
cargo add tokio --features macros,rt-multi-thread
```

Replace `src/main.rs` with:

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
    println!("Listening on http://localhost:3000");
    axum::serve(listener, app).await.unwrap();
}
```

Run `cargo run` and open <http://localhost:3000>. Two details that trip people up:

- `Document` is **not** in `molt::prelude`; import it with `use molt::Document;`.
- Calling `home().await` is not enough to serve anything: it only builds the page. A route
  (`Router::new().route("/", get(home))`) and `axum::serve` are what answer requests.

## Add Molt to a project you already have

From the folder that holds your `Cargo.toml`:

```bash
cargo add molt-framework
```

Then add `use molt::prelude::*;` where you build HTML. If the project is a library, or a web
server such as Axum, the same two steps apply; Molt only produces a `String`, so it fits
wherever you can return one.

## Run the examples from the repository

The repository ships runnable examples. Clone it and run one:

```bash
git clone https://github.com/micheltlutz/molt
cd molt
cargo run --example static_site
```

That writes a small site, with a sitemap and an RSS feed, into `dist/`. The other example is
`marketing_page`.

## When something goes wrong

| you see | what it means | what to do |
| --- | --- | --- |
| `command not found: cargo` | the terminal does not know where Rust is | open a new terminal, or run `source "$HOME/.cargo/env"` |
| `could not find Cargo.toml` | you are not inside the project folder | `cd hello-molt` first |
| `cannot find module or crate molt` | the dependency is missing | run `cargo add molt-framework` in the project folder |
| `cannot find type Document` | `Document` is not in the prelude | add `use molt::Document;` |
| `cannot find module or crate axum` (or `tokio`) | that dependency was never added | `cargo add axum`, `cargo add tokio --features macros,rt-multi-thread` |
| `cannot find function h1` (or `div`, `p`, …) | the prelude is not imported | add `use molt::prelude::*;` |
| an error mentioning `rustc` and a version number | your Rust is older than Molt's 1.85 | `rustup update` |
| the browser shows the old page | Molt only runs when you run it | `cargo run` again, then refresh |

Compiler errors in Rust are long but usually say exactly what to change; read the first one
and ignore the rest until it is fixed.

## Where to go next

- [`GETTING_STARTED.md`](../GETTING_STARTED.md): escaping, the `html!` macro, SEO, a whole
  site.
- [The Rust Book](https://doc.rust-lang.org/book/): the official, free introduction to the
  language.
- [docs.rs/molt-framework](https://docs.rs/molt-framework): the API reference.
- [www.molt-framework.io](https://www.molt-framework.io): the project site, built with Molt itself.
