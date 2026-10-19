# Security: OWASP Top 10 (2021) mapped onto Molt

Molt generates HTML and XML strings and writes files. Its security promise is narrow, and it
is stated in `SECURITY.md`: **text and attribute values you pass in cannot become markup.**
This page maps each OWASP category onto what exists in the code, what was found and fixed,
and what an agent must never do. Source of the categories: OWASP Top 10:2021,
<https://owasp.org/Top10/>.

## The surface, in one table

Every place a caller-controlled string reaches the output, and how it is made safe. If you add
a new such place, add a row *and* a test.

| input | enters through | made safe by | where |
| --- | --- | --- | --- |
| text content | `text()`, `Node::text`, `html!` `"…"`/`(expr)` | `escape_text` (`& < > " '`) on entry | `src/core/escape.rs` |
| attribute value | `attr()`, `Attribute::new` | `escape_attribute` (`& " '`) on entry; `<` `>` escaped at write under raw-text ancestors | `escape.rs`, `attribute.rs`, `node.rs` |
| URL attribute value | `href`, `src`, `action`, `ping`, `srcset`, … | scheme policy at write time (`javascript:`, `vbscript:`, `data:text/html` → `#`), also on raw values; on `iframe`/`frame`/`embed`/`object` only raster `data:` images pass | `src/core/url_policy.rs` |
| `srcdoc` | `attr("srcdoc", …)` | blocked (written empty) unless `trusted_url` | `url_policy.rs` |
| attribute **name** | `attr(key, …)` | omitted if it holds space, quote, `=`, `<`, `>`, `/`, control, or is empty | `attribute.rs` |
| tag **name** | `Element::new`, `new WElement` | element dropped unless letter + `[A-Za-z0-9-_.:]*` | `tags::is_valid_tag_name` |
| comment body | `Node::comment` | no adjacent hyphens, no trailing hyphen | `node.rs` |
| `<meta>` name/value | `meta_name`, … | `Attribute::new` (escaped) | `src/seo.rs` |
| XML text | sitemap, RSS | `escape_xml` (`'` → `&apos;`; XML-illegal characters dropped) | `sitemap.rs`, `feed.rs` |
| sitemap `<loc>`, RSS `<link>` | `SitemapUrl`, `RssItem`, `RssGenerator` | omitted unless absolute `http(s)` (`url_policy::is_http_url`) | `sitemap.rs`, `feed.rs` |
| output file path | `StaticSiteGenerator::*` | component whitelist, symlink refusal, atomic write | `src/ssg.rs` |
| indent string | `RenderOptions::with_indent` | only spaces and tabs, at most 16; the field is private | `render.rs`, `wasm.rs` |
| nesting depth | any tree | `RenderOptions::with_max_depth` drops deeper elements (opt-in) | `render.rs`, `node.rs` |
| JSON in `<script>` | `seo::json_ld`, `seo::script_json` | `escape_json_script` (JSON `\uXXXX` escapes), not HTML escaping | `escape.rs`, `seo.rs` |
| `raw_text`, `Node::raw`, `Attribute::raw` | caller | **not** escaped — the three declared hatches | — |

## A01 Broken Access Control → path traversal and destructive operations

Found and fixed (#6, #7, #8): `..` was checked but not symlinks; `""`, `"."`, `"a/"` named the
output directory itself; Windows drive prefixes replaced the base on `join`; `clean(".")` could
delete the working directory because `Path::new(".").parent()` is `Some("")`.

Rules: accept only `Component::Normal`/`CurDir`; require a file name; refuse existing symlink
components; make destructive guards pure functions and test them; refuse the cwd, its
ancestors, `$HOME` and top-level directories. Document the race you do not close.

## A02 Cryptographic Failures — not applicable

No cryptography, secrets or tokens in the library. Do not add any. The release workflow mints
a short-lived crates.io token (Trusted Publishing) and passes it to one step as
`CARGO_REGISTRY_TOKEN`; keep it out of logs and out of any file you write.

## A03 Injection → XSS, the whole point

The largest category here. Found and fixed: comment breakout (#2), unvalidated tag names (#3),
unescaped `meta_*` helpers (#4), attribute `<`/`>` under raw-text ancestors (#5).

Rules:

- Escape on entry, once. Never pre-escape (`&amp;amp;`).
- Treat names, comment bodies, paths, and indent strings as input too (L2).
- A neutraliser is proved on its output, not its pattern (L1).
- Context matters: the same `<` is inert in a quoted attribute and fatal inside `<noscript>`
  (L3). When the fixtures forbid the safe default everywhere, make it contextual and document it.
- `raw_text`, `Node::raw`, `Attribute::raw` take trusted, hand-written markup only. A comment
  next to every use says why it is safe.
- `<script>` and `<style>` bodies are **not** escaped by default.
  JSON data goes through `seo::json_ld` / `seo::script_json`, which use `escape_json_script`
  (`<` `>` `&` U+2028 U+2029 as JSON `\u` escapes). HTML-escaping would be wrong there: JSON
  does not decode `&lt;`. Untrusted text in script *code* has no safe form.
- Event-handler attributes (`on*`) and `style` are accepted by `attr()`. Do not map untrusted
  *keys* into `attr()`.
- A URL policy that ignores the element is wrong in both directions: `data:image/svg+xml` is
  inert on an `<img>` and runs script in an `<iframe>`. Pass the tag (`is_blocked_in`).

## A04 Insecure Design → unrepresentable states

The `Markup` newtype exists because "never pass user data here" is not a control (L11). Prefer
a type that cannot be built wrongly. Prefer an enum to a string for a closed set (`ChangeFreq`).
Fail closed: an invalid tag is dropped, a blocked URL becomes `#`, an unsafe `clean()` target
returns an error.

## A05 Security Misconfiguration → CI and defaults

Done (S11): `ci.yml` is `contents: read`; every action is pinned to a commit SHA (version in a
comment, kept current by Dependabot); checkouts use `persist-credentials: false`; jobs have
timeouts; the release refuses a tag not on `main` or without a green CI run; `verify.sh` uses
`mktemp`. **Still the owner's:** required reviewers on the `release` environment and Trusted
Publishing on crates.io. When you touch workflows: least privilege, pin by SHA (resolve it
with `gh api repos/<owner>/<repo>/commits/<tag> --jq .sha`, never from memory), never print
secrets, never `pull_request_target` with a checkout of untrusted code. You cannot run a
workflow locally: say which changes were only syntax-checked.

## A06 Vulnerable and Outdated Components

`deny.toml` bans yanked crates, wildcards, unknown registries/git; licences MIT, Apache-2.0,
BSD-2/3. `Cargo.lock` is committed and CI uses `--locked`. Runtime dependencies are all
optional (`wasm-bindgen`, `rayon`). A new **dev**-dependency still needs a licence check
(`stats_alloc`, MIT, was added for allocation tests). Prefer no dependency over a small one.

## A07 Identification and Authentication Failures — not applicable

## A08 Software and Data Integrity Failures

Atomic writes (temp file + `rename`, `create_new`) so a reader never sees half a page; unique
temp names so parallel writers do not share one (B1). Release: no unreviewed publish, no
`--no-verify` without the verify job (#29). Fixtures are integrity evidence: never hand-edit,
never regenerate to silence a failure.

## A09 Security Logging and Monitoring Failures

The library does no logging, so there is no log injection. Errors echo the caller's own path.
Do not add logging of user content.

## A10 Server-Side Request Forgery — not applicable

No network access in the library.

## Resource exhaustion (a denial-of-service lens, not its own OWASP item)

- Depth: rendering, audit, clone, eq, hash and drop are iterative; 100,000 levels are tested.
- Output: pretty mode is O(depth²) in output size (an indent per level per line). Bound depth
  from untrusted input with `with_max_depth`, render compact, or set an empty indent.
- XML: `escape_xml` drops the characters XML 1.0 cannot hold, links must be `http(s)`, and the
  sitemap limits are `MAX_URLS`/`MAX_BYTES` with `chunks` (S9). A single stray control
  character used to make a whole feed unparseable.

## Reviewing a diff for security (5 minutes)

1. `grep -n "raw_text\|Node::raw\|Attribute::raw\|trusted_url"` in the diff: each needs a
   reason in a comment.
2. Does a new string reach the output? Add a row to the surface table and a hostile-input test.
3. Does it walk a tree? Explicit stack, plus a 100,000-level test.
4. Does it touch a path? Component whitelist, symlink check, pure guard, unique temp name.
5. Does it add a `pub` item? See [`rust-guidelines.md`](rust-guidelines.md) § API.
