//! Whole-page assembly.
//!
//! Ports `Winged-Swift/Sources/WingedSwift/core/Document.swift`.

use std::io::{self, Write};

use crate::core::node::write_document_root;
use crate::core::{Attribute, Element, Node, Render, RenderOptions};
use crate::elements::{body, head, html_tag};

/// A complete HTML document: a language, a `<head>` and a `<body>`.
///
/// The `Document` owns the doctype — [`Document::render`] prepends `<!DOCTYPE html>\n`,
/// while [`Document::root`] gives you the bare `<html>` element without it.
///
/// # Rendering default
///
/// **`Document::render` defaults to pretty, while [`Element::render`] defaults to
/// compact.** That asymmetry is in the Swift original and the golden fixtures depend on it,
/// so it is preserved rather than tidied up.
///
/// # Examples
/// ```
/// use molt::prelude::*;
/// use molt::Document;
///
/// let page = Document::new(Some("pt-BR"))
///     .head_children([title().text("RideKeeper")])
///     .body_children([h1().text("Track every service")]);
///
/// assert!(page.render().starts_with("<!DOCTYPE html>\n<html lang=\"pt-BR\">"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Document {
    /// The document language, rendered as `<html lang="…">`. Omitted when `None`.
    pub lang: Option<String>,
    /// The `<head>` element.
    pub head: Element,
    /// The `<body>` element.
    pub body: Element,
}

impl Default for Document {
    /// An empty document with no `lang`, same as `Document::new(None)`.
    fn default() -> Self {
        Self::new(None)
    }
}

impl std::fmt::Display for Document {
    /// The pretty render with doctype, same as [`Document::render`].
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.render())
    }
}

impl Document {
    /// Creates an empty document with the given language.
    #[must_use]
    pub fn new(lang: Option<&str>) -> Self {
        Self {
            lang: lang.map(ToString::to_string),
            head: head(),
            body: body(),
        }
    }

    /// Creates a document from an existing `<head>` and `<body>`.
    #[must_use]
    pub fn with_parts(lang: Option<&str>, head: Element, body: Element) -> Self {
        Self {
            lang: lang.map(ToString::to_string),
            head,
            body,
        }
    }

    /// Appends nodes to the `<head>`.
    #[must_use]
    pub fn head_children<N: Into<Node>>(mut self, children: impl IntoIterator<Item = N>) -> Self {
        self.head = self.head.children_from(children);
        self
    }

    /// Appends nodes to the `<body>`.
    #[must_use]
    pub fn body_children<N: Into<Node>>(mut self, children: impl IntoIterator<Item = N>) -> Self {
        self.body = self.body.children_from(children);
        self
    }

    /// The `<html>` element, **without** the doctype.
    ///
    /// Kept separate from [`render`](Self::render) because the static site generator and
    /// several tests want the tree rather than the serialized page.
    #[must_use]
    pub fn root(&self) -> Element {
        let root = match &self.lang {
            Some(lang) => html_tag().attr("lang", lang),
            None => html_tag(),
        };
        root.child(self.head.clone()).child(self.body.clone())
    }

    /// Renders the document with the given options, prefixed by the doctype.
    #[must_use]
    pub fn render_with(&self, options: &RenderOptions) -> String {
        let mut out = String::with_capacity(2048);
        out.push_str("<!DOCTYPE html>\n");
        // Not `self.root().write_into(..)`: that clones head and body, and `Clone` is
        // recursive, so a deep body aborted the process before anything was written.
        write_document_root(
            self.lang.as_deref(),
            &self.head,
            &self.body,
            &mut out,
            options,
        );
        out
    }

    /// Renders the document **pretty-printed** — the Swift default for a document.
    #[must_use]
    pub fn render(&self) -> String {
        self.render_with(&RenderOptions::pretty())
    }

    /// Renders the document on a single line.
    #[must_use]
    pub fn render_compact(&self) -> String {
        self.render_with(&RenderOptions::compact())
    }

    /// Writes the same bytes as [`render_with`](Self::render_with) to `sink`.
    ///
    /// When `<body>` is only a list of children, each child is rendered with
    /// [`Render::write_into`] into its own buffer and then written out, so memory stays
    /// bounded by the largest section rather than the whole page. A body with text
    /// content, no children, or a self-closing flag is written in one piece — still the
    /// same bytes.
    ///
    /// # Errors
    /// Returns an error if `sink` does.
    ///
    /// # Examples
    /// ```
    /// use molt::prelude::*;
    /// use molt::{Document, RenderOptions};
    /// let page = Document::new(Some("en")).body_children([h1().text("Hi")]);
    /// let mut buf = Vec::new();
    /// page.render_to(&mut buf, &RenderOptions::compact())?;
    /// assert_eq!(buf, page.render_compact().into_bytes());
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn render_to(&self, mut sink: impl Write, options: &RenderOptions) -> io::Result<()> {
        if !streams_by_section(&self.body) {
            return sink.write_all(self.render_with(options).as_bytes());
        }
        sink.write_all(self.html_open().as_bytes())?;
        if options.pretty {
            self.write_pretty_sections(&mut sink, options)
        } else {
            self.write_compact_sections(&mut sink, options)
        }
    }

    fn html_open(&self) -> String {
        let mut open = String::from("<!DOCTYPE html>\n<html");
        if let Some(lang) = &self.lang {
            Attribute::new_static("lang", lang).write_into(&mut open);
        }
        open.push('>');
        open
    }

    fn write_pretty_sections(
        &self,
        sink: &mut impl Write,
        options: &RenderOptions,
    ) -> io::Result<()> {
        let mut head_markup = String::from("\n");
        self.head.write_into(&mut head_markup, options, 1);
        sink.write_all(head_markup.as_bytes())?;

        let mut body_open = String::from("\n");
        options.write_indent(&mut body_open, 1);
        write_open_tag(&self.body, &mut body_open);
        sink.write_all(body_open.as_bytes())?;

        for child in self.body.children() {
            let mut rendered = String::new();
            child.write_into(&mut rendered, options, 2);
            if !rendered.is_empty() {
                sink.write_all(b"\n")?;
                sink.write_all(rendered.as_bytes())?;
            }
        }

        let mut close = String::from("\n");
        options.write_indent(&mut close, 1);
        close.push_str("</body>\n</html>");
        sink.write_all(close.as_bytes())
    }

    fn write_compact_sections(
        &self,
        sink: &mut impl Write,
        options: &RenderOptions,
    ) -> io::Result<()> {
        let mut head_markup = String::new();
        self.head.write_into(&mut head_markup, options, 0);
        sink.write_all(head_markup.as_bytes())?;

        let mut body_open = String::new();
        write_open_tag(&self.body, &mut body_open);
        sink.write_all(body_open.as_bytes())?;

        for child in self.body.children() {
            let mut rendered = String::new();
            child.write_into(&mut rendered, options, 0);
            sink.write_all(rendered.as_bytes())?;
        }
        sink.write_all(b"</body></html>")
    }
}

/// A body made of sections can be written one child at a time. Anything else goes through
/// [`Document::render_with`] so the bytes cannot drift.
fn streams_by_section(body: &Element) -> bool {
    body.tag() == "body"
        && body.content().is_none()
        && !body.children().is_empty()
        && !body.is_self_closing()
}

fn write_open_tag(element: &Element, out: &mut String) {
    out.push('<');
    out.push_str(element.tag());
    for attribute in element.attributes() {
        attribute.write_into(out);
    }
    out.push('>');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Node;
    use crate::elements::{body, h1, h2, head, img, meta, p, section, title};

    /// Ports `DocumentTests.rendersDoctypeAndLanguage`.
    /// The borrowed-parts writer must produce what rendering a cloned `<html>` tree did.
    #[test]
    fn rendering_matches_the_cloned_root_tree_byte_for_byte() {
        use crate::elements::{div, p, pre, title};
        let documents = [
            Document::new(None),
            Document::new(Some("pt-BR")).head_children([title().text("T")]),
            Document::new(Some("a\"b")).body_children([div().child(p().text("x")), div()]),
            Document::new(Some("en")).body_children([Node::fragment([]), Node::text("")]),
            Document::with_parts(Some("en"), head(), pre().text("  keep  ")),
            Document::with_parts(None, Element::new("bad tag"), body().text("t")),
            Document::with_parts(None, head(), body().self_closing(true)),
        ];
        for document in &documents {
            for options in [
                RenderOptions::pretty(),
                RenderOptions::compact(),
                RenderOptions::pretty().with_xhtml_self_closing(true),
                RenderOptions::compact().with_indent("\t"),
            ] {
                let mut expected = String::from("<!DOCTYPE html>\n");
                document.root().write_into(&mut expected, &options, 0);
                assert_eq!(document.render_with(&options), expected, "{document:?}");
            }
        }
    }

    #[test]
    fn a_document_owns_the_doctype_and_the_lang_attribute() {
        let page = Document::new(Some("pt-BR"));
        assert!(
            page.render()
                .starts_with("<!DOCTYPE html>\n<html lang=\"pt-BR\">")
        );
    }

    /// Ports `DocumentTests.languageIsOptional`.
    #[test]
    fn the_lang_attribute_is_omitted_when_absent() {
        let page = Document::new(None);
        assert!(page.render().starts_with("<!DOCTYPE html>\n<html>"));
    }

    /// Ports `DocumentTests.rootExposesTheHTMLElementWithoutTheDoctype`.
    #[test]
    fn root_returns_the_tree_without_the_doctype() {
        let page = Document::new(Some("en"));
        assert!(!page.root().render().contains("DOCTYPE"));
        assert!(page.root().render().starts_with("<html lang=\"en\">"));
    }

    /// Ports `DocumentTests.prettyIsTheDefaultForDocuments`. The asymmetry with `Element::render`
    /// is deliberate — see the type docs.
    #[test]
    fn a_document_renders_pretty_by_default_unlike_an_element() {
        let page = Document::new(Some("en")).head_children([title().text("T")]);
        assert!(page.render().contains('\n'));
        assert!(
            !page
                .render_compact()
                .trim_start_matches("<!DOCTYPE html>\n")
                .contains('\n')
        );
    }

    /// Ports `DocumentTests.acceptsAnExistingHeadAndBody`.
    #[test]
    fn head_and_body_children_land_in_the_right_place() {
        let page = Document::new(Some("en"))
            .head_children([title().text("T")])
            .body_children([h1().text("H")]);
        let rendered = page.render();
        let head_at = rendered.find("<title>").expect("title rendered");
        let body_at = rendered.find("<h1>").expect("h1 rendered");
        assert!(head_at < body_at);
    }

    /// Ports `DocumentTests.documentIsAValue`.
    #[test]
    fn a_document_is_a_value_and_can_be_shared_across_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Document>();

        let page = Document::new(Some("en"));
        let copy = page.clone().head_children([title().text("changed")]);
        assert!(!page.render().contains("changed"));
        assert!(copy.render().contains("changed"));
    }

    /// Ports `DocumentTests.buildersSupportLoopsAndConditions`.
    #[test]
    fn body_children_accept_loops_and_conditions() {
        let show_banner = false;
        let page = Document::new(None)
            .head_children([title().text("T")])
            .body_children([crate::html! {
                @if show_banner { div { "banner" } }
                @for index in 1..=2 { p { "line " (index) } }
            }]);

        let rendered = page.render_compact();
        assert!(!rendered.contains("banner"));
        assert!(rendered.contains("<p>line 1</p><p>line 2</p>"));
    }

    /// Ports `ThreeZeroTests.fileMatchesRender` and
    /// `ThreeZeroTests.bodiesThatCannotStreamStillMatch`.
    #[test]
    fn render_to_matches_render_with() {
        let options = [
            RenderOptions::pretty(),
            RenderOptions::compact(),
            RenderOptions::pretty()
                .with_indent("\t")
                .with_xhtml_self_closing(true),
        ];
        let documents = [
            streaming_page(3, false),
            streaming_page(400, false),
            streaming_page(2, true),
            Document::with_parts(None, head(), body()),
            Document::with_parts(Some("en"), head(), body().raw_text("<raw>")),
        ];
        for page in &documents {
            for options in &options {
                let mut written = Vec::new();
                page.render_to(&mut written, options).expect("write");
                assert_eq!(written, page.render_with(options).into_bytes());
            }
        }
    }

    fn streaming_page(sections: usize, empty: bool) -> Document {
        let mut nodes = Vec::new();
        if empty {
            nodes.push(Node::fragment([]));
        }
        for index in 0..sections {
            nodes.push(
                section()
                    .add_class("s")
                    .child(h2().text(format!("Section {index}")))
                    .child(p().text("text & more ".repeat(50)))
                    .child(img().attr("src", format!("/{index}.png")).attr("alt", ""))
                    .into(),
            );
        }
        if empty {
            nodes.push(Node::fragment([]));
        }
        Document::new(Some("en"))
            .head_children([title().text("Streaming"), meta().attr("charset", "UTF-8")])
            .body_children(nodes)
    }
}
