# Nunca usou Rust? Comece aqui

> Tradução do [guia em inglês](../new-to-rust.md), que é o texto de referência.

Esta página leva você de uma máquina vazia até uma página feita com Molt aberta no navegador.
Ela presume que você nunca usou Rust. Se você já tem um projeto Rust, vá direto para
[Adicione o Molt a um projeto que você já tem](#adicione-o-molt-a-um-projeto-que-você-já-tem).

Cada comando abaixo foi executado em um projeto limpo antes de ser escrito aqui.

## 1. Instale o Rust

O Rust vem com duas ferramentas que você vai usar o tempo todo: `rustc` (o compilador) e
`cargo` (o gerenciador de pacotes e ferramenta de build, parecido com `npm` ou `pip`). O
instalador oficial, o `rustup`, instala as duas.

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

No Windows, baixe o `rustup-init.exe` em <https://rustup.rs>. Aceite as opções padrão, depois
**abra um novo terminal** para que a mudança no `PATH` valha, e confira se funcionou:

```bash
rustc --version
cargo --version
```

O Molt exige Rust 1.85 ou mais novo. Uma instalação recente do `rustup` já atende; se a sua for
antiga, `rustup update` a atualiza.

## 2. Crie um projeto

```bash
cargo new hello-molt
cd hello-molt
```

O `cargo new` cria uma pasta com dois arquivos que importam:

| arquivo | o que é |
| --- | --- |
| `Cargo.toml` | o manifesto do projeto: o nome dele e as bibliotecas de que depende |
| `src/main.rs` | onde o seu programa começa, em uma função chamada `main` |

## 3. Adicione o Molt

```bash
cargo add molt-framework
```

Isso acrescenta uma linha em `[dependencies]`, no `Cargo.toml`. Você também pode escrevê-la à
mão:

```toml
[dependencies]
molt-framework = "2.0"
```

O pacote se chama `molt-framework` (o nome `molt` no crates.io pertence a um projeto sem
relação), mas no seu código você o importa como `molt`.

## 4. Escreva sua primeira página

Troque todo o conteúdo de `src/main.rs` por:

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

## 5. Execute

```bash
cargo run
```

Na primeira execução o Molt é baixado e compilado, então leva um instante. Depois o programa
roda e imprime a página:

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

É só isso que o Molt faz: monta uma string de HTML. O que fazer com ela é com você. As duas
escolhas mais comuns são um arquivo em disco (próximo passo) e a resposta de um servidor web
(veja [Servindo a saída](README.md#servindo-a-saída)).

## 6. Grave em um arquivo e abra no navegador

Altere o `src/main.rs` para que ele salve a página em vez de imprimi-la:

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

Execute de novo:

```bash
cargo run
```

Agora o arquivo `dist/index.html` existe e você pode abri-lo direto no navegador (dê um duplo
clique, ou use `open dist/index.html` no macOS, `xdg-open dist/index.html` no Linux,
`start dist/index.html` no Windows).

Para ver a página do jeito que um site de verdade é servido, inicie um servidor local nessa
pasta:

```bash
python3 -m http.server --directory dist
```

e acesse <http://localhost:8000>. Pressione `Ctrl+C` para encerrá-lo.

O Molt não observa os seus arquivos. Depois de editar o `src/main.rs`, rode `cargo run` de novo
e atualize o navegador.

## Sirva a página com um servidor web (Axum)

O Molt não inclui um servidor web. O [Axum](https://crates.io/crates/axum) é um dos mais
populares e depende do [Tokio](https://crates.io/crates/tokio), o runtime assíncrono do Rust.
O seu projeto precisa adicionar os três. Se esquecer o `axum` ou o `tokio`, você verá
`cannot find module or crate`.

```bash
cargo add molt-framework axum
cargo add tokio --features macros,rt-multi-thread
```

Troque o conteúdo de `src/main.rs` por:

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

Rode `cargo run` e abra <http://localhost:3000>. Dois detalhes que costumam atrapalhar:

- `Document` **não** está em `molt::prelude`; importe-o com `use molt::Document;`.
- Chamar `home().await` não basta para servir nada: isso só monta a página. Quem atende as
  requisições é uma rota (`Router::new().route("/", get(home))`) junto com `axum::serve`.

## Adicione o Molt a um projeto que você já tem

Na pasta onde está o seu `Cargo.toml`:

```bash
cargo add molt-framework
```

Depois, coloque `use molt::prelude::*;` onde você monta o HTML. Os mesmos dois passos valem se
o projeto for uma biblioteca ou um servidor web como o Axum: o Molt só produz uma `String`,
então ele se encaixa em qualquer lugar onde você consiga devolver uma.

## Rode os exemplos do repositório

O repositório traz exemplos executáveis. Clone-o e rode um:

```bash
git clone https://github.com/micheltlutz/molt
cd molt
cargo run --example static_site
```

Isso grava em `dist/` um site pequeno, com sitemap e feed RSS. O outro exemplo é o
`marketing_page`.

## Quando algo dá errado

| você vê | o que significa | o que fazer |
| --- | --- | --- |
| `command not found: cargo` | o terminal não sabe onde o Rust está | abra um novo terminal, ou rode `source "$HOME/.cargo/env"` |
| `could not find Cargo.toml` | você não está dentro da pasta do projeto | rode `cd hello-molt` antes |
| `cannot find module or crate molt` | falta a dependência | rode `cargo add molt-framework` na pasta do projeto |
| `cannot find type Document` | `Document` não está no prelude | acrescente `use molt::Document;` |
| `cannot find module or crate axum` (ou `tokio`) | essa dependência nunca foi adicionada | `cargo add axum` e `cargo add tokio --features macros,rt-multi-thread` |
| `cannot find function h1` (ou `div`, `p`, …) | o prelude não foi importado | acrescente `use molt::prelude::*;` |
| um erro que cita `rustc` e um número de versão | o seu Rust é mais antigo que o 1.85 do Molt | `rustup update` |
| o navegador mostra a página antiga | o Molt só roda quando você o executa | rode `cargo run` de novo e atualize o navegador |

Os erros do compilador do Rust são longos, mas em geral dizem exatamente o que mudar. Leia o
primeiro e ignore os outros até corrigi-lo.

## Para onde ir depois

- [`GETTING_STARTED.md`](GETTING_STARTED.md): escape, a macro `html!`, SEO e um site inteiro.
- [O livro "The Rust Programming Language"](https://doc.rust-lang.org/book/) (inglês): a
  introdução oficial e gratuita à linguagem.
- [docs.rs/molt-framework](https://docs.rs/molt-framework): a referência da API.
- [www.molt-framework.io](https://www.molt-framework.io): o site do projeto, feito com o próprio Molt.
