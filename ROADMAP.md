# Roadmap

What comes after 1.0.0. These are intentions and leads, **not promises or dates**; the
authority on what is planned is the [issue tracker](https://github.com/micheltlutz/molt/issues).
What 1.0.0 contains is in [`CHANGELOG.md`](CHANGELOG.md).

## The 1.x line

**Compatibility.** Anything in the public API at 1.0.0 keeps working across 1.x. New variants
and fields are possible on the `#[non_exhaustive]` types; new required methods on `Render`
are not. A change to rendered output for valid input needs a fixture diff and a line in
`MIGRATION.md`'s *Output changes* table; a change that only affects input that used to be
unsafe is a security fix and goes in a patch release.

## Likely next

- **Report the findings upstream.** [`docs/upstream-findings.md`](docs/upstream-findings.md)
  lists nine places where Winged-Swift 3.0.1 is unsafe or wrong. Opening them there is the
  next step; it is outstanding because it needs the owner's go-ahead.
- **Trusted Publishing on crates.io**, and required reviewers on the `release` environment.
  Both are repository settings; the release workflow keeps working with the token until then.
- **An iterative `Debug`** for `Node` and `Element`. `Clone`, `PartialEq`, `Hash` and `Drop`
  are iterative; `Debug` is the one derive left, so `{:?}` on a very deep tree still aborts.
- **Fuzzing in the nightly workflow.** The targets exist (`scripts/fuzz.sh`: both escapers,
  the URL policy, the JSON-in-script escaper) but need nightly and are not in the pull-request
  gate.
- **Keep the pins current.** Dependabot proposes SHA bumps for the actions; each needs a
  look at the action's changelog.

## Leads, if there is a reason

Each of these was measured or reasoned and **not** done; the evidence is on the issue.
Reopen with a benchmark that shows the cost, not with "it should be faster".

- `Node::Element(Box<Element>)`: `Node` is 104 bytes, so a text node costs 104. Boxing would
  cost one allocation per element child. ([#37](https://github.com/micheltlutz/molt/issues/37))
- An `O(depth)` work stack instead of `O(width)`. ([#36](https://github.com/micheltlutz/molt/issues/36))
- Caching attribute validity for trees that are rendered many times.
  ([#34](https://github.com/micheltlutz/molt/issues/34))

## Winged-Swift features Molt does not have

Winged-Swift 2.1–2.4 grew a `Site`/`Page` layer, a Markdown parser, a CLI, an Atom feed and a
`robots.txt` generator. They are out of scope for the core crate: Markdown has `pulldown-cmark`
and a server has its own routing. If one of them is wanted, it would be a separate crate that
depends on this one, not a feature of it.

## Deliberately not planned

- **`unsafe`.** `unsafe_code = "forbid"` stays; some micro-optimisations are simply
  unavailable, and the benchmarks are read with that in mind.
- **A public `Visitor`, `Escaper` or strategy trait.** One consumer; frozen API.
  ([`design-patterns.md`](.claude/skills/molt/references/design-patterns.md))
- **A global render switch**, **sorting attributes**, **a `Drop` on `Node`**.
- **SIMD in the escaper**, a custom allocator, changing `panic = "abort"`.
