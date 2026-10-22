//! Fast, type-safe HTML generation for Rust and WebAssembly.
//!
//! A composite tree of plain values, escaped by default and rendered compact or pretty,
//! compiled to native code or to `wasm32-unknown-unknown`.
//!
//! The project site, built with Molt itself, is <https://www.molt-framework.io>.
//!
//! # Getting started
//!
//! ```
//! use molt::prelude::*;
//!
//! let card = article()
//!     .add_class("card p-4")
//!     .child(h1().text("Welcome to Molt!").add_class("text-2xl"))
//!     .child(p().text("Ultra-fast HTML generation compiled to WebAssembly."));
//!
//! println!("{}", card.render_pretty());
//! ```
//!
//! # Escaping
//!
//! Text content and attribute values are escaped **when they enter the tree**, not when it
//! is rendered, so nothing is escaped twice and nothing is missed. [`Node::Raw`] is the
//! documented way to inject markup you already trust.
//!
//! URL attributes (`href`, `src`, `action`, `srcset`, … — the list is in
//! [`core::url_policy`]) are checked **when the tree is written**: a `javascript:` or
//! `vbscript:` URL, or `data:text/html`, renders as `#`, and so does a `data:` document in
//! an `iframe`, `frame`, `embed` or `object`. The check also covers a value stored with
//! [`Attribute::raw`](core::Attribute::raw), and `srcdoc` is written empty.
//! [`Element::trusted_url`] opts out for a URL you wrote yourself. Tag and attribute names
//! are validated too. See `SECURITY.md`.
//!
//! ```
//! use molt::prelude::*;
//! assert_eq!(p().text("<script>").render(), "<p>&lt;script&gt;</p>");
//! ```
//!
//! # Render modes
//!
//! [`Render::render`] produces a single line; [`Render::render_pretty`] indents. Both take
//! their configuration from a [`RenderOptions`] value rather than global state, so two
//! threads can render the same tree differently at the same time.
//!
//! # Limits
//!
//! Rendering has no depth limit by default: the writer walks an explicit stack instead of
//! recursing, and so do `Clone`, `PartialEq`, `Hash` and `Drop`. What deep trees do cost is
//! output size in pretty mode, which is quadratic in depth because every line carries one
//! indent string per level. When depth comes from untrusted input, bound it with
//! [`RenderOptions::with_max_depth`]. See [`Render`] and `SECURITY.md`.
//!
//! # Golden fixtures
//!
//! The rendered markup of a whole page, a sitemap and a feed is compared byte for byte with
//! files in `tests/fixtures/`, so a change to the output of valid input cannot go unnoticed.
//! `MIGRATION.md` lists what changed between releases.

#![cfg_attr(docsrs, feature(doc_cfg))]
// The logo and favicon shown by rustdoc (docs.rs and the Pages build). The file lives at the
// repository root and is excluded from the package, so the URL has to be absolute.
#![doc(html_logo_url = "https://raw.githubusercontent.com/micheltlutz/molt/main/molt-logo.png")]
#![doc(html_favicon_url = "https://raw.githubusercontent.com/micheltlutz/molt/main/molt-logo.png")]

pub mod accessibility;
pub mod core;
pub mod document;
pub mod elements;
pub mod feed;
pub mod layout;
#[macro_use]
pub mod macros;
pub mod prelude;
pub mod seo;
pub mod sitemap;
#[cfg(all(feature = "ssg", not(target_arch = "wasm32")))]
#[cfg_attr(docsrs, doc(cfg(feature = "ssg")))]
pub mod ssg;
#[cfg(feature = "wasm")]
#[cfg_attr(docsrs, doc(cfg(feature = "wasm")))]
pub mod wasm;

pub use crate::core::{Attribute, Element, Node, Render, RenderOptions};
pub use crate::document::Document;
pub use crate::layout::Layout;

/// The prose guides are doc tests too, so a snippet in them cannot rot. Gated on `ssg`
/// because both use `molt::ssg`, and on not being `wasm32`, because there the module does not
/// exist even with the feature on (`wasm-pack test` runs the doc tests for that target).
/// Compiled only when running doc tests.
#[cfg(all(doctest, feature = "ssg", not(target_arch = "wasm32")))]
mod guides {
    #[doc = include_str!("../GETTING_STARTED.md")]
    mod getting_started {}

    #[doc = include_str!("../MIGRATION.md")]
    mod migration {}

    #[doc = include_str!("../docs/pt-BR/GETTING_STARTED.md")]
    mod getting_started_pt_br {}
}
