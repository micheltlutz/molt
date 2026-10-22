//! The [`Element`] type and its fluent builder API.
//!
//! The chainable surface is this file and nothing else: the attribute, class, style, role,
//! `data-*` and `aria-*` helpers, text and children.

use std::borrow::Cow;

use crate::core::attribute::Attribute;
use crate::core::escape::write_escaped_text;
use crate::core::node::{Node, Source};
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
pub struct Element {
    // `Cow` for the same reason as `Attribute::key`: the ~90 built-in constructors hand over
    // a `'static` literal, so `div()` allocates no tag name.
    tag: Cow<'static, str>,
    attributes: Vec<Attribute>,
    content: Option<String>,
    children: Vec<Node>,
    self_closing: bool,
}

/// How many levels of nesting `{:?}` writes before it writes `...` instead.
///
/// `Debug` is the one traversal that is not an explicit stack, because it has to print in
/// the shape `Formatter` expects, and `debug_struct` nests. The depth limit is what makes that
/// safe: the recursion is at most this deep, whatever the tree is, so a 100,000-level tree
/// costs the same stack as a 128-level one. It also keeps `assert_eq!` useful, since both
/// trees are formatted when it fails and the output of a tree that deep would be unreadable
/// and, pretty-printed, quadratic.
const DEBUG_MAX_DEPTH: usize = 128;

/// An [`Element`] being formatted, `depth` levels into the tree that `{:?}` started at.
pub(crate) struct DebugElement<'a>(pub(crate) &'a Element, pub(crate) usize);

impl std::fmt::Debug for DebugElement<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(element, depth) = *self;
        f.debug_struct("Element")
            .field("tag", &element.tag)
            .field("attributes", &element.attributes)
            .field("content", &element.content)
            .field("children", &DebugChildren(&element.children, depth + 1))
            .field("self_closing", &element.self_closing)
            .finish()
    }
}

/// A list of nodes `depth` levels in. Every path from a node to its descendants goes through
/// here, so this is the one place the limit is checked.
pub(crate) struct DebugChildren<'a>(pub(crate) &'a [Node], pub(crate) usize);

impl std::fmt::Debug for DebugChildren<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(children, depth) = *self;
        if depth > DEBUG_MAX_DEPTH {
            return f.write_str("...");
        }
        f.debug_list()
            .entries(children.iter().map(|child| DebugNode(child, depth)))
            .finish()
    }
}

/// A [`Node`] `depth` levels in.
pub(crate) struct DebugNode<'a>(pub(crate) &'a Node, pub(crate) usize);

impl std::fmt::Debug for DebugNode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self(node, depth) = *self;
        match node {
            Node::Element(element) => f
                .debug_tuple("Element")
                .field(&DebugElement(element, depth))
                .finish(),
            Node::Text(text) => f.debug_tuple("Text").field(text).finish(),
            Node::Raw(text) => f.debug_tuple("Raw").field(text).finish(),
            Node::Comment(text) => f.debug_tuple("Comment").field(text).finish(),
            Node::Fragment(children) => f
                .debug_tuple("Fragment")
                .field(&DebugChildren(children, depth + 1))
                .finish(),
        }
    }
}

/// Writes the same text the derive did for a tree up to 128 levels deep, with
/// `{:#?}` for the multi-line form. Below that depth the children are written as `...`, so
/// formatting a tree of any depth (a failing `assert_eq!` on one, say) cannot overflow the
/// stack. The text is for people, not a format to parse, and may change.
///
/// # Examples
/// ```
/// use molt::prelude::*;
///
/// let mut tree = p().text("bottom");
/// for _ in 0..100_000 {
///     tree = div().child(tree);
/// }
/// let shown = format!("{tree:?}");
/// assert!(shown.starts_with(r#"Element { tag: "div""#));
/// assert!(shown.contains("children: ..."));
/// ```
impl std::fmt::Debug for Element {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        DebugElement(self, 0).fmt(f)
    }
}

impl Clone for Element {
    fn clone(&self) -> Self {
        match clone_tree(Source::Element(self)) {
            Node::Element(element) => element,
            _ => unreachable!("cloning an element yields an element"),
        }
    }
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        trees_equal(Source::Element(self), Source::Element(other))
    }
}

impl Eq for Element {}

impl std::fmt::Display for Element {
    /// The compact render, or the pretty one with the alternate flag: `{el}` / `{el:#}`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&if f.alternate() {
            self.render_pretty()
        } else {
            self.render()
        })
    }
}

impl std::hash::Hash for Element {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        hash_tree(Source::Element(self), state);
    }
}

/// Appends nodes as children, so `element.extend(items)` works beside `children_from`.
impl<N: Into<Node>> Extend<N> for Element {
    fn extend<I: IntoIterator<Item = N>>(&mut self, iter: I) {
        self.children.extend(iter.into_iter().map(Into::into));
    }
}

/// The elements below some children, in document order, found with an explicit stack. A
/// fragment is transparent: its children count as if they stood where it does.
pub(crate) struct Descendants<'a> {
    pending: Vec<&'a Node>,
}

impl<'a> Descendants<'a> {
    pub(crate) fn new(children: &'a [Node]) -> Self {
        Self {
            pending: children.iter().rev().collect(),
        }
    }
}

impl<'a> Iterator for Descendants<'a> {
    type Item = &'a Element;

    fn next(&mut self) -> Option<&'a Element> {
        while let Some(node) = self.pending.pop() {
            match node {
                Node::Element(element) => {
                    self.pending.extend(element.children.iter().rev());
                    return Some(element);
                }
                Node::Fragment(children) => self.pending.extend(children.iter().rev()),
                _ => {}
            }
        }
        None
    }
}

/// Feeds a tree to a hasher with an explicit stack, matching [`trees_equal`]: trees that
/// compare equal hash equal, and a deep tree cannot overflow the stack on the way.
pub(crate) fn hash_tree<H: std::hash::Hasher>(root: Source<'_>, state: &mut H) {
    use std::hash::Hash;

    let mut pending = vec![root];
    while let Some(source) = pending.pop() {
        match source {
            Source::Element(element) | Source::Node(Node::Element(element)) => {
                0_u8.hash(state);
                element.tag.hash(state);
                element.attributes.hash(state);
                element.content.hash(state);
                element.self_closing.hash(state);
                element.children.len().hash(state);
                pending.extend(element.children.iter().rev().map(Source::Node));
            }
            Source::Node(Node::Fragment(children)) => {
                1_u8.hash(state);
                children.len().hash(state);
                pending.extend(children.iter().rev().map(Source::Node));
            }
            Source::Node(Node::Text(text)) => {
                2_u8.hash(state);
                text.hash(state);
            }
            Source::Node(Node::Raw(text)) => {
                3_u8.hash(state);
                text.hash(state);
            }
            Source::Node(Node::Comment(text)) => {
                4_u8.hash(state);
                text.hash(state);
            }
        }
    }
}

/// A copy of `element` with everything but its children.
fn shell(element: &Element) -> Element {
    Element {
        tag: element.tag.clone(),
        attributes: element.attributes.clone(),
        content: element.content.clone(),
        children: Vec::with_capacity(element.children.len()),
        self_closing: element.self_closing,
    }
}

/// An element or fragment whose children are being copied one at a time.
enum CloneFrame<'a> {
    Element {
        source: &'a [Node],
        built: Element,
    },
    Fragment {
        source: &'a [Node],
        built: Vec<Node>,
    },
}

/// Deep-copies a tree with an explicit stack, in the same shape as the writer.
pub(crate) fn clone_tree(root: Source<'_>) -> Node {
    let mut stack: Vec<CloneFrame<'_>> = Vec::new();
    let mut descend = Some(root);

    loop {
        // A finished subtree, once there is one, goes into the frame that was waiting for it.
        let mut finished = None;
        if let Some(source) = descend.take() {
            match source {
                Source::Element(element) | Source::Node(Node::Element(element)) => {
                    stack.push(CloneFrame::Element {
                        source: &element.children,
                        built: shell(element),
                    });
                }
                Source::Node(Node::Fragment(children)) => stack.push(CloneFrame::Fragment {
                    source: children,
                    built: Vec::with_capacity(children.len()),
                }),
                Source::Node(Node::Text(text)) => finished = Some(Node::Text(text.clone())),
                Source::Node(Node::Raw(text)) => finished = Some(Node::Raw(text.clone())),
                Source::Node(Node::Comment(text)) => finished = Some(Node::Comment(text.clone())),
            }
        }

        if finished.is_none() {
            // Take the next child of the innermost open frame, or close the frame.
            let Some(frame) = stack.last_mut() else {
                // Only reachable if `root` was a leaf that `finished` already carried out.
                return Node::Fragment(Vec::new());
            };
            let (source, built_len) = match frame {
                CloneFrame::Element { source, built } => (*source, built.children.len()),
                CloneFrame::Fragment { source, built } => (*source, built.len()),
            };
            if let Some(next) = source.get(built_len) {
                descend = Some(Source::Node(next));
                continue;
            }
            finished = stack.pop().map(|frame| match frame {
                CloneFrame::Element { built, .. } => Node::Element(built),
                CloneFrame::Fragment { built, .. } => Node::Fragment(built),
            });
        }

        let Some(done) = finished else { continue };
        match stack.last_mut() {
            None => return done,
            Some(CloneFrame::Element { built, .. }) => built.children.push(done),
            Some(CloneFrame::Fragment { built, .. }) => built.push(done),
        }
    }
}

/// Structural equality with an explicit stack of pairs still to compare.
pub(crate) fn trees_equal(left: Source<'_>, right: Source<'_>) -> bool {
    let mut pending = vec![(left, right)];
    while let Some((left, right)) = pending.pop() {
        match (left, right) {
            (
                Source::Element(a) | Source::Node(Node::Element(a)),
                Source::Element(b) | Source::Node(Node::Element(b)),
            ) => {
                if a.tag != b.tag
                    || a.attributes != b.attributes
                    || a.content != b.content
                    || a.self_closing != b.self_closing
                    || a.children.len() != b.children.len()
                {
                    return false;
                }
                pending.extend(
                    a.children
                        .iter()
                        .zip(&b.children)
                        .map(|(a, b)| (Source::Node(a), Source::Node(b))),
                );
            }
            (Source::Node(Node::Fragment(a)), Source::Node(Node::Fragment(b))) => {
                if a.len() != b.len() {
                    return false;
                }
                pending.extend(
                    a.iter()
                        .zip(b)
                        .map(|(a, b)| (Source::Node(a), Source::Node(b))),
                );
            }
            (Source::Node(Node::Text(a)), Source::Node(Node::Text(b)))
            | (Source::Node(Node::Raw(a)), Source::Node(Node::Raw(b)))
            | (Source::Node(Node::Comment(a)), Source::Node(Node::Comment(b))) => {
                if a != b {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

impl Element {
    /// Creates an element with the given tag name.
    pub fn new(tag: impl Into<String>) -> Self {
        Self::with_tag(Cow::Owned(tag.into()))
    }

    /// Creates an element whose tag is a `'static` string, without allocating it.
    ///
    /// What the built-in constructors (`div()`, `p()`, …) use. Reach for it when you define
    /// your own element helpers with a literal tag, such as a custom element.
    ///
    /// # Examples
    /// ```
    /// use molt::prelude::*;
    /// assert_eq!(Element::from_static("my-card").text("hi").render(), "<my-card>hi</my-card>");
    /// ```
    #[must_use]
    pub fn from_static(tag: &'static str) -> Self {
        Self::with_tag(Cow::Borrowed(tag))
    }

    fn with_tag(tag: Cow<'static, str>) -> Self {
        Self {
            tag,
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

    // MARK: - Query

    /// The first element below this one, in document order, with this tag.
    ///
    /// For asserting on a tree in a test without searching its rendered string. The search
    /// uses an explicit stack, so a deep tree cannot overflow it, and it borrows: nothing is
    /// cloned. This element itself is not a match, only what is inside it, and `tag` is
    /// compared as written (`"div"`, not `"DIV"`).
    ///
    /// # Examples
    /// ```
    /// use molt::prelude::*;
    ///
    /// let page = div()
    ///     .child(nav().child(a().attr("href", "/docs").text("Docs")))
    ///     .child(a().attr("href", "/blog").text("Blog"));
    ///
    /// let first = page.find("a").expect("a link");
    /// assert_eq!(first.attribute("href"), Some("/docs"));
    /// assert_eq!(first.render(), r#"<a href="/docs">Docs</a>"#);
    /// assert!(page.find("table").is_none());
    /// ```
    #[must_use]
    pub fn find(&self, tag: &str) -> Option<&Element> {
        Descendants::new(&self.children).find(|element| element.tag() == tag)
    }

    /// Every element below this one with this tag, in document order. See [`Self::find`].
    ///
    /// # Examples
    /// ```
    /// use molt::prelude::*;
    ///
    /// let list = ul().children_from(["a", "b", "c"].map(|x| li().text(x)));
    /// assert_eq!(list.find_all("li").count(), 3);
    /// let last = list.find_all("li").last().expect("three items");
    /// assert_eq!(last.render(), "<li>c</li>");
    /// ```
    pub fn find_all<'a>(&'a self, tag: &'a str) -> impl Iterator<Item = &'a Element> + 'a {
        Descendants::new(&self.children).filter(move |element| element.tag() == tag)
    }

    /// The value of the first attribute with this name, **as it is written** into the markup.
    ///
    /// The value was escaped when it was set, so `attr("title", "a & b")` reads back as
    /// `a &amp; b`. A boolean attribute reads as `""`; use [`Self::attributes`] and
    /// [`Attribute::is_boolean`] to tell it from an empty value. The name is compared as
    /// written. The URL policy is applied at render time, so a URL that will be written as
    /// `#` still reads as it was set.
    ///
    /// # Examples
    /// ```
    /// use molt::prelude::*;
    ///
    /// let link = a().attr("href", "/docs?x=1&y=2").bool_attr("download");
    /// assert_eq!(link.attribute("href"), Some("/docs?x=1&amp;y=2"));
    /// assert_eq!(link.attribute("download"), Some(""));
    /// assert_eq!(link.attribute("target"), None);
    /// ```
    #[must_use]
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|attribute| attribute.key() == name)
            .map(Attribute::value)
    }

    // MARK: - Content

    /// Appends text, escaping it, at the position of the call.
    ///
    /// Text added before any child lands in the element's **content**, which is the cheap
    /// shape for a leaf (`p().text("x")` allocates no child list). Once the element has a
    /// child, text becomes a text child instead, so the order of the calls is the order in the
    /// markup: `p().child(strong()).text("after")` renders the text after the `<strong>`.
    /// Calling it twice appends twice; it never replaces.
    ///
    /// # Examples
    /// ```
    /// use molt::prelude::*;
    /// let line = p()
    ///     .text("Owner: ")
    ///     .child(strong().text("Ada"))
    ///     .text(" (admin)");
    /// assert_eq!(line.render(), "<p>Owner: <strong>Ada</strong> (admin)</p>");
    /// ```
    #[must_use]
    pub fn text(mut self, content: impl AsRef<str>) -> Self {
        let raw = content.as_ref();
        if self.children.is_empty() {
            let slot = self
                .content
                .get_or_insert_with(|| String::with_capacity(raw.len()));
            write_escaped_text(slot, raw, false);
        } else {
            self.children.push(Node::text(raw));
        }
        self
    }

    /// Appends text **without** escaping it, at the position of the call.
    ///
    /// The documented escape hatch for markup you already trust. `<script>` and `<style>`
    /// use this by default. Position follows [`Self::text`]: content before the first child,
    /// a raw child after it.
    ///
    /// **Do not put untrusted data in a `<script>` or `<style>` through this.** The element
    /// ends at the first `</script` (or `</style`) wherever it appears, including inside a
    /// JSON string. For JSON data use [`seo::json_ld`](crate::seo::json_ld) or
    /// [`seo::script_json`](crate::seo::script_json), which escape it; there is no safe way to
    /// embed untrusted text in script code.
    ///
    /// # Examples
    /// ```
    /// use molt::prelude::*;
    /// // The string is trusted markup written right here.
    /// assert_eq!(p().raw_text("a &amp; b").render(), "<p>a &amp; b</p>");
    /// ```
    #[must_use]
    pub fn raw_text(mut self, content: impl Into<String>) -> Self {
        let content = content.into();
        if !self.children.is_empty() {
            self.children.push(Node::raw(content));
        } else if let Some(slot) = &mut self.content {
            slot.push_str(&content);
        } else {
            self.content = Some(content);
        }
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

    // MARK: - Content-Security-Policy

    /// The CSP hash of this `<script>` or `<style>`: `sha256-` and the base64 SHA-256 of what
    /// the browser reads between the tags. `None` for any other tag, and for a tag that is
    /// self-closing.
    ///
    /// A page with `script-src 'self'` and no `'unsafe-inline'` still runs an inline script
    /// whose hash is listed in the policy, and the same goes for `style-src`. Put the result
    /// in single quotes there: `script-src 'self' 'sha256-…'`. Without the quotes it is also
    /// the value of an `integrity` attribute.
    ///
    /// The hash covers the content **as it is written into the page**: text set with
    /// [`Self::text`] is hashed after escaping, and [`Self::raw_text`] as it is. The child
    /// nodes are hashed as the compact render writes them, so for an element with children the
    /// pretty render, which indents them, does not match. The usual element, a single text,
    /// renders the same in both. Change one byte and the hash changes, which is the point.
    ///
    /// # Examples
    /// ```
    /// use molt::prelude::*;
    ///
    /// // The script is code written right here, so `raw_text` is correct.
    /// let theme = script().raw_text("document.documentElement.dataset.theme='dark';");
    /// let hash = theme.csp_hash().expect("a script has a hash");
    /// assert_eq!(hash, "sha256-IL7rwv2VOjU6vInkRwJTvFERJLVY80azmQFKaT/moJg=");
    ///
    /// let policy = format!("script-src 'self' '{hash}'");
    /// assert!(policy.starts_with("script-src 'self' 'sha256-"));
    /// assert_eq!(div().csp_hash(), None);
    /// ```
    #[must_use]
    pub fn csp_hash(&self) -> Option<String> {
        if !matches!(self.tag(), "script" | "style") || self.self_closing {
            return None;
        }
        let mut body = String::new();
        if let Some(content) = &self.content {
            body.push_str(content);
        }
        let options = RenderOptions::compact();
        for child in &self.children {
            child.write_into(&mut body, &options, 0);
        }
        let digest = crate::core::digest::sha256(body.as_bytes());
        Some(format!("sha256-{}", crate::core::digest::base64(&digest)))
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
        // Compare against the borrowed key: copying it into a `String` just to call `retain`
        // was an allocation per call.
        self.attributes
            .retain(|existing| existing.key() != attribute.key());
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
    /// Takes an ordered iterator rather than a map: a hash map would render its
    /// attributes in a different order from one run to the next, and this makes the output
    /// stable.
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
    // Every other helper appends without deduplication — that asymmetry is deliberate.

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

    fn replace_attribute(mut self, key: &'static str, value: &str) -> Self {
        self.attributes.retain(|a| a.key() != key);
        self.attributes.push(Attribute::new_static(key, value));
        self
    }

    // MARK: - Classes

    /// Appends a class name to `class`, space-joined.
    ///
    /// Only the new value is escaped — the existing attribute is already escaped, and
    /// re-escaping it would double-encode, and a test pins exactly that.
    #[must_use]
    pub fn add_class(mut self, class_name: impl AsRef<str>) -> Self {
        let raw = class_name.as_ref();
        if let Some(existing) = self.attributes.iter_mut().find(|a| a.key() == "class") {
            existing.push_escaped_word(raw);
        } else {
            self.attributes.push(Attribute::new_static("class", raw));
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

    /// The hand-written `Clone`/`PartialEq` must agree with what the derives did.
    #[test]
    fn a_clone_equals_its_source_and_is_independent() {
        let source = div()
            .attr("id", "a")
            .add_class("x y")
            .child(p().text("t").child(Node::comment("c")))
            .child(Node::fragment([
                Node::text("a"),
                Node::raw("<b>"),
                Node::fragment([]),
                span().into(),
            ]))
            .child(Node::text("tail"));
        let copy = source.clone();
        assert_eq!(source, copy);
        assert_eq!(source.render(), copy.render());

        let changed = copy.clone().child(Node::text("more"));
        assert_ne!(source, changed);
        assert_eq!(source.render(), copy.render(), "the original is untouched");
    }

    #[test]
    fn equality_sees_every_field() {
        let base = div().attr("id", "a").text("t");
        assert_eq!(base, div().attr("id", "a").text("t"));
        assert_ne!(base, div().attr("id", "b").text("t"));
        assert_ne!(base, div().attr("id", "a").text("u"));
        assert_ne!(base, span().attr("id", "a").text("t"));
        assert_ne!(base, div().attr("id", "a").text("t").self_closing(true));
        assert_ne!(Node::text("a"), Node::raw("a"));
        assert_ne!(Node::text("a"), Node::comment("a"));
        assert_ne!(Node::fragment([]), Node::fragment([Node::text("")]));
        assert_eq!(Node::from(base.clone()), Node::from(base));
    }

    use super::*;

    /// What the derive wrote for this tree, captured before it was replaced, so the manual impl
    /// is the same text wherever the depth limit does not apply.
    #[test]
    fn debug_writes_what_the_derive_wrote() {
        let tree = div()
            .attr("id", "x")
            .bool_attr("hidden")
            .text("lead")
            .child(p().text("a<b"))
            .child(Node::fragment([
                Node::raw("<i>"),
                Node::comment("c"),
                Node::text("t"),
            ]));
        assert_eq!(
            format!("{tree:?}"),
            r#"Element { tag: "div", attributes: [Attribute { key: "id", value: "x", is_boolean: false, is_trusted_url: false }, Attribute { key: "hidden", value: "", is_boolean: true, is_trusted_url: false }], content: Some("lead"), children: [Element(Element { tag: "p", attributes: [], content: Some("a&lt;b"), children: [], self_closing: false }), Fragment([Raw(Markup("<i>")), Comment(Markup("c")), Text(Markup("t"))])], self_closing: false }"#
        );
        assert_eq!(
            format!("{:#?}", p().text("x").attr("id", "y")),
            "Element {\n    tag: \"p\",\n    attributes: [\n        Attribute {\n            key: \"id\",\n            value: \"y\",\n            is_boolean: false,\n            is_trusted_url: false,\n        },\n    ],\n    content: Some(\n        \"x\",\n    ),\n    children: [],\n    self_closing: false,\n}"
        );
    }

    /// A chain of `levels` nested `div`s.
    fn chain(levels: usize) -> Element {
        let mut tree = crate::elements::div();
        for _ in 1..levels {
            tree = crate::elements::div().child(tree);
        }
        tree
    }

    #[test]
    fn debug_shows_the_whole_tree_up_to_the_depth_limit_and_dots_below_it() {
        let at_the_limit = format!("{:?}", chain(DEBUG_MAX_DEPTH));
        assert!(!at_the_limit.contains("..."), "{at_the_limit}");

        let one_past = format!("{:?}", chain(DEBUG_MAX_DEPTH + 1));
        assert_eq!(one_past.matches("...").count(), 1, "{one_past}");
        assert!(one_past.contains("children: ..."), "{one_past}");
    }

    #[test]
    fn debug_of_a_fragment_chain_is_bounded_too() {
        let mut node = Node::fragment([]);
        for _ in 0..100_000 {
            node = Node::fragment([node]);
        }
        // Under an element, which frees what it holds iteratively; a bare chain of fragments is
        // the documented exception and would overflow on drop, not on `Debug`.
        let node = Node::from(crate::elements::div().child(node));
        let shown = format!("{node:?}");
        assert!(shown.contains("..."));
        assert!(shown.len() < 100_000, "{} bytes", shown.len());
    }

    #[test]
    fn csp_hash_covers_the_content_as_it_is_written_into_the_page() {
        // Reference values from an independent SHA-256 (Python's hashlib).
        assert_eq!(
            crate::elements::style()
                .raw_text("body{color:red}")
                .csp_hash()
                .as_deref(),
            Some("sha256-FcQqt3aNlV7AZnGV4zkQRVeCeJOxbMPnQSx258L803E=")
        );
        // `text` escapes, and the browser reads the escaped bytes, so that is what is hashed.
        assert_eq!(
            script().text("a<b").csp_hash().as_deref(),
            Some("sha256-ElKEL8tlu/71khLOyHevXk0zDglt1/FzVBvpN3U1gyU=")
        );
    }

    #[test]
    fn csp_hash_is_the_hash_of_what_sits_between_the_tags() {
        let element = script()
            .raw_text("let a = 1;")
            .child(Node::raw(" let b = 2;"));
        let rendered = element.render();
        let inner = rendered
            .strip_prefix("<script>")
            .and_then(|rest| rest.strip_suffix("</script>"))
            .expect("one script");
        assert_eq!(inner, "let a = 1; let b = 2;");
        let digest = crate::core::digest::sha256(inner.as_bytes());
        assert_eq!(
            element.csp_hash(),
            Some(format!("sha256-{}", crate::core::digest::base64(&digest)))
        );
    }

    #[test]
    fn csp_hash_is_none_outside_script_and_style_and_tracks_every_byte() {
        assert_eq!(div().text("x").csp_hash(), None);
        assert_eq!(script().self_closing(true).csp_hash(), None);
        assert_ne!(
            crate::elements::style().raw_text("a{}").csp_hash(),
            crate::elements::style().raw_text("a{} ").csp_hash()
        );
        assert!(
            script().csp_hash().is_some(),
            "an empty script still has a hash"
        );
    }
    use crate::core::RenderOptions;
    use crate::elements::{
        a, body, button, div, footer, head, header, html_tag, img, input_named, li, main_tag, meta,
        nav, ol, p, script, span, stylesheet, table, td, th, title, tr, ul,
    };

    /// `trusted_url` on the element builder.
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

    #[test]
    fn add_class_appends_to_one_attribute() {
        assert_eq!(
            div().add_class("a").add_class("b").render(),
            r#"<div class="a b"></div>"#
        );
    }

    #[test]
    fn chaining_add_class_does_not_double_escape() {
        let rendered = div().add_class("a&b").add_class("c").render();
        assert_eq!(rendered, r#"<div class="a&amp;b c"></div>"#);
        assert!(!rendered.contains("&amp;amp;"));
    }

    #[test]
    fn a_quote_in_a_class_name_cannot_break_out() {
        let rendered = div().add_class(r#"a" onload="alert(1)"#).render();
        assert!(!rendered.contains("onload=\""));
        assert!(rendered.contains("&quot;"));
    }

    #[test]
    fn add_classes_appends_all_of_them_in_order() {
        assert_eq!(
            div().add_classes(["card", "p-4", "shadow"]).render(),
            r#"<div class="card p-4 shadow"></div>"#
        );
    }

    #[test]
    fn set_id_replaces_rather_than_appending() {
        let rendered = div().set_id("first").set_id("second").render();
        assert_eq!(rendered, r#"<div id="second"></div>"#);
    }

    #[test]
    fn set_style_and_set_role_also_replace() {
        let rendered = div()
            .set_style("color:red")
            .set_style("color:blue")
            .set_role("main")
            .render();
        assert_eq!(rendered, r#"<div style="color:blue" role="main"></div>"#);
    }

    #[test]
    fn data_and_aria_attributes_get_their_prefixes() {
        let rendered = span()
            .data_attr("id", "7")
            .aria_attr("label", "Close")
            .render();
        assert_eq!(rendered, r#"<span data-id="7" aria-label="Close"></span>"#);
    }

    /// Ordered input means stable output.
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

    /// Plain attributes are appended and never deduplicated; only `set_id`, `set_style` and
    /// `set_role` replace.
    #[test]
    fn plain_attributes_are_appended_without_deduplication() {
        assert_eq!(
            div().attr("data-x", "1").attr("data-x", "2").render(),
            r#"<div data-x="1" data-x="2"></div>"#
        );
    }

    /// Escaping is the default, and turning it off is a separate method, so the escape hatch
    /// is greppable rather than hidden behind a default argument.
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

    #[test]
    fn a_tag_renders_its_attributes_then_its_content() {
        let tag = Element::new("p")
            .attr("class", "text")
            .text("Hello, World!");

        assert_eq!(tag.render(), r#"<p class="text">Hello, World!</p>"#);
    }

    /// Extra attributes can come before the typed ones. The `image(src, alt)` constructor puts
    /// `src` and `alt` first, so this order needs the generic builder.
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

    /// Built with `Element::new` rather than `dl()`/`dt()`/`dd()`: the point of the test is that
    /// the untyped escape hatch renders like any generated constructor.
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

    /// `raw_text`, not `text`: escaping the body would turn `'Hello World'` into
    /// `&#39;Hello World&#39;` and the browser would run that literally.
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

    #[test]
    fn a_title_renders_its_text() {
        let document = html_tag().child(title().text("Title my site"));

        assert_eq!(
            document.render(),
            "<html><title>Title my site</title></html>"
        );
    }

    #[test]
    fn a_span_renders_its_attributes_and_content() {
        let tag = span().attr("class", "text").text("Hello, World!");

        assert_eq!(tag.render(), r#"<span class="text">Hello, World!</span>"#);
    }

    /// `button_typed` puts the type first, so a type after the class comes from the generic
    /// builder.
    #[test]
    fn a_button_carries_its_type_after_its_class() {
        let document = html_tag().child(button().add_class("button-class").attr("type", "button"));

        assert_eq!(
            document.render(),
            r#"<html><button class="button-class" type="button"></button></html>"#
        );
    }

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

    /// The helpers take `self` and return `Self`, so the type survives by construction — what
    /// is worth pinning is the attribute order the chain produces.
    #[test]
    fn chaining_helpers_keeps_the_element_usable() {
        let card: Element = div().add_class("card").set_id("hero").set_role("region");

        assert_eq!(
            card.render(),
            r#"<div class="card" id="hero" role="region"></div>"#
        );
    }

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

    #[test]
    fn a_single_class_renders_on_its_own() {
        assert_eq!(
            div().add_class("container").render(),
            r#"<div class="container"></div>"#
        );
    }

    #[test]
    fn set_id_renders_an_id_attribute() {
        assert_eq!(
            div().set_id("main-content").render(),
            r#"<div id="main-content"></div>"#
        );
    }

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

    /// `data_attrs` takes an ordered iterator precisely so the output is deterministic, which
    /// is what this pins.
    #[test]
    fn several_data_attributes_keep_the_order_they_are_given() {
        let rendered = div()
            .data_attrs([("id", "123"), ("type", "product")])
            .render();

        assert_eq!(rendered, r#"<div data-id="123" data-type="product"></div>"#);
    }

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

    /// `attr` appends like every other helper — it does not replace an existing key; use
    /// `set_id`, `set_style` or `set_role` for that.
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
