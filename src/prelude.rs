//! One import that gets you from nothing to a rendered page.
//!
//! ```
//! use molt::prelude::*;
//!
//! let page = div().add_class("card").child(h1().text("Hello"));
//! assert_eq!(page.render(), r#"<div class="card"><h1>Hello</h1></div>"#);
//! ```
//!
//! This brings all 93 element constructors into scope. If that is too many names, import
//! [`crate::elements`] and qualify them instead.

pub use crate::core::{Attribute, Element, Node, Render, RenderOptions};
pub use crate::elements::*;
