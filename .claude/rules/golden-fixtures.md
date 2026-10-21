---
paths:
  - "tests/fixtures/**"
  - "tests/golden.rs"
  - "src/core/render.rs"
  - "src/core/escape.rs"
  - "src/core/attribute.rs"
  - "src/core/node.rs"
  - "src/core/element.rs"
  - "src/document.rs"
  - "src/sitemap.rs"
  - "src/feed.rs"
---

# Golden fixtures

Four files in `tests/fixtures/` (a marketing page compact and pretty, a sitemap, a feed) pin
the output byte for byte. `tests/golden.rs` builds them and compares; `verify.sh` runs it.

- **Never hand-edit a fixture.** A fixture is evidence that markup changed, not scaffolding
  to make a test pass.
- A golden failure means one of two things: the change is wrong, or the output was meant to
  change. Decide which before touching anything.
- Regenerating (`MOLT_UPDATE_FIXTURES=1 cargo test --test golden`) is an "ask every time"
  action: name it, wait for the yes, and do not generalise it to the next change.
- A deliberate output change is complete only with all three: a test that asserts the new
  rendered string, a `CHANGELOG.md` entry under *Security* or *Breaking*, and a row in the
  *Output changes* table of `MIGRATION.md`, so someone upgrading can find it in their own
  output.
- Say so in the PR. A fixture diff is the only place a markup change is visible to a
  reviewer, so the explanation is the review.
- When a fixture-encoded rule conflicts with a security fix, propose the contextual option
  (escape only where exploitable, L3) before widening the output change.
