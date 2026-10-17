# Spec: Transcrevendo Winged-Swift para Rust + WASM (Molt)

> **Tipo de Documento:** Contexto Técnico & Blueprint de Arquitetura para LLMs  
> **Objetivo:** Orientar um modelo de linguagem (LLM) a reimplementar a biblioteca Swift `Winged-Swift` em **Rust** com suporte nativo a **WebAssembly (WASM)**.  
> **Projeto de Origem:** [micheltlutz/Winged-Swift](https://github.com/micheltlutz/Winged-Swift)  
> **Projeto Alvo:** `molt` (Crate Rust + Módulo WASM)

---

## 1. Visão Geral & Objetivos

### 1.1 Contexto do Winged-Swift
`Winged-Swift` é uma biblioteca DSL em Swift para geração de HTML de forma segura (*type-safe*), expressiva e modular. Ela se baseia no padrão de projeto **Composite**, permitindo construir árvores DOM estruturadas usando uma sintaxe declarativa e fluente.

Principais recursos a preservar:
* **DSL Fluente & Composta:** Métodos encadeados para adicionar classes, IDs, estilos, atributos `data-*` e `aria-*`.
* **Segurança XSS por Padrão:** Sanitização automática de conteúdo textual.
* **Tags Semânticas HTML5:** Suporte completo para `<article>`, `<section>`, `<nav>`, `<figure>`, etc.
* **Helpers SEO:** Geração facilitada de Meta tags, Open Graph (OG) e Twitter Cards.
* **Saída Formatada (Pretty Print):** Opção de renderização minificada ou formatada com indentação.

### 1.2 Metas para o Molt
1. **Paridade de Funcionalidades:** Mapear a API do Swift para construções idiomáticas em Rust.
2. **Zero-Cost Abstractions:** Aproveitar o sistema de *Ownership* e *Traits* do Rust para zerar a sobrecarga em tempo de execução (*runtime overhead*).
3. **Suporte NATIVO a WebAssembly (WASM):** Permitir a compilação para `wasm32-unknown-unknown`, gerando um pacote NPM para ser consumido via JavaScript/TypeScript no navegador ou Node.js.
4. **Concorrência Segura:** Permitir geração de HTML em massa (ex: geradores de sites estáticos) utilizando processamento paralelo (`rayon`).

---

## 2. Mapeamento de Arquitetura: Swift vs. Rust

| Conceito em Swift (`Winged-Swift`) | Mapeamento Idiomático em Rust (`molt`) |
| :--- | :--- |
| `protocol HTMLRenderable` | `pub trait Render { fn render(&self) -> String; fn pretty_render(&self, indent: usize) -> String; }` |
| Structs de Elementos (`Div`, `P`, `H1`) | `struct Element` genérica ou structs/enums específicos com suporte ao padrão Builder |
| Closures / Result Builders (`@resultBuilder`) | **Opção A:** Builder API encadeada (`.child(...)`)<br>**Opção B:** Procedural Macros (`html! { div { p { "Texto" } } }`) |
| Encodamento XSS (`escapeContent`) | Utilização de crate de sanitização (ex: `html_escape`) ou implementação zero-dep de sanitização de strings. |
| ARC (Automatic Reference Counting) | Sem alocação dinâmica desnecessária; uso de `Cow<'a, str>` ou `String` com posse única (*Ownership*). |

---

## 3. Especificação do Motor Core (Rust Engine)

### 3.1 Abstração do Nós da Árvore HTML

```rust
use std::collections::HashMap;
use std::fmt;

/// Representa a opção de formatação de saída.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderMode {
    Compact,
    Pretty { indent_size: usize },
}

/// Trait principal implementado por qualquer nó HTML.
pub trait Render {
    fn render(&self) -> String {
        self.render_mode(RenderMode::Compact, 0)
    }

    fn render_pretty(&self) -> String {
        self.render_mode(RenderMode::Pretty { indent_size: 2 }, 0)
    }

    fn render_mode(&self, mode: RenderMode, depth: usize) -> String;
}

/// Nó genérico da árvore DOM.
#[derive(Debug, Clone)]
pub enum Node {
    Element(Element),
    Text(String),
    RawHtml(String),
    Comment(String),
}

#[derive(Debug, Clone)]
pub struct Element {
    pub tag: String,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<Node>,
    pub is_self_closing: bool,
}
```

### 3.2 Implementação do Padrão Builder (Fluente)

```rust
impl Element {
    pub fn new(tag: &str) -> Self {
        Self {
            tag: tag.to_string(),
            attributes: Vec::new(),
            children: Vec::new(),
            is_self_closing: false,
        }
    }

    pub fn self_closing(tag: &str) -> Self {
        Self {
            tag: tag.to_string(),
            attributes: Vec::new(),
            children: Vec::new(),
            is_self_closing: true,
        }
    }

    pub fn set_id(mut self, id: &str) -> Self {
        self.attributes.push(("id".to_string(), id.to_string()));
        self
    }

    pub fn add_class(mut self, class_name: &str) -> Self {
        if let Some((_, val)) = self.attributes.iter_mut().find(|(k, _)| k == "class") {
            val.push(' ');
            val.push_str(class_name);
        } else {
            self.attributes.push(("class".to_string(), class_name.to_string()));
        }
        self
    }

    pub fn attr(mut self, key: &str, value: &str) -> Self {
        self.attributes.push((key.to_string(), value.to_string()));
        self
    }

    pub fn data_attr(mut self, key: &str, value: &str) -> Self {
        self.attributes.push((format!("data-{}", key), value.to_string()));
        self
    }

    pub fn aria_attr(mut self, key: &str, value: &str) -> Self {
        self.attributes.push((format!("aria-{}", key), value.to_string()));
        self
    }

    pub fn child(mut self, child: impl Into<Node>) -> Self {
        self.children.push(child.into());
        self
    }

    pub fn text(mut self, content: &str) -> Self {
        let escaped = html_escape(content);
        self.children.push(Node::Text(escaped));
        self
    }

    pub fn raw_text(mut self, content: &str) -> Self {
        self.children.push(Node::RawHtml(content.to_string()));
        self
    }
}
```

### 3.3 Sanitização XSS (Escape em Tempo de Execução)

```rust
pub fn html_escape(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '&' => output.push_str("&amp;"),
            '"' => output.push_str("&quot;"),
            ''' => output.push_str("&#x27;"),
            _ => output.push(c),
        }
    }
    output
}
```

---

## 4. Compilação e Exportação para WebAssembly (WASM)

Para garantir interoperabilidade com JavaScript e WebAssembly, o Rust utilizará a crate `wasm-bindgen`.

### 4.1 Estrutura do Módulo WASM (`src/wasm.rs`)

```rust
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub struct WasmDocument {
    title: String,
    lang: String,
    body_nodes: Vec<String>,
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
impl WasmDocument {
    #[wasm_bindgen(constructor)]
    pub fn new(title: &str, lang: &str) -> Self {
        Self {
            title: title.to_string(),
            lang: lang.to_string(),
            body_nodes: Vec::new(),
        }
    }

    pub fn add_heading(&mut self, level: u8, text: &str, class_name: Option<String>) {
        let tag = format!("h{}", level);
        let mut el = Element::new(&tag).text(text);
        if let Some(cls) = class_name {
            el = el.add_class(&cls);
        }
        self.body_nodes.push(el.render());
    }

    pub fn add_paragraph(&mut self, text: &str, class_name: Option<String>) {
        let mut el = Element::new("p").text(text);
        if let Some(cls) = class_name {
            el = el.add_class(&cls);
        }
        self.body_nodes.push(el.render());
    }

    pub fn render_html(&self) -> String {
        format!(
            "<!DOCTYPE html><html lang="{}"><head><title>{}</title></head><body>{}</body></html>",
            self.lang,
            self.title,
            self.body_nodes.join("")
        )
    }
}
```

---

## 5. Estrutura do Projeto (`Cargo.toml`)

O arquivo `Cargo.toml` deve suportar tanto o uso como biblioteca Rust nativa quanto a compilação para `wasm32`:

```toml
[package]
name = "molt"
version = "0.1.0"
edition = "2021"
authors = ["Molt Community"]
description = "Fast, Type-Safe HTML DSL and Static Site Generation Engine in Rust & WASM"
license = "MIT"

[lib]
crate-type = ["cdylib", "rlib"]

[features]
default = []
wasm = ["wasm-bindgen"]

[dependencies]
wasm-bindgen = { version = "0.2", optional = true }
rayon = { version = "1.8", optional = true }

[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

---

## 6. Mapeamento de Helpers SEO (SEO Module)

Exemplo de transposição do módulo de SEO do Swift para Rust:

```rust
pub struct SeoBuilder {
    title: String,
    description: String,
    image: Option<String>,
    url: Option<String>,
    twitter_card: Option<String>,
}

impl SeoBuilder {
    pub fn new(title: &str, description: &str) -> Self {
        Self {
            title: title.to_string(),
            description: description.to_string(),
            image: None,
            url: None,
            twitter_card: Some("summary_large_image".to_string()),
        }
    }

    pub fn image(mut self, image_url: &str) -> Self {
        self.image = Some(image_url.to_string());
        self
    }

    pub fn url(mut self, page_url: &str) -> Self {
        self.url = Some(page_url.to_string());
        self
    }

    pub fn build_nodes(&self) -> Vec<Element> {
        let mut nodes = vec![
            Element::new("title").text(&self.title),
            Element::self_closing("meta").attr("name", "description").attr("content", &self.description),
            Element::self_closing("meta").attr("property", "og:title").attr("content", &self.title),
            Element::self_closing("meta").attr("property", "og:description").attr("content", &self.description),
        ];

        if let Some(ref img) = self.image {
            nodes.push(Element::self_closing("meta").attr("property", "og:image").attr("content", img));
        }

        if let Some(ref u) = self.url {
            nodes.push(Element::self_closing("meta").attr("property", "og:url").attr("content", u));
        }

        nodes
    }
}
```

---

## 7. Roteiro de Implementação para o LLM

Ao instruir o LLM a implementar o projeto, exija a execução nas seguintes etapas:

1. **Fase 1: Core Engine (`src/core.rs`)**
   - Implementar `Node`, `Element`, `RenderTrait` e sanitização `html_escape`.
   - Adicionar testes unitários para verificação de montagem do HTML e escape XSS.
2. **Fase 2: Element Helpers (`src/elements.rs`)**
   - Criar construtores para tags HTML5 (`div`, `p`, `h1`-`h6`, `article`, `section`, `nav`, `footer`, `form`, `input`, etc.).
3. **Fase 3: Document & Layout Builder (`src/document.rs`)**
   - Estruturas para montagem do `Document` completo com `<!DOCTYPE html>`, `head` e `body`.
4. **Fase 4: SEO & ARIA (`src/seo.rs`, `src/accessibility.rs`)**
   - Helpers fluentes para inclusão de metadados OpenGraph, Twitter e acessibilidade ARIA.
5. **Fase 5: Exportação WASM (`src/wasm.rs`)**
   - Anotar e exportar funções chave com `#[wasm_bindgen]` para consumo em JS.
6. **Fase 6: Benchmarks (`benches/render_benchmark.rs`)**
   - Testar o desempenho de renderização de 10.000 nós vs Swift native.

---

## 7. Exemplo Final de Uso Idiomático em Rust

```rust
use molt::prelude::*;

fn main() {
    let html_doc = document("pt-BR")
        .head(vec![
            title("Página Inicial - Molt"),
            meta_charset("UTF-8"),
        ])
        .body(vec![
            header().child(
                nav().add_class("navbar").child(
                    a("#home").text("Início")
                )
            ),
            main_tag().child(
                article().add_class("card p-4").child(
                    h1("Bem-vindo ao Molt!").add_class("text-2xl font-bold")
                ).child(
                    p("Geração ultra-rápida de HTML compilado para WebAssembly.")
                )
            ),
        ]);

    println!("{}", html_doc.render_pretty());
}
```

---
*Fim da Especificação Técnica. Este arquivo serve de contexto completo e instrução para qualquer LLM gerar o código-fonte da crate `molt`.*
