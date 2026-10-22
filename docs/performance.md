# Performance checklist for a site built with Molt

🇧🇷 [Versão em português](pt-BR/performance.md)

Molt builds an HTML string. Whether the page then loads fast depends mostly on what you send
along with it: stylesheets, fonts, scripts and the headers they travel with. This page lists
what the official site, <https://www.molt-framework.io>, ran into, and which Molt helper
removes each trap.

**How to read the evidence.** Every claim below says where it comes from. *Reported* means the
site's author measured it and wrote it down (the friction log kept in the site's repository,
from a PageSpeed Insights run on 22 October 2026 and local Lighthouse 13.5.0 runs); it was not
re-measured for this page. *Web platform* is how browsers are documented to behave and was not
tested here. *Run* means the snippet on this page is compiled and run by `cargo test --doc`.

## Where Molt is not the problem

For the site's home page, reported: the HTML was 21 KB raw and 5.5 KB with Brotli, the server
answered in 1 ms, and PageSpeed gave Total Blocking Time and Cumulative Layout Shift of 0 and a
score of 97 on mobile and 100 on desktop. The one point lost was the mobile Speed Index (4.2 s),
which depends on what the page does after the HTML arrives (CSS, fonts, animation), not on
generating it. Look at the checklist before you look at the renderer.

## The checklist

### 1. Put the theme on `<html>`, not in a script

A dark theme set by JavaScript flashes light first. An attribute on `<html>` is there before any
script runs. Use `Document::html_attr`.

### 2. Preload fonts with `crossorigin`

Fonts are fetched in anonymous CORS mode even from your own domain, so a font preload without
`crossorigin` does not match the request the page makes later and the font is downloaded twice
(web platform; not tested here). `preload_font`
always writes it and takes the `type` from the extension.

### 3. Give web fonts a fallback with the same metrics

A web font that swaps in after the first paint moves the text if the fallback is a different
size. Reported: the site's layout shift under a throttled connection was 0.086 and dropped to
0.000 after the fallback fonts were given adjusted metrics in `@font-face` (`size-adjust`,
`ascent-override`, `descent-override`, `line-gap-override`), with the numbers computed from the
font files. This is CSS; Molt has nothing to add.

### 4. Do not fade the largest element in from `opacity: 0`

Reported: the browser counts an element as painted for Largest Contentful Paint only when its
fade ends (the friction log's explanation for the numbers that follow). The site's title and subtitle now rise with `transform` and no fade, and the measured LCP
went from 1.67 s to 1.50 s. Moving an element without fading it keeps the animation. Also CSS.

### 5. Inline critical CSS and scripts under a strict CSP

A policy of `style-src 'self'` and `script-src 'self'`, with no `'unsafe-inline'`, blocks inline
code, so the stylesheet and the theme script become render-blocking requests (reported by
PageSpeed for the site). A policy can instead list the hash of each inline block, and
`Element::csp_hash`
computes it from the element, so the hash cannot drift from the code. *Run:*

```rust
use molt::prelude::*;
use molt::Document;

// The script is code written right here, so `raw_text` is correct.
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

Send `policy` in a `Content-Security-Policy` header. The hash covers the exact bytes between the
tags, so edit the script and the hash changes with it. Whether a given browser accepts the
policy was not tested here.

### 6. Cache for a year by putting the content in the file name

With fixed names (`site.css`) a cache can only be told to keep a file for a short time, because
a rebuild would otherwise leave visitors on the old copy. The site reported exactly that
locally. A file named by its content can be cached as `public, max-age=31536000, immutable`.
`StaticSiteGenerator::copy_asset_hashed`
copies a file to `name.<hash>.ext` and returns the URL to put in the page. It renames one file
and rewrites nothing inside it: a module that imports another by relative path, and a
`url(...)` in a stylesheet, still point at the old names. Either concatenate your scripts into
one file, or copy the files a stylesheet refers to first and write their returned URLs into the
stylesheet you then copy. Compiled, not run (it writes to disk):

```rust,no_run
use molt::prelude::*;
use molt::ssg::StaticSiteGenerator;

let site = StaticSiteGenerator::new("dist");
site.clean(true)?; // an earlier build's hashed files go with it

let font = site.copy_asset_hashed("static/fonts/inter.woff2", "fonts")?;
// ...write `font` into the stylesheet that names it, then:
let css = site.copy_asset_hashed("static/css/site.css", "css")?;

let head = [preload_font(&font), stylesheet(&css)];
# let _ = head;
# Ok::<(), molt::ssg::SsgError>(())
```

## Measuring it

These are the lessons from the site's measurements, reported with Lighthouse 13.5.0 in mobile
emulation against `localhost` served with gzip.

- **PageSpeed's simulated mode does not see animation or fonts.** Variants that changed an
  animation and a font preload gave identical numbers to the second decimal until the run
  applied real throttling (`--throttling-method=devtools`).
- **Discard the first run.** The baseline measured a Speed Index of 2.36 s once and 1.65 s on
  the next three.
- **Layout shift can be hidden.** PageSpeed reported a CLS of 0 while a throttled run measured
  0.086 (the "good" limit is 0.1). Field data is what counts, and the site has none yet.
- **A fade can hide a layout problem.** Without the entrance animation the CLS rose to 0.137,
  from the font swap.
- **Delaying a decorative loop made Speed Index worse** (2.3 s against 1.89 s), because
  Lighthouse compares every frame with the last one, and a late movement makes the last frame
  differ from all the earlier ones.

The effect of any of this on the live PageSpeed score is confirmed only after the deploy.
