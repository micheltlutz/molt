# Porting from Winged-Swift

A map from `Winged-Swift` 2.0.0 to `molt`, and an honest list of everywhere the two
behave differently.

Markup parity is not a claim here — it is a test. `tests/golden.rs` reproduces all four of
Winged-Swift's golden fixtures byte for byte. Everything below is either a rename, or a
difference that shows up somewhere those four files do not reach.

## Type and function mapping

| Winged-Swift | molt | notes |
| --- | --- | --- |
| `HTMLTag` (class) | `Element` (struct) + `Node` (enum) | value type, so the tree is `Send + Sync` |
| `HTMLTag.write(into:options:indentLevel:)` | `Render::write_into` | the overridable primitive in both |
| `HTMLTag.render(_:)` | `Render::render_with` | |
| `HTMLTag.render()` | `Render::render` | compact in both |
| `RenderOptions` | `RenderOptions` | `pretty`, `indent`, `xhtml_self_closing` |
| `RenderOptions.compact` / `.pretty` | `RenderOptions::compact()` / `::pretty()` | |
| `Attribute(key:value:)` | `Attribute::new` | |
| `Attribute.boolean(_:)` | `Attribute::boolean` | |
| `HTMLEscape.escape(_:)` | `core::escape::escape_text` | |
| `HTMLEscape.escapeAttribute(_:)` | `core::escape::escape_attribute` | |
| private `escapeXML` ×2 | `core::escape::escape_xml` | Swift has two identical copies |
| `Fragment` (subclass) | `Node::Fragment` | |
| `RawHTML` (subclass) | `Node::Raw` | |
| — | `Node::Comment` | **new**, see below |
| `Document` | `Document` | |
| `Layout` (protocol) | `Layout` (trait) | |
| `Layout.render(contents:)` | `Layout::render_many` | |
| `SEO.openGraph(…)` | `seo::open_graph` | |
| `SEO.openGraphArticle(…)` | `seo::open_graph_article` | |
| `SEO.twitterCard(…)` | `seo::twitter_card` | |
| `SEO.common(…)` | `seo::common` | now emits `<title>`, see below |
| `SEO.complete(…)` | `seo::SeoBuilder::build` | |
| `SitemapURL` / `SitemapGenerator` | `sitemap::SitemapUrl` / `SitemapGenerator` | |
| `RSSItem` / `RSSGenerator` | `feed::RssItem` / `RssGenerator` | |
| `StaticSiteGenerator` | `ssg::StaticSiteGenerator` | behind `feature = "ssg"` |
| `@HTMLBuilder` / `@HTMLFragmentBuilder` | `html!` macro | |
| `html { … }` | `html! { … }` | |
| `fragment { … }` | `Node::fragment([…])` or several roots in `html!` | |

### Builder methods

| Winged-Swift | molt |
| --- | --- |
| `addClass(_:)` | `add_class` |
| `addClasses(_:)` | `add_classes` |
| `setId(_:)` | `set_id` |
| `setStyle(_:)` | `set_style` |
| `setRole(_:)` | `set_role` |
| `setAttribute(key:value:)` | `attr` |
| `dataAttribute(key:value:)` | `data_attr` |
| `dataAttributes(_:)` | `data_attrs` |
| `ariaAttribute(key:value:)` | `aria_attr` |
| `ariaAttributes(_:)` | `aria_attrs` |
| `addAttribute(_:)` | `add_attribute` |
| `addChild(_:)` | `child` |
| `setContent(_:escape:)` | `text` / `raw_text` |
| `Attribute.trustedURL(key:value:)` (3.0.1) | `Attribute::trusted_url` | still attribute-escaped |
| `trustedURL(_:_:)` (3.0.1) | `Element::trusted_url` | replaces the previous value |
| `Document.write(toFile:)` (3.0.1) | `Document::render_to` | same bytes as `render_with` |

### Renamed elements

| Winged-Swift | molt | why |
| --- | --- | --- |
| `MainTag` | `main_tag()` | a binary's own `fn main` shadows a glob-imported `main()` |
| `VarTag` | `var()` | same |
| `Label(for:)` | `label_for(for_id)` | `for` *is* a Rust keyword; the rendered attribute is still `for` |
| `A(href:)` | `link_to(href)` | `a()` also exists, without the `href` |
| `Img(src:alt:)` | `image(src, alt)` | `img()` also exists; `alt` is required on `image` |
| `Iframe(src:title:)` | `iframe_titled(src, title)` | `title` required, as in Swift |

## Deliberate behavioural differences

Each of these is a decision, not an accident.

### 1. `data_attrs` and `aria_attrs` output is now deterministic

Swift's `dataAttributes(_:)` and `ariaAttributes(_:)` take a `Dictionary`, so the order in
which the attributes render varies between runs. That makes output diffs noisy and golden
files impossible for any page that uses them.

The Rust versions take an ordered `IntoIterator<Item = (K, V)>`, so the output is stable.

### 2. The tree is `Send + Sync`

`HTMLTag` is a mutable class; a Swift tag tree is not `Sendable`, and reusing one instance
in two places silently shares the node. Winged-Swift's `ROADMAP.md` lists both as open
questions for 3.0.

`Node` is an enum of value types. Reuse copies, and a tree can cross threads — which is
what the `parallel` feature is built on. It also means the "always return a fresh tag from
a component function" rule in Winged-Swift's docs simply does not apply here.

### 3. `seo::common` emits a `<title>`

`SEO.common(title:description:…)` in Swift accepts a `title` argument and never uses it —
no `<title>` element is produced. The Rust version emits one.

If you were relying on the Swift behaviour, build your `<title>` separately and do not pass
one here.

### 4. `Node::Comment` is new

Winged-Swift has no comment node. `MOLT_SPEC.md` §3.1 asks for one, so it exists.
A space is put between every two adjacent hyphens and after a trailing one (`--->` becomes
`- - ->`), so a comment cannot be closed early and inject markup.

### 5. `clean()` refuses unsafe paths

`StaticSiteGenerator.clean()` in Swift removes whatever directory it is pointed at. The
Rust version returns an error for an empty path, a filesystem root, or a path containing a
`..` component. Page paths that escape the output directory are rejected too.

### 6. Writes are atomic

`write_file` writes to a temporary file in the destination directory and renames it, so a
concurrent reader never sees a half-written page. (Winged-Swift 3.0 writes atomically too:
`Document.write(toFile:)` goes through a `.winged-tmp` file and a move, and `writeFile` uses
`atomically: true`. An earlier version of this note said otherwise; it was wrong.) The
temporary name here is unique per process and per call, because the first version of this
port replaced the extension and so made `index.html` and `index.xml` share a file — a bug in
the port, not in Swift. `generate` fills
that temporary file through `Document::render_to`, which matches `render_with` byte for
byte and renders each body section on its own.

### 7. `generate_multiple` reports every failure

Swift throws on the first error. The Rust version collects them all and returns one error
naming each failed page — a bulk build that names one of twelve broken pages is worse than
useless.

### 8. The global XHTML switch is gone

`HTMLTag.xhtmlSelfClosing` is process-wide mutable state, already deprecated in Swift 2.0
and slated for removal in 3.0. It is not ported. Use
`RenderOptions::compact().with_xhtml_self_closing(true)`.

### 9. URL attributes are checked when they are written

Winged-Swift 2.0 writes `href="javascript:…"` as given. 3.0.1 neutralises script URLs at
write time, and this crate follows 3.0.1: `href`, `src`, `action`, `formaction`, `poster`,
`cite`, `background`, `xlink:href` and `data` become `#` when the stored value is a
`javascript:`, `vbscript:` or `data:text/html` URL. `Attribute::raw` does not escape, but
it does not skip the check. `trusted_url` does.

This crate goes further than 3.0.1 in four places, all in `url_policy.rs`: `ping`, `manifest`
and `longdesc` are checked; `srcset` is checked per candidate; `srcdoc` is blocked (written
empty) unless `trusted_url` is used; and on `iframe`, `frame`, `embed` and `object` a
`data:` URL is blocked unless it is a raster image. Winged-Swift blocks only `data:text/html`,
which leaves `data:image/svg+xml` and `data:application/xhtml+xml` working in a frame.

`escape_attribute` still does not escape `<` or `>`. The fixtures and the table above fix
that escaper; widening it would change markup that is not a URL.

### 10. A non-void tag can close itself

`Element::self_closing(true)` emits `>` or ` />` and drops children, including for a tag
that is not in the void set. The default is `false`. Void tags are unchanged.

### 12. Sitemap and feed output is checked

Three differences from Winged-Swift, all in `sitemap.rs` and `feed.rs`:

- **Characters XML 1.0 cannot hold are dropped.** `escape_xml` removes the control characters
  U+0000–U+0008, U+000B, U+000C, U+000E–U+001F and U+FFFE/U+FFFF. Swift writes them, and one
  such character makes the whole document ill-formed.
- **Links must be absolute `http(s)` URLs.** A sitemap entry whose `loc` is not, and an RSS
  `<link>` that is not, are left out; a feed reader shows the link as something to click.
  `javascript:` there is as dangerous as in an `href`. A relative `loc` is not valid in a
  sitemap in the first place.
- **`isPermaLink` says the truth.** Swift always writes `isPermaLink="true"`. Here it is
  `true` only when the guid is an `http(s)` URL; `urn:uuid:…` gets `false`. The RSS 2.0
  specification makes `true` a promise that readers may open the guid.

The fixtures use `https` links and a URL guid, so they are unaffected.

### 11. Rust has no default arguments

Swift initializers like `Button(type: String? = "button", …)` become either an explicit
constructor (`button_typed("submit")`) or a plain `attr` call. Nothing silently defaults.

## Checking you are done

Winged-Swift's `MIGRATION.md` ends with a `grep` the reader can run to prove the migration
is complete. The same idea, for this port — none of these should appear in ported code:

```bash
grep -rn 'xhtmlSelfClosing\|renderCompact\|renderPretty\|MainTag\|VarTag\|HTMLTag\|RawHTML(' src/
```

And the real check:

```bash
./scripts/verify.sh
```
