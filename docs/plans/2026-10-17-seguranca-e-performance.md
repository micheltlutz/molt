# Plano de segurança e performance do Molt

Status: aceito e implementado em 2026-10-17. Este texto é o contrato. O que está em
[Fora deste plano](#fora-deste-plano) não entra no crate.

Fontes: o relatório da PoC em `mlv-engine-r/docs/relatorio-poc.md`, o benchmark Winged no
MacBook Intel (`winged-benchmark/report/report.html`, gerado `2026-10-16T23:52`, host
`MLI-MK03`) e a suíte `URLPolicyTests` do Winged-Swift 3.0.1.

## O que não é trabalho deste crate

O motor Rust renderiza com o Molt. O HTML da PoC fechou 49/49. O gargalo de
`/admin/modules` é o Postgres, não a CPU do render. Sessão, JOIN, cache, `try_join!` e
benchmark em Linux são do motor. Compressão é `tower-http`. LTO e `mimalloc` do binário
do motor também. O perfil `panic = "abort"` deste crate fica, porque o orçamento WASM
(`scripts/build-wasm.sh`, 120 KB gzip) depende dele; `unwind` em servidor é decisão do
`mlv-engine-r`.

O que a PoC expunha na API, e o que este plano fecha:

- Atributos passavam por `escape_text` e entravam com `Attribute::raw`, porque
  `escape_attribute` não escapa `<` `>` e porque o motor precisa de ordem alfabética.
  `Attribute::raw` era o hatch do `javascript&#58;`. A política agora roda na escrita,
  inclusive sobre valor cru.
- Tag não-void com self-closing virava `Node::Raw` inteiro, e esse raw fugia de qualquer
  política aplicada em `Attribute::write_into`. `Element::self_closing(true)` emite o
  sufixo e não renderiza filhos, então os atributos passam pela política.
- `Document` pretty não é bug. O motor quer compacto com XHTML, e isso já existia
  (`Element::render` + `RenderOptions`).

## Benchmark no MacBook Intel (16/10/2026)

Não é o M4 Pro da PoC. Host: MacBook Pro, Intel i5-8257U (4 núcleos / 8 threads, 8 GB),
macOS 15.8.1. Carga no Docker (Colima): Ubuntu 24.04, VM com 4 CPUs e 5,78 GB. Cada
servidor limitado a 1 vCPU e 512 MB. Checkouts medidos: Swift `develop`
`v3.0.0-4-g68e7bf9`, Rust `develop` `v1.0.0-27-ge4e5e95`. Não é `cargo bench` do crate
isolado.

`/api/render` é só a DSL. Em stress, 1 vCPU, o Rust sustentou 4.000 req/s e não saturou
(p50 0,42 ms, p95 3,11 ms, CPU média 28%). O Swift saturou a 175 req/s. Micro-otimizar o
escaper ou o writer não é o que esse número pede. O rebuild (Markdown + disco) mede
`pulldown-cmark` contra o `Markdown.parse` do Swift; portar o parser do Swift não herda
os 2.139 req/s e fica fora desta leva.

A projeção de Amdahl do cenário B2s (R² 0,58, ponto de 3 vCPUs contaminado pelo k6) não
serve para dimensionar instância.

## Já feito, para ninguém reimplementar

- Árvore `Send + Sync`. Não portar `HTMLTag.copy()`.
- Render e `Drop` de `Element` iterativos. O residual (cadeia de `Node::Fragment` sem
  nenhum `Element`) está em `SECURITY.md`. `Drop` em `Node` é breaking e fica para 2.0.
- `data_attrs` / `aria_attrs` determinísticos, `clean()` recusa raiz e `..`, escrita
  atômica, `generate_multiple` junta erros, sem switch global de XHTML.
- `unsafe_code = "forbid"`, MSRV 1.85, edition 2024, `Cargo.lock` commitado.
- Três escapers distintos. O `MOLT_SPEC.md` §3.3 está errado; os fixtures dependem
  da diferença. Não unificar.

## Padrões que não entram

A política de URL é uma função em `src/core/url_policy.rs`, chamada no fim de
`Attribute::write_into`. Sem trait, sem Decorator, sem Visitor. `trusted_url` é um bool
no `Attribute`, irmão de `is_boolean`. O módulo não importa `Element`.

## Fase 1 — política de URL

Comportamento copiado de `URLPolicy` e de `URLPolicyTests` no Winged-Swift 3.0.1. A
checagem roda na escrita, sobre o valor que vai para o HTML, inclusive `Attribute::raw`.
Valor passado por `Attribute::new` continua escapado: `javascript&#58;` vira
`javascript&amp;#58;` e não é bloqueado. Valor cru `javascript&#58;` é bloqueado.

Contrato:

- Atributos: `href`, `src`, `action`, `formaction`, `poster`, `cite`, `background`,
  `xlink:href`, `data`. Nome comparado em minúsculas (`ACTION` bloqueia).
- Schemes: `javascript:`, `vbscript:`, `data:text/html`. `data:image/...` passa.
- Normalização: decodificar uma vez referências numéricas (com ou sem `;`) e `&colon;` /
  `&Tab;` / `&NewLine;` (case-sensitive, com `;`); descartar escalares `<= U+0020`;
  comparar o prefixo em minúsculas.
- Substituto: `#`.
- `Attribute::trusted_url` e `Element::trusted_url` pulam a checagem; o valor ainda passa
  por escape de atributo. No WASM, `WElement.trustedUrl`.
- Booleanos não entram na checagem.
- `title="javascript:fine"` e `https://example/javascript:` não são bloqueados.
- Caminho sem `&` e sem controle não aloca: o scan lê o `&str` guardado.
- Chave com espaço, aspas, `'`, `=`, `<`, `>`, `/`, controle, ou vazia: o atributo não é
  emitido. Fixtures usam chaves válidas.

Testes no módulo `url_policy`, um caso por entrada da suíte Swift (schemes, controles,
referências, trusted, `data:image`, atributo que não é URL). O caso de Markdown do Swift
não tem parser aqui e não foi portado.

## Fase 2 — self-closing sem `Node::Raw`

`Element::self_closing(true)` emite ` />` ou `>` conforme `xhtml_self_closing`, mesmo
quando a tag não é void, e não renderiza filhos. Default `false`.

Não ordenar atributos no crate. Não alargar `escape_attribute` para escapar `<` `>`.

## Fase 3 — performance que cabe neste crate

O `/api/render` do harness não saturou. Cada item abaixo não é uma tentativa de subir
aquele RPS.

- `Document::render_to` escreve pelo mesmo `write_into`, byte a byte igual a
  `render_with`. `StaticSiteGenerator::generate` grava no arquivo temporário por esse
  caminho. Teste no estilo de `StreamingWriteTests`: pretty, compacto e XHTML, página
  vazia e página de centenas de seções.
- `overflow-checks = true` no `[profile.release]`. O orçamento é o pacote WASM de 120 KB
  gzip, não a imagem Docker do harness. Medido em 2026-10-17 no alvo `nodejs`: 22 KB
  gzip (23.195 bytes) com a flag ligada, abaixo do teto, então a flag fica.

Não adicionar `mimalloc`, não mudar `panic = "abort"`, não SIMD no escaper, não reescrever
o Markdown do Swift 2.4.

## Fase 4 — cadeia de suprimentos

- `cargo deny check advisories bans licenses sources`, com `deny.toml`. Licenças
  permitidas: MIT, Apache-2.0, BSD-2-Clause e BSD-3-Clause. O grafo de hoje fecha só com
  MIT; as outras três ficam na política para uma crate que seja só Apache ou BSD.
  `cargo test` e `clippy` com `--locked`.
- Fuzz fora do CI obrigatório: `scripts/fuzz.sh` (`escape_text` não emite `<` cru,
  `escape_attribute` não emite `"` cru, política de URL não entra em pânico e, se o valor
  normalizado começa com scheme bloqueado, a saída contém `="#`).
- `unwrap` / `expect` negados no código de biblioteca. `clippy.toml` permite os dois em
  teste.
- Publicação: o workflow separa o job `verify` (sem token) do job `release` (sem cache,
  `cargo publish --locked --no-verify`, Trusted Publishing). **Passo humano:** registrar
  o ambiente `release` no crates.io e aposentar `CARGO_REGISTRY_TOKEN`.

`cargo-vet` fica de fora: duas dependências opcionais, custo de auditoria maior que o
risco restante depois do `cargo-deny`.

## Fora deste plano

- Portar `Site` / `Page` / `Markdown` / CLI / feed Atom / `robots.txt` do Swift 2.1–2.4.
- Itens de banco, sessão e cache do relatório da PoC.
- Usar a projeção de Amdahl do B2s para escolher tamanho de instância.
- Unificar escapers, ordenar atributos, `Drop` em `Node`, Strategy/Visitor.
- Sanitizar corpo de `<script>` / `<style>` ou CSS em `style`. Quem passa dado não
  confiável usa `text()`, não `raw_text()`.
