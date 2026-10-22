# Primeiros passos

> Tradução do [guia em inglês](../../GETTING_STARTED.md), que é o texto de referência.

Molt monta uma **string** de HTML. Você monta uma árvore de valores e a renderiza uma única
vez. Não há DOM, runtime nem linguagem de template. Todo trecho de código desta página é
compilado e executado por `cargo test --doc`.

## Instalação

Nunca usou Rust? O guia [`new-to-rust.md`](new-to-rust.md) vai da instalação do Rust até uma
página aberta no navegador. A versão curta vem a seguir.

**Em um projeto novo:**

```bash
cargo new hello-molt
cd hello-molt
cargo add molt-framework
```

**Em um projeto que você já tem**, rode o último comando na pasta onde está o `Cargo.toml`:

```bash
cargo add molt-framework
```

Ou adicione a linha você mesmo:

```toml
[dependencies]
molt-framework = "1.1"
```

O pacote se chama `molt-framework` (`molt` no crates.io é um projeto sem relação), e você o
importa como `molt`.

## Executando

Coloque os trechos abaixo em `src/main.rs`, dentro de `fn main() { … }`, e execute com:

```bash
cargo run
```

`Element::render()` devolve uma `String`. Imprima-a com `println!("{}", card.render());` ou
grave-a em um arquivo, como mostra a seção [Gravando um site em disco](#gravando-um-site-em-disco).

## Seu primeiro elemento

```rust
use molt::prelude::*;

let card = div()
    .add_class("card")
    .child(h1().text("Fuel & chain"))
    .child(p().text("All in one place."));

assert_eq!(
    card.render(),
    r#"<div class="card"><h1>Fuel &amp; chain</h1><p>All in one place.</p></div>"#
);
```

`Element::render()` gera uma linha só. `render_pretty()` indenta; `render_with(&RenderOptions)`
aceita todo o resto.

## O texto é escapado para você, uma única vez

`text()` e `attr()` escapam o valor quando ele entra na árvore. Nunca escape você mesmo, ou
você vai obter `&amp;amp;`.

```rust
use molt::prelude::*;

let hostile = r#""><script>alert(1)</script>"#;
let link = a().attr("href", "/search").attr("title", hostile).text(hostile);

assert_eq!(
    link.render(),
    concat!(
        r#"<a href="/search" title="&quot;><script>alert(1)</script>">"#,
        "&quot;&gt;&lt;script&gt;alert(1)&lt;/script&gt;</a>"
    )
);
```

Dentro do atributo, `"` vira `&quot;`, o que basta para manter o valor entre as aspas; `<` e
`>` são inofensivos ali e ficam como estão. No texto, todos eles são escapados. (Dentro de
`<noscript>`, `<iframe>`, `<script>` e dos outros elementos de texto bruto, o parser lê o
conteúdo de outra forma, e o Molt escapa `<` e `>` também nos atributos.)

As saídas de emergência são `raw_text`, `Node::raw` e `Attribute::raw`. Elas têm esses nomes
para que você possa encontrá-las com `grep`; use-as só para marcação que você mesmo escreveu.

## Uma página inteira

`Document` acrescenta o doctype e o `<html>`, e renderiza indentado por padrão.

```rust
use molt::prelude::*;
use molt::Document;

let page = Document::new(Some("pt-BR"))
    .head_children([
        meta().attr("charset", "UTF-8"),
        title().text("RideKeeper"),
    ])
    .body_children([h1().text("Track every service")]);

let html = page.render();
assert!(html.starts_with("<!DOCTYPE html>\n<html lang=\"pt-BR\">"));
assert!(html.contains("<h1>Track every service</h1>"));
```

## A macro `html!`

É açúcar sintático sobre o mesmo builder, nunca um segundo caminho de código.

```rust
use molt::{html, prelude::*};

let names = ["Ana", "Bruno"];
let list = html! {
    ul(class = "people") {
        @for name in names { li { (name) } }
    }
};

assert_eq!(
    list.render(),
    r#"<ul class="people"><li>Ana</li><li>Bruno</li></ul>"#
);
```

## Metadados para mecanismos de busca

```rust
use molt::prelude::*;
use molt::seo::{SeoBuilder, json_ld};

let tags = SeoBuilder::new("RideKeeper", "Motorcycle maintenance companion")
    .url("https://ridekeeper.example")
    .build();
let head: String = tags.iter().map(Render::render).collect();
assert!(head.contains(r#"<meta property="og:title" content="RideKeeper">"#));

// JSON-LD: seguro de embutir mesmo que um campo contenha `</script>`.
let ld = json_ld(r#"{"@type":"WebSite","name":"RideKeeper"}"#);
assert!(ld.render().starts_with(r#"<script type="application/ld+json">"#));
```

## Gravando um site em disco

Precisa da feature `ssg`, que é a padrão. As páginas são gravadas de forma atômica, e um
caminho que sairia do diretório de saída é recusado.

```rust,no_run
use molt::prelude::*;
use molt::ssg::{SsgError, StaticSiteGenerator};
use molt::Document;

fn main() -> Result<(), SsgError> {
    let site = StaticSiteGenerator::new("dist");
    site.clean(true)?;
    site.generate(
        &Document::new(Some("en")).body_children([h1().text("Home")]),
        "index.html",
        &RenderOptions::pretty(),
    )
}
```

## No navegador

Gere a build com `wasm-pack build --release --target web -- --features wasm` e então:

```js
import init, { element, WDocument } from "./pkg/molt.js";
await init();

const page = new WDocument("en")
  .addBody(element("h1").text("Hello"));
document.body.innerHTML = page.render();
```

Toda chamada do builder **consome** o handle que você passa. Encadeie as chamadas, ou guarde o
handle que cada uma devolve; reutilizar um handle lança `null pointer passed to rust`.

## Para onde ir depois

- [`README.md`](README.md): o que há na crate e o que ela não faz.
- [`MIGRATION.md`](../../MIGRATION.md) (inglês): vindo da 0.1.0.
- [`SECURITY.md`](../../SECURITY.md) (inglês): o que é garantido e os limites dessa garantia.
  Leia antes de renderizar qualquer coisa que um usuário digitou.
- [`docs/tag-catalog.md`](../tag-catalog.md): todos os construtores de elemento.
- [www.molt-framework.io](https://www.molt-framework.io): o site do projeto, feito com o próprio Molt.
