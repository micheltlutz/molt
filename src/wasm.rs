//! WebAssembly bindings.
//!
//! # Why this does not follow `MOLT_SPEC.md` §4.1
//!
//! The spec's `WasmDocument` holds `body_nodes: Vec<String>`: it renders each node to a
//! string on insertion and concatenates at the end. That throws the tree away, so
//! `render_pretty()` becomes impossible, nothing can be modified after insertion, and the
//! API is limited to the two element types the spec hard-codes.
//!
//! These bindings wrap the real [`Element`] and [`Document`] behind opaque handles instead.
//! JavaScript therefore drives the *same* renderer as Rust does, which is what lets
//! `tests/wasm.rs` build one document through both APIs and compare the bytes. If native and
//! WASM ever diverge, that comparison catches it.

use wasm_bindgen::prelude::*;

use crate::core::{Element, Node, Render, RenderOptions};
use crate::document::Document;
use crate::seo::SeoBuilder;

/// An HTML element, usable from JavaScript.
///
/// The builder methods consume and return the handle so they chain in JS exactly as they
/// do in Rust:
///
/// ```js
/// const card = wElement("div").addClass("card").child(wElement("h1").text("Hi"));
/// card.render();  // '<div class="card"><h1>Hi</h1></div>'
/// ```
#[wasm_bindgen(js_name = WElement)]
#[derive(Debug, Clone)]
pub struct WElement(Element);

#[wasm_bindgen(js_class = WElement)]
impl WElement {
    /// Creates an element with the given tag name.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(tag: &str) -> Self {
        Self(Element::new(tag))
    }

    /// Sets `id`, replacing any existing one.
    #[wasm_bindgen(js_name = setId)]
    #[must_use]
    pub fn set_id(self, id: &str) -> Self {
        Self(self.0.set_id(id))
    }

    /// Appends a class name.
    #[wasm_bindgen(js_name = addClass)]
    #[must_use]
    pub fn add_class(self, class_name: &str) -> Self {
        Self(self.0.add_class(class_name))
    }

    /// Sets `style`, replacing any existing one.
    #[wasm_bindgen(js_name = setStyle)]
    #[must_use]
    pub fn set_style(self, style: &str) -> Self {
        Self(self.0.set_style(style))
    }

    /// Appends an attribute, escaping the value.
    ///
    /// URL attributes go through the same policy as [`Element::attr`]: a script URL renders
    /// as `#`.
    #[must_use]
    pub fn attr(self, key: &str, value: &str) -> Self {
        Self(self.0.attr(key, value))
    }

    /// Replaces `key` with a URL written as given, even if it is a script URL.
    ///
    /// The value is still attribute-escaped. Use this only for a URL the page author wrote.
    #[wasm_bindgen(js_name = trustedUrl)]
    #[must_use]
    pub fn trusted_url(self, key: &str, value: &str) -> Self {
        Self(self.0.trusted_url(key, value))
    }

    /// Appends a boolean attribute, which renders as a bare key.
    #[wasm_bindgen(js_name = boolAttr)]
    #[must_use]
    pub fn bool_attr(self, key: &str) -> Self {
        Self(self.0.bool_attr(key))
    }

    /// Appends `data-{key}="{value}"`.
    #[wasm_bindgen(js_name = dataAttr)]
    #[must_use]
    pub fn data_attr(self, key: &str, value: &str) -> Self {
        Self(self.0.data_attr(key, value))
    }

    /// Appends `aria-{key}="{value}"`.
    #[wasm_bindgen(js_name = ariaAttr)]
    #[must_use]
    pub fn aria_attr(self, key: &str, value: &str) -> Self {
        Self(self.0.aria_attr(key, value))
    }

    /// Sets the text content, escaping it.
    #[must_use]
    pub fn text(self, content: &str) -> Self {
        Self(self.0.text(content))
    }

    /// Sets the content **without** escaping it.
    #[wasm_bindgen(js_name = rawText)]
    #[must_use]
    pub fn raw_text(self, content: &str) -> Self {
        Self(self.0.raw_text(content))
    }

    /// Appends a child element.
    #[must_use]
    pub fn child(self, child: &WElement) -> Self {
        Self(self.0.child(child.0.clone()))
    }

    /// Appends an HTML comment as a child.
    #[must_use]
    pub fn comment(self, content: &str) -> Self {
        Self(self.0.child(Node::comment(content)))
    }

    /// Renders on a single line.
    #[must_use]
    pub fn render(&self) -> String {
        self.0.render()
    }

    /// Renders with indentation.
    #[wasm_bindgen(js_name = renderPretty)]
    #[must_use]
    pub fn render_pretty(&self) -> String {
        self.0.render_pretty()
    }
}

/// A complete HTML document, usable from JavaScript.
#[wasm_bindgen(js_name = WDocument)]
#[derive(Debug, Clone)]
pub struct WDocument(Document);

#[wasm_bindgen(js_class = WDocument)]
impl WDocument {
    /// Creates a document. Pass `null` for `lang` to omit the attribute.
    // `Option<String>` rather than `Option<&str>`: wasm-bindgen cannot marshal an optional
    // borrowed string across the boundary, so the owned form is the only one that compiles.
    #[allow(clippy::needless_pass_by_value)]
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(lang: Option<String>) -> Self {
        Self(Document::new(lang.as_deref()))
    }

    /// Appends an element to the `<head>`.
    #[wasm_bindgen(js_name = addHead)]
    #[must_use]
    pub fn add_head(self, element: &WElement) -> Self {
        Self(self.0.head_children([element.0.clone()]))
    }

    /// Appends an element to the `<body>`.
    #[wasm_bindgen(js_name = addBody)]
    #[must_use]
    pub fn add_body(self, element: &WElement) -> Self {
        Self(self.0.body_children([element.0.clone()]))
    }

    /// Renders the document, pretty-printed and prefixed by the doctype.
    #[must_use]
    pub fn render(&self) -> String {
        self.0.render()
    }

    /// Renders the document on a single line.
    #[wasm_bindgen(js_name = renderCompact)]
    #[must_use]
    pub fn render_compact(&self) -> String {
        self.0.render_compact()
    }

    /// Renders with a custom indent string.
    #[wasm_bindgen(js_name = renderWithIndent)]
    #[must_use]
    pub fn render_with_indent(&self, indent: &str) -> String {
        self.0
            .render_with(&RenderOptions::pretty().with_indent(indent))
    }
}

/// A page's SEO metadata block, usable from JavaScript.
#[wasm_bindgen(js_name = WSeo)]
#[derive(Debug, Clone)]
pub struct WSeo(SeoBuilder);

#[wasm_bindgen(js_class = WSeo)]
impl WSeo {
    /// Starts a metadata block.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(title: &str, description: &str) -> Self {
        Self(SeoBuilder::new(title, description))
    }

    /// Sets the preview image.
    #[must_use]
    pub fn image(self, image_url: &str) -> Self {
        Self(self.0.image(image_url))
    }

    /// Sets the canonical URL.
    #[must_use]
    pub fn url(self, page_url: &str) -> Self {
        Self(self.0.url(page_url))
    }

    /// Sets `twitter:site`.
    #[wasm_bindgen(js_name = twitterSite)]
    #[must_use]
    pub fn twitter_site(self, site: &str) -> Self {
        Self(self.0.twitter_site(site))
    }

    /// Renders the whole metadata block as markup.
    #[must_use]
    pub fn render(&self) -> String {
        self.0.build().iter().map(Render::render).collect()
    }
}

/// Creates an element. The idiomatic JS entry point — `element("div")` reads better than
/// `new WElement("div")`.
#[wasm_bindgen]
#[must_use]
pub fn element(tag: &str) -> WElement {
    WElement::new(tag)
}

/// Escapes text for HTML content, exposed for callers doing their own assembly.
#[wasm_bindgen(js_name = escapeText)]
#[must_use]
pub fn escape_text(input: &str) -> String {
    crate::core::escape::escape_text(input)
}

/// The crate version, so a page can report which build produced it.
#[wasm_bindgen]
#[must_use]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trusted_url_skips_the_script_url_check() {
        assert_eq!(
            WElement::new("a")
                .trusted_url("href", "javascript:print()")
                .render(),
            "<a href=\"javascript:print()\"></a>"
        );
        assert_eq!(
            WElement::new("a").attr("href", "javascript:x").render(),
            "<a href=\"#\"></a>"
        );
    }
}
