//! RSS 2.0 feed generation.
//!

use std::fmt::Write as _;

use crate::core::escape::escape_xml;
use crate::core::url_policy::is_http_url;

/// One entry in a feed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RssItem {
    /// The item headline.
    pub title: String,
    /// The item's canonical URL.
    pub link: String,
    /// A summary or the full content.
    pub description: String,
    /// An RFC 822 date, e.g. `Tue, 11 Aug 2026 10:00:00 +0000`.
    pub pub_date: String,
    /// A globally unique identifier. Defaults to `link`.
    pub guid: Option<String>,
    /// The author's email address.
    pub author: Option<String>,
    /// Category labels.
    pub categories: Option<Vec<String>>,
}

impl RssItem {
    /// Creates an item. `guid` defaults to `link`; the default is applied here, not
    /// at render time.
    pub fn new(
        title: impl Into<String>,
        link: impl Into<String>,
        description: impl Into<String>,
        pub_date: impl Into<String>,
    ) -> Self {
        let link = link.into();
        Self {
            title: title.into(),
            guid: Some(link.clone()),
            link,
            description: description.into(),
            pub_date: pub_date.into(),
            author: None,
            categories: None,
        }
    }

    /// Overrides the GUID.
    #[must_use]
    pub fn guid(mut self, guid: impl Into<String>) -> Self {
        self.guid = Some(guid.into());
        self
    }

    /// Sets the author.
    #[must_use]
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Sets the categories.
    #[must_use]
    pub fn categories<S: Into<String>>(mut self, categories: impl IntoIterator<Item = S>) -> Self {
        self.categories = Some(categories.into_iter().map(Into::into).collect());
        self
    }
}

/// Renders an RSS 2.0 feed.
///
/// # Examples
/// ```
/// use molt::feed::{RssGenerator, RssItem};
///
/// let feed = RssGenerator::new("RideKeeper", "https://ridekeeper.example", "Release notes")
///     .language("pt-BR");
/// let xml = feed.generate(&[RssItem::new(
///     "1.2 — tyres & chain",
///     "https://ridekeeper.example/blog/1-2",
///     "Tyre pressure log",
///     "Tue, 11 Aug 2026 10:00:00 +0000",
/// )]);
/// assert!(xml.contains("<title>1.2 — tyres &amp; chain</title>"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RssGenerator {
    /// The channel title.
    pub title: String,
    /// The channel's site URL.
    pub link: String,
    /// What the channel is about.
    pub description: String,
    /// An RFC 5646 language tag.
    pub language: Option<String>,
    /// A copyright notice.
    pub copyright: Option<String>,
    /// The editorial contact address.
    pub managing_editor: Option<String>,
    /// The technical contact address.
    ///
    /// Renders as `<webMaster>`. The camelCase spelling is the RSS specification's, not a
    /// style slip. Do not "fix" it.
    pub webmaster: Option<String>,
}

impl RssGenerator {
    /// Creates a channel.
    pub fn new(
        title: impl Into<String>,
        link: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            title: title.into(),
            link: link.into(),
            description: description.into(),
            language: None,
            copyright: None,
            managing_editor: None,
            webmaster: None,
        }
    }

    /// Sets the channel language.
    #[must_use]
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = Some(language.into());
        self
    }

    /// Sets the copyright notice.
    #[must_use]
    pub fn copyright(mut self, copyright: impl Into<String>) -> Self {
        self.copyright = Some(copyright.into());
        self
    }

    /// Sets the editorial contact.
    #[must_use]
    pub fn managing_editor(mut self, editor: impl Into<String>) -> Self {
        self.managing_editor = Some(editor.into());
        self
    }

    /// Sets the technical contact, rendered as `<webMaster>`.
    #[must_use]
    pub fn webmaster(mut self, webmaster: impl Into<String>) -> Self {
        self.webmaster = Some(webmaster.into());
        self
    }

    /// Renders the feed.
    #[must_use]
    pub fn generate(&self, items: &[RssItem]) -> String {
        let mut xml = String::with_capacity(512 + items.len() * 320);
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<rss version=\"2.0\" xmlns:atom=\"http://www.w3.org/2005/Atom\">\n");
        xml.push_str("  <channel>\n");

        let link = escape_xml(&self.link);
        let _ = writeln!(xml, "    <title>{}</title>", escape_xml(&self.title));
        // A feed reader shows the link as something to click, so only an absolute http(s)
        // URL is written; the self link is built from it and goes with it.
        let linkable = is_http_url(&self.link);
        if linkable {
            let _ = writeln!(xml, "    <link>{link}</link>");
        }
        let _ = writeln!(
            xml,
            "    <description>{}</description>",
            escape_xml(&self.description)
        );
        if linkable {
            let _ = writeln!(
                xml,
                "    <atom:link href=\"{link}/feed.xml\" rel=\"self\" type=\"application/rss+xml\" />"
            );
        }

        write_optional(&mut xml, "language", self.language.as_deref());
        write_optional(&mut xml, "copyright", self.copyright.as_deref());
        write_optional(&mut xml, "managingEditor", self.managing_editor.as_deref());
        write_optional(&mut xml, "webMaster", self.webmaster.as_deref());

        for item in items {
            write_item(&mut xml, item);
        }

        xml.push_str("  </channel>\n");
        xml.push_str("</rss>");
        xml
    }
}

/// Writes `<tag>value</tag>` at channel depth, or nothing when the value is absent.
fn write_optional(xml: &mut String, tag: &str, value: Option<&str>) {
    if let Some(value) = value {
        let _ = writeln!(xml, "    <{tag}>{}</{tag}>", escape_xml(value));
    }
}

fn write_item(xml: &mut String, item: &RssItem) {
    xml.push_str("    <item>\n");
    let _ = writeln!(xml, "      <title>{}</title>", escape_xml(&item.title));
    if is_http_url(&item.link) {
        let _ = writeln!(xml, "      <link>{}</link>", escape_xml(&item.link));
    }
    let _ = writeln!(
        xml,
        "      <description>{}</description>",
        escape_xml(&item.description)
    );
    let _ = writeln!(
        xml,
        "      <pubDate>{}</pubDate>",
        escape_xml(&item.pub_date)
    );

    if let Some(guid) = &item.guid {
        // `isPermaLink="true"` promises the guid is a URL a reader may open; for any other
        // identifier it must say so.
        let _ = writeln!(
            xml,
            "      <guid isPermaLink=\"{}\">{}</guid>",
            is_http_url(guid),
            escape_xml(guid)
        );
    }
    if let Some(author) = &item.author {
        let _ = writeln!(xml, "      <author>{}</author>", escape_xml(author));
    }
    if let Some(categories) = &item.categories {
        for category in categories {
            let _ = writeln!(xml, "      <category>{}</category>", escape_xml(category));
        }
    }

    xml.push_str("    </item>\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel() -> RssGenerator {
        RssGenerator::new("RideKeeper", "https://ridekeeper.example", "Release notes")
    }

    #[test]
    fn the_channel_carries_its_metadata_and_an_atom_self_link() {
        let xml = channel().language("pt-BR").generate(&[]);
        assert!(xml.contains("<title>RideKeeper</title>"));
        assert!(xml.contains("<language>pt-BR</language>"));
        assert!(xml.contains(
            "<atom:link href=\"https://ridekeeper.example/feed.xml\" rel=\"self\" \
             type=\"application/rss+xml\" />"
        ));
    }

    #[test]
    fn absent_channel_fields_are_omitted_rather_than_emitted_empty() {
        let xml = channel().generate(&[]);
        for tag in [
            "<language>",
            "<copyright>",
            "<managingEditor>",
            "<webMaster>",
        ] {
            assert!(!xml.contains(tag), "{tag} should be absent");
        }
    }

    #[test]
    fn the_guid_defaults_to_the_link() {
        let item = RssItem::new(
            "T",
            "https://e.com/a",
            "D",
            "Tue, 11 Aug 2026 10:00:00 +0000",
        );
        assert_eq!(item.guid.as_deref(), Some("https://e.com/a"));

        let overridden = item.clone().guid("urn:custom");
        assert_eq!(overridden.guid.as_deref(), Some("urn:custom"));
    }

    #[test]
    fn item_content_is_xml_escaped() {
        let item = RssItem::new(
            "1.2 — tyres & chain",
            "https://e.com/a",
            "Tyre pressure log <and> chain reminders",
            "Tue, 11 Aug 2026 10:00:00 +0000",
        );
        let xml = channel().generate(&[item]);
        assert!(xml.contains("<title>1.2 — tyres &amp; chain</title>"));
        assert!(
            xml.contains(
                "<description>Tyre pressure log &lt;and&gt; chain reminders</description>"
            )
        );
    }

    #[test]
    fn categories_render_one_element_each() {
        let item = RssItem::new(
            "T",
            "https://e.com/a",
            "D",
            "Tue, 11 Aug 2026 10:00:00 +0000",
        )
        .categories(["release", "ios"]);
        let xml = channel().generate(&[item]);
        assert_eq!(xml.matches("<category>").count(), 2);
    }

    #[test]
    fn a_feed_with_no_items_is_still_valid() {
        let xml = channel().generate(&[]);
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss "));
        assert!(xml.ends_with("</rss>"));
        assert!(!xml.contains("<item>"));
    }

    #[test]
    fn every_optional_channel_field_is_rendered_when_supplied() {
        let xml = channel()
            .language("en")
            .copyright("\u{a9} 2026")
            .managing_editor("editor@e.com")
            .webmaster("web@e.com")
            .generate(&[]);

        assert!(xml.contains("<copyright>\u{a9} 2026</copyright>"));
        assert!(xml.contains("<managingEditor>editor@e.com</managingEditor>"));
        assert!(xml.contains("<webMaster>web@e.com</webMaster>"));
    }

    #[test]
    fn an_item_without_optional_fields_emits_neither() {
        let xml = channel().generate(&[RssItem::new(
            "T",
            "https://e.com/p",
            "D",
            "Tue, 11 Aug 2026 10:00:00 +0000",
        )]);

        assert!(!xml.contains("<author>"));
        assert!(!xml.contains("<category>"));
    }

    /// `isPermaLink="true"` promises readers they may open the guid, so only a URL guid gets it;
    /// `urn:uuid:1234` gets `false`.
    #[test]
    fn an_explicit_guid_wins_over_the_link() {
        let xml = channel().generate(&[RssItem::new(
            "T",
            "https://e.com/p",
            "D",
            "Tue, 11 Aug 2026 10:00:00 +0000",
        )
        .guid("urn:uuid:1234")]);

        assert!(xml.contains(r#"<guid isPermaLink="false">urn:uuid:1234</guid>"#));
        let url_guid = channel().generate(&[RssItem::new(
            "T",
            "https://e.com/p",
            "D",
            "Tue, 11 Aug 2026 10:00:00 +0000",
        )
        .guid("https://e.com/permalink")]);
        assert!(url_guid.contains(r#"<guid isPermaLink="true">https://e.com/permalink</guid>"#));
    }

    #[test]
    fn a_link_that_is_not_http_is_left_out_of_the_feed() {
        let item = |link: &str| RssItem::new("T", link, "D", "Tue, 11 Aug 2026 10:00:00 +0000");
        for bad in [
            "javascript:alert(1)",
            "/relative",
            "ftp://e.com",
            "data:text/html,x",
        ] {
            let xml = channel().generate(&[item(bad)]);
            assert!(!xml.contains(&format!("<link>{bad}")), "{bad}: {xml}");
            // The channel's own link is still there; only the item's is gone.
            assert_eq!(xml.matches("<link>").count(), 1, "{bad}");
        }
        let bad_channel = RssGenerator::new("T", "javascript:x", "D").generate(&[]);
        assert!(!bad_channel.contains("<link>"), "{bad_channel}");
        assert!(!bad_channel.contains("atom:link"), "{bad_channel}");
        assert_eq!(
            channel()
                .generate(&[item("https://e.com/p")])
                .matches("<link>")
                .count(),
            2
        );
    }

    #[test]
    fn characters_xml_cannot_hold_do_not_reach_the_feed() {
        let xml = channel().generate(&[RssItem::new(
            "a\u{0}b\u{1F}c\u{FFFE}d",
            "https://e.com/p",
            "ok\ttab",
            "Tue, 11 Aug 2026 10:00:00 +0000",
        )]);
        assert!(xml.contains("<title>abcd</title>"));
        assert!(xml.contains("ok\ttab"));
        assert!(!xml.chars().any(|c| matches!(c, '\u{0}'..='\u{8}' | '\u{B}' | '\u{C}' | '\u{E}'..='\u{1F}' | '\u{FFFE}' | '\u{FFFF}')));
    }

    #[test]
    fn an_item_renders_every_field_it_is_given_xml_escaped() {
        let xml = channel().generate(&[RssItem::new(
            "Hello & welcome",
            "https://example.com/hello",
            "First <post>",
            "Tue, 11 Aug 2026 10:00:00 +0000",
        )
        .author("me@example.com")
        .categories(["rust", "html"])]);

        assert!(xml.contains("<title>Hello &amp; welcome</title>"));
        assert!(xml.contains("<description>First &lt;post&gt;</description>"));
        assert!(xml.contains("<pubDate>Tue, 11 Aug 2026 10:00:00 +0000</pubDate>"));
        assert!(xml.contains("<category>rust</category>"));
        assert!(xml.contains("<category>html</category>"));
    }
}
