// Node smoke test.
//
// Builds a page through the WebAssembly bindings and diffs it against the *same* golden
// fixture the Rust tests use. If native and WASM ever diverge, this is what catches it.
//
// Run with `node npm/smoke.mjs`, or against a packed tarball — see scripts/verify-npm.sh,
// which installs the tarball into a temp directory first. That is what catches a broken
// `files` or `exports` field, which a workspace link would hide.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import assert from "node:assert/strict";

const here = dirname(fileURLToPath(import.meta.url));
const modulePath = process.env.MOLT_MODULE ?? join(here, "node", "molt.js");
// The Node build is CommonJS, so the named exports arrive under `default` when imported
// from an ES module. The web and bundler builds are ESM and expose them directly.
const imported = await import(modulePath);
const { element, WDocument, WSeo, version } = imported.default ?? imported;

const DESCRIPTION = "Motorcycle maintenance companion";
const OG_IMAGE = "https://ridekeeper.example/og.jpg";

const meta = (key, name, content) => element("meta").attr(key, name).attr("content", content);

const page = new WDocument("pt-BR");

const head = [
  element("meta").attr("charset", "UTF-8"),
  meta("name", "viewport", "width=device-width, initial-scale=1.0"),
  meta("name", "description", DESCRIPTION),
  meta("name", "robots", "index, follow"),
  meta("name", "keywords", "swift, motorcycle"),
  meta("name", "author", "Michel Lutz"),
  meta("property", "og:title", "RideKeeper"),
  meta("property", "og:description", DESCRIPTION),
  meta("property", "og:image", OG_IMAGE),
  meta("property", "og:url", "https://ridekeeper.example"),
  meta("property", "og:type", "website"),
  meta("name", "twitter:card", "summary_large_image"),
  meta("name", "twitter:title", "RideKeeper"),
  meta("name", "twitter:description", DESCRIPTION),
  meta("name", "twitter:image", OG_IMAGE),
  meta("name", "twitter:site", "@micheltlutz"),
  element("title").text("RideKeeper — track every service"),
  element("link").attr("href", "/css/style.css").attr("rel", "stylesheet"),
];

let doc = page;
for (const node of head) doc = doc.addHead(node);

const link = (href, text) => element("a").attr("href", href).text(text);

doc = doc.addBody(
  element("header").child(
    element("nav")
      .child(element("a").attr("href", "/").addClass("logo").text("RideKeeper"))
      .child(
        element("ul")
          .child(element("li").child(link("/", "Home")))
          .child(element("li").child(link("/pricing", "Pricing & plans"))),
      ),
  ),
);

doc = doc.addBody(
  element("main")
    .child(
      element("section")
        .setId("hero")
        .child(element("h1").text("Track every service"))
        .child(element("p").text("Fuel, tyres & chain — all in one place."))
        .child(
          element("a")
            .attr("href", "https://apps.example/app")
            .addClass("button")
            .text("Download"),
        ),
    )
    .child(
      element("table")
        .child(element("caption").text("Plans"))
        .child(
          element("thead").child(
            element("tr").child(element("th").text("Plan")).child(element("th").text("Price")),
          ),
        )
        .child(
          element("tbody")
            .child(
              element("tr").child(element("td").text("Free")).child(element("td").text("R$ 0")),
            )
            .child(
              element("tr")
                .child(element("td").text("Pro"))
                .child(element("td").text("R$ 9,90/mês")),
            ),
        ),
    )
    .child(
      element("details")
        .boolAttr("open")
        .child(element("summary").text("Is my data private?"))
        .child(
          element("p").text("Yes — everything syncs through your own iCloud account."),
        ),
    )
    .child(
      element("form")
        .attr("action", "/subscribe")
        .child(
          element("fieldset")
            .child(element("legend").text("Newsletter"))
            .child(element("label").attr("for", "email").text("E-mail"))
            .child(
              element("input").attr("type", "email").attr("name", "email").boolAttr("required"),
            )
            .child(element("button").attr("type", "submit").text("Subscribe")),
        ),
    )
    .child(element("pre").child(element("code").text("let page = html { }")))
    .child(
      element("figure")
        .child(element("img").attr("src", "/img/app.png").attr("alt", "App screenshot"))
        .child(element("figcaption").text("The garage screen")),
    ),
);

doc = doc.addBody(
  element("footer").child(
    element("p").text("© 2026 RideKeeper — built with Swift & WingedSwift"),
  ),
);

const fixture = readFileSync(
  join(here, "..", "tests", "fixtures", "marketing-pretty.html"),
  "utf8",
);

const rendered = doc.render();

if (rendered !== fixture) {
  const a = fixture.split("\n");
  const b = rendered.split("\n");
  for (let i = 0; i < Math.max(a.length, b.length); i++) {
    if (a[i] !== b[i]) {
      console.error(`line ${i + 1}:`);
      console.error(`  fixture: ${JSON.stringify(a[i] ?? "<missing>")}`);
      console.error(`  wasm:    ${JSON.stringify(b[i] ?? "<missing>")}`);
    }
  }
  throw new Error("WASM output does not match the golden fixture");
}

// Escaping must survive the JS boundary.
assert.equal(
  element("p").text("<script>alert('x')</script>").render(),
  "<p>&lt;script&gt;alert(&#x27;x&#x27;)&lt;/script&gt;</p>",
);

// Pretty printing — impossible under the string-accumulating design the spec proposed.
assert.equal(
  element("div").child(element("p").text("deep")).renderPretty(),
  "<div>\n  <p>deep</p>\n</div>",
);

// The SEO builder is reachable and ordered.
const seo = new WSeo("RideKeeper", DESCRIPTION).image(OG_IMAGE).url("https://ridekeeper.example").render();
assert.ok(seo.indexOf("charset") < seo.indexOf("og:title"));
assert.ok(seo.indexOf("og:title") < seo.indexOf("twitter:card"));

console.log(`ok — WASM ${version()} reproduces marketing-pretty.html byte for byte`);
