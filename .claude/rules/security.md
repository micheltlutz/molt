---
paths:
  - "src/**/*.rs"
  - "SECURITY.md"
  - "fuzz/**"
---

# Security

Every place a caller-controlled string reaches the output is a row in the surface table of
[`security-owasp.md`](../skills/molt/references/security-owasp.md). Start there.

- **Make it safe at write time**, in the one place every construction path goes through, so
  no builder, macro or wasm binding can bypass it. Validation on entry is for values;
  policy that depends on the ancestor (raw-text parents, `data:` URLs on an `iframe`) runs
  at write.
- **Drop, never repair.** An invalid tag drops the element, an invalid attribute name drops
  the attribute, a blocked URL becomes `#`, a non-`http(s)` sitemap `loc` is omitted. A
  guess at what the caller meant is a new injection surface.
- **Pick the escaper for the language the text will be read in** (L26): HTML text, HTML
  attribute, XML and JSON-in-`<script>` are four different things. Reject what the format
  cannot represent (XML-illegal characters), not only what is dangerous (L27).
- **A new string reaching the output** gets a row in the surface table, a hostile-input test
  that asserts the rendered string, and a fuzz target if it is parsed.
- **The escape hatches** are `raw_text`, `Node::raw`, `Attribute::raw` and `trusted_url`.
  Nothing else bypasses escaping or the URL policy; do not add a fifth. Each call site says
  why its input is trusted.
- **Filesystem (`src/ssg.rs`):** output paths go through the component whitelist, the
  symlink refusal and the atomic write. Destructive functions have a pure guard, and the
  guard is what the tests exercise; never point them at the real working directory (L5).
  `clean()` also checks ownership, not just that the path is not catastrophic (L41).
- **`unsafe` is forbidden** crate-wide, tests included. If it seems necessary, the plan is
  wrong: stop and say so.
- A security fix that changes output for some input updates `SECURITY.md`, `CHANGELOG.md`
  (*Security*) and the *Output changes* table of `MIGRATION.md` in the same PR.
- The fix carries the hostile input that used to work, as a test that now fails it.
