# Pitfalls

Mistakes grouped by **symptom**, so you can find yours by what you saw. Each entry says what
is going on and the fix. Behaviours pinned by the crate's golden fixtures are marked
*(fixtures)*: they are deliberate, and changing one would be a breaking change, so do not
expect them to go away.

Written for people and AI agents who *use* `molt-framework`. Which APIs exist in which version
is in [`capabilities.md`](capabilities.md).

## The output has the wrong characters

| you see | cause | fix |
| --- | --- | --- |
| `&amp;amp;` or `&amp;lt;` | you escaped before calling `text()`/`attr()`, which escape themselves | pass the raw value |
| `<b>` shown literally on the page | you used `text()` for markup | `raw_text()` / `Node::raw()` **only for markup you wrote** |
| an apostrophe becomes `&#x27;` in HTML but `&apos;` in the sitemap | three escapers exist on purpose *(fixtures)* | do not unify them; |
| `<` is not escaped inside `alt="…"` | attribute escaping leaves `<` `>` alone *(fixtures)* | it is escaped automatically under `noscript`/`iframe`/`script`/… |
| an attribute value breaks out of its quotes | you used `Attribute::raw`, which does **not** escape (it still runs the URL policy) | `Attribute::new`; `raw` only for a value you wrote and already escaped, with a comment |
| `href="#"` where you wrote a URL | the URL policy blocked `javascript:`, `vbscript:`, `data:text/html` | use a real URL; `trusted_url` only for a URL **you** wrote |

## Something I added is missing

| you see | cause | fix |
| --- | --- | --- |
| children of `<img>`, `<br>`, `<input>` vanish | void elements take no children or content *(fixtures)* | wrap in a container |
| an element and all its children render nothing | its tag name is invalid (space, `<`, `>`, leading digit, …) | use a valid name: letter, then letters/digits/`-_.:` |
| an attribute is missing | its name holds a space, quote, `=`, `<`, `>`, `/`, a control character, or is empty | fix the name |
| a boolean attribute shows as `required="true"` | you used `attr("required", "true")` | `bool_attr("required")` |
| nothing renders for a false `@if` | by design: an empty fragment writes no blank line | — |

## The output has the wrong shape

| you see | cause | fix |
| --- | --- | --- |
| `Element::render()` is one line, `Document::render()` is indented | asymmetric on purpose *(fixtures)* | `render_pretty()` / `render_compact()` / `RenderOptions` |
| `<pre>`/`<code>`/`<textarea>` contents are not indented | their whitespace is displayed *(fixtures)* | — |
| `set_id`/`set_style`/`set_role` replaced my value | those three replace; every other setter appends *(fixtures)* | call it once, or use `attr` |
| `text("a").text("b")` renders `ab`, and `.child(x).text(y)` puts `y` after `x` | `text` and `raw_text` append in call order since 2.0 (#110); before 2.0 they replaced and always came first | build the string first if you meant to replace |
| `add_class("a").add_class("b")` gives `class="a b"` | classes append, space-joined | — |
| `<html lang="My site">` | `Document::new` takes the *language*, not the title | `Document::new(Some("en"))`, and `head_children([title().text("My site")])` |
| attributes are in insertion order | not sorted, on purpose | do not ask for sorting in the crate |
| pretty output of a deep tree is enormous | one indent per level per line: O(depth²) | render compact, or `with_indent("")`, or bound depth |

## It will not compile

| you see | cause | fix |
| --- | --- | --- |
| "cannot construct `Node::Text` / struct literal of a `#[non_exhaustive]` type" | `Markup` and the 1.0 structs have no public constructors | `Node::text()`, `Document::new`, `RenderOptions::pretty()`, `SitemapUrl::new(..)` |
| `match node { … }` is non-exhaustive | `Node` is `#[non_exhaustive]` | add a `_ =>` arm |
| `Layout::render` not found | renamed to `wrap`/`wrap_many` (it clashed with `Render::render`) | rename |
| `changefreq("weekly")` mismatched types | it takes `ChangeFreq` now | `ChangeFreq::Weekly`, or `"weekly".parse()` |
| `escape_text(x)` is not a `String` | returns `Cow<str>` | `.into_owned()` or `.to_string()` |
| `E0618` "expected function, found …" on `title()`, `link()`, `label()`, `span()` | a local variable or parameter with the same name shadows the constructor (reported while building the official site) | rename it (`page_title`) |
| the error of `StaticSiteGenerator` is not `io::Error` | it is `SsgError` | `?` converts it into `io::Error`; or match its variants |

## JavaScript / WASM

| you see | cause | fix |
| --- | --- | --- |
| `null pointer passed to rust` | you used a handle after passing it to `child`/`addBody`/`addHead`/`addSeo`, or after chaining from it | chain calls, or keep the handle each call returns |
| `renderPretty` missing on a document | documents have `render()` (pretty), `renderCompact()`, `renderWithIndent()`, `renderWithOptions()` | use those |

## Tests

| you see | cause | fix |
| --- | --- | --- |
| a deep-tree test aborts with a stack overflow in `assert_eq!` | before 2.0 the failure message formatted the tree with a recursive `Debug` | on 1.x compare with `assert!(a == b)`; since 2.0 `Debug` stops at 128 levels (#79) |
| a test needs "the `<a>` inside this tree" | there was no way to search a tree | since 2.0, `Element::find`, `find_all` and `attribute`; see [`capabilities.md`](capabilities.md) |
