//! XML sitemap generation.
//!
//! Ports `Winged-Swift/Sources/WingedSwift/seo/SitemapGenerator.swift`.

use std::fmt::Write as _;

use crate::core::escape::escape_xml;

/// One entry in a sitemap.
///
/// # Examples
/// ```
/// use molt::sitemap::{SitemapGenerator, SitemapUrl};
///
/// let xml = SitemapGenerator::generate(&[
///     SitemapUrl::new("https://example.com/").changefreq("weekly").priority(1.0),
/// ]);
/// assert!(xml.contains("<priority>1.0</priority>"));
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SitemapUrl {
    /// The absolute URL of the page.
    pub loc: String,
    /// The last modification date, typically `YYYY-MM-DD`.
    pub lastmod: Option<String>,
    /// How often the page changes: `always`, `hourly`, `daily`, `weekly`, `monthly`,
    /// `yearly`, `never`.
    pub changefreq: Option<String>,
    /// Relative priority within the site, from `0.0` to `1.0`.
    pub priority: Option<f64>,
}

impl SitemapUrl {
    /// Creates an entry for the given URL.
    pub fn new(loc: impl Into<String>) -> Self {
        Self {
            loc: loc.into(),
            ..Self::default()
        }
    }

    /// Sets the last modification date.
    #[must_use]
    pub fn lastmod(mut self, lastmod: impl Into<String>) -> Self {
        self.lastmod = Some(lastmod.into());
        self
    }

    /// Sets the change frequency.
    #[must_use]
    pub fn changefreq(mut self, changefreq: impl Into<String>) -> Self {
        self.changefreq = Some(changefreq.into());
        self
    }

    /// Sets the priority.
    #[must_use]
    pub fn priority(mut self, priority: f64) -> Self {
        self.priority = Some(priority);
        self
    }
}

/// Renders sitemaps and sitemap indexes.
#[derive(Debug, Clone, Copy)]
pub struct SitemapGenerator;

impl SitemapGenerator {
    /// Renders a `<urlset>` sitemap.
    ///
    /// An empty list still produces a well-formed, empty `<urlset>`.
    #[must_use]
    pub fn generate(urls: &[SitemapUrl]) -> String {
        let mut xml = String::with_capacity(256 + urls.len() * 160);
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

        for url in urls {
            xml.push_str("  <url>\n");
            let _ = writeln!(xml, "    <loc>{}</loc>", escape_xml(&url.loc));
            if let Some(lastmod) = &url.lastmod {
                let _ = writeln!(xml, "    <lastmod>{}</lastmod>", escape_xml(lastmod));
            }
            if let Some(changefreq) = &url.changefreq {
                let _ = writeln!(
                    xml,
                    "    <changefreq>{}</changefreq>",
                    escape_xml(changefreq)
                );
            }
            if let Some(priority) = url.priority {
                let _ = writeln!(
                    xml,
                    "    <priority>{}</priority>",
                    format_priority(priority)
                );
            }
            xml.push_str("  </url>\n");
        }

        xml.push_str("</urlset>");
        xml
    }

    /// Renders a `<sitemapindex>` pointing at several sitemaps.
    #[must_use]
    pub fn generate_index(sitemaps: &[(String, Option<String>)]) -> String {
        let mut xml = String::with_capacity(256 + sitemaps.len() * 96);
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

        for (loc, lastmod) in sitemaps {
            xml.push_str("  <sitemap>\n");
            let _ = writeln!(xml, "    <loc>{}</loc>", escape_xml(loc));
            if let Some(lastmod) = lastmod {
                let _ = writeln!(xml, "    <lastmod>{}</lastmod>", escape_xml(lastmod));
            }
            xml.push_str("  </sitemap>\n");
        }

        xml.push_str("</sitemapindex>");
        xml
    }
}

/// Formats a priority the way Swift's `Double` interpolation does.
///
/// Swift renders `1.0` as `"1.0"`; Rust's `{}` renders it as `"1"`. The golden fixture was
/// produced by Swift, so a whole number keeps one decimal place here. Fractional values
/// already agree between the two languages, both using the shortest round-tripping form.
fn format_priority(priority: f64) -> String {
    if priority.fract() == 0.0 && priority.is_finite() {
        format!("{priority:.1}")
    } else {
        format!("{priority}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ports `SitemapGeneratorTests.testGenerateProducesValidURLSet`.
    #[test]
    fn a_sitemap_has_an_xml_declaration_and_a_urlset() {
        let xml = SitemapGenerator::generate(&[SitemapUrl::new("https://example.com/")]);
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset "));
        assert!(xml.ends_with("</urlset>"));
    }

    /// Ports `SitemapGeneratorTests.testGenerateEscapesAmpersandsInURLs`.
    #[test]
    fn a_loc_containing_an_ampersand_is_escaped() {
        let xml = SitemapGenerator::generate(&[SitemapUrl::new("https://e.com/a?x=1&y=2")]);
        assert!(xml.contains("<loc>https://e.com/a?x=1&amp;y=2</loc>"));
    }

    /// Ports `SitemapGeneratorTests.testPriorityIsRenderedWithOneDecimal`. This is the subtle one: Swift
    /// prints `1.0`, Rust's default `{}` prints `1`.
    #[test]
    fn whole_priorities_keep_one_decimal_place() {
        assert_eq!(format_priority(1.0), "1.0");
        assert_eq!(format_priority(0.0), "0.0");
        assert_eq!(format_priority(0.7), "0.7");
        assert_eq!(format_priority(0.25), "0.25");
    }

    /// Ports `SitemapGeneratorTests.testOptionalFieldsAreOmitted`.
    #[test]
    fn optional_fields_are_omitted_entirely() {
        let xml = SitemapGenerator::generate(&[SitemapUrl::new("https://e.com/")]);
        assert!(!xml.contains("<lastmod>"));
        assert!(!xml.contains("<changefreq>"));
        assert!(!xml.contains("<priority>"));
    }

    /// Ports `SitemapGeneratorTests.testEmptyURLListStillProducesAWellFormedDocument`.
    #[test]
    fn an_empty_sitemap_is_still_well_formed() {
        let xml = SitemapGenerator::generate(&[]);
        assert!(xml.contains("<urlset"));
        assert!(xml.ends_with("</urlset>"));
    }

    /// Ports `SitemapGeneratorTests.testGeneratesASitemapIndex`.
    #[test]
    fn an_index_lists_each_sitemap() {
        let xml = SitemapGenerator::generate_index(&[
            (
                "https://e.com/sitemap-posts.xml".into(),
                Some("2026-01-15".into()),
            ),
            ("https://e.com/sitemap-pages.xml".into(), None),
        ]);
        assert!(xml.contains("<sitemapindex"));
        assert!(xml.contains("<loc>https://e.com/sitemap-posts.xml</loc>"));
        assert!(xml.contains("<lastmod>2026-01-15</lastmod>"));
        assert_eq!(xml.matches("<sitemap>").count(), 2);
        assert_eq!(xml.matches("<lastmod>").count(), 1);

        // The Swift case checks that a query string in an index entry is escaped too, not
        // just in `<urlset>`.
        let escaped = SitemapGenerator::generate_index(&[(
            "https://e.com/sitemap-pages.xml?v=2&full=1".into(),
            None,
        )]);
        assert!(escaped.contains("v=2&amp;full=1"));
    }

    /// Ports `SitemapGeneratorTests.testSitemapIndexOmitsAMissingLastmod`.
    #[test]
    fn an_index_entry_without_a_lastmod_emits_no_element() {
        let xml = SitemapGenerator::generate_index(&[("https://e.com/sitemap.xml".into(), None)]);

        assert!(!xml.contains("<lastmod>"));
    }
}
