//! SEO metadata: Open Graph, Twitter Cards and the common `<meta>` block.
//!
//! The free functions here build the tags; [`SeoBuilder`] is sugar over them, never a second
//! implementation.

use crate::core::escape::escape_json_script;
use crate::core::{Attribute, Element};
use crate::elements::{link, meta, script, title};

/// A `<script type="application/ld+json">` carrying structured data for search engines.
///
/// `json` is a JSON document, most often built from page fields that include user text. It
/// is made safe to embed: `<`, `>`, `&`, U+2028 and U+2029 are written as JSON escapes, so a
/// value containing `</script>` cannot end the element. It is **not validated**; pass valid
/// JSON, from a serializer rather than string concatenation.
///
/// Before this helper the only way was `script().raw_text(json)`, which writes `json` as it is
/// and lets a `</script>` in any field break out of the element.
///
/// # Examples
/// ```
/// use molt::prelude::*;
/// use molt::seo::json_ld;
///
/// let tag = json_ld(r#"{"@type":"Article","headline":"</script><script>alert(1)</script>"}"#);
/// assert_eq!(
///     tag.render(),
///     r#"<script type="application/ld+json">{"@type":"Article","headline":"\u003c/script\u003e\u003cscript\u003ealert(1)\u003c/script\u003e"}</script>"#
/// );
/// ```
#[must_use]
pub fn json_ld(json: &str) -> Element {
    script_json("application/ld+json", json)
}

/// A `<script type="…">` holding a JSON document, made safe to embed. See [`json_ld`].
///
/// Use it for other data blocks a page reads with `JSON.parse`, such as
/// `application/json`. `mime` is attribute-escaped. Do not use it for `type="module"` or for
/// classic scripts: that content is code, not data, and no escaping makes arbitrary code safe.
///
/// # Examples
/// ```
/// use molt::prelude::*;
/// use molt::seo::script_json;
/// assert_eq!(
///     script_json("application/json", r#"{"id":7}"#).render(),
///     r#"<script type="application/json">{"id":7}</script>"#
/// );
/// ```
#[must_use]
pub fn script_json(mime: &str, json: &str) -> Element {
    // `raw_text` because `<script>` content must not be HTML-escaped, which would write `&lt;`
    // that JSON does not decode. It is safe here: `escape_json_script` has removed every `<`.
    script()
        .attr("type", mime)
        .raw_text(escape_json_script(json).into_owned())
}

/// A `<meta name="…" content="…">`.
///
/// Both `name` and `content` are attribute-escaped. The name is usually a literal, but these
/// functions are `pub` and take `&str`, so a name built from user input (`og:` plus a field, say) would break out of
/// the quotes. Every legitimate name (`description`, `og:title`) is unchanged by escaping.
#[must_use]
pub fn meta_name(name: &str, content: impl AsRef<str>) -> Element {
    meta()
        .add_attribute(Attribute::new_static("name", name))
        .attr("content", content)
}

/// A `<meta property="…" content="…">`, used by Open Graph.
#[must_use]
pub fn meta_property(property: &str, content: impl AsRef<str>) -> Element {
    meta()
        .add_attribute(Attribute::new_static("property", property))
        .attr("content", content)
}

/// A `<meta charset="…">`.
#[must_use]
pub fn meta_charset(charset: &str) -> Element {
    meta().add_attribute(Attribute::new_static("charset", charset))
}

/// A `<meta http-equiv="…" content="…">`.
#[must_use]
pub fn meta_http_equiv(http_equiv: &str, content: impl AsRef<str>) -> Element {
    meta()
        .add_attribute(Attribute::new_static("http-equiv", http_equiv))
        .attr("content", content)
}

/// Open Graph tags for a page.
///
/// Emits `og:title`, `og:description`, `og:image`, `og:url`, `og:type` and, when supplied,
/// `og:site_name` — in that order.
#[must_use]
pub fn open_graph(
    page_title: &str,
    description: &str,
    image: &str,
    url: &str,
    og_type: &str,
    site_name: Option<&str>,
) -> Vec<Element> {
    let mut tags = vec![
        meta_property("og:title", page_title),
        meta_property("og:description", description),
        meta_property("og:image", image),
        meta_property("og:url", url),
        meta_property("og:type", og_type),
    ];
    if let Some(site_name) = site_name {
        tags.push(meta_property("og:site_name", site_name));
    }
    tags
}

/// Open Graph tags for an article, with the `article:*` extensions.
#[must_use]
pub fn open_graph_article(
    page_title: &str,
    description: &str,
    image: &str,
    url: &str,
    author: Option<&str>,
    published_time: Option<&str>,
    modified_time: Option<&str>,
) -> Vec<Element> {
    let mut tags = open_graph(page_title, description, image, url, "article", None);
    if let Some(author) = author {
        tags.push(meta_property("article:author", author));
    }
    if let Some(published) = published_time {
        tags.push(meta_property("article:published_time", published));
    }
    if let Some(modified) = modified_time {
        tags.push(meta_property("article:modified_time", modified));
    }
    tags
}

/// Twitter Card tags.
#[must_use]
pub fn twitter_card(
    page_title: &str,
    description: &str,
    image: &str,
    card: &str,
    site: Option<&str>,
    creator: Option<&str>,
) -> Vec<Element> {
    let mut tags = vec![
        meta_name("twitter:card", card),
        meta_name("twitter:title", page_title),
        meta_name("twitter:description", description),
        meta_name("twitter:image", image),
    ];
    if let Some(site) = site {
        tags.push(meta_name("twitter:site", site));
    }
    if let Some(creator) = creator {
        tags.push(meta_name("twitter:creator", creator));
    }
    tags
}

/// The common `<meta>` block: charset, viewport, description, robots, keywords, author —
/// plus the `<title>`.
///
/// # A bug fixed in the port
///
/// Emits a `<title>` element from `title`, followed by the charset, viewport, description and
/// robots tags.
#[must_use]
pub fn common(
    page_title: &str,
    description: &str,
    keywords: Option<&[&str]>,
    author: Option<&str>,
    viewport: &str,
    robots: &str,
) -> Vec<Element> {
    let mut tags = vec![
        meta_charset("UTF-8"),
        meta_name("viewport", viewport),
        meta_name("description", description),
        meta_name("robots", robots),
    ];
    // An empty list is the same as no list: `content=""` says nothing, so the tag
    // is omitted. The guard lives here rather than in `SeoBuilder` so both entry points
    // agree.
    if let Some(keywords) = keywords.filter(|list| !list.is_empty()) {
        tags.push(meta_name("keywords", keywords.join(", ")));
    }
    if let Some(author) = author {
        tags.push(meta_name("author", author));
    }
    tags.push(title().text(page_title));
    tags
}

/// The default viewport.
pub const DEFAULT_VIEWPORT: &str = "width=device-width, initial-scale=1.0";
/// The default robots directive.
pub const DEFAULT_ROBOTS: &str = "index, follow";
/// The default Twitter card type.
pub const DEFAULT_TWITTER_CARD: &str = "summary_large_image";

/// A fluent builder for a page's whole metadata block.
///
/// Implements `docs/history/MOLT_SPEC.md` §6 on top of the functions above.
///
/// # Examples
/// ```
/// use molt::prelude::*;
/// use molt::seo::SeoBuilder;
///
/// let tags = SeoBuilder::new("RideKeeper", "Motorcycle maintenance companion")
///     .image("https://ridekeeper.example/og.jpg")
///     .url("https://ridekeeper.example")
///     .twitter_site("@micheltlutz")
///     .build();
///
/// let rendered: String = tags.iter().map(Render::render).collect();
/// assert!(rendered.contains(r#"<meta property="og:title" content="RideKeeper">"#));
/// ```
#[derive(Debug, Clone)]
pub struct SeoBuilder {
    title: String,
    description: String,
    image: Option<String>,
    url: Option<String>,
    site_name: Option<String>,
    author: Option<String>,
    keywords: Vec<String>,
    twitter_card: String,
    twitter_site: Option<String>,
    twitter_creator: Option<String>,
    viewport: String,
    robots: String,
    canonical: Option<String>,
    alternates: Vec<(String, String)>,
    locale: Option<String>,
    locale_alternates: Vec<String>,
    image_size: Option<(u32, u32)>,
    image_alt: Option<String>,
    theme_color: Option<String>,
    color_scheme: Option<String>,
}

impl SeoBuilder {
    /// Starts a metadata block from the two fields every page needs.
    pub fn new(page_title: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            title: page_title.into(),
            description: description.into(),
            image: None,
            url: None,
            site_name: None,
            author: None,
            keywords: Vec::new(),
            twitter_card: DEFAULT_TWITTER_CARD.to_string(),
            twitter_site: None,
            twitter_creator: None,
            viewport: DEFAULT_VIEWPORT.to_string(),
            robots: DEFAULT_ROBOTS.to_string(),
            canonical: None,
            alternates: Vec::new(),
            locale: None,
            locale_alternates: Vec::new(),
            image_size: None,
            image_alt: None,
            theme_color: None,
            color_scheme: None,
        }
    }

    /// Sets the preview image used by both Open Graph and Twitter.
    #[must_use]
    pub fn image(mut self, image_url: impl Into<String>) -> Self {
        self.image = Some(image_url.into());
        self
    }

    /// Sets the page URL, written as `og:url`.
    ///
    /// This is not `<link rel="canonical">`; for that, see [`SeoBuilder::canonical`].
    #[must_use]
    pub fn url(mut self, page_url: impl Into<String>) -> Self {
        self.url = Some(page_url.into());
        self
    }

    /// Sets `og:site_name`.
    #[must_use]
    pub fn site_name(mut self, site_name: impl Into<String>) -> Self {
        self.site_name = Some(site_name.into());
        self
    }

    /// Sets the page author.
    #[must_use]
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Sets the keyword list.
    #[must_use]
    pub fn keywords<S: Into<String>>(mut self, keywords: impl IntoIterator<Item = S>) -> Self {
        self.keywords = keywords.into_iter().map(Into::into).collect();
        self
    }

    /// Overrides the Twitter card type.
    #[must_use]
    pub fn twitter_card(mut self, card: impl Into<String>) -> Self {
        self.twitter_card = card.into();
        self
    }

    /// Sets `twitter:site`.
    #[must_use]
    pub fn twitter_site(mut self, site: impl Into<String>) -> Self {
        self.twitter_site = Some(site.into());
        self
    }

    /// Sets `twitter:creator`.
    #[must_use]
    pub fn twitter_creator(mut self, creator: impl Into<String>) -> Self {
        self.twitter_creator = Some(creator.into());
        self
    }

    /// Overrides the `robots` directive (default [`DEFAULT_ROBOTS`]).
    ///
    /// # Examples
    /// ```
    /// use molt::prelude::*;
    /// use molt::seo::SeoBuilder;
    /// let tags = SeoBuilder::new("T", "D")
    ///     .robots("index, follow, max-image-preview:large")
    ///     .build();
    /// let html: String = tags.iter().map(Render::render).collect();
    /// assert!(html.contains(r#"<meta name="robots" content="index, follow, max-image-preview:large">"#));
    /// ```
    #[must_use]
    pub fn robots(mut self, robots: impl Into<String>) -> Self {
        self.robots = robots.into();
        self
    }

    /// Overrides the `viewport` directive (default [`DEFAULT_VIEWPORT`]).
    #[must_use]
    pub fn viewport(mut self, viewport: impl Into<String>) -> Self {
        self.viewport = viewport.into();
        self
    }

    /// Adds `<link rel="canonical" href="…">`. Omitted unless called.
    ///
    /// The `href` goes through the URL policy, like any `href`.
    ///
    /// # Examples
    /// ```
    /// use molt::prelude::*;
    /// use molt::seo::SeoBuilder;
    /// let tags = SeoBuilder::new("T", "D").canonical("https://example.com/en/").build();
    /// let html: String = tags.iter().map(Render::render).collect();
    /// assert!(html.contains(r#"<link rel="canonical" href="https://example.com/en/">"#));
    /// ```
    #[must_use]
    pub fn canonical(mut self, url: impl Into<String>) -> Self {
        self.canonical = Some(url.into());
        self
    }

    /// Adds `<link rel="alternate" hreflang="…" href="…">`, once per call, in call order.
    ///
    /// `hreflang` is a language tag or `x-default`.
    ///
    /// # Examples
    /// ```
    /// use molt::prelude::*;
    /// use molt::seo::SeoBuilder;
    /// let tags = SeoBuilder::new("T", "D")
    ///     .alternate("en", "https://example.com/en/")
    ///     .alternate("x-default", "https://example.com/")
    ///     .build();
    /// let html: String = tags.iter().map(Render::render).collect();
    /// assert!(html.contains(
    ///     r#"<link rel="alternate" hreflang="x-default" href="https://example.com/">"#
    /// ));
    /// ```
    #[must_use]
    pub fn alternate(mut self, hreflang: impl Into<String>, url: impl Into<String>) -> Self {
        self.alternates.push((hreflang.into(), url.into()));
        self
    }

    /// Sets `og:locale`, such as `pt_BR`.
    #[must_use]
    pub fn locale(mut self, locale: impl Into<String>) -> Self {
        self.locale = Some(locale.into());
        self
    }

    /// Adds an `og:locale:alternate`, once per call, in call order.
    #[must_use]
    pub fn locale_alternate(mut self, locale: impl Into<String>) -> Self {
        self.locale_alternates.push(locale.into());
        self
    }

    /// Sets `og:image:width` and `og:image:height`, in pixels.
    #[must_use]
    pub fn image_size(mut self, width: u32, height: u32) -> Self {
        self.image_size = Some((width, height));
        self
    }

    /// Sets `og:image:alt` and `twitter:image:alt`.
    #[must_use]
    pub fn image_alt(mut self, alt: impl Into<String>) -> Self {
        self.image_alt = Some(alt.into());
        self
    }

    /// Adds `<meta name="theme-color">`. Omitted unless called.
    #[must_use]
    pub fn theme_color(mut self, color: impl Into<String>) -> Self {
        self.theme_color = Some(color.into());
        self
    }

    /// Adds `<meta name="color-scheme">`, such as `dark light`. Omitted unless called.
    #[must_use]
    pub fn color_scheme(mut self, scheme: impl Into<String>) -> Self {
        self.color_scheme = Some(scheme.into());
        self
    }

    /// Builds the tags: the common block, then Open Graph, then Twitter Cards.
    ///
    /// The order is fixed, and the golden fixture depends on it. A tag that needs a setter
    /// (`canonical`, `alternate`, `theme_color`, `color_scheme`, the `og:locale` and
    /// `og:image:*` tags, `twitter:image:alt`) is written only when that setter was called, so
    /// a builder that never calls them produces what it always did:
    ///
    /// 1. the common block, ending in `<title>`;
    /// 2. `canonical`, the `alternate` links, `theme-color`, `color-scheme`;
    /// 3. Open Graph, then `og:image:width`, `og:image:height`, `og:image:alt`, `og:locale`,
    ///    the `og:locale:alternate` tags;
    /// 4. Twitter Cards, then `twitter:image:alt`.
    #[must_use]
    pub fn build(&self) -> Vec<Element> {
        let keywords: Vec<&str> = self.keywords.iter().map(String::as_str).collect();
        let mut tags = common(
            &self.title,
            &self.description,
            Some(&keywords[..]),
            self.author.as_deref(),
            &self.viewport,
            &self.robots,
        );

        if let Some(canonical) = &self.canonical {
            tags.push(link().attr("rel", "canonical").attr("href", canonical));
        }
        for (hreflang, url) in &self.alternates {
            tags.push(
                link()
                    .attr("rel", "alternate")
                    .attr("hreflang", hreflang)
                    .attr("href", url),
            );
        }
        if let Some(color) = &self.theme_color {
            tags.push(meta_name("theme-color", color));
        }
        if let Some(scheme) = &self.color_scheme {
            tags.push(meta_name("color-scheme", scheme));
        }

        let image = self.image.as_deref().unwrap_or_default();
        let url = self.url.as_deref().unwrap_or_default();

        tags.extend(open_graph(
            &self.title,
            &self.description,
            image,
            url,
            "website",
            self.site_name.as_deref(),
        ));
        if let Some((width, height)) = self.image_size {
            tags.push(meta_property("og:image:width", width.to_string()));
            tags.push(meta_property("og:image:height", height.to_string()));
        }
        if let Some(alt) = &self.image_alt {
            tags.push(meta_property("og:image:alt", alt));
        }
        if let Some(locale) = &self.locale {
            tags.push(meta_property("og:locale", locale));
        }
        for locale in &self.locale_alternates {
            tags.push(meta_property("og:locale:alternate", locale));
        }
        tags.extend(twitter_card(
            &self.title,
            &self.description,
            image,
            &self.twitter_card,
            self.twitter_site.as_deref(),
            self.twitter_creator.as_deref(),
        ));
        if let Some(alt) = &self.image_alt {
            tags.push(meta_name("twitter:image:alt", alt));
        }
        tags
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn json_ld_cannot_be_broken_out_of_by_a_value() {
        for hostile in [
            "</script><script>alert(1)</script>",
            "</SCRIPT >",
            "<!--<script>",
            "-->",
            "<img src=x onerror=alert(1)>",
        ] {
            let json = format!(r#"{{"name":"{hostile}"}}"#);
            let html = json_ld(&json).render();
            let inner = html
                .strip_prefix(r#"<script type="application/ld+json">"#)
                .and_then(|rest| rest.strip_suffix("</script>"))
                .expect("one script element");
            for forbidden in ['<', '>', '&'] {
                assert!(!inner.contains(forbidden), "{hostile:?} -> {html}");
            }
        }
    }

    #[test]
    fn json_ld_keeps_the_document_and_only_rewrites_what_is_dangerous() {
        let plain = r#"{"@context":"https://schema.org","name":"Tyres","n":[1,2.5,true,null]}"#;
        assert_eq!(
            json_ld(plain).render(),
            format!(r#"<script type="application/ld+json">{plain}</script>"#)
        );
        // `&`, `<`, `>` and the two JS line terminators become JSON escapes; the rest is as given.
        let json = format!(r#"{{"t":"a & b <c> {} {}"}}"#, '\u{2028}', '\u{2029}');
        assert_eq!(
            json_ld(&json).render(),
            r#"<script type="application/ld+json">{"t":"a \u0026 b \u003cc\u003e \u2028 \u2029"}</script>"#
        );
        assert_eq!(
            script_json("a\"b", "{}").render(),
            r#"<script type="a&quot;b">{}</script>"#
        );
    }

    use super::*;
    use crate::core::Render;

    fn built(builder: &SeoBuilder) -> String {
        builder.build().iter().map(Render::render).collect()
    }

    #[test]
    fn a_builder_that_calls_no_new_setter_writes_exactly_what_it_always_did() {
        let html = built(&SeoBuilder::new("T", "D").image("https://x.example/i.png"));
        for absent in [
            "canonical",
            "hreflang",
            "theme-color",
            "color-scheme",
            "og:locale",
            "og:image:width",
            "og:image:height",
            "og:image:alt",
            "twitter:image:alt",
        ] {
            assert!(!html.contains(absent), "{absent} in {html}");
        }
    }

    #[test]
    fn robots_and_viewport_can_be_overridden() {
        let html = built(
            &SeoBuilder::new("T", "D")
                .robots("noindex")
                .viewport("width=device-width"),
        );
        assert!(html.contains(r#"<meta name="robots" content="noindex">"#));
        assert!(html.contains(r#"<meta name="viewport" content="width=device-width">"#));
    }

    #[test]
    fn a_bilingual_page_gets_its_links_and_locales_in_a_fixed_order() {
        let html = built(
            &SeoBuilder::new("T", "D")
                .image("https://x.example/i.png")
                .site_name("Molt")
                .canonical("https://x.example/en/")
                .alternate("en", "https://x.example/en/")
                .alternate("pt-BR", "https://x.example/pt/")
                .alternate("x-default", "https://x.example/")
                .theme_color("#15140f")
                .color_scheme("dark light")
                .image_size(1200, 630)
                .image_alt("A molting crab")
                .locale("en_US")
                .locale_alternate("pt_BR")
                .twitter_site("@molt"),
        );
        let in_order = [
            "<title>T</title>",
            r#"<link rel="canonical" href="https://x.example/en/">"#,
            r#"<link rel="alternate" hreflang="en" href="https://x.example/en/">"#,
            r#"<link rel="alternate" hreflang="pt-BR" href="https://x.example/pt/">"#,
            r#"<link rel="alternate" hreflang="x-default" href="https://x.example/">"#,
            r##"<meta name="theme-color" content="#15140f">"##,
            r#"<meta name="color-scheme" content="dark light">"#,
            r#"<meta property="og:site_name" content="Molt">"#,
            r#"<meta property="og:image:width" content="1200">"#,
            r#"<meta property="og:image:height" content="630">"#,
            r#"<meta property="og:image:alt" content="A molting crab">"#,
            r#"<meta property="og:locale" content="en_US">"#,
            r#"<meta property="og:locale:alternate" content="pt_BR">"#,
            r#"<meta name="twitter:card" content="summary_large_image">"#,
            r#"<meta name="twitter:site" content="@molt">"#,
            r#"<meta name="twitter:image:alt" content="A molting crab">"#,
        ];
        let mut from = 0;
        for tag in in_order {
            let at = html[from..]
                .find(tag)
                .unwrap_or_else(|| panic!("{tag} missing or out of order in {html}"));
            from += at + tag.len();
        }
    }

    #[test]
    fn hostile_values_in_the_new_setters_stay_inside_their_attribute() {
        let html = built(
            &SeoBuilder::new("T", "D")
                .canonical("javascript:alert(1)")
                .alternate("en\"><script>", "https://x.example/\"><script>")
                .theme_color("\"><script>alert(1)</script>")
                .image_alt("\"><img src=x onerror=alert(1)>")
                .locale("\"><b>"),
        );
        assert!(
            html.contains(r##"<link rel="canonical" href="#">"##),
            "{html}"
        );
        // `<` and `>` stay as text inside a quoted value; what matters is that no raw quote
        // lets the payload out of its attribute.
        for payload in ["\"><script", "\"><img", "\"><b>"] {
            assert!(!html.contains(payload), "{payload} in {html}");
        }
    }

    fn rendered(tags: &[Element]) -> String {
        tags.iter().map(Render::render).collect()
    }

    /// The names were inserted unescaped, so a name carrying a quote closed the attribute.
    #[test]
    fn a_hostile_meta_name_cannot_break_out_of_its_attribute() {
        let hostile = "a\"><script>alert(1)</script><x y=\"";
        // Inside the quotes `<` is inert (the S4 raw-text-parent case is handled by the
        // writer); what must hold is that no `"` from the input closes a value early.
        for (tag, quotes) in [
            (meta_name(hostile, "v"), 4),
            (meta_property(hostile, "v"), 4),
            (meta_charset(hostile), 2),
            (meta_http_equiv(hostile, "v"), 4),
        ] {
            let html = tag.render();
            assert_eq!(html.matches('"').count(), quotes, "{html}");
            assert!(html.contains("&quot;"), "{html}");
        }
        assert_eq!(
            meta_property("og:title", "T").render(),
            r#"<meta property="og:title" content="T">"#
        );
    }

    #[test]
    fn open_graph_emits_the_five_core_properties() {
        let tags = open_graph("T", "D", "/i.png", "https://e.com", "website", None);
        assert_eq!(tags.len(), 5);
        let html = rendered(&tags);
        for property in [
            "og:title",
            "og:description",
            "og:image",
            "og:url",
            "og:type",
        ] {
            assert!(html.contains(property), "{property} missing");
        }
    }

    #[test]
    fn og_site_name_is_only_emitted_when_supplied() {
        assert_eq!(open_graph("T", "D", "i", "u", "website", None).len(), 5);
        assert_eq!(
            open_graph("T", "D", "i", "u", "website", Some("Site")).len(),
            6
        );
    }

    #[test]
    fn an_article_adds_the_article_extensions_and_sets_the_type() {
        let tags = open_graph_article("T", "D", "i", "u", Some("Ana"), Some("2026-01-01"), None);
        let html = rendered(&tags);
        assert!(html.contains(r#"content="article""#));
        assert!(html.contains("article:author"));
        assert!(html.contains("article:published_time"));
        assert!(!html.contains("article:modified_time"));
    }

    #[test]
    fn twitter_card_defaults_to_a_large_image_summary() {
        let tags = twitter_card("T", "D", "i", DEFAULT_TWITTER_CARD, None, None);
        assert!(rendered(&tags).contains(r#"content="summary_large_image""#));
        assert_eq!(tags.len(), 4);
    }

    #[test]
    fn common_emits_a_title() {
        let tags = common("Page", "D", None, None, DEFAULT_VIEWPORT, DEFAULT_ROBOTS);
        assert!(
            rendered(&tags).contains("<title>Page</title>"),
            "common must emit the title it is given"
        );
    }

    #[test]
    fn keywords_are_joined_with_a_comma_and_a_space() {
        let tags = common(
            "P",
            "D",
            Some(&["rust", "motorcycle"]),
            None,
            DEFAULT_VIEWPORT,
            DEFAULT_ROBOTS,
        );
        assert!(rendered(&tags).contains(r#"content="rust, motorcycle""#));
    }

    /// The order is what the golden fixture encodes.
    #[test]
    fn the_builder_emits_common_then_open_graph_then_twitter() {
        let html = rendered(&SeoBuilder::new("T", "D").image("i").url("u").build());
        let charset = html.find("charset").expect("charset");
        let og = html.find("og:title").expect("og:title");
        let twitter = html.find("twitter:card").expect("twitter:card");
        assert!(charset < og && og < twitter);
    }

    #[test]
    fn meta_values_are_escaped_but_keys_are_not() {
        let tag = meta_property("og:title", r#"Tom & Jerry's "show""#);
        let html = tag.render();
        assert!(html.contains(r#"property="og:title""#));
        assert!(html.contains("&amp;"));
        assert!(html.contains("&quot;"));
    }

    #[test]
    fn an_article_carries_its_timestamps() {
        let markup = rendered(&open_graph_article(
            "T",
            "D",
            "I",
            "U",
            Some("Michel"),
            Some("2026-08-11T10:00:00Z"),
            Some("2026-08-12T10:00:00Z"),
        ));

        assert!(markup.contains(r#"<meta property="article:author" content="Michel">"#));
        assert!(markup.contains(
            r#"<meta property="article:published_time" content="2026-08-11T10:00:00Z">"#
        ));
        assert!(
            markup.contains(
                r#"<meta property="article:modified_time" content="2026-08-12T10:00:00Z">"#
            )
        );
    }

    /// "No keywords" is `None`, and an empty slice has to behave the same way — an empty `content=""` would
    /// be worse than nothing.
    #[test]
    fn empty_keywords_emit_no_tag() {
        for keywords in [None, Some(&[][..])] {
            let markup = rendered(&common(
                "T",
                "D",
                keywords,
                None,
                DEFAULT_VIEWPORT,
                DEFAULT_ROBOTS,
            ));
            assert!(!markup.contains("keywords"), "emitted for {keywords:?}");
        }
    }

    #[test]
    fn meta_property_renders_a_property_attribute() {
        assert_eq!(
            meta_property("og:title", "Test Title").render(),
            r#"<meta property="og:title" content="Test Title">"#
        );
    }

    #[test]
    fn meta_charset_renders_a_charset_attribute() {
        assert_eq!(meta_charset("UTF-8").render(), r#"<meta charset="UTF-8">"#);
    }
}
