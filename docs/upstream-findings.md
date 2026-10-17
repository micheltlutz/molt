# Findings for Winged-Swift

Molt is a port of Winged-Swift, and porting means reading it closely. These are the places
where the Swift behaviour is unsafe or wrong, that Molt fixes instead of copying. They are
written down so they can be reported upstream and fixed there too.

**Examined:** [micheltlutz/Winged-Swift](https://github.com/micheltlutz/Winged-Swift) at
`68e7bf9` (the merge of `fix/3.0.1`, 3.0.0 plus four commits). Every "Swift" line below was
checked against that source on the date of this file's commit; none is inferred from the port.
Whether a finding is exploitable in a given Swift site depends on how that site passes data.

Each entry gives the Swift location, the input that shows it, what Molt does, and the Molt
test that asserts it. The Molt side is in [`PORTING.md`](../PORTING.md) and
[`CHANGELOG.md`](../CHANGELOG.md).

| # | severity | Swift | Molt |
| --- | --- | --- | --- |
| 1 | medium | `<` and `>` in attribute values inside raw-text parents | escaped there |
| 2 | medium | `Meta(name:)` inserts the name unescaped | escaped |
| 3 | medium | tag names are not validated | invalid tag not rendered |
| 4 | medium | URL policy ignores `srcdoc`, `srcset`, `ping` and `data:` in frames | covered |
| 5 | medium | `clean()` has no guard | refuses `.`, cwd, `$HOME`, top level |
| 6 | low | `escapeXML` keeps characters XML 1.0 cannot hold | dropped |
| 7 | low | `isPermaLink="true"` for every guid | only for `http(s)` |
| 8 | low | sitemap and feed links are not checked | `http(s)` only |
| 9 | low | writes are not atomic | temp file + rename |

## 1. `<` and `>` in attribute values under raw-text parents

- **Swift:** `HTMLEscape.escapeAttribute` (`core/HTMLEscape.swift:43`) escapes `&`, `"` and
  `'` only. That is fine in an attribute the parser reads as an attribute.
- **Input:** `Noscript` containing `Img(alt: "</noscript><img src=x onerror=alert(1)>")`.
  Under `noscript` (with scripting on), `iframe`, `xmp`, `noembed`, `noframes`, `script`,
  `style` and `plaintext` the parser is in a raw-text state and the first `</noscript>` ends
  the element, wherever it appears.
- **Molt:** the writer escapes `<` and `>` in attribute values of descendants of those
  elements, and nowhere else, so the fixtures are untouched.
  Test: `angle_brackets_in_attributes_are_escaped_only_under_raw_text_parents`.
- **Suggested upstream fix:** the same contextual rule, or escape `<` `>` in attributes
  everywhere and regenerate the fixtures.

## 2. `Meta(name:)` inserts the name unescaped

- **Swift:** `HTML/Commons/Meta.swift:13`: `Attribute(key: "name", value: name, escape: false)`.
- **Input:** `Meta(name: "a\"><script>alert(1)</script><x y=\"", content: "v")`. The helper is
  `public` and takes a `String`.
- **Molt:** `meta_name`, `meta_property`, `meta_charset` and `meta_http_equiv` escape.
  Test: `a_hostile_meta_name_cannot_break_out_of_its_attribute`.
- **Suggested upstream fix:** drop `escape: false`; legitimate names are unchanged by
  escaping.

## 3. Tag names are not validated

- **Swift:** `core/HTMLTag.swift:28`: `init(_ name: String, …)` stores `name` as given and the
  renderer writes it between `<` and `>`.
- **Input:** `HTMLTag("img src=x onerror=alert(1)")`, or `HTMLTag("div><script>…")`.
- **Molt:** a tag that is not a letter followed by letters, digits, `-`, `_`, `.` or `:` is
  not rendered, children included, and leaves no blank line in pretty mode.
  Test: `an_unsafe_tag_name_renders_nothing`.
- **Suggested upstream fix:** validate at render time with the same production.

## 4. The URL policy's gaps

- **Swift:** `core/Attribute.swift:66-71`: the attributes are `href`, `src`, `action`,
  `formaction`, `poster`, `cite`, `background`, `xlink:href`, `data`, and the blocked schemes
  are `javascript:`, `vbscript:` and `data:text/html`.
- **Missing:** `ping`, `manifest`, `longdesc`; `srcset` (a list, so each candidate needs
  checking); `srcdoc` (a whole HTML document the frame runs); and `data:image/svg+xml` or
  `data:application/xhtml+xml` as the `src` of an `iframe`, `frame`, `embed` or `object`,
  which run script. The same SVG as an `<img>` source is inert, so the rule needs the tag.
- **Molt:** all covered; `srcdoc` is written empty unless `trusted_url`.
  Tests: `data_documents_are_blocked_where_they_run_and_allowed_where_they_do_not`,
  `srcdoc_is_blocked_unless_it_is_opted_into`, `srcset_is_checked_one_candidate_at_a_time`.

## 5. `clean()` has no guard

- **Swift:** `static/StaticSiteGenerator.swift:143`: `clean` calls `removeIfPresent` on
  `outputDirectory` as it is.
- **Input:** `StaticSiteGenerator(outputDirectory: ".")`, `"/"`, or `NSHomeDirectory()`.
- **Molt:** `clean` refuses an empty path, a root, `..`, the working directory and its
  ancestors, `$HOME` and top-level directories. Note for anyone porting the guard:
  `Path::new(".").parent()` is `Some("")`, so a check built on "has no parent" lets `.`
  through. Test: `the_clean_guard_refuses_the_working_directory_and_what_contains_it`.

## 6. `escapeXML` keeps characters XML 1.0 cannot hold

- **Swift:** `feed/RSSGenerator.swift:199` and `seo/SitemapGenerator.swift:118` replace
  `& < > " '` and nothing else.
- **Input:** a title containing U+0008, or any of U+0000–U+0008, U+000B, U+000C,
  U+000E–U+001F, U+FFFE, U+FFFF. Even as `&#8;` it is not representable; the document is
  ill-formed and a reader rejects all of it.
- **Molt:** those characters are dropped.
  Test: `xml_drops_exactly_the_characters_xml_1_0_cannot_hold`.

## 7. `isPermaLink="true"` for every guid

- **Swift:** `feed/RSSGenerator.swift:173`, and a test that expects it for `urn:uuid:1234`
  (`RSSGeneratorTests.testExplicitGuidWinsOverTheLink`).
- **Why it is wrong:** RSS 2.0 lets a reader assume a `true` guid is a permalink it may open.
- **Molt:** `true` only for an `http(s)` guid. The ported test asserts `false` and says so.
  Test: `an_explicit_guid_wins_over_the_link`.

## 8. Sitemap and feed links are not checked

- **Swift:** the `loc` and `link` strings go through `escapeXML` and no scheme check.
- **Input:** a `link` of `javascript:alert(1)`; readers render it as something to click.
- **Molt:** a `loc` or `link` that is not an absolute `http(s)` URL is omitted.
  Tests: `an_entry_whose_loc_is_not_http_is_left_out`,
  `a_link_that_is_not_http_is_left_out_of_the_feed`.

## 9. Writes are not atomic

- **Swift:** the generator writes in place, so a reader or a failed write can see a partial
  page, and two pages written in parallel are not isolated.
- **Molt:** a unique temporary file in the destination directory, then a rename;
  `copy_asset` copies beside the destination and renames.
  Tests: `pages_with_the_same_stem_do_not_share_a_temporary_file`,
  `a_failed_asset_copy_keeps_the_previous_file`.

## Not findings

`HTMLTag` is an `open class` (`core/HTMLTag.swift:4`), a reference type, and Swift's
`ROADMAP.md` records the decision to keep it that way (#38: trees are not `Sendable`, sharing
is rare and solved by `HTMLTag.copy()`). That is a design choice, not a finding; Molt's value
tree is `Send + Sync` because it is a different design.

Not examined, so not reported: whether Swift's `write(into:options:indentLevel:)` recurses once
per nesting level (Molt's did too, until its writer was made iterative), and the Swift
`RenderOptions` indent. Both are worth a look by whoever reports the above.
