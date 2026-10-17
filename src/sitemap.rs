//! XML sitemap generation.
//!

use std::fmt::Write as _;

use crate::core::escape::escape_xml;
use crate::core::url_policy::is_http_url;

/// How often a page is expected to change, as the sitemap protocol spells it.
///
/// An enum rather than a string so `"weakly"` is a compile error instead of a sitemap that
/// search engines quietly ignore. Use [`str::parse`] when the value arrives as text.
///
/// # Examples
/// ```
/// use molt::sitemap::ChangeFreq;
/// assert_eq!(ChangeFreq::Weekly.as_str(), "weekly");
/// assert_eq!("monthly".parse::<ChangeFreq>(), Ok(ChangeFreq::Monthly));
/// assert!("weakly".parse::<ChangeFreq>().is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ChangeFreq {
    /// `always` — changes on every access.
    Always,
    /// `hourly`.
    Hourly,
    /// `daily`.
    Daily,
    /// `weekly`.
    Weekly,
    /// `monthly`.
    Monthly,
    /// `yearly`.
    Yearly,
    /// `never` — archived content.
    Never,
}

impl ChangeFreq {
    /// The value written between `<changefreq>` tags.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Hourly => "hourly",
            Self::Daily => "daily",
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::Yearly => "yearly",
            Self::Never => "never",
        }
    }
}

impl std::fmt::Display for ChangeFreq {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The text was not one of the seven values the protocol defines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseChangeFreqError(String);

impl std::fmt::Display for ParseChangeFreqError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?} is not a change frequency (always, hourly, daily, weekly, monthly, yearly, never)",
            self.0
        )
    }
}

impl std::error::Error for ParseChangeFreqError {}

impl std::str::FromStr for ChangeFreq {
    type Err = ParseChangeFreqError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ok(match text {
            "always" => Self::Always,
            "hourly" => Self::Hourly,
            "daily" => Self::Daily,
            "weekly" => Self::Weekly,
            "monthly" => Self::Monthly,
            "yearly" => Self::Yearly,
            "never" => Self::Never,
            other => return Err(ParseChangeFreqError(other.to_owned())),
        })
    }
}

/// The most `<url>` entries the sitemap protocol allows in one file.
pub const MAX_URLS: usize = 50_000;

/// The largest an uncompressed sitemap file may be under the protocol: 50 MiB.
pub const MAX_BYTES: usize = 50 * 1024 * 1024;

/// One entry in a sitemap.
///
/// # Examples
/// ```
/// use molt::sitemap::{ChangeFreq, SitemapGenerator, SitemapUrl};
///
/// let xml = SitemapGenerator::generate(&[
///     SitemapUrl::new("https://example.com/").changefreq(ChangeFreq::Weekly).priority(1.0),
/// ]);
/// assert!(xml.contains("<priority>1.0</priority>"));
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct SitemapUrl {
    /// The absolute URL of the page.
    pub loc: String,
    /// The last modification date, typically `YYYY-MM-DD`.
    pub lastmod: Option<String>,
    /// How often the page changes.
    pub changefreq: Option<ChangeFreq>,
    /// Relative priority within the site, from `0.0` to `1.0`.
    ///
    /// Set through [`SitemapUrl::priority`], which clamps. A value written to this field
    /// directly is checked again when the sitemap is rendered: out of range is clamped and
    /// a non-finite one is left out, because `NaN` is not a number a sitemap can hold.
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
    pub fn changefreq(mut self, changefreq: ChangeFreq) -> Self {
        self.changefreq = Some(changefreq);
        self
    }

    /// Sets the priority, clamped to `0.0..=1.0`. A non-finite value clears it.
    ///
    /// Clamping rather than failing keeps the builder chainable; the protocol allows no
    /// value outside the range, so there is nothing to preserve.
    #[must_use]
    pub fn priority(mut self, priority: f64) -> Self {
        self.priority = valid_priority(priority);
        self
    }
}

/// Renders sitemaps and sitemap indexes.
#[derive(Debug, Clone, Copy)]
pub struct SitemapGenerator;

impl SitemapGenerator {
    /// Renders a `<urlset>` sitemap.
    ///
    /// An empty list still produces a well-formed, empty `<urlset>`. An entry whose `loc`
    /// is not an absolute `http://` or `https://` URL is **left out**: the protocol requires
    /// a full URL, and a crawler or tool that lists the entries would otherwise be handed
    /// `javascript:…`.
    ///
    /// This does not enforce the protocol's limit of [`MAX_URLS`] URLs and [`MAX_BYTES`]
    /// bytes per file; split a larger site with [`SitemapGenerator::chunks`] and list the
    /// parts in a [`generate_index`](Self::generate_index).
    #[must_use]
    pub fn generate(urls: &[SitemapUrl]) -> String {
        let mut xml = String::with_capacity(256 + urls.len() * 160);
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

        for url in urls.iter().filter(|url| is_http_url(&url.loc)) {
            xml.push_str("  <url>\n");
            let _ = writeln!(xml, "    <loc>{}</loc>", escape_xml(&url.loc));
            if let Some(lastmod) = &url.lastmod {
                let _ = writeln!(xml, "    <lastmod>{}</lastmod>", escape_xml(lastmod));
            }
            if let Some(changefreq) = url.changefreq {
                let _ = writeln!(xml, "    <changefreq>{changefreq}</changefreq>");
            }
            if let Some(priority) = url.priority.and_then(valid_priority) {
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

    /// Splits `urls` into slices that each fit in one sitemap file (at most [`MAX_URLS`]).
    ///
    /// The byte limit depends on how long the URLs are, so check
    /// `generate(chunk).len() <= MAX_BYTES` and split further if a chunk is over.
    ///
    /// # Examples
    /// ```
    /// use molt::sitemap::{SitemapGenerator, SitemapUrl, MAX_URLS};
    /// let urls: Vec<SitemapUrl> = (0..=MAX_URLS)
    ///     .map(|i| SitemapUrl::new(format!("https://example.com/{i}")))
    ///     .collect();
    /// assert_eq!(SitemapGenerator::chunks(&urls).count(), 2);
    /// ```
    pub fn chunks(urls: &[SitemapUrl]) -> impl Iterator<Item = &[SitemapUrl]> {
        urls.chunks(MAX_URLS)
    }

    /// Renders a `<sitemapindex>` pointing at several sitemaps. An entry whose location is
    /// not an absolute `http://` or `https://` URL is left out, as in [`generate`](Self::generate).
    #[must_use]
    pub fn generate_index(sitemaps: &[(String, Option<String>)]) -> String {
        let mut xml = String::with_capacity(256 + sitemaps.len() * 96);
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str("<sitemapindex xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");

        for (loc, lastmod) in sitemaps.iter().filter(|(loc, _)| is_http_url(loc)) {
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

/// A priority the protocol can hold: clamped into `0.0..=1.0`, and `None` for `NaN`/infinity.
fn valid_priority(priority: f64) -> Option<f64> {
    priority.is_finite().then(|| priority.clamp(0.0, 1.0))
}

/// Formats a priority with at least one decimal place.
///
/// Rust's `{}` renders `1.0` as `"1"`, but the golden fixture has `"1.0"`, so a whole number
/// keeps one decimal place here. Fractional values use the shortest round-tripping form.
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

    #[test]
    fn a_sitemap_has_an_xml_declaration_and_a_urlset() {
        let xml = SitemapGenerator::generate(&[SitemapUrl::new("https://example.com/")]);
        assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset "));
        assert!(xml.ends_with("</urlset>"));
    }

    #[test]
    fn a_loc_containing_an_ampersand_is_escaped() {
        let xml = SitemapGenerator::generate(&[SitemapUrl::new("https://e.com/a?x=1&y=2")]);
        assert!(xml.contains("<loc>https://e.com/a?x=1&amp;y=2</loc>"));
    }

    /// This is the subtle one: `1.0` must print as `1.0`, and Rust's default `{}` prints `1`.
    #[test]
    fn whole_priorities_keep_one_decimal_place() {
        assert_eq!(format_priority(1.0), "1.0");
        assert_eq!(format_priority(0.0), "0.0");
        assert_eq!(format_priority(0.7), "0.7");
        assert_eq!(format_priority(0.25), "0.25");
    }

    #[test]
    fn a_change_frequency_round_trips_through_text() {
        for freq in [
            ChangeFreq::Always,
            ChangeFreq::Hourly,
            ChangeFreq::Daily,
            ChangeFreq::Weekly,
            ChangeFreq::Monthly,
            ChangeFreq::Yearly,
            ChangeFreq::Never,
        ] {
            assert_eq!(freq.as_str().parse(), Ok(freq));
            assert_eq!(freq.to_string(), freq.as_str());
        }
        let error = "Weekly".parse::<ChangeFreq>().expect_err("case-sensitive");
        assert!(error.to_string().contains("\"Weekly\""));
    }

    #[test]
    fn a_priority_outside_the_range_is_clamped_and_a_non_number_is_left_out() {
        let render = |priority: f64| {
            SitemapGenerator::generate(&[SitemapUrl::new("https://e.com/").priority(priority)])
        };
        assert!(render(7.0).contains("<priority>1.0</priority>"));
        assert!(render(-3.0).contains("<priority>0.0</priority>"));
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(!render(bad).contains("<priority>"), "{bad}");
        }

        // The field is public, so the renderer checks again.
        let mut url = SitemapUrl::new("https://e.com/");
        url.priority = Some(f64::NAN);
        assert!(!SitemapGenerator::generate(&[url]).contains("NaN"));
    }

    #[test]
    fn an_entry_whose_loc_is_not_http_is_left_out() {
        let xml = SitemapGenerator::generate(&[
            SitemapUrl::new("https://e.com/ok"),
            SitemapUrl::new("javascript:alert(1)"),
            SitemapUrl::new("/relative"),
            SitemapUrl::new("HTTP://e.com/upper"),
        ]);
        assert_eq!(xml.matches("<url>").count(), 2, "{xml}");
        assert!(!xml.contains("javascript"));
        let index = SitemapGenerator::generate_index(&[
            ("https://e.com/s.xml".into(), None),
            ("javascript:x".into(), None),
        ]);
        assert_eq!(index.matches("<sitemap>").count(), 1);
    }

    #[test]
    fn a_large_site_is_split_into_files_that_fit_the_protocol() {
        let urls: Vec<SitemapUrl> = (0..=MAX_URLS * 2)
            .map(|i| SitemapUrl::new(format!("https://e.com/{i}")))
            .collect();
        let chunks: Vec<&[SitemapUrl]> = SitemapGenerator::chunks(&urls).collect();
        assert_eq!(chunks.len(), 3);
        assert!(chunks.iter().all(|chunk| chunk.len() <= MAX_URLS));
        assert_eq!(
            chunks.iter().map(|chunk| chunk.len()).sum::<usize>(),
            urls.len()
        );
        assert_eq!(MAX_BYTES, 52_428_800);
    }

    #[test]
    fn optional_fields_are_omitted_entirely() {
        let xml = SitemapGenerator::generate(&[SitemapUrl::new("https://e.com/")]);
        assert!(!xml.contains("<lastmod>"));
        assert!(!xml.contains("<changefreq>"));
        assert!(!xml.contains("<priority>"));
    }

    #[test]
    fn an_empty_sitemap_is_still_well_formed() {
        let xml = SitemapGenerator::generate(&[]);
        assert!(xml.contains("<urlset"));
        assert!(xml.ends_with("</urlset>"));
    }

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

        // A query string in an index entry is escaped too, not just in `<urlset>`.
        let escaped = SitemapGenerator::generate_index(&[(
            "https://e.com/sitemap-pages.xml?v=2&full=1".into(),
            None,
        )]);
        assert!(escaped.contains("v=2&amp;full=1"));
    }

    #[test]
    fn an_index_entry_without_a_lastmod_emits_no_element() {
        let xml = SitemapGenerator::generate_index(&[("https://e.com/sitemap.xml".into(), None)]);

        assert!(!xml.contains("<lastmod>"));
    }
}
