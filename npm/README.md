# Molt

Fast, type-safe HTML generation in WebAssembly — a port of
[Winged-Swift](https://github.com/micheltlutz/Winged-Swift).

This is the WebAssembly build. The Rust crate is on
[crates.io](https://crates.io/crates/molt), documented at
[docs.rs](https://docs.rs/molt).

```bash
npm install molt
```

```js
import { element, WDocument } from "molt";

const page = new WDocument("pt-BR")
  .addHead(element("title").text("RideKeeper"))
  .addBody(element("h1").text("Track every service"))
  .addBody(element("p").addClass("lead").text("Fuel, tyres & chain."));

console.log(page.render());
```

## Why

**Escaping you cannot forget.** `text()` escapes; `rawText()` does not, and is named so an
audit can grep for it.

```js
element("p").text("<script>alert(1)</script>").render();
// '<p>&lt;script&gt;alert(1)&lt;/script&gt;</p>'
```

**The same renderer as the Rust crate.** These bindings wrap the real node tree rather than
concatenating pre-rendered strings, so `renderPretty()` works and the output is byte-identical
to the Rust library's. That is verified on every build: the package's smoke test renders a
page and diffs it against the same golden fixture the Rust tests use.

**20 KB gzipped.**

## API

| | |
| --- | --- |
| `element(tag)` | a new element; `new WElement(tag)` also works |
| `.setId(id)` `.addClass(c)` `.setStyle(s)` | class, id and style |
| `.attr(k, v)` `.boolAttr(k)` | attributes; `boolAttr` renders a bare key |
| `.dataAttr(k, v)` `.ariaAttr(k, v)` | `data-*` and `aria-*` |
| `.text(s)` `.rawText(s)` | escaped and unescaped content |
| `.child(el)` `.comment(s)` | children |
| `.render()` `.renderPretty()` | markup |
| `new WDocument(lang)` | a page; `lang` may be `undefined` |
| `.addHead(el)` `.addBody(el)` | content |
| `.render()` `.renderCompact()` `.renderWithIndent(s)` | markup; `render()` is pretty |
| `new WSeo(title, description)` | Open Graph and Twitter Cards |
| `escapeText(s)` `version()` | helpers |

## Entry points

| import | build |
| --- | --- |
| `molt` | CommonJS in Node, ESM in a bundler, ESM in a browser |
| `molt/web` | the `--target web` build, for a plain `<script type="module">` |

## TypeScript

Typings ship with the package. They reference `Symbol.dispose` and the `WebAssembly`
namespace, so your `tsconfig.json` needs both in `lib`:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "lib": ["ES2022", "ESNext.Disposable", "DOM"]
  }
}
```

In Node without `DOM`, `@types/node` supplies the `WebAssembly` namespace instead.

## License

MIT
