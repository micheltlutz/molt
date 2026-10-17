//! Rendering options and the [`Render`] trait.
//!
//! Ports `core/RenderOptions.swift` and the render half of `core/HTMLTag.swift`.
//!
//! `MOLT_SPEC.md` §2 claims Winged-Swift has a `protocol HTMLRenderable`. It does
//! not — the only protocol in that library is `Layout`. The real design is a class
//! hierarchy whose single overridable primitive is
//! `write(into:options:indentLevel:)`, and that is what [`Render::write_into`] ports.
//! Everything else is a provided method on top of it, so the whole tree renders into one
//! buffer with one allocation.

/// How a node tree is turned into markup.
///
/// A value passed into the render call, never global state — two threads can render the
/// same tree with different settings at the same time. Winged-Swift's deprecated
/// process-wide `HTMLTag.xhtmlSelfClosing` switch is deliberately not ported.
///
/// # Examples
/// ```
/// use molt::core::RenderOptions;
/// let compact = RenderOptions::compact();
/// let pretty = RenderOptions::pretty();
/// let four_spaces = RenderOptions::pretty().with_indent("    ");
/// # let _ = (compact, pretty, four_spaces);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RenderOptions {
    /// When true, children go on their own lines and are indented.
    pub pretty: bool,
    /// One indentation level. Only used when `pretty` is true. Defaults to two spaces.
    pub indent: String,
    /// When true, void elements close with ` />` instead of `>`.
    pub xhtml_self_closing: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            pretty: false,
            indent: "  ".to_string(),
            xhtml_self_closing: false,
        }
    }
}

impl RenderOptions {
    /// Minified output on a single line. The default, and what you should ship.
    #[must_use]
    pub fn compact() -> Self {
        Self::default()
    }

    /// Indented, human-readable output.
    #[must_use]
    pub fn pretty() -> Self {
        Self {
            pretty: true,
            ..Self::default()
        }
    }

    /// Sets the indentation string.
    #[must_use]
    pub fn with_indent(mut self, indent: impl Into<String>) -> Self {
        self.indent = indent.into();
        self
    }

    /// Closes void elements with ` />` instead of `>`.
    #[must_use]
    pub fn with_xhtml_self_closing(mut self, yes: bool) -> Self {
        self.xhtml_self_closing = yes;
        self
    }

    /// Appends `depth` levels of indentation to a buffer.
    pub(crate) fn write_indent(&self, out: &mut String, depth: usize) {
        // The early return is not just a micro-optimisation: with an empty indent the loop
        // below still runs once per level, which is quadratic in depth and costs minutes on
        // a deeply nested tree while producing nothing.
        if depth == 0 || self.indent.is_empty() {
            return;
        }

        out.reserve(self.indent.len() * depth);
        for _ in 0..depth {
            out.push_str(&self.indent);
        }
    }
}

/// Anything that can be written as HTML.
///
/// Implement [`write_into`](Render::write_into); the rest comes free.
///
/// # Depth
///
/// The provided implementations walk an explicit work stack rather than recursing, so
/// nesting depth costs heap rather than stack and there is no depth at which rendering
/// aborts. A 100,000-level tree is covered by a test.
///
/// Pretty mode still writes one indent string per level on every line, which makes its
/// *output* quadratic in depth. That is a size to be aware of when depth comes from
/// untrusted input — see `SECURITY.md` — not a limit on what renders.
pub trait Render {
    /// Writes this node and its subtree into an existing buffer.
    ///
    /// This is the primitive every other method here is built on. Writing into a shared
    /// buffer avoids allocating an intermediate string per node.
    fn write_into(&self, out: &mut String, options: &RenderOptions, depth: usize);

    /// Renders as a single line of markup.
    ///
    /// Note the asymmetry with [`crate::Document::render`], which defaults to *pretty*.
    /// That difference is in the Swift original and the golden fixtures depend on it.
    fn render(&self) -> String {
        self.render_with(&RenderOptions::compact())
    }

    /// Renders with indentation and line breaks.
    fn render_pretty(&self) -> String {
        self.render_with(&RenderOptions::pretty())
    }

    /// Renders with the given options.
    fn render_with(&self, options: &RenderOptions) -> String {
        let mut out = String::with_capacity(1024);
        self.write_into(&mut out, options, 0);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::{div, img, p};

    /// Ports `RenderOptionsTests.compactIsTheDefault`.
    #[test]
    fn compact_is_the_default() {
        let options = RenderOptions::default();
        assert!(!options.pretty);
        assert_eq!(options.indent, "  ");
        assert!(!options.xhtml_self_closing);
    }

    /// Ports `RenderOptionsTests.indentIsConfigurable`.
    #[test]
    fn the_indent_string_is_configurable() {
        let options = RenderOptions::pretty().with_indent("    ");
        let mut out = String::new();
        options.write_indent(&mut out, 2);
        assert_eq!(out, "        ");
    }

    /// Ports `RenderOptionsTests.optionsAreValues`. Options are a value: changing a
    /// copy must not affect the original.
    #[test]
    fn options_have_value_semantics() {
        let base = RenderOptions::compact();
        let derived = base.clone().with_xhtml_self_closing(true);
        assert!(!base.xhtml_self_closing);
        assert!(derived.xhtml_self_closing);
    }

    /// Ports `RenderOptionsTests.prettyIndentsChildren`.
    #[test]
    fn pretty_indents_children_by_two_spaces() {
        let tree = div().child(p().text("Hi"));

        assert_eq!(tree.render_pretty(), "<div>\n  <p>Hi</p>\n</div>");
    }

    /// Ports `RenderOptionsTests.xhtmlSelfClosingIsPerCall`.
    ///
    /// Winged-Swift used to carry this on a process-wide `HTMLTag.xhtmlSelfClosing` switch
    /// and is removing it. Here it was never anything but a field on the options value, so
    /// a render cannot leak into the next one.
    #[test]
    fn xhtml_self_closing_is_per_call() {
        let tag = img().attr("src", "a.png");

        assert_eq!(tag.render(), r#"<img src="a.png">"#);
        assert_eq!(
            tag.render_with(&RenderOptions::compact().with_xhtml_self_closing(true)),
            r#"<img src="a.png" />"#
        );
        assert_eq!(tag.render(), r#"<img src="a.png">"#);
    }

    /// Ports `RenderOptionsTests.writeAppendsToAnExistingBuffer`.
    #[test]
    fn write_into_appends_rather_than_replacing() {
        let mut buffer = String::from("<!-- header -->");
        div()
            .text("x")
            .write_into(&mut buffer, &RenderOptions::compact(), 0);

        assert_eq!(buffer, "<!-- header --><div>x</div>");
    }

    /// Ports `RenderOptionsTests.concurrentRendersDoNotShareState`.
    ///
    /// The Swift suite needs this because its tree is reference-typed and its options used
    /// to be global. Here the tree is `Send + Sync` and the options are a value, so the
    /// test is a guard against ever reintroducing shared state.
    #[test]
    fn concurrent_renders_do_not_share_state() {
        let results: Vec<(usize, String)> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|index| {
                    scope.spawn(move || {
                        let tag = img().attr("src", format!("{index}.png"));
                        let options =
                            RenderOptions::compact().with_xhtml_self_closing(index % 2 == 0);
                        (index, tag.render_with(&options))
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().expect("thread"))
                .collect()
        });

        for (index, rendered) in results {
            assert_eq!(
                rendered.ends_with(" />"),
                index % 2 == 0,
                "{index} rendered {rendered:?}"
            );
        }
    }
}
