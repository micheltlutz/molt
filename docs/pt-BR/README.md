> 🇧🇷 Tradução para português do [README em inglês](../../README.md), que é o texto de
> referência. Os demais documentos (`SECURITY.md`, `MIGRATION.md`, `CHANGELOG.md`,
> `CONTRIBUTING.md`) existem só em inglês.

<p align="center">
  <img src="https://raw.githubusercontent.com/micheltlutz/molt/main/molt-logo.png" alt="Molt" width="220">
</p>

# Molt

> **Uma DSL de HTML segura e sem dependências para Rust e WebAssembly.**

[![crates.io](https://img.shields.io/crates/v/molt-framework.svg?logo=rust)](https://crates.io/crates/molt-framework)
[![docs.rs](https://img.shields.io/docsrs/molt-framework?logo=docsdotrs)](https://docs.rs/molt-framework)
[![CI](https://github.com/micheltlutz/molt/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/micheltlutz/molt/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](../../LICENSE)
[![Rust 1.85+](https://img.shields.io/badge/rust-1.85%2B-dea584.svg)](https://www.rust-lang.org)
[![WebAssembly](https://img.shields.io/badge/wasm32-supported-654ff0.svg)](#webassembly)
[![Sponsor](https://img.shields.io/github/sponsors/micheltlutz?logo=githubsponsors&label=sponsor)](https://github.com/sponsors/micheltlutz)

🌐 **[molt-framework.io](https://www.molt-framework.io)** · 📦 [crates.io](https://crates.io/crates/molt-framework) · 📚 [docs.rs](https://docs.rs/molt-framework) · 🚀 [Primeiros passos](GETTING_STARTED.md)

Molt é uma DSL embutida: você descreve a página em Rust, com um builder fluente ou com a macro
`html!`, e o compilador a verifica. O resultado é uma árvore de valores simples, renderizada
uma única vez como uma **string**, compacta ou indentada. Não há DOM, runtime, linguagem de
template nem reflexão. Por isso não existe nada para interpretar em tempo de execução e nada
para manter sincronizado.

Molt é só a camada de HTML: a árvore de elementos, o renderizador, metadados, uma auditoria de
acessibilidade, sitemaps, feeds, geração de site estático e uma build WebAssembly. Roteamento,
servidores e Markdown ficam com as crates que já fazem isso bem, e se conectam por meio de uma
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

Ou com a macro `html!`, que se expande nas mesmas chamadas do builder, nunca em um segundo
caminho de código:

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

## Por que Molt

**Seguro por construção.** A garantia central é que o texto e os valores de atributo que você
passa nunca viram marcação, e ela é aplicada no ponto em que o dado entra, não onde você se
lembra de aplicá-la.

- Texto e valores de atributo são escapados uma única vez, na entrada. Não há como esquecer
  nem como escapar duas vezes.
- Nomes de tag e de atributo são validados na hora de escrever. Um nome inválido é descartado,
  nunca "consertado".
- Atributos de URL (`href`, `src`, `action`, `srcset`, …) que resolvem para `javascript:`,
  `vbscript:` ou `data:text/html` são escritos como `#`, inclusive quando disfarçados com
  referências de caractere. `srcdoc` nunca é escrito com um valor que você não garantiu.
- JSON dentro de `<script>` passa por um escapador próprio, então um `</script>` em um valor
  não consegue fechar o elemento.
- `raw_text`, `Node::raw`, `Attribute::raw` e `trusted_url` são as únicas formas de contornar
  qualquer uma dessas proteções. Têm nomes próprios para que uma auditoria seja um único
  `grep`.
- `unsafe_code = "forbid"`, `cargo deny` sobre o grafo de dependências, quatro alvos de fuzzing
  e um [`SECURITY.md`](../../SECURITY.md) (em inglês) que mapeia cada garantia no OWASP Top 10.

```rust
# use molt::prelude::*;
assert_eq!(p().text("<script>alert(1)</script>").render(),
           "<p>&lt;script&gt;alert(1)&lt;/script&gt;</p>");
assert_eq!(a().attr("href", "javascript:alert(1)").render(), r##"<a href="#"></a>"##);
```

**Robusto diante de entrada hostil.** Renderizar, clonar, comparar, auditar e descartar uma
árvore usam sempre uma pilha explícita, nunca a pilha de chamadas. Assim, a profundidade do
aninhamento não consegue derrubar o processo. Uma árvore de 100.000 níveis é renderizada nos
dois modos na suíte de testes, e `RenderOptions::with_max_depth` limita a profundidade que vem
de fora. A saída é fixada byte a byte por fixtures *golden*: uma mudança na marcação é um teste
que falha, nunca uma surpresa em produção. A crate está na versão 1.x, então o que compila
hoje continua compilando, e o [`MIGRATION.md`](../../MIGRATION.md) (em inglês) lista toda
mudança que já alterou a saída.

**Leve.** Zero dependências obrigatórias: `cargo tree` com as features padrão mostra uma única
crate, esta. `wasm-bindgen` e `rayon` são features opcionais. A build WebAssembly tem cerca de
25 KB comprimida com gzip. O renderizador escreve a árvore inteira em um único buffer, o número
de alocações é verificado no CI, e `Node` é `Send + Sync`, então a geração em lote pode ser
paralelizada sem cerimônia.

**Rápido o bastante para não ser o gargalo.** Medido na máquina de desenvolvimento com
`cargo bench --bench render` (Criterion, `opt-level = 3`):

| benchmark | mediana |
| --- | --- |
| renderização compacta, lista de 10.000 itens | ≈ 400 µs |
| documento completo com 50 cards, indentado / compacto | ≈ 8 µs / 6,5 µs |

Montar a árvore custa mais do que renderizá-la; os dois custam muito menos do que a requisição
que pediu a página.

## Instalação

A crate está no [crates.io](https://crates.io/crates/molt-framework) e a documentação da API no
[docs.rs](https://docs.rs/molt-framework). Ela é publicada como `molt-framework` (`molt` no
crates.io é um projeto sem relação) e importada como `molt`.

```bash
cargo add molt-framework
```

ou, à mão, no `Cargo.toml`:

```toml
[dependencies]
molt-framework = "1.1"
```

Nunca usou Rust? O guia [`new-to-rust.md`](new-to-rust.md) vai da instalação do Rust até uma
página aberta no navegador, e explica como executar o projeto.

```rust
use molt::prelude::*;
```

Features:

| feature | padrão | o que faz |
| --- | :-: | --- |
| `ssg` | ✓ | `StaticSiteGenerator`: grava páginas e arquivos estáticos em disco |
| `wasm` | | bindings `wasm-bindgen` para JavaScript |
| `parallel` | | geração de várias páginas em paralelo, com `rayon` |

### Servindo a saída

Molt renderiza uma `String`, e esse é todo o contrato de integração com um servidor web: não
existe feature para `axum`, `actix` ou `tokio`, e nenhuma está planejada, para que o núcleo
continue sem dependências obrigatórias. Com o [Axum](https://crates.io/crates/axum), o `Html` já
aceita uma `String`, então um programa completo, a partir de um `cargo new` vazio, fica assim:

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

Rode `cargo run` e abra <http://localhost:3000>. `Document` não está no prelude; importe-o de
`molt`, como acima. Molt não depende de `axum` nem de `tokio`, então o seu projeto precisa
adicioná-los.

Se quiser que um tipo seu seja uma resposta, envolva o documento em um newtype na sua crate e
implemente a trait do framework nele. A regra do órfão permite isso, porque o newtype é seu:

```rust,ignore
use axum::response::{Html, IntoResponse, Response};

struct Page(molt::Document);

impl IntoResponse for Page {
    fn into_response(self) -> Response {
        Html(self.0.render()).into_response()
    }
}
```

O mesmo padrão serve para qualquer framework. Uma crate específica de framework (`molt-axum`,
por exemplo) é bem-vinda como projeto separado que dependa desta; veja o
[`ROADMAP.md`](../../ROADMAP.md).

## O que tem dentro

- **93 elementos HTML**, gerados a partir de uma única tabela. Veja
  [`docs/tag-catalog.md`](../tag-catalog.md).
- **API de builder fluente**: `set_id`, `add_class`, `add_classes`, `set_style`, `attr`,
  `data_attr(s)`, `aria_attr(s)`, `set_role`, `child`, `text`, `raw_text`
- **Macro `html!`**: aninhamento, atributos, `@if` / `@else`, `@for`
- **`Document`**: cuida do doctype e renderiza indentado por padrão
- **Trait `Layout`**: estruturas de página reutilizáveis
- **SEO**: Open Graph, Twitter Cards, o bloco `<meta>` comum e um `SeoBuilder`
- **Acessibilidade**: helpers de ARIA e uma auditoria que encontra imagens sem `alt`, botões
  sem nome acessível e iframes sem título
- **Sitemaps e RSS 2.0**
- **Geração de site estático**: gravação atômica, cópia de arquivos estáticos, paralelismo
  opcional
- **WebAssembly**: a mesma API a partir de JavaScript

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

Os bindings envolvem a árvore real, em vez de acumular strings já renderizadas. Por isso
`renderPretty()` funciona e o JavaScript usa exatamente o mesmo renderizador que o Rust.

**Os handles são consumidos.** Os métodos do builder e os argumentos de elemento movem o valor
para dentro do Rust. Um handle que você já repassou (ou do qual já encadeou uma chamada) deixa
de existir: usá-lo lança `null pointer passed to rust`. Encadeie as chamadas, ou guarde o
handle que cada chamada devolve.

## Limitações conhecidas

**A saída indentada cresce com o quadrado da profundidade.** Cada linha carrega uma indentação
por nível acima dela, então uma árvore muito profunda gera uma string muito grande. A
renderização em si não tem limite de profundidade (o escritor usa uma pilha explícita em vez de
recursão), mas vale conhecer esse tamanho quando a profundidade vem de uma entrada não
confiável. Veja o [`SECURITY.md`](../../SECURITY.md#rendering-depth-is-no-longer-bounded-by-the-stack)
(em inglês).

## Documentação

- [`new-to-rust.md`](new-to-rust.md): nunca usou Rust? Instale, crie um projeto, adicione o
  Molt e execute.
- [`GETTING_STARTED.md`](GETTING_STARTED.md): uma primeira página, escape, `html!`, SEO e um
  site em disco.
- [`MIGRATION.md`](../../MIGRATION.md) (inglês): da 0.1.0 para a 1.0.0, mudança por mudança.
- [`SECURITY.md`](../../SECURITY.md) (inglês): o que é garantido, os limites e como relatar um
  problema.
- [`ROADMAP.md`](../../ROADMAP.md) e [`CHANGELOG.md`](../../CHANGELOG.md) (inglês).
- Referência da API no [docs.rs](https://docs.rs/molt-framework).
- O site do projeto, feito com o próprio Molt: [www.molt-framework.io](https://www.molt-framework.io).

## Desenvolvimento

```bash
./scripts/verify.sh        # build, testes, clippy, fmt, catálogo, fixtures, smoke test
cargo test                 # testes unitários, de integração e de documentação
cargo bench                # desempenho da renderização
```

`verify.sh` é um superconjunto do CI. Rode-o antes de dizer que uma mudança está pronta.

## Contribuindo

Molt é código aberto e contribuições são bem-vindas. Um relato de bug com a entrada que o
provocou é tão útil quanto um patch.

- **Encontrou um bug ou quer uma funcionalidade?**
  [Abra uma issue](https://github.com/micheltlutz/molt/issues). O
  [`ROADMAP.md`](../../ROADMAP.md) diz o que está planejado, o que é só uma pista e o que está
  fora de propósito.
- **Encontrou um problema de segurança?** Por favor, não abra uma issue pública; use um
  [aviso de segurança privado](https://github.com/micheltlutz/molt/security/advisories/new).
  Veja o [`SECURITY.md`](../../SECURITY.md).
- **Vai enviar código?** Crie a branch a partir de `develop`, adicione um teste que verifique a
  marcação renderizada e rode `./scripts/verify.sh` antes de abrir o pull request. As regras e
  seus motivos estão no [`CONTRIBUTING.md`](../../CONTRIBUTING.md).
- **Usa um agente de IA?** Aponte-o para o [`AGENTS.md`](../../AGENTS.md), o mapa das regras em
  [`.claude/rules`](../../.claude/rules), dos slash commands, dos agentes de revisão e da base
  de conhecimento em [`.claude/skills/molt`](../../.claude/skills/molt/SKILL.md): regras de
  segurança, práticas de Rust e as lições aprendidas até chegar à 1.0. O estilo de código que
  todo contribuidor segue está em
  [`.claude/rules/code-style.md`](../../.claude/rules/code-style.md).

### Apoie o projeto

Se o Molt economiza o seu tempo, você pode ajudar a mantê-lo
**[patrocinando no GitHub](https://github.com/sponsors/micheltlutz)**. Dar uma estrela no
repositório, divulgá-lo e me contar o que você construiu com ele ajudam tanto quanto.

## O nome

*Molt* (muda) é a troca do exoesqueleto, o jeito como um crustáceo cresce: uma homenagem ao
Ferris e a de onde este projeto veio. Ele começou como um port de uma biblioteca de HTML em
Swift, criada para rodar produtos SaaS comerciais com pouca infraestrutura na nuvem, e foi para
Rust pela eficiência e pelas garantias. A casca que ele manteve é a API; o que ele largou foi o
excesso.

## Licença

MIT. Veja o [`LICENSE`](../../LICENSE).

Criado por [Michel Anderson Lutz Teixeira](https://micheltlutz.me).
