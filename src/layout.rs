//! Reusable page layouts.
//!
//! Ports `Winged-Swift/Sources/WingedSwift/templates/Layout.swift` — the only protocol in
//! the entire Swift library.

use crate::core::Node;
use crate::elements::div;

/// Wraps content in a consistent structure: a header, a footer, navigation.
///
/// # A footgun that does not exist here
///
/// Winged-Swift's docs spend considerable space warning that `HTMLTag` is a reference type,
/// so reusing one instance in two places silently shares the node — which is why its
/// documented component pattern is "a function that returns a fresh tag each call". In Rust
/// [`Node`] is a value type, so reuse copies. Write components however you like.
///
/// # Examples
/// ```
/// use molt::prelude::*;
/// use molt::Layout;
///
/// struct Blog { site_title: String }
///
/// impl Layout for Blog {
///     fn wrap(&self, content: Node) -> Node {
///         body()
///             .child(header().child(h1().text(&self.site_title)))
///             .child(main_tag().child(content))
///             .child(footer().child(p().text("© 2026")))
///             .into()
///     }
/// }
///
/// let page = Blog { site_title: "RideKeeper".into() }.wrap(p().text("Hello").into());
/// assert!(page.render().contains("<h1>RideKeeper</h1>"));
/// ```
pub trait Layout {
    /// Wraps a single node in the layout.
    ///
    /// Named `wrap`, not `render`: a type that is both a `Layout` and a [`Render`](crate::core::Render)
    /// would otherwise have two `render` methods and every call would be ambiguous.
    fn wrap(&self, content: Node) -> Node;

    /// Wraps several nodes, grouping them in a `<div>` first.
    ///
    /// Matches the Swift extension, which also introduces a `Div`. Use
    /// [`Node::fragment`] and [`Layout::wrap`] if you would rather not have the wrapper.
    fn wrap_many(&self, contents: Vec<Node>) -> Node {
        self.wrap(div().children_from(contents).into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Render;
    use crate::elements::{body, h1, main_tag, p};

    struct Minimal;

    impl Layout for Minimal {
        fn wrap(&self, content: Node) -> Node {
            body()
                .child(h1().text("Site"))
                .child(main_tag().child(content))
                .into()
        }
    }

    /// Ports `LayoutTests.wrapsASingleTag`.
    #[test]
    fn a_layout_wraps_a_single_node() {
        let page = Minimal.wrap(p().text("body").into());
        assert_eq!(
            page.render(),
            "<body><h1>Site</h1><main><p>body</p></main></body>"
        );
    }

    /// Ports `LayoutTests.wrapsSeveralTagsInAContainer`.
    #[test]
    fn wrap_many_groups_the_contents_in_a_div() {
        let page = Minimal.wrap_many(vec![p().text("a").into(), p().text("b").into()]);
        assert_eq!(
            page.render(),
            "<body><h1>Site</h1><main><div><p>a</p><p>b</p></div></main></body>"
        );
    }

    /// Ports `LayoutTests.anEmptyContentListStillProducesAPage`.
    #[test]
    fn wrap_many_with_no_contents_still_produces_the_wrapper() {
        let page = Minimal.wrap_many(vec![]);
        assert!(page.render().contains("<div></div>"));
    }
}
