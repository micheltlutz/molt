//! The node tree.
//!
//! Elements, text, raw markup, comments and fragments are the variants of one **closed enum**,
//! with a single `match` in the writer. A closed enum rather than trait objects means the
//! tree is `Send + Sync`, `Clone`, `PartialEq` and `Hash` without ceremony, and that is what
//! makes the `parallel` feature sound.

use crate::core::attribute::Attribute;
use crate::core::element::Element;
use crate::core::escape::write_escaped_text;
use crate::core::render::{Render, RenderOptions};
use crate::core::tags::{is_raw_text, is_valid_tag_name, is_void, is_whitespace_sensitive};

/// A node in the HTML tree.
///
/// # Examples
/// ```
/// use molt::prelude::*;
/// let node = Node::from(div().text("hi"));
/// assert_eq!(node.render(), "<div>hi</div>");
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum Node {
    /// An element with a tag name, attributes and children.
    Element(Element),
    /// Escaped text. The escaping already happened — see [`crate::core::escape`].
    Text(Markup),
    /// Verbatim markup, never escaped. For imported snippets: an SVG, an embed code.
    ///
    /// Unlike [`Node::Fragment`], this is an opaque string, so pretty printing can only
    /// indent the whole blob. Reach for a fragment when you want the children indented.
    Raw(Markup),
    /// An HTML comment.
    ///
    /// The content has `--` neutralised so the comment cannot be closed early.
    Comment(Markup),
    /// A transparent group that renders its children with no wrapper element.
    ///
    /// Use it to return several nodes from one expression — a `map` of cards, the body of
    /// an `if`, a group of `<meta>` tags — without introducing an extra `<div>`.
    Fragment(Vec<Node>),
}

/// A string that is safe to write into the output exactly as it is.
///
/// What a [`Node::Text`], [`Node::Raw`] or [`Node::Comment`] holds. It has **no public
/// constructor**: the only ways in are [`Node::text`] (which escapes), [`Node::comment`]
/// (which neutralises `--`) and [`Node::raw`] (the greppable escape hatch). Before this
/// type the variants held a bare `String`, so `Node::Text("<script>".into())` skipped the
/// escaping that the rest of the crate promises happens once, on entry.
///
/// # Examples
/// ```
/// use molt::prelude::*;
/// let Node::Text(text) = Node::text("a < b") else { unreachable!() };
/// assert_eq!(text.as_str(), "a &lt; b");
/// ```
///
/// Building one directly does not compile, which is the point:
///
/// ```compile_fail
/// use molt::core::{Markup, Node};
/// let _ = Node::Text(Markup("<script>".into()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Markup(String);

impl Markup {
    /// Takes the string out, so a caller that owns the node does not copy it.
    pub(crate) fn into_string(self) -> String {
        self.0
    }

    /// The content, already escaped or neutralised as its node requires.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::ops::Deref for Markup {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for Markup {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Markup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq<str> for Markup {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Markup {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

impl Node {
    /// Creates a text node, escaping the content.
    #[must_use]
    pub fn text(content: impl AsRef<str>) -> Self {
        let raw = content.as_ref();
        let mut escaped = String::with_capacity(raw.len());
        write_escaped_text(&mut escaped, raw, false);
        Self::Text(Markup(escaped))
    }

    /// A text node formatted from a `Display` value directly into its escaped form, with no
    /// intermediate `String`. What `html!` uses for `(expr)`.
    pub(crate) fn text_display(value: &impl std::fmt::Display) -> Self {
        use std::fmt::Write as _;
        let mut escaped = String::new();
        // Writing into a `String` cannot fail; a `Display` impl that errors leaves a short value.
        let _ = write!(crate::core::escape::TextWriter(&mut escaped), "{value}");
        Self::Text(Markup(escaped))
    }

    /// Creates a raw markup node. The content is **not** escaped.
    ///
    /// One of the three escape hatches, with [`Element::raw_text`](crate::core::Element::raw_text)
    /// and [`Attribute::raw`](crate::core::Attribute::raw); grep for the three to audit a
    /// codebase. Use it for markup you wrote or already sanitised: an SVG icon, an embed
    /// snippet. **Never pass text a user typed.** `<` in it becomes markup, `</script>` ends a
    /// script, and nothing downstream will stop it.
    ///
    /// For JSON in a `<script>` use [`seo::json_ld`](crate::seo::json_ld), which escapes it.
    /// To show untrusted text, use [`Node::text`].
    ///
    /// # Examples
    /// ```
    /// use molt::prelude::*;
    /// let icon = Node::raw(r#"<svg viewBox="0 0 1 1"><path d="M0 0h1v1z"/></svg>"#);
    /// assert!(icon.render().starts_with("<svg"));
    ///
    /// // The same text through `Node::text` is shown, not interpreted.
    /// assert_eq!(Node::text("<b>").render(), "&lt;b&gt;");
    /// ```
    #[must_use]
    pub fn raw(content: impl Into<String>) -> Self {
        Self::Raw(Markup(content.into()))
    }

    /// Creates a comment node.
    ///
    /// A space is put between every two adjacent `-` and after a trailing `-`, so no run of
    /// two hyphens survives and the comment cannot terminate early and inject markup.
    ///
    /// # Examples
    /// ```
    /// use molt::prelude::*;
    /// assert_eq!(Node::comment("a -- b").render(), "<!-- a - - b -->");
    /// assert_eq!(Node::comment("x--->").render(), "<!-- x- - -> -->");
    /// ```
    #[must_use]
    pub fn comment(content: impl AsRef<str>) -> Self {
        let mut neutralised = String::with_capacity(content.as_ref().len());
        push_neutralised_comment(&mut neutralised, content.as_ref());
        Self::Comment(Markup(neutralised))
    }

    /// Creates a transparent group of nodes.
    #[must_use]
    pub fn fragment(children: impl IntoIterator<Item = Node>) -> Self {
        Self::Fragment(children.into_iter().collect())
    }

    /// Whether this node renders to nothing.
    ///
    /// The pretty-print writer arrives at the same answer without calling this: it lets a
    /// child write straight into the output and rolls the separator back if the child wrote
    /// nothing, which is what stops an empty fragment — the body of a false `if` — from
    /// leaving a blank line behind.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Element(_) | Self::Comment(_) => false,
            Self::Text(s) | Self::Raw(s) => s.is_empty(),
            Self::Fragment(children) => {
                // Iterative: a chain of nested fragments, which nothing else bounds, must
                // not cost a stack frame per level.
                let mut pending: Vec<&Node> = children.iter().collect();
                while let Some(node) = pending.pop() {
                    match node {
                        Self::Element(_) | Self::Comment(_) => return false,
                        Self::Text(s) | Self::Raw(s) => {
                            if !s.is_empty() {
                                return false;
                            }
                        }
                        Self::Fragment(children) => pending.extend(children),
                    }
                }
                true
            }
        }
    }
}

// `Clone` and `PartialEq` are written by hand, not derived: the derives recurse once per
// level and a stack overflow is an abort, so comparing or copying a tree the renderer
// handles fine would have killed the process. `Debug` is still derived — formatting a
// tree that deep is a diagnostic nobody reads, and `{:?}` on one is documented as unsafe
// in `SECURITY.md`.
impl Clone for Node {
    fn clone(&self) -> Self {
        crate::core::element::clone_tree(Source::Node(self))
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        crate::core::element::trees_equal(Source::Node(self), Source::Node(other))
    }
}

impl Eq for Node {}

/// Where an iterative tree walk starts: a node, or an element that has no `Node` around it.
#[derive(Clone, Copy)]
pub(crate) enum Source<'a> {
    Node(&'a Node),
    Element(&'a Element),
}

impl std::fmt::Display for Node {
    /// The compact render, or the pretty one with the alternate flag: `{node}` / `{node:#}`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&if f.alternate() {
            self.render_pretty()
        } else {
            self.render()
        })
    }
}

impl std::hash::Hash for Node {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        crate::core::element::hash_tree(Source::Node(self), state);
    }
}

/// Collects nodes into a [`Node::Fragment`], so `.collect::<Node>()` replaces
/// `Node::fragment(iter)`.
impl<N: Into<Node>> FromIterator<N> for Node {
    fn from_iter<I: IntoIterator<Item = N>>(iter: I) -> Self {
        Self::Fragment(iter.into_iter().map(Into::into).collect())
    }
}

impl From<Vec<Node>> for Node {
    fn from(children: Vec<Node>) -> Self {
        Self::Fragment(children)
    }
}

/// `None` is an empty fragment, which renders to nothing and leaves no blank line.
impl<T: Into<Node>> From<Option<T>> for Node {
    fn from(value: Option<T>) -> Self {
        value.map_or_else(|| Self::Fragment(Vec::new()), Into::into)
    }
}

impl From<char> for Node {
    fn from(c: char) -> Self {
        Self::text(c.to_string())
    }
}

impl From<Element> for Node {
    fn from(element: Element) -> Self {
        Self::Element(element)
    }
}

impl From<&str> for Node {
    fn from(text: &str) -> Self {
        Self::text(text)
    }
}

impl From<String> for Node {
    fn from(text: String) -> Self {
        Self::text(text)
    }
}

impl Render for Node {
    fn write_into(&self, out: &mut String, options: &RenderOptions, depth: usize) {
        write_tree(
            if options.pretty {
                Step::Pretty(self, depth)
            } else {
                Step::Compact(self)
            },
            out,
            options,
        );
    }
}

/// Renders an element without wrapping it in a [`Node`] first.
///
/// [`Render for Element`](Element) used to do `Node::Element(self.clone()).write_into(..)`,
/// which deep-copied the whole subtree on every render — and cloning is itself recursive,
/// so a deep tree aborted in `Clone` before the writer ever saw it.
pub(crate) fn write_element_tree(
    element: &Element,
    out: &mut String,
    options: &RenderOptions,
    depth: usize,
) {
    write_tree(
        if options.pretty {
            Step::PrettyElement(element, depth)
        } else {
            Step::CompactElement(element)
        },
        out,
        options,
    );
}

/// Writes `<html>` around a borrowed `head` and `body`, without copying either.
///
/// `Document::root` builds the same tree by cloning both, and cloning is recursive, so
/// rendering a document with a deep body aborted in `Clone` before the writer ran. The
/// bytes are identical to rendering that tree: this queues the same steps
/// `push_compact_element`/`push_pretty_element` would for an `<html>` with two children.
pub(crate) fn write_document_root(
    lang: Option<&str>,
    head: &Element,
    body: &Element,
    out: &mut String,
    options: &RenderOptions,
) {
    if options.pretty {
        options.write_indent(out, 0);
    }
    out.push_str("<html");
    if let Some(lang) = lang {
        Attribute::new_static("lang", lang).write_into(out);
    }
    out.push('>');

    // The stack is a LIFO: the last step pushed runs first.
    let steps = if options.pretty {
        vec![
            Step::ClosePretty("html", 0),
            Step::PrettyChildElement {
                element: body,
                depth: 1,
            },
            Step::PrettyChildElement {
                element: head,
                depth: 1,
            },
        ]
    } else {
        vec![
            Step::Close("html"),
            Step::CompactElement(body),
            Step::CompactElement(head),
        ]
    };
    // `<html>` is written above by hand, so head and body already sit one level down.
    run_steps(steps, out, options, 1);
}

/// One unit of work for the writer.
///
/// The writer used to recurse once per nesting level, which made tree depth a stack
/// limit: roughly 2,000 levels aborted the process, and a stack overflow is an abort, not
/// a panic anything can catch. Each variant here is what one of those recursive calls
/// used to do, held on an explicit stack instead — so depth costs heap, bounded by a tree
/// that is already in memory.
enum Step<'a> {
    /// Write this node on one line.
    Compact(&'a Node),
    /// Write this node indented to `depth`.
    Pretty(&'a Node, usize),
    /// Write this element on one line.
    CompactElement(&'a Element),
    /// Write this element indented to `depth`.
    PrettyElement(&'a Element, usize),
    /// `</tag>`.
    Close(&'a str),
    /// A newline, indentation to `depth`, then `</tag>`.
    ClosePretty(&'a str, usize),
    /// A child in pretty mode: writes the separating newline, renders the child, and then
    /// takes both back out if the child rendered to nothing.
    ///
    /// `group_start` is `Some` inside a fragment, where the separator is only written once
    /// something has already been emitted, and `None` inside an element, where every child
    /// gets one.
    PrettyChild {
        node: &'a Node,
        depth: usize,
        group_start: Option<usize>,
    },
    /// An element child of a borrowed parent, in pretty mode: [`Step::PrettyChild`] for
    /// callers that hold `&Element` and have no `Node` to point at.
    PrettyChildElement { element: &'a Element, depth: usize },
    /// Truncates back to `mark` when nothing was appended after `after`.
    ///
    /// This replaces rendering each child into a scratch buffer to test it for emptiness:
    /// the child writes straight into the output and its separator is rolled back if it
    /// wrote nothing, which is what still keeps an empty fragment — the body of a false
    /// `@if` — from leaving a blank line behind.
    DropIfEmpty { mark: usize, after: usize },
}

/// Drives the steps until the tree is written.
fn write_tree(start: Step<'_>, out: &mut String, options: &RenderOptions) {
    let mut stack = Vec::with_capacity(16);
    stack.push(start);
    run_steps(stack, out, options, 0);
}

/// Runs a prepared stack to completion.
fn run_steps(stack: Vec<Step<'_>>, out: &mut String, options: &RenderOptions, already_open: usize) {
    // Two monomorphised copies: the one that counts open elements for `max_depth`, and the
    // one that does not. The counter costs a few percent per element on a wide tree, and most
    // renders set no limit, so they run the copy without it.
    if options.max_depth.is_some() {
        run_steps_with::<true>(stack, out, options, already_open);
    } else {
        run_steps_with::<false>(stack, out, options, already_open);
    }
}

// One `match` over every `Step`, which is the writer's whole state machine: splitting it
// would hide the single place where all the cases can be read together.
#[allow(clippy::too_many_lines)]
fn run_steps_with<const LIMITED: bool>(
    mut stack: Vec<Step<'_>>,
    out: &mut String,
    options: &RenderOptions,
    already_open: usize,
) {
    // How many raw-text elements (`noscript`, `script`, …) enclose the node being written.
    // Their descendants' attribute values get `<` and `>` escaped; see
    // `Attribute::write_into_in`.
    let mut raw_text_depth = 0usize;
    // How many elements are open around the node being written, for `max_depth`. Counted
    // here, not read from the pretty indent depth, so compact mode is covered too.
    let mut open_depth = already_open;
    let max_depth = options.max_depth;
    while let Some(step) = stack.pop() {
        match step {
            // Escaped text and verbatim markup write the same way; they differ only in
            // when the escaping happened, which is at construction.
            Step::Compact(Node::Text(text) | Node::Raw(text)) => out.push_str(text),
            Step::Compact(Node::Comment(content)) => write_comment(out, content),
            Step::Compact(Node::Fragment(children)) => {
                stack.extend(children.iter().rev().map(Step::Compact));
            }
            Step::Compact(Node::Element(element)) => stack.push(Step::CompactElement(element)),

            Step::Pretty(Node::Text(text) | Node::Raw(text), depth) => {
                options.write_indent(out, depth);
                out.push_str(text);
            }
            Step::Pretty(Node::Comment(content), depth) => {
                options.write_indent(out, depth);
                write_comment(out, content);
            }
            Step::Pretty(Node::Fragment(children), depth) => {
                // The children sit at the *parent's* depth, joined by newlines, with empty
                // ones skipped entirely.
                let group_start = out.len();
                stack.extend(children.iter().rev().map(|child| Step::PrettyChild {
                    node: child,
                    depth,
                    group_start: Some(group_start),
                }));
            }
            Step::Pretty(Node::Element(element), depth) => {
                stack.push(Step::PrettyElement(element, depth));
            }

            Step::CompactElement(element) => {
                // Too deep: write nothing, before any indent, so pretty mode's rollback of
                // the child separator leaves no blank line (the path an invalid tag takes).
                if LIMITED && max_depth.is_some_and(|max| open_depth >= max) {
                    continue;
                }
                let in_raw_text = raw_text_depth > 0;
                if push_compact_element(element, out, options, &mut stack, in_raw_text) {
                    if LIMITED {
                        open_depth += 1;
                    }
                    if is_raw_text(element.tag()) {
                        raw_text_depth += 1;
                    }
                }
            }
            Step::PrettyElement(element, depth) => {
                if LIMITED && max_depth.is_some_and(|max| open_depth >= max) {
                    continue;
                }
                let in_raw_text = raw_text_depth > 0;
                if push_pretty_element(element, depth, out, options, &mut stack, in_raw_text) {
                    if LIMITED {
                        open_depth += 1;
                    }
                    if is_raw_text(element.tag()) {
                        raw_text_depth += 1;
                    }
                }
            }

            Step::Close(tag) => {
                if LIMITED {
                    open_depth = open_depth.saturating_sub(1);
                }
                if is_raw_text(tag) {
                    raw_text_depth = raw_text_depth.saturating_sub(1);
                }
                out.push_str("</");
                out.push_str(tag);
                out.push('>');
            }
            Step::ClosePretty(tag, depth) => {
                if LIMITED {
                    open_depth = open_depth.saturating_sub(1);
                }
                if is_raw_text(tag) {
                    raw_text_depth = raw_text_depth.saturating_sub(1);
                }
                out.push('\n');
                options.write_indent(out, depth);
                out.push_str("</");
                out.push_str(tag);
                out.push('>');
            }

            Step::PrettyChild {
                node,
                depth,
                group_start,
            } => {
                let mark = out.len();
                let needs_separator = group_start.is_none_or(|start| out.len() > start);
                if needs_separator {
                    out.push('\n');
                }
                let after = out.len();
                // Pushed first so it runs last: the child goes on top of it.
                stack.push(Step::DropIfEmpty { mark, after });
                stack.push(Step::Pretty(node, depth));
            }
            Step::PrettyChildElement { element, depth } => {
                let mark = out.len();
                out.push('\n');
                let after = out.len();
                stack.push(Step::DropIfEmpty { mark, after });
                stack.push(Step::PrettyElement(element, depth));
            }
            Step::DropIfEmpty { mark, after } => {
                if out.len() == after {
                    out.truncate(mark);
                }
            }
        }
    }
}

/// Writes an element's opening token and queues everything that follows it.
///
/// Returns whether a closing step was queued, i.e. whether the element is now open.
fn push_compact_element<'a>(
    element: &'a Element,
    out: &mut String,
    options: &RenderOptions,
    stack: &mut Vec<Step<'a>>,
    in_raw_text: bool,
) -> bool {
    // An unsafe tag name is dropped with its whole subtree rather than repaired: there is
    // no right guess for what `div><script>` was meant to be.
    if !is_valid_tag_name(element.tag()) {
        return false;
    }
    out.push('<');
    out.push_str(element.tag());
    write_attributes(element.attributes(), out, in_raw_text, element.tag());

    // A void element takes no content and no children — both are silently dropped; that is
    // the contract. `Element::self_closing` opts a non-void tag into the same
    // shape, so a caller can emit `<input />` without building a `Node::Raw` that would
    // skip the URL policy in `Attribute::write_into`.
    if is_void(element.tag()) || element.is_self_closing() {
        write_void_suffix(out, options);
        return false;
    }

    out.push('>');
    if let Some(content) = element.content() {
        out.push_str(content);
    }
    stack.push(Step::Close(element.tag()));
    stack.extend(element.children().iter().rev().map(Step::Compact));
    true
}

/// The same, indented, with the children queued one level deeper.
fn push_pretty_element<'a>(
    element: &'a Element,
    depth: usize,
    out: &mut String,
    options: &RenderOptions,
    stack: &mut Vec<Step<'a>>,
    in_raw_text: bool,
) -> bool {
    // Checked before the indent is written, so a dropped element leaves nothing behind for
    // `DropIfEmpty` to trip over.
    if !is_valid_tag_name(element.tag()) {
        return false;
    }
    // `<pre>`, `<code>` and `<textarea>` render every whitespace character they contain, so
    // indenting their children would change what the browser shows.
    if is_whitespace_sensitive(element.tag()) {
        options.write_indent(out, depth);
        stack.push(Step::CompactElement(element));
        return false;
    }

    options.write_indent(out, depth);
    out.push('<');
    out.push_str(element.tag());
    write_attributes(element.attributes(), out, in_raw_text, element.tag());

    if is_void(element.tag()) || element.is_self_closing() {
        write_void_suffix(out, options);
        return false;
    }

    out.push('>');

    if element.children().is_empty() {
        if let Some(content) = element.content() {
            out.push_str(content);
        }
        out.push_str("</");
        out.push_str(element.tag());
        out.push('>');
        return false;
    }

    if let Some(content) = element.content() {
        out.push('\n');
        options.write_indent(out, depth + 1);
        out.push_str(content);
    }

    stack.push(Step::ClosePretty(element.tag(), depth));
    stack.extend(
        element
            .children()
            .iter()
            .rev()
            .map(|child| Step::PrettyChild {
                node: child,
                depth: depth + 1,
                group_start: None,
            }),
    );
    true
}

fn write_comment(out: &mut String, content: &str) {
    // The content was neutralised when the node was built, and `Markup` has no public
    // constructor, so a `Node::Comment` cannot hold anything else.
    out.push_str("<!-- ");
    out.push_str(content);
    out.push_str(" -->");
}

/// Appends `content` with a space between adjacent `-` and after a trailing `-`.
///
/// A single `replace("--", "- -")` is not enough: `---` becomes `- --`, which still holds
/// `--`, and `--->` still closes the comment.
fn push_neutralised_comment(out: &mut String, content: &str) {
    let mut previous_hyphen = false;
    for c in content.chars() {
        if c == '-' && previous_hyphen {
            out.push(' ');
        }
        out.push(c);
        previous_hyphen = c == '-';
    }
    if previous_hyphen {
        out.push(' ');
    }
}

fn write_attributes(attributes: &[Attribute], out: &mut String, in_raw_text: bool, tag: &str) {
    for attribute in attributes {
        attribute.write_into_in(out, in_raw_text, tag);
    }
}

/// Appends the closing token of a void element.
fn write_void_suffix(out: &mut String, options: &RenderOptions) {
    out.push_str(if options.xhtml_self_closing {
        " />"
    } else {
        ">"
    });
}

#[cfg(test)]
mod tests {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    fn hash_of(value: &impl Hash) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn equal_trees_hash_equal_and_different_trees_do_not() {
        let a = div()
            .attr("id", "x")
            .child(p().text("t"))
            .child(Node::comment("c"));
        let b = div()
            .attr("id", "x")
            .child(p().text("t"))
            .child(Node::comment("c"));
        assert_eq!(a, b);
        assert_eq!(hash_of(&a), hash_of(&b));
        assert_eq!(hash_of(&Node::from(a.clone())), hash_of(&Node::from(b)));
        assert_ne!(hash_of(&a), hash_of(&a.clone().add_class("y")));
        assert_ne!(hash_of(&Node::text("a")), hash_of(&Node::raw("a")));
        assert_ne!(
            hash_of(&Node::fragment([Node::text("a"), Node::text("b")])),
            hash_of(&Node::fragment([
                Node::fragment([Node::text("a")]),
                Node::text("b")
            ]))
        );
    }

    #[test]
    fn display_is_compact_and_alternate_is_pretty() {
        let el = div().child(p().text("hi"));
        assert_eq!(el.to_string(), "<div><p>hi</p></div>");
        assert_eq!(format!("{el:#}"), "<div>\n  <p>hi</p>\n</div>");
        let node = Node::from(el);
        assert_eq!(node.to_string(), "<div><p>hi</p></div>");
        assert_eq!(format!("{node:#}"), "<div>\n  <p>hi</p>\n</div>");
    }

    #[test]
    fn nodes_collect_and_convert_from_the_common_shapes() {
        let collected: Node = ["a", "b"].into_iter().map(Node::text).collect();
        assert_eq!(collected.render(), "ab");
        assert_eq!(
            Node::from(vec![Node::text("a"), Node::text("b")]).render(),
            "ab"
        );
        assert_eq!(Node::from(Some(p().text("x"))).render(), "<p>x</p>");
        assert!(Node::from(None::<Element>).is_empty());
        assert_eq!(Node::from('<').render(), "&lt;");

        let mut list = ul();
        list.extend([li().text("1"), li().text("2")]);
        list.extend(["tail"]);
        assert_eq!(list.render(), "<ul><li>1</li><li>2</li>tail</ul>");
    }

    #[test]
    fn references_boxes_and_options_render_like_what_they_hold() {
        // Through a generic bound, so the impl for the wrapper (`&T`, `Box<T>`, `Option<T>`) is
        // what runs. Calling `.render()` on a `Box<Element>` would auto-deref to the element's
        // own method and test nothing about the `Box` impl.
        fn through<T: Render>(value: T) -> String {
            value.render()
        }
        let el = p().text("x");
        assert_eq!(through(&el), "<p>x</p>");
        assert_eq!(through(Box::new(el.clone())), "<p>x</p>");
        assert_eq!(through(Some(el.clone())), "<p>x</p>");
        assert_eq!(through(None::<Element>), "");
        let dynamic: Box<dyn Render> = Box::new(el);
        assert_eq!(dynamic.render(), "<p>x</p>");
    }

    #[test]
    fn a_document_has_a_default_and_a_display() {
        let page = crate::document::Document::default();
        assert_eq!(page.to_string(), page.render());
        assert!(page.to_string().starts_with("<!DOCTYPE html>\n<html>"));
    }

    use super::*;
    use crate::elements::{
        body, code, div, h1, head, html_tag, i, img, li, p, pre, span, textarea, title, ul,
    };
    // `macros` is declared after `core` in lib.rs, so `html!` is not in scope by position;
    // it is `#[macro_export]`ed, which puts it at the crate root.
    use crate::html;

    #[test]
    fn the_tree_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Node>();
        assert_send_sync::<Element>();
    }

    #[test]
    fn text_nodes_are_escaped() {
        assert_eq!(Node::text("a & b").render(), "a &amp; b");
    }

    /// The node-level escape hatch; `Element::raw_text` is the element-level one.
    #[test]
    fn raw_nodes_are_never_escaped() {
        assert_eq!(Node::raw("<b>bold</b>").render(), "<b>bold</b>");
        assert_eq!(Node::raw("<b>bold</b>").render_pretty(), "<b>bold</b>");
    }

    #[test]
    fn a_comment_cannot_close_itself_early() {
        let rendered = Node::comment("a --> b").render();
        assert_eq!(rendered.matches("-->").count(), 1);
        assert!(rendered.ends_with("-->"));
    }

    /// `replace("--", "- -")` turned `--->` into `- -->`, which still closed the comment.
    /// `Element::new` takes any string, and the tag is written verbatim between `<` and `>`.
    #[test]
    fn an_unsafe_tag_name_renders_nothing() {
        for hostile in [
            "img src=x onerror=alert(1)",
            "div><script>alert(1)</script><div",
            "a\"b",
            "",
            "1div",
            "p\n",
        ] {
            let tree = div()
                .child(Element::new(hostile).text("kept?"))
                .child(p().text("after"));
            assert_eq!(tree.render(), "<div><p>after</p></div>", "{hostile:?}");
            assert_eq!(
                tree.render_pretty(),
                "<div>\n  <p>after</p>\n</div>",
                "{hostile:?} must leave no blank line in pretty mode"
            );
            assert_eq!(Element::new(hostile).render(), "", "{hostile:?}");
        }
    }

    #[test]
    fn custom_and_namespaced_tags_still_render() {
        assert_eq!(
            Element::new("my-card").text("x").render(),
            "<my-card>x</my-card>"
        );
        assert_eq!(Element::new("svg:rect").render(), "<svg:rect></svg:rect>");
    }

    /// Inside `<noscript>` the parser reads raw text, so `</noscript>` in a descendant's
    /// attribute value ends the element. Elsewhere `<` in a quoted value is inert, and the
    /// golden fixtures depend on it staying unescaped.
    #[test]
    fn angle_brackets_in_attributes_are_escaped_only_under_raw_text_parents() {
        let hostile = "</noscript><img src=x onerror=alert(1)>";
        let escaped = "&lt;/noscript&gt;&lt;img src=x onerror=alert(1)&gt;";

        for parent in [
            "noscript", "iframe", "xmp", "noembed", "noframes", "style", "script",
        ] {
            let tree = Element::new(parent).child(img().attr("alt", hostile));
            assert_eq!(
                tree.render(),
                format!(r#"<{parent}><img alt="{escaped}"></{parent}>"#),
                "{parent}"
            );
        }

        assert_eq!(
            div().child(img().attr("alt", hostile)).render(),
            format!(r#"<div><img alt="{hostile}"></div>"#),
            "ordinary parents leave `<` and `>` in an attribute value alone"
        );
    }

    #[test]
    fn the_raw_text_escape_covers_deep_descendants_and_ends_with_the_parent() {
        let hostile = "a<b>c";
        let tree = div()
            .child(
                Element::new("noscript")
                    .attr("title", hostile)
                    .child(div().child(span().child(img().attr("alt", hostile)))),
            )
            .child(img().attr("alt", hostile));
        // The noscript's own attribute is read before raw-text mode starts, so it keeps the
        // plain form; its descendant is escaped; the sibling after it is plain again.
        assert_eq!(
            tree.render(),
            concat!(
                r#"<div><noscript title="a<b>c"><div><span>"#,
                r#"<img alt="a&lt;b&gt;c"></span></div></noscript>"#,
                r#"<img alt="a<b>c"></div>"#
            )
        );
        assert_eq!(tree.render_pretty().matches("a&lt;b&gt;c").count(), 1);
        assert_eq!(tree.render_pretty().matches("a<b>c").count(), 2);
    }

    #[test]
    fn a_comment_survives_runs_of_hyphens_and_bang_closers() {
        for hostile in [
            "x---><img src=x onerror=alert(1)>",
            "--!><b>",
            "a-",
            "---",
            "-->",
            "--",
        ] {
            let rendered = Node::comment(hostile).render();
            let inner = rendered
                .strip_prefix("<!-- ")
                .and_then(|r| r.strip_suffix(" -->"));
            let inner = inner.expect("comment keeps its delimiters");
            assert!(!inner.contains("--"), "{hostile:?} -> {rendered:?}");
            assert!(!inner.ends_with('-'), "{hostile:?} -> {rendered:?}");
        }
    }

    #[test]
    fn an_empty_fragment_leaves_no_blank_line() {
        let tree = div()
            .child(p().text("a"))
            .child(Node::fragment([]))
            .child(p().text("b"));
        assert_eq!(
            tree.render_pretty(),
            "<div>\n  <p>a</p>\n  <p>b</p>\n</div>"
        );
    }

    #[test]
    fn a_fragment_renders_its_children_without_a_wrapper() {
        let tree = Node::fragment([p().text("a").into(), p().text("b").into()]);
        assert_eq!(tree.render(), "<p>a</p><p>b</p>");
        assert_eq!(tree.render_pretty(), "<p>a</p>\n<p>b</p>");
    }

    #[test]
    fn void_elements_take_no_children() {
        let tree = img().attr("src", "/a.png").child(span().text("dropped"));
        assert_eq!(tree.render(), r#"<img src="/a.png">"#);
    }

    #[test]
    fn xhtml_mode_closes_void_elements_with_a_slash() {
        let options = RenderOptions::compact().with_xhtml_self_closing(true);
        assert_eq!(
            img().attr("src", "/a.png").render_with(&options),
            r#"<img src="/a.png" />"#
        );
    }

    /// Indenting inside `<pre>` would change what the browser shows.
    #[test]
    fn whitespace_sensitive_tags_are_not_indented_inside() {
        let tree = div().child(pre().child(code().text("let page = html { }")));
        assert_eq!(
            tree.render_pretty(),
            "<div>\n  <pre><code>let page = html { }</code></pre>\n</div>"
        );
    }

    /// Content and children can be mixed; the content comes first, on its own line.
    #[test]
    fn content_goes_on_its_own_line_before_children() {
        let tree = div().text("lead").child(p().text("body"));
        assert_eq!(tree.render_pretty(), "<div>\n  lead\n  <p>body</p>\n</div>");
    }

    /// The site's contact block: a label child, then the value as text. It rendered the value
    /// first because `text` filled a slot that the renderer always emits before the children.
    #[test]
    fn child_then_text_keeps_call_order() {
        let tree = p().child(i().text("Owner: ")).text("Ada");
        assert_eq!(tree.render(), "<p><i>Owner: </i>Ada</p>");
        assert_eq!(tree.render_pretty(), "<p>\n  <i>Owner: </i>\n  Ada\n</p>");
    }

    #[test]
    fn text_then_child_then_text_keeps_call_order() {
        let tree = p().text("a ").child(span().text("b")).text(" c");
        assert_eq!(tree.render(), "<p>a <span>b</span> c</p>");
    }

    #[test]
    fn two_text_calls_append() {
        assert_eq!(p().text("a").text("b").render(), "<p>ab</p>");
        assert_eq!(p().text("a<").text("<b").render(), "<p>a&lt;&lt;b</p>");
    }

    #[test]
    fn raw_text_follows_the_same_order_and_text_after_it_is_still_escaped() {
        // The raw string is trusted markup written in the test.
        let tree = p().raw_text("<i>").child(span()).raw_text("</i>").text("<");
        assert_eq!(tree.render(), "<p><i><span></span></i>&lt;</p>");
    }

    #[test]
    fn a_code_window_line_ends_with_its_newline() {
        let line = span()
            .child(span().text("1"))
            .child(span().text(" let x = 1;"))
            .text("\n");
        assert_eq!(
            line.render(),
            "<span><span>1</span><span> let x = 1;</span>\n</span>"
        );
    }

    #[test]
    fn an_element_with_content_and_no_children_stays_on_one_line() {
        assert_eq!(p().text("hi").render_pretty(), "<p>hi</p>");
    }

    #[test]
    fn compact_is_the_default_for_an_element() {
        let tree = div().child(p().text("a"));
        assert_eq!(tree.render(), "<div><p>a</p></div>");
    }

    #[test]
    fn raw_markup_renders_with_no_wrapper_around_it() {
        let raw = Node::raw(r#"<span class="x">hi</span>"#);

        assert_eq!(raw.render(), r#"<span class="x">hi</span>"#);
        assert!(!raw.render().contains("<div"));
    }

    #[test]
    fn raw_markup_as_a_child_adds_no_wrapper() {
        let markup = div()
            .child(Node::raw(r#"<i class="fa fa-home"></i>"#))
            .child(span().text("Home"));

        assert_eq!(
            markup.render(),
            r#"<div><i class="fa fa-home"></i><span>Home</span></div>"#
        );
    }

    #[test]
    fn a_fragment_renders_its_children_with_no_wrapper() {
        let fragment = Node::fragment([
            i().add_class("fa fa-star").into(),
            span().text(" Featured").into(),
        ]);

        assert_eq!(
            fragment.render(),
            r#"<i class="fa fa-star"></i><span> Featured</span>"#
        );
    }

    #[test]
    fn a_fragment_takes_a_mapped_sequence() {
        let fragment = Node::fragment(
            ["One", "Two", "Three"].map(|title| div().add_class("card").text(title).into()),
        );

        assert_eq!(
            fragment.render(),
            concat!(
                r#"<div class="card">One</div><div class="card">Two</div>"#,
                r#"<div class="card">Three</div>"#,
            )
        );
    }

    // These pin the empty-fragment behaviour the pretty writer exists to preserve, so they are
    // also the independent check on the iterative rewrite.

    #[test]
    fn an_empty_fragment_renders_nothing_in_either_mode() {
        assert_eq!(Node::fragment([]).render(), "");
        assert_eq!(Node::fragment([]).render_pretty(), "");
    }

    #[test]
    fn a_fragments_children_are_indented_as_the_parents_own() {
        let list = ul().child(Node::fragment([
            li().text("a").into(),
            li().text("b").into(),
        ]));

        assert_eq!(
            list.render_pretty(),
            "<ul>\n  <li>a</li>\n  <li>b</li>\n</ul>"
        );
    }

    #[test]
    fn a_fragment_takes_a_filtered_and_mapped_sequence() {
        let names = ["Ana", "Bruno", "Carla"];
        let group = Node::fragment(
            names
                .iter()
                .filter(|name| name.len() > 3)
                .map(|name| li().text(name).into()),
        );

        assert_eq!(group.render(), "<li>Bruno</li><li>Carla</li>");
    }

    #[test]
    fn a_false_condition_emits_no_stray_node() {
        let show_banner = false;
        let page = html_tag()
            .child(head().child(title().text("Home")))
            .child(html! { @if show_banner { div { "banner" } } })
            .child(body().child(h1().text("Hi")));

        assert_eq!(
            page.render(),
            "<html><head><title>Home</title></head><body><h1>Hi</h1></body></html>"
        );
    }

    #[test]
    fn a_true_condition_emits_its_branch() {
        let show_banner = true;
        let page = html_tag().child(html! { @if show_banner { div { "banner" } } });

        assert_eq!(page.render(), "<html><div>banner</div></html>");
    }

    #[test]
    fn a_loop_emits_one_node_per_iteration() {
        let page = html_tag().child(html! {
            @for index in 1..=3 { p { "line " (index) } }
        });

        assert_eq!(
            page.render(),
            "<html><p>line 1</p><p>line 2</p><p>line 3</p></html>"
        );
    }

    #[test]
    fn raw_markup_is_indented_as_one_blob() {
        let container = div().child(Node::raw("<custom-element></custom-element>"));

        assert_eq!(
            container.render_pretty(),
            "<div>\n  <custom-element></custom-element>\n</div>"
        );
    }

    #[test]
    fn a_condition_can_pick_either_branch() {
        fn badge(is_beta: bool) -> Node {
            html! { @if is_beta { span { "beta" } } @else { span { "stable" } } }
        }

        assert_eq!(badge(true).render(), "<span>beta</span>");
        assert_eq!(badge(false).render(), "<span>stable</span>");
    }

    /// Two levels of nesting with an empty fragment between them: the rollback has to
    /// survive being nested inside another rollback.
    #[test]
    fn nested_fragments_flatten_without_leaving_gaps() {
        let list = ul().child(Node::fragment([
            Node::fragment([li().text("a").into()]),
            Node::fragment([]),
            li().text("b").into(),
        ]));

        assert_eq!(
            list.render_pretty(),
            "<ul>\n  <li>a</li>\n  <li>b</li>\n</ul>"
        );
    }

    #[test]
    fn raw_markup_is_emitted_verbatim() {
        let raw = Node::raw(r#"<custom-element data-x="1"></custom-element>"#);

        assert_eq!(
            raw.render(),
            r#"<custom-element data-x="1"></custom-element>"#
        );
    }

    #[test]
    fn a_tag_with_only_content_stays_on_one_line_when_pretty() {
        assert_eq!(
            div().text("Hello World").render_pretty(),
            "<div>Hello World</div>"
        );
    }

    #[test]
    fn children_each_get_their_own_line_when_pretty() {
        let tree = div()
            .child(p().text("Paragraph 1"))
            .child(p().text("Paragraph 2"));

        assert_eq!(
            tree.render_pretty(),
            "<div>\n  <p>Paragraph 1</p>\n  <p>Paragraph 2</p>\n</div>"
        );
    }

    #[test]
    fn compact_keeps_everything_on_one_line() {
        assert_eq!(
            div().child(p().text("Test")).render(),
            "<div><p>Test</p></div>"
        );
    }

    #[test]
    fn a_void_element_does_not_self_close_when_pretty() {
        let rendered = img()
            .attr("src", "test.jpg")
            .attr("alt", "Test")
            .render_pretty();

        assert!(rendered.starts_with("<img"));
        assert!(!rendered.contains("/>"));
        assert!(rendered.ends_with('>'));
    }

    #[test]
    fn a_textarea_keeps_its_newlines() {
        let field = textarea().attr("name", "bio").text("line 1\nline 2");

        assert_eq!(
            field.render_pretty(),
            "<textarea name=\"bio\">line 1\nline 2</textarea>"
        );
    }

    #[test]
    fn a_pre_block_is_indented_from_the_outside_but_not_within() {
        let container = div().child(pre().child(code().text("cargo build")));

        assert_eq!(
            container.render_pretty(),
            "<div>\n  <pre><code>cargo build</code></pre>\n</div>"
        );
    }
}
