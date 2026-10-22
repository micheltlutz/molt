# Checklist de desempenho para um site feito com o Molt

🇬🇧 [English version](../performance.md)

O Molt monta uma string de HTML. Se a página carrega rápido depende, em boa parte, do que você
envia junto: folhas de estilo, fontes, scripts e os cabeçalhos com que viajam. Esta página lista
o que o site oficial, <https://www.molt-framework.io>, encontrou, e qual recurso do Molt tira
cada armadilha.

**Como ler a evidência.** Cada afirmação diz de onde vem. *Relatado* quer dizer que o autor do
site mediu e anotou (o registro de atritos guardado no repositório do site, a partir de uma
execução do PageSpeed Insights em 22 de outubro de 2026 e de execuções locais do Lighthouse
13.5.0); não foi medido de novo para esta página. *Plataforma web* é como os navegadores estão
documentados para se comportar, e não foi testado aqui. *Executado* quer dizer que o trecho da
página é compilado e executado por `cargo test --doc`.

## Onde o Molt não é o problema

Para a página inicial do site, relatado: o HTML tinha 21 KB cru e 5,5 KB com Brotli, o servidor
respondeu em 1 ms, e o PageSpeed deu 0 de Total Blocking Time e de Cumulative Layout Shift e nota
97 no celular e 100 no computador. O único ponto perdido foi o Speed Index do celular (4,2 s),
que depende do que a página faz depois que o HTML chega (CSS, fontes, animação), não de gerá-lo.
Olhe o checklist antes de olhar o renderizador.

## O checklist

### 1. Ponha o tema no `<html>`, não num script

Um tema escuro definido por JavaScript pisca claro primeiro. Um atributo no `<html>` já está lá
antes de qualquer script rodar. Use `Document::html_attr`.

### 2. Pré-carregue fontes com `crossorigin`

Fontes são buscadas em modo CORS anônimo mesmo do seu próprio domínio, então um preload de fonte
sem `crossorigin` não corresponde à requisição que a página faz depois e a fonte é baixada duas
vezes (plataforma web; não testado aqui). `preload_font` sempre escreve o atributo e deduz o
`type` da extensão.

### 3. Dê às fontes web um reserva com as mesmas métricas

Uma fonte web que entra depois da primeira pintura desloca o texto se a reserva tem outro
tamanho. Relatado: o deslocamento de layout do site sob conexão limitada era 0,086 e caiu para
0,000 depois que as fontes reserva ganharam métricas ajustadas no `@font-face` (`size-adjust`,
`ascent-override`, `descent-override`, `line-gap-override`), com os números calculados a partir
dos arquivos das fontes. Isso é CSS; o Molt não tem o que acrescentar.

### 4. Não faça o maior elemento entrar com fade a partir de `opacity: 0`

Relatado: o navegador só conta um elemento como pintado para o Largest Contentful Paint quando o
fade termina (a explicação do registro de atritos para os números a seguir). O título e o
subtítulo do site agora sobem com `transform`, sem fade, e o LCP medido foi de 1,67 s para
1,50 s. Mover um elemento sem esmaecê-lo mantém a animação. Também é CSS.

### 5. CSS e scripts críticos inline sob uma CSP estrita

Uma política com `style-src 'self'` e `script-src 'self'`, sem `'unsafe-inline'`, bloqueia código
inline, então a folha de estilo e o script do tema viram requisições que bloqueiam a renderização
(relatado pelo PageSpeed para o site). Uma política pode, em vez disso, listar o hash de cada
bloco inline, e `Element::csp_hash` o calcula a partir do elemento, para o hash não se afastar
do código. *Executado:*

```rust
use molt::prelude::*;
use molt::Document;

// O script é código escrito aqui mesmo, então `raw_text` é o correto.
let theme = script().raw_text("document.documentElement.dataset.theme='dark';");
let hash = theme.csp_hash().expect("a script has a hash");
let policy = format!("script-src 'self' '{hash}'");

let page = Document::new(Some("en"))
    .html_attr("data-theme", "dark")
    .head_children([
        preload_font("/fonts/inter.woff2"),
        stylesheet("/css/site.css"),
        theme,
    ]);
let html = page.render();

assert!(policy.starts_with("script-src 'self' 'sha256-"));
assert!(html.contains(r#"<html lang="en" data-theme="dark">"#));
assert!(html.contains(r#"type="font/woff2" href="/fonts/inter.woff2" crossorigin>"#));
```

Envie `policy` num cabeçalho `Content-Security-Policy`. O hash cobre os bytes exatos entre as
tags, então se você editar o script o hash muda junto. Se um navegador específico aceita a
política não foi testado aqui.

### 6. Faça cache de um ano pondo o conteúdo no nome do arquivo

Com nomes fixos (`site.css`) só dá para mandar o cache guardar o arquivo por pouco tempo, porque
um rebuild deixaria os visitantes na cópia antiga. O site relatou exatamente isso no ambiente
local. Um arquivo nomeado pelo conteúdo pode ser cacheado como
`public, max-age=31536000, immutable`. `StaticSiteGenerator::copy_asset_hashed` copia um arquivo
para `nome.<hash>.ext` e devolve a URL para pôr na página. Ele renomeia um arquivo e não reescreve
nada dentro dele: um módulo que importa outro por caminho relativo e um `url(...)` numa folha de
estilo continuam apontando para os nomes antigos. Ou concatene seus scripts num arquivo só, ou
copie antes os arquivos que a folha de estilo cita e escreva as URLs devolvidas na folha que
você vai copiar depois. Compilado, não executado (grava em disco):

```rust,no_run
use molt::prelude::*;
use molt::ssg::StaticSiteGenerator;

let site = StaticSiteGenerator::new("dist");
site.clean(true)?; // os arquivos com hash de uma build anterior saem junto

let font = site.copy_asset_hashed("static/fonts/inter.woff2", "fonts")?;
// ...escreva `font` na folha de estilo que a cita, e então:
let css = site.copy_asset_hashed("static/css/site.css", "css")?;

let head = [preload_font(&font), stylesheet(&css)];
# let _ = head;
# Ok::<(), molt::ssg::SsgError>(())
```

## Medir

Estas são as lições das medições do site, relatadas com o Lighthouse 13.5.0 em emulação de
celular contra `localhost` servido com gzip.

- **O modo simulado do PageSpeed não enxerga animação nem fonte.** Variantes que mudavam uma
  animação e a pré-carga de uma fonte deram números idênticos até a segunda casa decimal, até a
  execução aplicar limitação real (`--throttling-method=devtools`).
- **Descarte a primeira execução.** A linha de base mediu Speed Index de 2,36 s uma vez e 1,65 s
  nas três seguintes.
- **O deslocamento de layout pode ficar escondido.** O PageSpeed relatou CLS 0 enquanto uma
  execução limitada mediu 0,086 (o limite "bom" é 0,1). O que vale é o dado de campo, e o site
  ainda não tem.
- **Um fade pode esconder um problema de layout.** Sem a animação de entrada o CLS subiu para
  0,137, por causa da troca de fonte.
- **Adiar um loop decorativo piorou o Speed Index** (2,3 s contra 1,89 s), porque o Lighthouse
  compara cada quadro com o último, e um movimento tardio faz o último quadro diferir de todos os
  anteriores.

O efeito de qualquer destes itens na nota real do PageSpeed só se confirma depois do deploy.
