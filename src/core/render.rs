//! Rendering options and the [`Render`] trait.
//!
//! [`Render::write_into`] is the single primitive: everything else is a provided method on top
//! of it, so the whole tree renders into one buffer with one allocation.

/// How a node tree is turned into markup.
///
/// A value passed into the render call, never global state — two threads can render the
/// same tree with different settings at the same time. There is deliberately no process-wide
/// switch.
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
    /// One indentation level, only used when `pretty` is true. Private so it can only be set
    /// through [`RenderOptions::with_indent`], which keeps it to spaces and tabs.
    indent: String,
    /// When true, void elements close with ` />` instead of `>`.
    pub xhtml_self_closing: bool,
    /// Elements nested deeper than this are not written. `None` (the default) writes any
    /// depth. See [`RenderOptions::with_max_depth`].
    pub max_depth: Option<usize>,
}

/// The longest indentation level kept. Wider than this is not indentation, and it is
/// multiplied by depth on every line.
const MAX_INDENT_LEN: usize = 16;

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            pretty: false,
            indent: "  ".to_string(),
            xhtml_self_closing: false,
            max_depth: None,
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
    ///
    /// Only spaces and tabs are kept, and at most 16 of them; anything else is dropped. The
    /// indent is written verbatim at the start of every line, so a string with markup in it
    /// (`renderWithIndent(untrusted)` from JavaScript, say) would put that markup on every
    /// line of the page. An indent that is not whitespace is not an indent.
    ///
    /// # Examples
    /// ```
    /// use molt::core::RenderOptions;
    /// assert_eq!(RenderOptions::pretty().with_indent("\t").indent(), "\t");
    /// assert_eq!(RenderOptions::pretty().with_indent("<script>").indent(), "");
    /// assert_eq!(RenderOptions::pretty().with_indent("  <b> ").indent(), "   ");
    /// ```
    #[must_use]
    pub fn with_indent(mut self, indent: impl AsRef<str>) -> Self {
        self.indent = indent
            .as_ref()
            .chars()
            .filter(|c| matches!(c, ' ' | '\t'))
            .take(MAX_INDENT_LEN)
            .collect();
        self
    }

    /// The indentation string for one level: spaces and tabs only.
    #[must_use]
    pub fn indent(&self) -> &str {
        &self.indent
    }

    /// Stops writing elements nested deeper than `max_depth`; the subtree is dropped.
    ///
    /// Pretty output grows with the *square* of the depth (one indent per level on every
    /// line), so a tree whose depth comes from untrusted input can ask for gigabytes. This
    /// bounds it. Browsers flatten nesting beyond roughly 512 elements, so a limit of 512 or
    /// less loses nothing a page could show. Text and fragments do not count as a level.
    ///
    /// # Examples
    /// ```
    /// use molt::prelude::*;
    /// let tree = div().child(div().child(div().child(p().text("deep"))));
    /// let shallow = RenderOptions::compact().with_max_depth(2);
    /// assert_eq!(tree.render_with(&shallow), "<div><div></div></div>");
    /// ```
    #[must_use]
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = Some(max_depth);
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
/// # Implementing it for your own type
///
/// Most types should not: build an [`Element`](crate::core::Element) and render that. Do it
/// when a type *is* a piece of markup and callers want to pass it where a node is expected,
/// by delegating to the tree it builds.
///
/// ```
/// use molt::prelude::*;
///
/// struct Badge(&'static str);
///
/// impl Render for Badge {
///     fn write_into(&self, out: &mut String, options: &RenderOptions, depth: usize) {
///         span().add_class("badge").text(self.0).write_into(out, options, depth);
///     }
/// }
///
/// assert_eq!(Badge("new").render(), r#"<span class="badge">new</span>"#);
/// ```
///
/// Whatever you write must already be escaped: use [`Element`](crate::core::Element) and
/// `text()` rather than pushing user text into `out` yourself.
///
/// # Stability
///
/// `Render` is part of the 1.x API. `write_into(&self, &mut String, &RenderOptions, usize)`
/// will not change, and no **required** method will be added; provided methods may be. The
/// buffer is a `String` on purpose, so a whole tree renders into one allocation, and
/// `depth` is the indentation level the output starts at (`0` for a root). Implementations
/// for `&T`, `Box<T>` and `Option<T>` exist; `Document` deliberately has none, because its
/// inherent `render()` is pretty and this trait's is compact.
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
    /// The golden fixtures depend on that difference.
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

impl<T: Render + ?Sized> Render for &T {
    fn write_into(&self, out: &mut String, options: &RenderOptions, depth: usize) {
        (**self).write_into(out, options, depth);
    }
}

impl<T: Render + ?Sized> Render for Box<T> {
    fn write_into(&self, out: &mut String, options: &RenderOptions, depth: usize) {
        (**self).write_into(out, options, depth);
    }
}

/// `None` writes nothing.
impl<T: Render> Render for Option<T> {
    fn write_into(&self, out: &mut String, options: &RenderOptions, depth: usize) {
        if let Some(inner) = self {
            inner.write_into(out, options, depth);
        }
    }
}

#[cfg(test)]
mod tests {
    /// The indent is written at the start of every line, so markup in it lands on every line.
    #[test]
    fn an_indent_that_is_not_whitespace_is_dropped() {
        for hostile in ["<script>alert(1)</script>", "\n", "x", "  <b>  ", "&nbsp;"] {
            let kept: String = hostile
                .chars()
                .filter(|c| matches!(c, ' ' | '\t'))
                .collect();
            let options = RenderOptions::pretty().with_indent(hostile);
            assert_eq!(options.indent(), kept, "{hostile:?}");
            let rendered = div().child(p().text("x")).render_with(&options);
            assert!(
                !rendered.contains('<') || rendered.matches('<').count() == 4,
                "{rendered}"
            );
            assert!(!rendered.contains("script"), "{rendered}");
        }
        assert_eq!(RenderOptions::pretty().with_indent("\t").indent(), "\t");
    }

    #[test]
    fn an_indent_is_capped_at_sixteen_columns() {
        let options = RenderOptions::pretty().with_indent(" ".repeat(10_000));
        assert_eq!(options.indent().len(), 16);
    }

    #[test]
    fn a_depth_limit_drops_what_is_deeper_in_both_modes() {
        let tree = div()
            .child(div().child(div().child(p().text("deep"))))
            .child(p().text("side"));
        let compact = RenderOptions::compact().with_max_depth(2);
        assert_eq!(
            tree.render_with(&compact),
            "<div><div></div><p>side</p></div>"
        );
        // The dropped subtree leaves its parent open and closed on separate lines, but no
        // blank line between them: the child's separator is rolled back, same as for an
        // invalid tag.
        let pretty = RenderOptions::pretty().with_max_depth(2);
        assert_eq!(
            tree.render_with(&pretty),
            "<div>\n  <div>\n  </div>\n  <p>side</p>\n</div>"
        );
        assert_eq!(
            tree.render_with(&RenderOptions::compact().with_max_depth(0)),
            ""
        );
        assert_eq!(
            tree.render_with(&RenderOptions::compact().with_max_depth(99)),
            tree.render()
        );
    }

    #[test]
    fn a_depth_limit_bounds_the_quadratic_pretty_output() {
        let mut node = p().text("bottom");
        for _ in 0..50_000 {
            node = div().child(node);
        }
        let limited = node.render_with(&RenderOptions::pretty().with_max_depth(512));
        // 512 levels, an open and a close line each, at most 512 * 16 columns of indent.
        assert!(
            limited.len() < 512 * 2 * (512 * 2 + 16),
            "{} bytes",
            limited.len()
        );
    }

    use super::*;
    use crate::elements::{div, img, p};

    #[test]
    fn compact_is_the_default() {
        let options = RenderOptions::default();
        assert!(!options.pretty);
        assert_eq!(options.indent, "  ");
        assert!(!options.xhtml_self_closing);
    }

    #[test]
    fn the_indent_string_is_configurable() {
        let options = RenderOptions::pretty().with_indent("    ");
        let mut out = String::new();
        options.write_indent(&mut out, 2);
        assert_eq!(out, "        ");
    }

    /// Options are a value: changing a copy must not affect the original.
    #[test]
    fn options_have_value_semantics() {
        let base = RenderOptions::compact();
        let derived = base.clone().with_xhtml_self_closing(true);
        assert!(!base.xhtml_self_closing);
        assert!(derived.xhtml_self_closing);
    }

    #[test]
    fn pretty_indents_children_by_two_spaces() {
        let tree = div().child(p().text("Hi"));

        assert_eq!(tree.render_pretty(), "<div>\n  <p>Hi</p>\n</div>");
    }

    /// XHTML-style closing is a field on the options value, never a process-wide switch, so a
    /// render cannot leak into the next one.
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

    #[test]
    fn write_into_appends_rather_than_replacing() {
        let mut buffer = String::from("<!-- header -->");
        div()
            .text("x")
            .write_into(&mut buffer, &RenderOptions::compact(), 0);

        assert_eq!(buffer, "<!-- header --><div>x</div>");
    }

    /// The tree is `Send + Sync` and the options are a value, so this is a guard against ever
    /// reintroducing shared state.
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
