//! The [`Element`] type and its fluent builder API.
//!
//! Ports the state half of `core/HTMLTag.swift`, all of `core/CSSHelpers.swift` and all of
//! `core/AttributeHelpers.swift`. Those three files are the *complete* chainable surface
//! of Winged-Swift — there is nothing else.

use crate::core::attribute::Attribute;
use crate::core::escape::{write_escaped_attribute, write_escaped_text};
use crate::core::node::Node;
use crate::core::render::{Render, RenderOptions};

/// An HTML element: a tag name, attributes, optional text content, and children.
///
/// Every builder method takes `self` and returns `Self`, so they chain. Content and
/// attribute values are escaped as they go in, not when the element is rendered.
///
/// # Examples
/// ```
/// use molt::prelude::*;
/// let card = div()
///     .add_class("card")
///     .set_id("hero")
///     .child(h1().text("Welcome"))
///     .child(p().text("Fuel, tyres & chain."));
/// assert_eq!(
///     card.render(),
///     r#"<div class="card" id="hero"><h1>Welcome</h1><p>Fuel, tyres &amp; chain.</p></div>"#
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    tag: String,
    attributes: Vec<Attribute>,
    content: Option<String>,
    children: Vec<Node>,
    self_closing: bool,
}

impl Element {
    /// Creates an element with the given tag name.
    pub fn new(tag: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            attributes: Vec::new(),
            content: None,
            children: Vec::new(),
            self_closing: false,
        }
    }

    // MARK: - Accessors

    /// The tag name.
    #[must_use]
    pub fn tag(&self) -> &str {
        &self.tag
    }

    /// The attributes, in insertion order.
    #[must_use]
    pub fn attributes(&self) -> &[Attribute] {
        &self.attributes
    }

    /// The escaped text content, if any.
    #[must_use]
    pub fn content(&self) -> Option<&str> {
        self.content.as_deref()
    }

    /// The child nodes.
    #[must_use]
    pub fn children(&self) -> &[Node] {
        &self.children
    }

    /// Whether this element closes without an end tag, dropping content and children.
    ///
    /// Void tags do this on their own. [`Self::self_closing`] opts any other tag into the
    /// same shape.
    ///
    /// # Examples
    /// ```
    /// use molt::prelude::*;
    /// assert!(!div().is_self_closing());
    /// assert!(div().self_closing(true).is_self_closing());
    /// ```
    #[must_use]
    pub fn is_self_closing(&self) -> bool {
        self.self_closing
    }

    // MARK: - Content

    /// Sets the text content, escaping it.
    ///
    /// Replaces any content already set.
    #[must_use]
    pub fn text(mut self, content: impl AsRef<str>) -> Self {
        let raw = content.as_ref();
        let mut escaped = String::with_capacity(raw.len());
        write_escaped_text(&mut escaped, raw, false);
        self.content = Some(escaped);
        self
    }

    /// Sets the text content **without** escaping it.
    ///
    /// The documented escape hatch for markup you already trust. `<script>` and `<style>`
    /// use this by default.
    #[must_use]
    pub fn raw_text(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Appends a child node.
    #[must_use]
    pub fn child(mut self, child: impl Into<Node>) -> Self {
        self.children.push(child.into());
        self
    }

    /// Appends several child nodes.
    #[must_use]
    pub fn children_from<N: Into<Node>>(mut self, children: impl IntoIterator<Item = N>) -> Self {
        self.children.extend(children.into_iter().map(Into::into));
        self
    }

    // MARK: - Attributes

    /// Appends an attribute. Does not deduplicate.
    #[must_use]
    pub fn add_attribute(mut self, attribute: Attribute) -> Self {
        self.attributes.push(attribute);
        self
    }

    /// Appends `key="value"`, escaping the value. Does not deduplicate.
    ///
    /// Ports `setAttribute(key:value:)`.
    #[must_use]
    pub fn attr(self, key: impl Into<String>, value: impl AsRef<str>) -> Self {
        self.add_attribute(Attribute::new(key, value))
    }

    /// Appends a boolean attribute, which renders as a bare key.
    #[must_use]
    pub fn bool_attr(self, key: impl Into<String>) -> Self {
        self.add_attribute(Attribute::boolean(key))
    }

    /// Replaces `key` with a URL that is written as given, even if it is a script URL.
    ///
    /// The value is still attribute-escaped. Use this only for a URL you wrote yourself.
    ///
    /// # Examples
    /// ```
    /// use molt::prelude::*;
    /// assert_eq!(
    ///     a().attr("href", "javascript:x")
    ///         .trusted_url("href", "javascript:print()")
    ///         .render(),
    ///     "<a href=\"javascript:print()\"></a>"
    /// );
    /// ```
    #[must_use]
    pub fn trusted_url(mut self, key: impl Into<String>, value: impl AsRef<str>) -> Self {
        let attribute = Attribute::trusted_url(key, value);
        let name = attribute.key().to_string();
        self.attributes.retain(|existing| existing.key() != name);
        self.attributes.push(attribute);
        self
    }

    /// Closes this element without an end tag, and does not render its content or children.
    ///
    /// `true` emits `>` or, when [`RenderOptions::xhtml_self_closing`] is set, ` />`, even
    /// when the tag is not void. The default is `false`, so existing markup is unchanged.
    /// Attributes are still written, which is what keeps the URL policy in front of a tag
    /// that would otherwise have been emitted as raw markup.
    ///
    /// # Examples
    /// ```
    /// use molt::core::RenderOptions;
    /// use molt::prelude::*;
    /// let field = div().attr("id", "x").self_closing(true).child(span().text("no"));
    /// assert_eq!(field.render(), "<div id=\"x\">");
    /// assert_eq!(
    ///     field.render_with(&RenderOptions::compact().with_xhtml_self_closing(true)),
    ///     "<div id=\"x\" />"
    /// );
    /// ```
    #[must_use]
    pub fn self_closing(mut self, yes: bool) -> Self {
        self.self_closing = yes;
        self
    }

    /// Appends `data-{key}="{value}"`.
    #[must_use]
    pub fn data_attr(self, key: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.attr(format!("data-{}", key.as_ref()), value)
    }

    /// Appends several `data-*` attributes.
    ///
    /// Takes an ordered iterator rather than a map. Winged-Swift's `dataAttributes(_:)`
    /// takes a Swift `Dictionary`, so its rendered attribute order is nondeterministic
    /// between runs; this signature makes the output stable. See `PORTING.md`.
    #[must_use]
    pub fn data_attrs<K: AsRef<str>, V: AsRef<str>>(
        mut self,
        data: impl IntoIterator<Item = (K, V)>,
    ) -> Self {
        for (key, value) in data {
            self = self.data_attr(key, value);
        }
        self
    }

    /// Appends `aria-{key}="{value}"`.
    #[must_use]
    pub fn aria_attr(self, key: impl AsRef<str>, value: impl AsRef<str>) -> Self {
        self.attr(format!("aria-{}", key.as_ref()), value)
    }

    /// Appends several `aria-*` attributes, in the order given.
    #[must_use]
    pub fn aria_attrs<K: AsRef<str>, V: AsRef<str>>(
        mut self,
        aria: impl IntoIterator<Item = (K, V)>,
    ) -> Self {
        for (key, value) in aria {
            self = self.aria_attr(key, value);
        }
        self
    }

    // MARK: - Replacing helpers
    //
    // `set_id`, `set_style` and `set_role` remove any existing value before appending.
    // Every other helper appends without deduplication — that asymmetry is Winged-Swift's.

    /// Sets `id`, replacing any existing one.
    #[must_use]
    pub fn set_id(self, id: impl AsRef<str>) -> Self {
        self.replace_attribute("id", id.as_ref())
    }

    /// Sets `style`, replacing any existing one.
    #[must_use]
    pub fn set_style(self, style: impl AsRef<str>) -> Self {
        self.replace_attribute("style", style.as_ref())
    }

    /// Sets `role`, replacing any existing one.
    #[must_use]
    pub fn set_role(self, role: impl AsRef<str>) -> Self {
        self.replace_attribute("role", role.as_ref())
    }

    fn replace_attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.retain(|a| a.key() != key);
        self.attributes.push(Attribute::new(key, value));
        self
    }

    // MARK: - Classes

    /// Appends a class name to `class`, space-joined.
    ///
    /// Only the new value is escaped — the existing attribute is already escaped, and
    /// re-escaping it would double-encode. `Tests/WingedSwiftTests/CSSHelpersTests.swift`
    /// has a regression test for exactly that.
    #[must_use]
    pub fn add_class(mut self, class_name: impl AsRef<str>) -> Self {
        let raw = class_name.as_ref();
        if let Some(existing) = self.attributes.iter_mut().find(|a| a.key() == "class") {
            let mut merged = existing.value().to_string();
            merged.push(' ');
            write_escaped_attribute(&mut merged, raw);
            // `Attribute::raw` because `merged` is already escaped.
            *existing = Attribute::raw("class", merged);
        } else {
            self.attributes.push(Attribute::new("class", raw));
        }
        self
    }

    /// Appends several class names.
    #[must_use]
    pub fn add_classes<S: AsRef<str>>(mut self, class_names: impl IntoIterator<Item = S>) -> Self {
        for name in class_names {
            self = self.add_class(name);
        }
        self
    }
}

impl Render for Element {
    fn write_into(&self, out: &mut String, options: &RenderOptions, depth: usize) {
        crate::core::node::write_element_tree(self, out, options, depth);
    }
}

/// Tears the subtree down iteratively.
///
/// The derived drop glue recurses once per nesting level, so a tree deep enough to need
/// the iterative writer would abort while being *freed* instead — after rendering fine.
/// Draining into an explicit worklist keeps teardown flat.
///
/// Each node has its children moved out before it goes out of scope, so the `Drop` that
/// runs for it finds nothing left to recurse into.
impl Drop for Element {
    fn drop(&mut self) {
        let mut pending = core::mem::take(&mut self.children);
        while let Some(node) = pending.pop() {
            match node {
                Node::Element(mut element) => pending.append(&mut element.children),
                Node::Fragment(mut children) => pending.append(&mut children),
                Node::Text(_) | Node::Raw(_) | Node::Comment(_) => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::RenderOptions;
    use crate::elements::{
        a, body, button, div, footer, head, header, html_tag, img, input_named, li, main_tag, meta,
        nav, ol, p, script, span, stylesheet, table, td, th, title, tr, ul,
    };

    /// Ports `ThreeZeroTests.trustedURLsRenderAsGiven` on the element builder.
    #[test]
    fn trusted_url_replaces_a_blocked_value() {
        assert_eq!(
            a().attr("href", "javascript:x")
                .trusted_url("href", "javascript:print()")
                .render(),
            r#"<a href="javascript:print()"></a>"#
        );
    }

    #[test]
    fn a_non_void_tag_marked_self_closing_drops_its_children() {
        let tag = div()
            .attr("id", "x")
            .self_closing(true)
            .child(span().text("no"));
        assert_eq!(tag.render(), r#"<div id="x">"#);
        assert_eq!(
            tag.render_with(&RenderOptions::compact().with_xhtml_self_closing(true)),
            r#"<div id="x" />"#
        );
        assert_eq!(tag.render_pretty(), r#"<div id="x">"#);
        assert!(!div().is_self_closing());
    }

    /// Ports `CSSHelpersTests.testAddMultipleClasses`.
    #[test]
    fn add_class_appends_to_one_attribute() {
        assert_eq!(
            div().add_class("a").add_class("b").render(),
            r#"<div class="a b"></div>"#
        );
    }

    /// Ports `CSSHelpersTests.testAddClassDoesNotDoubleEscapeExistingValues`.
    #[test]
    fn chaining_add_class_does_not_double_escape() {
        let rendered = div().add_class("a&b").add_class("c").render();
        assert_eq!(rendered, r#"<div class="a&amp;b c"></div>"#);
        assert!(!rendered.contains("&amp;amp;"));
    }

    /// Ports `CSSHelpersTests.testAddClassEscapesQuotesInsteadOfBreakingOutOfTheAttribute`.
    #[test]
    fn a_quote_in_a_class_name_cannot_break_out() {
        let rendered = div().add_class(r#"a" onload="alert(1)"#).render();
        assert!(!rendered.contains("onload=\""));
        assert!(rendered.contains("&quot;"));
    }

    /// Ports `CSSHelpersTests.testAddClassesArray`.
    #[test]
    fn add_classes_appends_all_of_them_in_order() {
        assert_eq!(
            div().add_classes(["card", "p-4", "shadow"]).render(),
            r#"<div class="card p-4 shadow"></div>"#
        );
    }

    /// Ports `CSSHelpersTests.testSetIdReplacesExisting`.
    #[test]
    fn set_id_replaces_rather_than_appending() {
        let rendered = div().set_id("first").set_id("second").render();
        assert_eq!(rendered, r#"<div id="second"></div>"#);
    }

    /// Ports `CSSHelpersTests.testSetStyle` and `AttributeHelpersTests.testSetRole`.
    #[test]
    fn set_style_and_set_role_also_replace() {
        let rendered = div()
            .set_style("color:red")
            .set_style("color:blue")
            .set_role("main")
            .render();
        assert_eq!(rendered, r#"<div style="color:blue" role="main"></div>"#);
    }

    /// Ports `AttributeHelpersTests.testDataAttribute` and
    /// `AttributeHelpersTests.testAriaAttribute`.
    #[test]
    fn data_and_aria_attributes_get_their_prefixes() {
        let rendered = span()
            .data_attr("id", "7")
            .aria_attr("label", "Close")
            .render();
        assert_eq!(rendered, r#"<span data-id="7" aria-label="Close"></span>"#);
    }

    /// The improvement over Swift: ordered input means stable output.
    #[test]
    fn bulk_attribute_order_is_stable() {
        let build = || {
            div()
                .data_attrs([("a", "1"), ("b", "2"), ("c", "3")])
                .render()
        };
        let expected = r#"<div data-a="1" data-b="2" data-c="3"></div>"#;
        for _ in 0..16 {
            assert_eq!(build(), expected);
        }
    }

    /// No Swift counterpart: `HTMLTag` has no deduplication either, but nothing in its
    /// suite pins it. Kept as a Rust-side guarantee.
    #[test]
    fn plain_attributes_are_appended_without_deduplication() {
        assert_eq!(
            div().attr("data-x", "1").attr("data-x", "2").render(),
            r#"<div data-x="1" data-x="2"></div>"#
        );
    }

    /// Ports `HTMLEscapeTests.testHTMLTagEscapesContentByDefault` and
    /// `HTMLEscapeTests.testHTMLTagCanDisableEscape`. Swift turns escaping off with an
    /// `escapeContent:`
    /// flag on the initialiser; Rust has a separate method instead, so the escape hatch is
    /// greppable rather than hidden behind a default argument.
    #[test]
    fn text_is_escaped_and_raw_text_is_not() {
        assert_eq!(p().text("<b>").render(), "<p>&lt;b&gt;</p>");
        assert_eq!(p().raw_text("<b>").render(), "<p><b></p>");
    }

    #[test]
    fn boolean_attributes_render_bare() {
        assert_eq!(
            button().bool_attr("disabled").render(),
            "<button disabled></button>"
        );
    }

    #[test]
    fn children_from_appends_a_sequence() {
        let list = div().children_from([p().text("a"), p().text("b")]);
        assert_eq!(list.render(), "<div><p>a</p><p>b</p></div>");
    }

    // Ports `HTMLTagTests`. Swift builds these through a result builder — `html { ... }` —
    // which wraps everything in `<html>`; the Rust equivalent is an explicit `html_tag()`.

    /// Ports `HTMLTagTests.testHTMLTagCreation`.
    #[test]
    fn a_tag_renders_its_attributes_then_its_content() {
        let tag = Element::new("p")
            .attr("class", "text")
            .text("Hello, World!");

        assert_eq!(tag.render(), r#"<p class="text">Hello, World!</p>"#);
    }

    /// Ports `HTMLTagTests.testHTMLTagWithMultipleAttributes`.
    ///
    /// Swift's `Img(src:alt:attributes:)` emits the extra attributes *before* `src` and
    /// `alt`. The Rust `image(src, alt)` constructor puts them first instead, so matching
    /// this byte for byte means going through the generic builder.
    #[test]
    fn extra_attributes_can_precede_the_typed_ones() {
        let tag = img()
            .attr("width", "100")
            .attr("height", "100")
            .attr("src", "image.png")
            .attr("alt", "An image");

        assert_eq!(
            tag.render(),
            r#"<img width="100" height="100" src="image.png" alt="An image">"#
        );
    }

    /// Ports `HTMLTagTests.testHTMLBuilder`.
    #[test]
    fn nested_children_render_in_order() {
        let document = html_tag().child(
            div()
                .child(p().text("This is a paragraph."))
                .child(img().attr("src", "image.png").attr("alt", "An image")),
        );

        assert_eq!(
            document.render(),
            concat!(
                "<html><div><p>This is a paragraph.</p>",
                r#"<img src="image.png" alt="An image"></div></html>"#,
            )
        );
    }

    /// Ports `HTMLTagTests.testHTMLBuilderWithAttributes`.
    #[test]
    fn a_container_renders_its_class_before_its_children() {
        let document = html_tag().child(
            div()
                .add_class("main-body")
                .child(p().text("Title"))
                .child(p().text("This is a paragraph.")),
        );

        assert_eq!(
            document.render(),
            r#"<html><div class="main-body"><p>Title</p><p>This is a paragraph.</p></div></html>"#
        );
    }

    /// Ports `HTMLTagTests.testHTMLTable`.
    #[test]
    fn a_table_renders_its_rows_and_cells() {
        let document = html_tag().child(
            table()
                .add_class("table")
                .child(
                    tr().child(th().text("Header 1"))
                        .child(th().text("Header 2")),
                )
                .child(
                    tr().child(td().text("Row 1, Cell 1"))
                        .child(td().text("Row 1, Cell 2")),
                )
                .child(
                    tr().child(td().text("Row 2, Cell 1"))
                        .child(td().text("Row 2, Cell 2")),
                ),
        );

        assert_eq!(
            document.render(),
            concat!(
                r#"<html><table class="table"><tr><th>Header 1</th><th>Header 2</th></tr>"#,
                "<tr><td>Row 1, Cell 1</td><td>Row 1, Cell 2</td></tr>",
                "<tr><td>Row 2, Cell 1</td><td>Row 2, Cell 2</td></tr></table></html>",
            )
        );
    }

    /// Ports `HTMLTagTests.testHTMLList`.
    #[test]
    fn an_unordered_list_renders_its_items() {
        let document = html_tag().child(
            ul().add_class("unordered-list")
                .child(li().text("Item 1"))
                .child(li().text("Item 2"))
                .child(li().text("Item 3")),
        );

        assert_eq!(
            document.render(),
            concat!(
                r#"<html><ul class="unordered-list">"#,
                "<li>Item 1</li><li>Item 2</li><li>Item 3</li></ul></html>",
            )
        );
    }

    /// Ports `HTMLTagTests.testHTMLOrderedList`.
    #[test]
    fn an_ordered_list_renders_its_items() {
        let document = html_tag().child(
            ol().add_class("ordered-list")
                .child(li().text("First"))
                .child(li().text("Second"))
                .child(li().text("Third")),
        );

        assert_eq!(
            document.render(),
            concat!(
                r#"<html><ol class="ordered-list">"#,
                "<li>First</li><li>Second</li><li>Third</li></ol></html>",
            )
        );
    }

    /// Ports `HTMLTagTests.testHTMLDescriptionList`.
    ///
    /// Swift reaches for the untyped `HTMLTag("dt", content:)` here; `Element::new` is the
    /// same escape hatch in Rust, and the point of the test is that it renders like any
    /// generated constructor.
    #[test]
    fn a_description_list_renders_terms_and_descriptions() {
        let document = html_tag().child(
            Element::new("dl")
                .add_class("description-list")
                .child(Element::new("dt").text("Term 1"))
                .child(Element::new("dd").text("Description 1"))
                .child(Element::new("dt").text("Term 2"))
                .child(Element::new("dd").text("Description 2")),
        );

        assert_eq!(
            document.render(),
            concat!(
                r#"<html><dl class="description-list">"#,
                "<dt>Term 1</dt><dd>Description 1</dd>",
                "<dt>Term 2</dt><dd>Description 2</dd></dl></html>",
            )
        );
    }

    /// Ports `HTMLTagTests.testHTMLStructuralTags`.
    #[test]
    fn the_structural_tags_nest_into_a_page() {
        let document = html_tag()
            .child(
                head()
                    .child(
                        meta()
                            .attr("name", "description")
                            .attr("content", "A description of the page"),
                    )
                    .child(stylesheet("styles.css")),
            )
            .child(
                body()
                    .child(
                        header().child(
                            nav()
                                .child(a().attr("href", "#home").text("Home"))
                                .child(a().attr("href", "#about").text("About"))
                                .child(a().attr("href", "#contact").text("Contact")),
                        ),
                    )
                    .child(main_tag().child(p().text("Welcome to our website!")))
                    .child(footer().child(p().text("\u{a9} 2024 Company, Inc."))),
            );

        assert_eq!(
            document.render(),
            concat!(
                r#"<html><head><meta name="description" content="A description of the page">"#,
                r#"<link href="styles.css" rel="stylesheet"></head><body><header><nav>"#,
                r##"<a href="#home">Home</a><a href="#about">About</a>"##,
                r##"<a href="#contact">Contact</a></nav></header>"##,
                "<main><p>Welcome to our website!</p></main>",
                "<footer><p>\u{a9} 2024 Company, Inc.</p></footer></body></html>",
            )
        );
    }

    /// Ports `HTMLTagTests.testHTMLScript`.
    ///
    /// `raw_text`, not `text`: escaping the body would turn `'Hello World'` into
    /// `&#39;Hello World&#39;` and the browser would run that literally. Swift's `Script`
    /// has the same carve-out built into the type.
    #[test]
    fn a_script_body_is_not_escaped() {
        let document = html_tag().child(
            script()
                .attr("type", "text/javascript")
                .raw_text("alert('Hello World');"),
        );

        assert_eq!(
            document.render(),
            r#"<html><script type="text/javascript">alert('Hello World');</script></html>"#
        );
    }

    /// Ports `HTMLTagTests.testMetaWithName`.
    #[test]
    fn meta_renders_both_the_named_and_the_charset_form() {
        let document = html_tag()
            .child(
                meta()
                    .attr("name", "description")
                    .attr("content", "A description of the page"),
            )
            .child(meta().attr("charset", "utf-8"));

        assert_eq!(
            document.render(),
            concat!(
                r#"<html><meta name="description" content="A description of the page">"#,
                r#"<meta charset="utf-8"></html>"#,
            )
        );
    }

    /// Ports `HTMLTagTests.testHTMLTitle`.
    #[test]
    fn a_title_renders_its_text() {
        let document = html_tag().child(title().text("Title my site"));

        assert_eq!(
            document.render(),
            "<html><title>Title my site</title></html>"
        );
    }

    /// Ports `HTMLTagTests.testHTMLSpan`.
    #[test]
    fn a_span_renders_its_attributes_and_content() {
        let tag = span().attr("class", "text").text("Hello, World!");

        assert_eq!(tag.render(), r#"<span class="text">Hello, World!</span>"#);
    }

    /// Ports `HTMLTagTests.testHTMLButton`.
    ///
    /// Swift's `Button` defaults to `type="button"` and appends it after the caller's
    /// attributes. `button_typed` in Rust puts the type first, so the order here comes from
    /// the generic builder.
    #[test]
    fn a_button_carries_its_type_after_its_class() {
        let document = html_tag().child(button().add_class("button-class").attr("type", "button"));

        assert_eq!(
            document.render(),
            r#"<html><button class="button-class" type="button"></button></html>"#
        );
    }

    /// Ports `HTMLTagTests.testHTMLButtonChildren`.
    #[test]
    fn a_button_renders_its_children() {
        let document = html_tag().child(
            button()
                .add_class("button-class")
                .attr("type", "button")
                .child(span().add_class("icon-bar")),
        );

        assert_eq!(
            document.render(),
            concat!(
                r#"<html><button class="button-class" type="button">"#,
                r#"<span class="icon-bar"></span></button></html>"#,
            )
        );
    }

    // Ports the rest of `CSSHelpersTests` and `AttributeHelpersTests`.

    /// Ports `CSSHelpersTests.testChainingPreservesTheConcreteType`.
    ///
    /// Swift needs the test because its helpers are declared on a protocol and could erase
    /// the tag type. Rust's take `self` and return `Self`, so the type survives by
    /// construction — what is worth pinning is the attribute order the chain produces.
    #[test]
    fn chaining_helpers_keeps_the_element_usable() {
        let card: Element = div().add_class("card").set_id("hero").set_role("region");

        assert_eq!(
            card.render(),
            r#"<div class="card" id="hero" role="region"></div>"#
        );
    }

    /// Ports `CSSHelpersTests.testSetStyleEscapesQuotes`.
    #[test]
    fn set_style_escapes_quotes() {
        let rendered = div()
            .set_style(r#"font-family: "Inter", sans-serif"#)
            .render();

        assert_eq!(
            rendered,
            r#"<div style="font-family: &quot;Inter&quot;, sans-serif"></div>"#
        );
    }

    /// Ports `CSSHelpersTests.testAddSingleClass`.
    #[test]
    fn a_single_class_renders_on_its_own() {
        assert_eq!(
            div().add_class("container").render(),
            r#"<div class="container"></div>"#
        );
    }

    /// Ports `CSSHelpersTests.testSetId`.
    #[test]
    fn set_id_renders_an_id_attribute() {
        assert_eq!(
            div().set_id("main-content").render(),
            r#"<div id="main-content"></div>"#
        );
    }

    /// Ports `CSSHelpersTests.testChainedHelpers`.
    #[test]
    fn the_helpers_chain_in_the_order_they_are_called() {
        let rendered = div()
            .set_id("content")
            .add_class("container")
            .add_class("active")
            .set_style("padding: 20px;")
            .render();

        assert_eq!(
            rendered,
            r#"<div id="content" class="container active" style="padding: 20px;"></div>"#
        );
    }

    /// Ports `AttributeHelpersTests.testMultipleDataAttributes`.
    ///
    /// Swift passes a `Dictionary`, whose order is unspecified, so its test can only check
    /// that both survive. `data_attrs` takes an ordered iterator precisely so the output is
    /// deterministic, which is what this pins instead.
    #[test]
    fn several_data_attributes_keep_the_order_they_are_given() {
        let rendered = div()
            .data_attrs([("id", "123"), ("type", "product")])
            .render();

        assert_eq!(rendered, r#"<div data-id="123" data-type="product"></div>"#);
    }

    /// Ports `AttributeHelpersTests.testMultipleAriaAttributes`.
    #[test]
    fn several_aria_attributes_keep_the_order_they_are_given() {
        let rendered = nav()
            .aria_attrs([("label", "Main navigation"), ("expanded", "true")])
            .render();

        assert_eq!(
            rendered,
            r#"<nav aria-label="Main navigation" aria-expanded="true"></nav>"#
        );
    }

    /// Ports `AttributeHelpersTests.testSetAttribute`.
    ///
    /// Swift's `setAttribute` appends like every other helper — the name says *set* but it
    /// does not replace. `attr` is the same operation under an honest name.
    #[test]
    fn attr_appends_arbitrary_attributes() {
        let rendered = input_named("text", "email")
            .attr("placeholder", "Enter email")
            .attr("required", "true")
            .render();

        assert_eq!(
            rendered,
            r#"<input type="text" name="email" placeholder="Enter email" required="true">"#
        );
    }
}
