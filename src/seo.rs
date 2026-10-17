//! SEO metadata: Open Graph, Twitter Cards and the common `<meta>` block.
//!
//! Ports `Winged-Swift/Sources/WingedSwift/seo/SEOHelpers.swift`, and implements the
//! [`SeoBuilder`] from `MOLT_SPEC.md` §6 on top of it — the builder is sugar over
//! these functions, never a second implementation.

use crate::core::{Attribute, Element};
use crate::elements::{meta, title};

/// A `<meta name="…" content="…">`.
///
/// Both `name` and `content` are attribute-escaped. Winged-Swift inserts the name
/// unescaped because its callers pass literals, but these functions are `pub` and take
/// `&str`, so a name built from user input (`og:` plus a field, say) would break out of
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
/// Winged-Swift's `SEO.common(title:…)` accepts a `title` argument and never uses it — no
/// `<title>` element is emitted. This version emits one. See `PORTING.md`.
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
    // An empty list is the same as no list: `content=""` says nothing and Winged-Swift
    // omits the tag. The guard lives here rather than in `SeoBuilder` so both entry points
    // agree — it used to be in the builder only, and calling `common` directly with an
    // empty slice emitted the empty tag.
    if let Some(keywords) = keywords.filter(|list| !list.is_empty()) {
        tags.push(meta_name("keywords", keywords.join(", ")));
    }
    if let Some(author) = author {
        tags.push(meta_name("author", author));
    }
    tags.push(title().text(page_title));
    tags
}

/// The default viewport Winged-Swift uses.
pub const DEFAULT_VIEWPORT: &str = "width=device-width, initial-scale=1.0";
/// The default robots directive Winged-Swift uses.
pub const DEFAULT_ROBOTS: &str = "index, follow";
/// The default Twitter card type Winged-Swift uses.
pub const DEFAULT_TWITTER_CARD: &str = "summary_large_image";

/// A fluent builder for a page's whole metadata block.
///
/// Implements `MOLT_SPEC.md` §6 on top of the functions above.
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
        }
    }

    /// Sets the preview image used by both Open Graph and Twitter.
    #[must_use]
    pub fn image(mut self, image_url: impl Into<String>) -> Self {
        self.image = Some(image_url.into());
        self
    }

    /// Sets the canonical page URL.
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

    /// Builds the tags: the common block, then Open Graph, then Twitter Cards.
    ///
    /// That order matches Winged-Swift's `SEO.complete`, and the golden fixture depends on
    /// it.
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
        tags.extend(twitter_card(
            &self.title,
            &self.description,
            image,
            &self.twitter_card,
            self.twitter_site.as_deref(),
            self.twitter_creator.as_deref(),
        ));
        tags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Render;

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

    /// Ports `SEOTests.testOpenGraphMetaTags`.
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

    /// Ports `SEOTests.testOpenGraphIncludesTheSiteName`.
    #[test]
    fn og_site_name_is_only_emitted_when_supplied() {
        assert_eq!(open_graph("T", "D", "i", "u", "website", None).len(), 5);
        assert_eq!(
            open_graph("T", "D", "i", "u", "website", Some("Site")).len(),
            6
        );
    }

    /// Ports `SEOTests.testOpenGraphArticle`.
    #[test]
    fn an_article_adds_the_article_extensions_and_sets_the_type() {
        let tags = open_graph_article("T", "D", "i", "u", Some("Ana"), Some("2026-01-01"), None);
        let html = rendered(&tags);
        assert!(html.contains(r#"content="article""#));
        assert!(html.contains("article:author"));
        assert!(html.contains("article:published_time"));
        assert!(!html.contains("article:modified_time"));
    }

    /// Ports `SEOTests.testTwitterCardMetaTags`.
    #[test]
    fn twitter_card_defaults_to_a_large_image_summary() {
        let tags = twitter_card("T", "D", "i", DEFAULT_TWITTER_CARD, None, None);
        assert!(rendered(&tags).contains(r#"content="summary_large_image""#));
        assert_eq!(tags.len(), 4);
    }

    /// Ports `SEOTests.testCommonSEOTags`, with the fixed `<title>`.
    #[test]
    fn common_emits_a_title_unlike_the_swift_original() {
        let tags = common("Page", "D", None, None, DEFAULT_VIEWPORT, DEFAULT_ROBOTS);
        assert!(
            rendered(&tags).contains("<title>Page</title>"),
            "SEO.common accepts a title and drops it in Winged-Swift; the port emits it"
        );
    }

    #[test]
    fn keywords_are_joined_with_a_comma_and_a_space() {
        let tags = common(
            "P",
            "D",
            Some(&["swift", "motorcycle"]),
            None,
            DEFAULT_VIEWPORT,
            DEFAULT_ROBOTS,
        );
        assert!(rendered(&tags).contains(r#"content="swift, motorcycle""#));
    }

    /// Ports `SEOTests.testCompleteSEOTags`. The order is what the golden fixture encodes.
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

    /// Ports `SEOTests.testArticleCarriesItsTimestamps`.
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

    /// Ports `SEOTests.testCommonOmitsEmptyKeywords`.
    ///
    /// Swift passes an empty array and expects no tag. Rust distinguishes "no keywords" as
    /// `None`, and an empty slice has to behave the same way — an empty `content=""` would
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

    /// Ports `SEOTests.testMetaWithProperty`.
    #[test]
    fn meta_property_renders_a_property_attribute() {
        assert_eq!(
            meta_property("og:title", "Test Title").render(),
            r#"<meta property="og:title" content="Test Title">"#
        );
    }

    /// Ports `SEOTests.testMetaWithCharset`.
    #[test]
    fn meta_charset_renders_a_charset_attribute() {
        assert_eq!(meta_charset("UTF-8").render(), r#"<meta charset="UTF-8">"#);
    }
}
