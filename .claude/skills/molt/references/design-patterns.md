# Design patterns: what this repository uses, and what it refuses

A **guide, not a rulebook**. <https://refactoring.guru/design-patterns/rust> is a catalog of the
22 Gang-of-Four patterns with Rust examples; it says nothing about Rust-specific idioms such as
typestate or newtype, and a pattern is a vocabulary for something you already have, not a
target to hit. The question to ask is "what does this buy that a function does not?".

For Molt the answer is usually "nothing", and the 1.0 plan rejected most patterns on purpose.
Read this *before* proposing a trait, a visitor, or a new abstraction.

## Used here, and where

| pattern (refactoring.guru) | Molt's form | where |
| --- | --- | --- |
| **Composite** | a closed `enum Node` (`Element`, `Text`, `Raw`, `Comment`, `Fragment`) | `src/core/node.rs` |
| **Builder** — the fluent kind | `self -> Self` chained setters; **not** the GoF Director/trait form | `Element`, `SeoBuilder`, `SitemapUrl`, `RssItem`, `RssGenerator` |
| **Strategy** | a *value*: `RenderOptions` passed in. No trait, no global | `src/core/render.rs` |
| **Facade** | `Document` (html/head/body), `SeoBuilder::build`, `SitemapGenerator`, `RssGenerator` | `document.rs`, `seo.rs`, `sitemap.rs`, `feed.rs` |
| **Iterator** | `IntoIterator` parameters; the writer's explicit work stack replaces recursion | `children_from`, `Step` in `node.rs` |
| **Flyweight** | `Cow<'static, str>` borrowing a literal tag/key. No interner, no cache | `Element::from_static`, `Attribute::new_static` |
| **Command** (internal only) | `Step<'a>`, a reified unit of work on a stack, incl. the `DropIfEmpty` rollback | `node.rs` — private |
| **Newtype** (Rust idiom, not in the catalog) | `Markup`: a string with no public constructor | `node.rs` |
| **Template Method** | `Layout::wrap_many` is provided on top of `wrap` | `src/layout.rs` |

## Refused, with the reason

| pattern | why not here |
| --- | --- |
| **Composite as `Box<dyn Component>`** | loses `Clone`, `PartialEq`, `Hash`, and `Send + Sync` for free; an enum is exhaustive and cheap. The file header of `node.rs` argues it |
| **Visitor (public trait)** | a public trait is frozen API and there is one consumer. If you need to walk a tree, write a private iterator with an explicit stack (that is what `audit` does) |
| **Escaper / strategy trait for escaping** | three free functions chosen at the call site are the right shape; a trait has no second implementer and would be frozen |
| **Decorator** | there is nothing wrapping the same interface; `From<Element> for Node` is conversion, not decoration |
| **Command/undo, Memento, Observer, State** | no history, no reactivity, no mutable lifecycle. The mental model is "build a value, serialize once" |
| **Typestate for void elements** (`Element<Void>` with no `.child`) | would make `Element` generic and break `Vec<Element>`, `ALL_TAGS`, `From<Element> for Node`; and "void drops children silently" is the *parity* contract |
| **Director / `Builder` trait with an associated output** | exists to build several product types; Molt builds one |
| **Global render switch** | AGENTS.md rule 10: `RenderOptions` is passed by value. Swift had a global and is removing it |
| **A second code path in `html!`** | AGENTS.md rule 9: `html!` is sugar over the builder. Any form it accepts must expand to the same tree, and a test asserts it |
| **Interior mutability / `Rc` in the tree** | breaks `Send + Sync`, which `parallel` relies on |
| **Singleton / lazy global registry** (tag tables, interner) | `matches!` or a sorted `&'static [&str]` is enough |

## Decision procedure

1. Can a function do it? Write the function.
2. Can a value do it (`RenderOptions`, an enum)? Use the value.
3. Does the abstraction have **two** implementers *today*? If not, do not publish a trait.
4. Will it appear in a `pub` signature? Then it is semver: go to
   [`rust-guidelines.md`](rust-guidelines.md) § API, and justify it in the PR.
5. Does it change what `html!` expands to, the writer's bytes, or the three escapers? Stop; it
   is probably a parity or rule 6/9/10 violation.

## If a pattern genuinely helps

Keep it `pub(crate)`. The private iterative walkers (`clone_tree`, `trees_equal`, `hash_tree`,
`walk` in the audit) are a visitor in everything but name, and cost the public API nothing.
Promote to `pub` only when a second, real consumer exists and you accept freezing it.
