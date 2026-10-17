//! Fast, type-safe HTML generation for Rust and WebAssembly.
//!
//! A port of [Winged-Swift](https://github.com/micheltlutz/Winged-Swift) 2.0.0: the same
//! composite tree, the same escape-by-default guarantee, the same compact and pretty
//! render modes — compiled to native code or to `wasm32-unknown-unknown`.
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
//! URL attributes are checked **when the tree is written**. `href`, `src`, `action`,
//! `formaction`, `poster`, `cite`, `background`, `xlink:href` and `data` render as `#`
//! when the value is a `javascript:`, `vbscript:` or `data:text/html` URL, including a
//! value stored with [`Attribute::raw`](core::Attribute::raw). [`Element::trusted_url`]
//! opts out for a URL you wrote yourself. See `SECURITY.md`.
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
//! Rendering has no depth limit: the writer walks an explicit stack instead of recursing.
//! What deep trees do cost is output size in pretty mode, which is quadratic in depth
//! because every line carries one indent string per level. See [`Render`] and
//! `SECURITY.md`.
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
