//! Fast, type-safe HTML generation for Rust and WebAssembly.
//!
//! A port of [Winged-Swift](https://github.com/micheltlutz/Winged-Swift) 3.0.1: the same
//! composite tree, the same escape-by-default guarantee, the same compact and pretty
//! render modes — compiled to native code or to `wasm32-unknown-unknown`. Where the Swift
//! original is unsafe or wrong, Molt fixes it instead of copying it.
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
//! # Parity
//!
//! The crate is verified against Winged-Swift's own golden fixtures — the same bytes, from
//! the same page, built through both APIs. Behavioural differences that were introduced on
//! purpose are listed in `PORTING.md`.

#![cfg_attr(docsrs, feature(doc_cfg))]

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
/// because both use `molt::ssg`; compiled only when running doc tests.
#[cfg(all(doctest, feature = "ssg"))]
mod guides {
    #[doc = include_str!("../GETTING_STARTED.md")]
    mod getting_started {}

    #[doc = include_str!("../MIGRATION.md")]
    mod migration {}
}
