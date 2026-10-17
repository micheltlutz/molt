// Type-level smoke test.
//
// Generated `.d.ts` files that do not typecheck are a silent break: nothing fails until a
// TypeScript user tries to build. `scripts/verify-npm.sh` runs `tsc --noEmit` over this
// file to catch it.

import { element, WDocument, WSeo, escapeText, version } from "./bundler/molt.js";

const card: import("./bundler/molt.js").WElement = element("div")
  .addClass("card")
  .setId("hero")
  .setStyle("color: red")
  .attr("data-kind", "promo")
  .dataAttr("id", "7")
  .ariaAttr("label", "Promotion")
  .boolAttr("hidden")
  .text("Fuel, tyres & chain")
  .child(element("p").rawText("<b>bold</b>"))
  .comment("a note");

const compact: string = card.render();
const pretty: string = card.renderPretty();

const page: WDocument = new WDocument("pt-BR")
  .addHead(element("title").text("RideKeeper"))
  .addBody(card);

const html: string = page.render();
const minified: string = page.renderCompact();
const indented: string = page.renderWithIndent("    ");

// `lang` is optional — `undefined` must be accepted.
const noLang: WDocument = new WDocument(undefined);

const seo: string = new WSeo("RideKeeper", "Motorcycle maintenance companion")
  .image("https://ridekeeper.example/og.jpg")
  .url("https://ridekeeper.example")
  .twitterSite("@micheltlutz")
  .render();

const escaped: string = escapeText("<script>");
const v: string = version();

// Keep every binding used, so `noUnusedLocals` stays available to the project.
export const surface = {
  compact,
  pretty,
  html,
  minified,
  indented,
  noLang,
  seo,
  escaped,
  v,
};
