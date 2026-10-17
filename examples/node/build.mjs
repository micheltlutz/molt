// Generates a small static site from Node, using the WebAssembly build.
//
// Run with:
//   ./scripts/build-wasm.sh
//   node examples/node/build.mjs
//
// The Rust equivalent is `cargo run --example static_site`. Both produce the same markup —
// same tree, same renderer.

import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";

const here = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);
const { element, WDocument, WSeo } = require(join(here, "../../pkg/node/molt.js"));

const OUT = join(here, "dist");
const SITE = "https://ridekeeper.example";

/** The shell every page shares. */
function shell(titleText, content) {
  let page = new WDocument("pt-BR");

  for (const tag of [
    element("meta").attr("charset", "UTF-8"),
    element("meta")
      .attr("name", "viewport")
      .attr("content", "width=device-width, initial-scale=1.0"),
    element("title").text(titleText),
    element("link").attr("href", "/css/style.css").attr("rel", "stylesheet"),
  ]) {
    page = page.addHead(tag);
  }

  return page
    .addBody(
      element("header").child(
        element("nav")
          .child(element("a").attr("href", "/").addClass("logo").text("RideKeeper"))
          .child(element("a").attr("href", "/blog/").text("Blog")),
      ),
    )
    .addBody(element("main").child(content))
    .addBody(element("footer").child(element("p").text("© 2026 RideKeeper")));
}

const pages = [
  ["index.html", shell("RideKeeper", element("h1").text("Track every service"))],
  [
    "blog/index.html",
    shell(
      "Blog — RideKeeper",
      element("section")
        .child(element("h1").text("Blog"))
        .child(element("p").text("Tyres, chain & fuel — notes from the garage.")),
    ),
  ],
];

for (const [path, page] of pages) {
  const target = join(OUT, path);
  await mkdir(dirname(target), { recursive: true });
  await writeFile(target, page.render(), "utf8");
}

// The SEO block is the same one the Rust crate builds.
const seo = new WSeo("RideKeeper", "Motorcycle maintenance companion")
  .image(`${SITE}/og.jpg`)
  .url(SITE)
  .render();
await writeFile(join(OUT, "meta.html"), seo, "utf8");

await writeFile(join(OUT, "robots.txt"), `Sitemap: ${SITE}/sitemap.xml\n`, "utf8");

console.log(`Generated ${pages.length} pages into examples/node/dist/`);
