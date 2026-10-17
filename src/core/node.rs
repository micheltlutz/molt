//! The node tree.
//!
//! Ports `core/HTMLTag.swift` (state), `core/Fragment.swift` and `core/RawHTML.swift`.
//!
//! Winged-Swift models fragments and raw markup as **subclasses** of `HTMLTag` that
//! override `write(into:)`. Rust models the same thing as a closed enum with a `match` in
//! the writer. That is strictly better here: the tree becomes `Send + Sync` for free,
//! which Winged-Swift's own `ROADMAP.md` lists as an unresolved 3.0 problem ("tag trees
//! are not `Sendable`"), and which is what makes the `parallel` feature sound.

use crate::core::attribute::Attribute;
use crate::core::element::Element;
use crate::core::escape::write_escaped_text;
use crate::core::render::{Render, RenderOptions};
use crate::core::tags::{is_valid_tag_name, is_void, is_whitespace_sensitive};

/// A node in the HTML tree.
///
/// # Examples
/// ```
/// use molt::prelude::*;
/// let node = Node::from(div().text("hi"));
/// assert_eq!(node.render(), "<div>hi</div>");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// An element with a tag name, attributes and children.
    Element(Element),
    /// Escaped text. The escaping already happened — see [`crate::core::escape`].
    Text(String),
    /// Verbatim markup, never escaped. For imported snippets: an SVG, an embed code.
    ///
    /// Unlike [`Node::Fragment`], this is an opaque string, so pretty printing can only
    /// indent the whole blob. Reach for a fragment when you want the children indented.
    Raw(String),
    /// An HTML comment. **New in the Rust port** — Winged-Swift has no comment node.
    ///
    /// The content has `--` neutralised so the comment cannot be closed early.
    Comment(String),
    /// A transparent group that renders its children with no wrapper element.
    ///
    /// Use it to return several nodes from one expression — a `map` of cards, the body of
    /// an `if`, a group of `<meta>` tags — without introducing an extra `<div>`.
    Fragment(Vec<Node>),
}

impl Node {
    /// Creates a text node, escaping the content.
    #[must_use]
    pub fn text(content: impl AsRef<str>) -> Self {
        let raw = content.as_ref();
        let mut escaped = String::with_capacity(raw.len());
        write_escaped_text(&mut escaped, raw, false);
        Self::Text(escaped)
    }

    /// Creates a raw markup node. The content is **not** escaped.
    #[must_use]
    pub fn raw(content: impl Into<String>) -> Self {
        Self::Raw(content.into())
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
        Self::Comment(neutralised)
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
            Self::Fragment(children) => children.iter().all(Self::is_empty),
        }
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
                push_compact_element(element, out, options, &mut stack);
            }
            Step::PrettyElement(element, depth) => {
                push_pretty_element(element, depth, out, options, &mut stack);
            }

            Step::Close(tag) => {
                out.push_str("</");
                out.push_str(tag);
                out.push('>');
            }
            Step::ClosePretty(tag, depth) => {
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
            Step::DropIfEmpty { mark, after } => {
                if out.len() == after {
                    out.truncate(mark);
                }
            }
        }
    }
}

/// Writes an element's opening token and queues everything that follows it.
fn push_compact_element<'a>(
    element: &'a Element,
    out: &mut String,
    options: &RenderOptions,
    stack: &mut Vec<Step<'a>>,
) {
    // An unsafe tag name is dropped with its whole subtree rather than repaired: there is
    // no right guess for what `div><script>` was meant to be.
    if !is_valid_tag_name(element.tag()) {
        return;
    }
    out.push('<');
    out.push_str(element.tag());
    write_attributes(element.attributes(), out);

    // A void element takes no content and no children — both are silently dropped, which
    // is what Winged-Swift does. `Element::self_closing` opts a non-void tag into the same
    // shape, so a caller can emit `<input />` without building a `Node::Raw` that would
    // skip the URL policy in `Attribute::write_into`.
    if is_void(element.tag()) || element.is_self_closing() {
        write_void_suffix(out, options);
        return;
    }

    out.push('>');
    if let Some(content) = element.content() {
        out.push_str(content);
    }
    stack.push(Step::Close(element.tag()));
    stack.extend(element.children().iter().rev().map(Step::Compact));
}

/// The same, indented, with the children queued one level deeper.
fn push_pretty_element<'a>(
    element: &'a Element,
    depth: usize,
    out: &mut String,
    options: &RenderOptions,
    stack: &mut Vec<Step<'a>>,
) {
    // Checked before the indent is written, so a dropped element leaves nothing behind for
    // `DropIfEmpty` to trip over.
    if !is_valid_tag_name(element.tag()) {
        return;
    }
    // `<pre>`, `<code>` and `<textarea>` render every whitespace character they contain, so
    // indenting their children would change what the browser shows.
    if is_whitespace_sensitive(element.tag()) {
        options.write_indent(out, depth);
        stack.push(Step::CompactElement(element));
        return;
    }

    options.write_indent(out, depth);
    out.push('<');
    out.push_str(element.tag());
    write_attributes(element.attributes(), out);

    if is_void(element.tag()) || element.is_self_closing() {
        write_void_suffix(out, options);
        return;
    }

    out.push('>');

    if element.children().is_empty() {
        if let Some(content) = element.content() {
            out.push_str(content);
        }
        out.push_str("</");
        out.push_str(element.tag());
        out.push('>');
        return;
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
}

fn write_comment(out: &mut String, content: &str) {
    out.push_str("<!-- ");
    // `Node::Comment` is a public variant, so the content may not have come through
    // `Node::comment`. Neutralising again is idempotent and keeps the guarantee in the
    // writer, where it cannot be bypassed.
    push_neutralised_comment(out, content);
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

fn write_attributes(attributes: &[Attribute], out: &mut String) {
    for attribute in attributes {
        attribute.write_into(out);
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

    /// No direct Swift counterpart: Winged-Swift turns escaping off per tag with
    /// `escapeContent:`, which `Element::raw_text` ports. This is the node-level version,
    /// which has no equivalent because Swift models raw markup as an `HTMLTag` subclass.
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

    /// `Node::Comment` is a public variant: building it directly must not bypass the guard.
    #[test]
    fn a_directly_built_comment_is_neutralised_by_the_writer() {
        let rendered = Node::Comment("x---><img onerror=1>".into()).render();
        assert_eq!(rendered.matches("-->").count(), 1);
        assert!(rendered.ends_with(" -->"));
    }

    /// Ports `FragmentTests.testEmptyFragmentDoesNotLeaveBlankLines`.
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

    /// Ports `FragmentTests.testFragmentRendersChildrenWithoutWrapper`.
    #[test]
    fn a_fragment_renders_its_children_without_a_wrapper() {
        let tree = Node::fragment([p().text("a").into(), p().text("b").into()]);
        assert_eq!(tree.render(), "<p>a</p><p>b</p>");
        assert_eq!(tree.render_pretty(), "<p>a</p>\n<p>b</p>");
    }

    /// Ports the void-element cases in `TagCatalogTests`.
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

    /// Ports `WhitespaceTests.testPreWithCodeChildIsNotIndented` and
    /// `WhitespaceTests.testPrettyMatchesCompactForWhitespaceSensitiveTags`. Indenting inside `<pre>` would change what the browser shows.
    #[test]
    fn whitespace_sensitive_tags_are_not_indented_inside() {
        let tree = div().child(pre().child(code().text("let page = html { }")));
        assert_eq!(
            tree.render_pretty(),
            "<div>\n  <pre><code>let page = html { }</code></pre>\n</div>"
        );
    }

    /// No Swift counterpart: its `PrettyPrintTests` never mixes content with children.
    #[test]
    fn content_goes_on_its_own_line_before_children() {
        let tree = div().text("lead").child(p().text("body"));
        assert_eq!(tree.render_pretty(), "<div>\n  lead\n  <p>body</p>\n</div>");
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

    // Ports the `RawHTML` and fragment half of `HTML14FeaturesTests`; the element half
    // lives in `crate::elements`.

    /// Ports `HTML14FeaturesTests.testRawHTMLRendersWithoutWrapper`.
    #[test]
    fn raw_markup_renders_with_no_wrapper_around_it() {
        let raw = Node::raw(r#"<span class="x">hi</span>"#);

        assert_eq!(raw.render(), r#"<span class="x">hi</span>"#);
        assert!(!raw.render().contains("<div"));
    }

    /// Ports `HTML14FeaturesTests.testRawHTMLAsChildHasNoWrapper`.
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

    /// Ports `HTML14FeaturesTests.testFragmentHelper`.
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

    /// Ports `HTML14FeaturesTests.testFragmentBuilderBuildArray`.
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

    // Ports `FragmentTests`. This is the suite that pins the empty-fragment behaviour the
    // pretty writer exists to preserve, so it is also the independent check on the
    // iterative rewrite.

    /// Ports `FragmentTests.testEmptyFragmentRendersNothing`.
    #[test]
    fn an_empty_fragment_renders_nothing_in_either_mode() {
        assert_eq!(Node::fragment([]).render(), "");
        assert_eq!(Node::fragment([]).render_pretty(), "");
    }

    /// Ports `FragmentTests.testFragmentKeepsPrettyIndentation`.
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

    /// Ports `FragmentTests.testFragmentBuilderSupportsMapAndFilter`.
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

    /// Ports `FragmentTests.testFalseConditionDoesNotEmitStrayHTMLNode`.
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

    /// Ports `FragmentTests.testTrueConditionEmitsTheBranch`.
    #[test]
    fn a_true_condition_emits_its_branch() {
        let show_banner = true;
        let page = html_tag().child(html! { @if show_banner { div { "banner" } } });

        assert_eq!(page.render(), "<html><div>banner</div></html>");
    }

    /// Ports `FragmentTests.testLoopInsideHTMLBuilder`.
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

    /// Ports `FragmentTests.testRawHTMLIsIndentedInsideAPrettyTree`.
    #[test]
    fn raw_markup_is_indented_as_one_blob() {
        let container = div().child(Node::raw("<custom-element></custom-element>"));

        assert_eq!(
            container.render_pretty(),
            "<div>\n  <custom-element></custom-element>\n</div>"
        );
    }

    /// Ports `FragmentTests.testFragmentBuilderTakesBothBranchesOfAnIf`.
    #[test]
    fn a_condition_can_pick_either_branch() {
        fn badge(is_beta: bool) -> Node {
            html! { @if is_beta { span { "beta" } } @else { span { "stable" } } }
        }

        assert_eq!(badge(true).render(), "<span>beta</span>");
        assert_eq!(badge(false).render(), "<span>stable</span>");
    }

    /// Ports `FragmentTests.testNestedFragmentsFlattenInPrettyOutput`.
    ///
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

    /// Ports `FragmentTests.testRawHTMLStillEmitsMarkupVerbatim`.
    #[test]
    fn raw_markup_is_emitted_verbatim() {
        let raw = Node::raw(r#"<custom-element data-x="1"></custom-element>"#);

        assert_eq!(
            raw.render(),
            r#"<custom-element data-x="1"></custom-element>"#
        );
    }

    /// Ports `PrettyPrintTests.testPrettyPrintSimpleTag`.
    #[test]
    fn a_tag_with_only_content_stays_on_one_line_when_pretty() {
        assert_eq!(
            div().text("Hello World").render_pretty(),
            "<div>Hello World</div>"
        );
    }

    /// Ports `PrettyPrintTests.testPrettyPrintWithChildren`.
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

    /// Ports `PrettyPrintTests.testCompactRenderStillWorks`.
    #[test]
    fn compact_keeps_everything_on_one_line() {
        assert_eq!(
            div().child(p().text("Test")).render(),
            "<div><p>Test</p></div>"
        );
    }

    /// Ports `PrettyPrintTests.testSelfClosingTagPrettyPrint`.
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

    /// Ports `WhitespaceTests.testTextareaKeepsItsContentIntact`.
    #[test]
    fn a_textarea_keeps_its_newlines() {
        let field = textarea().attr("name", "bio").text("line 1\nline 2");

        assert_eq!(
            field.render_pretty(),
            "<textarea name=\"bio\">line 1\nline 2</textarea>"
        );
    }

    /// Ports `WhitespaceTests.testNestedInsideAPrettyDocumentKeepsOuterIndentation`.
    #[test]
    fn a_pre_block_is_indented_from_the_outside_but_not_within() {
        let container = div().child(pre().child(code().text("swift build")));

        assert_eq!(
            container.render_pretty(),
            "<div>\n  <pre><code>swift build</code></pre>\n</div>"
        );
    }
}
