//! The core engine: the node tree, the renderer and escaping.
//!
//! Every type is reachable by two paths — `molt::core::Element` and
//! `molt::core::element::Element` are the same item — and **both are stable API**. The
//! short one is what the examples use; the module path is there for the helpers that have
//! no re-export (`molt::core::escape::escape_text`, `molt::core::tags::is_void`).

pub mod attribute;
pub mod element;
pub mod escape;
pub mod node;
pub mod render;
pub mod tags;
pub mod url_policy;

pub use attribute::Attribute;
pub use element::Element;
pub use node::Node;
pub use render::{Render, RenderOptions};
