//! Script-URL check for attributes the browser navigates to or fetches.
//!
//! The check runs when the attribute is written, on the value that will appear in the
//! HTML — including values stored with [`Attribute::raw`](crate::core::Attribute::raw).
//! A browser decodes character references before it reads the scheme, so the policy does
//! the same once. [`Attribute::new`](crate::core::Attribute::new) has already escaped
//! `&`, which turns `javascript&#58;` into `javascript&amp;#58;`; that form is not a
//! script URL and is left alone.

const URL_ATTRIBUTES: &[&str] = &[
    "href",
    "src",
    "action",
    "formaction",
    "poster",
    "cite",
    "background",
    "xlink:href",
    "data",
    "ping",
    "manifest",
    "longdesc",
];

/// `srcset` is a URL attribute too, but its value is a comma-separated list of
/// `url descriptor` candidates, so it is checked one candidate at a time.
const SRCSET: &str = "srcset";

/// `srcdoc` is not a URL: it is a whole HTML document the browser runs inside the `<iframe>`,
/// scripts included, with the parent's origin unless the frame is sandboxed. It is blocked
/// for any non-empty value unless the caller opts in with
/// [`Attribute::trusted_url`](crate::core::Attribute::trusted_url).
const SRCDOC: &str = "srcdoc";

const BLOCKED_SCHEMES: &[&str] = &["javascript:", "vbscript:", "data:text/html"];

/// Elements that load their `src`/`data` as a *document*, with a script-running origin of
/// their own. A `data:` URL is harmless as an `<img>` source (images do not run script, SVG
/// included) and a way to run script as any of these.
const EMBEDDING_TAGS: &[&str] = &["iframe", "frame", "embed", "object"];

/// The `data:` media types an embedding element may still load: raster images.
const RASTER_DATA_PREFIXES: &[&str] = &[
    "data:image/png",
    "data:image/jpeg",
    "data:image/gif",
    "data:image/webp",
    "data:image/avif",
    "data:image/bmp",
];

/// Whether `url` is an absolute `http://` or `https://` URL.
///
/// What a sitemap `<loc>` and an RSS `<link>` must be: both specifications call for a full
/// URL, and a feed reader renders the link as something to click, so `javascript:` there is
/// as dangerous as in an `href`. The scheme is compared ASCII case-insensitively and a
/// leading run of whitespace is ignored, as a browser does.
///
/// # Examples
/// ```
/// use molt::core::url_policy::is_http_url;
/// assert!(is_http_url("https://example.com/a?b=1&c=2"));
/// assert!(is_http_url("HTTP://example.com"));
/// assert!(!is_http_url("javascript:alert(1)"));
/// assert!(!is_http_url("/relative/path"));
/// assert!(!is_http_url("ftp://example.com"));
/// assert!(!is_http_url("https:"));
/// ```
#[must_use]
pub fn is_http_url(url: &str) -> bool {
    let url = url.trim_start();
    ["http://", "https://"].iter().any(|scheme| {
        url.len() > scheme.len()
            && url.is_char_boundary(scheme.len())
            && url[..scheme.len()].eq_ignore_ascii_case(scheme)
    })
}

/// The value written in place of a blocked URL.
pub const REPLACEMENT: &str = "#";

/// What to write for a blocked `key`: `#` for a URL, nothing for `srcdoc`, which is markup
/// and for which `#` would be shown as the frame's content.
#[must_use]
pub fn replacement_for(key: &str) -> &'static str {
    if eq_ignore_ascii_case(key, SRCDOC) {
        ""
    } else {
        REPLACEMENT
    }
}

/// Whether `value` would be replaced with [`REPLACEMENT`] when written as `key`.
///
/// `value` is the string stored on the attribute. Boolean attributes never reach this
/// function. A path with no `&` and no character at or below U+0020 is checked in place.
///
/// # Examples
/// ```
/// use molt::core::url_policy::is_blocked;
/// assert!(is_blocked("href", "javascript:alert(1)"));
/// assert!(is_blocked("HREF", "JaVaScRiPt:alert(1)"));
/// assert!(!is_blocked("href", "https://example.com/javascript:"));
/// assert!(!is_blocked("title", "javascript:fine"));
/// assert!(!is_blocked("href", "javascript&amp;#58;x"));
/// ```
#[must_use]
pub fn is_blocked(key: &str, value: &str) -> bool {
    is_blocked_in("", key, value)
}

/// As [`is_blocked`], for an attribute on the element `tag`.
///
/// The tag matters for one rule: on `iframe`, `frame`, `embed` and `object`, a `src` or
/// `data` that starts with `data:` is blocked unless it is a raster image. Those elements
/// load a document, and `data:image/svg+xml`, `data:application/xhtml+xml` and the like run
/// script in it. The same URL on an `<img>` is inert and is left alone.
///
/// # Examples
/// ```
/// use molt::core::url_policy::is_blocked_in;
/// assert!(is_blocked_in("iframe", "src", "data:image/svg+xml,<svg onload=alert(1)>"));
/// assert!(!is_blocked_in("img", "src", "data:image/svg+xml;base64,PHN2Zz4="));
/// assert!(!is_blocked_in("iframe", "src", "data:image/png;base64,AAAA"));
/// assert!(is_blocked_in("iframe", "srcdoc", "<script>alert(1)</script>"));
/// ```
#[must_use]
pub fn is_blocked_in(tag: &str, key: &str, value: &str) -> bool {
    if eq_ignore_ascii_case(key, SRCDOC) {
        return !value.is_empty();
    }
    if eq_ignore_ascii_case(key, SRCSET) {
        return srcset_candidates(value).any(is_script_url);
    }
    if !is_url_attribute(key) {
        return false;
    }
    if value.bytes().any(|byte| byte == b'&' || byte <= b' ') {
        let normalized = normalize(value);
        return has_blocked_scheme(&normalized) || blocked_for_embedding(tag, key, &normalized);
    }
    has_blocked_scheme(value) || blocked_for_embedding(tag, key, value)
}

/// The URL of each candidate in a `srcset`: the first whitespace-delimited token after a comma.
fn srcset_candidates(value: &str) -> impl Iterator<Item = &str> {
    value
        .split(',')
        .filter_map(|candidate| candidate.split_whitespace().next())
}

fn is_script_url(url: &str) -> bool {
    has_blocked_scheme(&normalize(url))
}

/// A `data:` document loaded by an element that runs documents, and not a raster image.
fn blocked_for_embedding(tag: &str, key: &str, normalized: &str) -> bool {
    EMBEDDING_TAGS
        .iter()
        .any(|name| eq_ignore_ascii_case(tag, name))
        && (eq_ignore_ascii_case(key, "src") || eq_ignore_ascii_case(key, "data"))
        && starts_with_ignore_case(normalized, "data:")
        && !RASTER_DATA_PREFIXES
            .iter()
            .any(|prefix| starts_with_ignore_case(normalized, prefix))
}

fn starts_with_ignore_case(value: &str, prefix: &str) -> bool {
    value.len() >= prefix.len()
        && value.is_char_boundary(prefix.len())
        && value[..prefix.len()].eq_ignore_ascii_case(prefix)
}

fn is_url_attribute(key: &str) -> bool {
    URL_ATTRIBUTES
        .iter()
        .any(|name| eq_ignore_ascii_case(key, name))
}

fn eq_ignore_ascii_case(left: &str, right: &str) -> bool {
    left.len() == right.len()
        && left
            .bytes()
            .zip(right.bytes())
            .all(|(got, expected)| got.eq_ignore_ascii_case(&expected))
}

fn has_blocked_scheme(value: &str) -> bool {
    BLOCKED_SCHEMES.iter().any(|scheme| {
        let mut chars = value.chars();
        scheme.chars().all(|expected| {
            chars
                .next()
                .is_some_and(|got| got.eq_ignore_ascii_case(&expected))
        })
    })
}

/// One decode pass, then drop every scalar at or below U+0020.
///
/// Named references are case-sensitive and require `;`. Numeric references accept an
/// optional `;`, matching what browsers do. The result is not decoded again, so
/// `&amp;` stays `&amp;` rather than becoming `&`.
fn normalize(value: &str) -> String {
    let mut out = String::new();
    let mut rest = value;
    while let Some(first) = rest.chars().next() {
        if first == '&' {
            if let Some((decoded, consumed)) = decode_reference(rest) {
                push_visible(&mut out, decoded);
                rest = &rest[consumed..];
                continue;
            }
        }
        push_visible(&mut out, first);
        rest = &rest[first.len_utf8()..];
    }
    out
}

fn push_visible(out: &mut String, scalar: char) {
    if scalar > '\u{20}' {
        out.push(scalar);
    }
}

/// `value` starts at `&`. Returns the decoded scalar and the number of bytes consumed,
/// including the `&`.
fn decode_reference(value: &str) -> Option<(char, usize)> {
    let after = value.get(1..)?;
    if let Some((scalar, used)) = numeric_reference(after) {
        return Some((scalar, 1 + used));
    }
    named_reference(after).map(|(scalar, used)| (scalar, 1 + used))
}

fn numeric_reference(after: &str) -> Option<(char, usize)> {
    let bytes = after.as_bytes();
    if bytes.first().copied() != Some(b'#') {
        return None;
    }
    let mut index = 1;
    let hex = matches!(bytes.get(index).copied(), Some(b'x' | b'X'));
    if hex {
        index += 1;
    }
    let digits_start = index;
    let radix: u32 = if hex { 16 } else { 10 };
    let mut code: u32 = 0;
    while index < bytes.len() {
        let digit = match bytes[index] {
            byte @ b'0'..=b'9' => u32::from(byte - b'0'),
            byte @ b'a'..=b'f' if hex => u32::from(byte - b'a' + 10),
            byte @ b'A'..=b'F' if hex => u32::from(byte - b'A' + 10),
            _ => break,
        };
        code = code.wrapping_mul(radix).wrapping_add(digit).min(0x11_0000);
        index += 1;
    }
    if index == digits_start {
        return None;
    }
    if bytes.get(index).copied() == Some(b';') {
        index += 1;
    }
    Some((char::from_u32(code).unwrap_or('\u{FFFD}'), index))
}

fn named_reference(after: &str) -> Option<(char, usize)> {
    let end = after
        .char_indices()
        .nth(8)
        .map_or(after.len(), |(index, _)| index);
    let window = &after[..end];
    let semi = window.find(';')?;
    let scalar = match &window[..semi] {
        "colon" => ':',
        "Tab" => '\t',
        "NewLine" => '\n',
        _ => return None,
    };
    Some((scalar, semi + 1))
}

#[cfg(test)]
mod tests {
    use crate::core::Element;
    use crate::core::Render;
    use crate::elements::{a, div, embed, iframe, img};

    #[test]
    fn data_documents_are_blocked_where_they_run_and_allowed_where_they_do_not() {
        let svg = "data:image/svg+xml,<svg onload=alert(1)>";
        let xhtml = "data:application/xhtml+xml,<script>alert(1)</script>";
        assert_eq!(
            iframe().attr("src", svg).render(),
            r##"<iframe src="#"></iframe>"##
        );
        assert_eq!(
            iframe().attr("src", xhtml).render(),
            r##"<iframe src="#"></iframe>"##
        );
        assert_eq!(embed().attr("src", svg).render(), r##"<embed src="#">"##);
        assert_eq!(
            Element::new("object").attr("data", svg).render(),
            r##"<object data="#"></object>"##
        );
        // Obfuscation does not change what the browser parses.
        assert_eq!(
            iframe().attr("src", "  DATA:image/svg+xml,x").render(),
            r##"<iframe src="#"></iframe>"##
        );
        // A raster image is not a document, in any embedding element.
        let png = "data:image/png;base64,AAAA";
        assert_eq!(
            iframe().attr("src", png).render(),
            format!(r#"<iframe src="{png}"></iframe>"#)
        );
        // And the same SVG is inert as an image source.
        let inline = "data:image/svg+xml;base64,PHN2Zz4=";
        assert_eq!(
            img().attr("src", inline).render(),
            format!(r#"<img src="{inline}">"#)
        );
        assert_eq!(
            a().attr("href", "/page").render(),
            r#"<a href="/page"></a>"#
        );
    }

    #[test]
    fn srcdoc_is_blocked_unless_it_is_opted_into() {
        let page = "<script>alert(1)</script>";
        assert_eq!(
            iframe().attr("srcdoc", page).render(),
            r#"<iframe srcdoc=""></iframe>"#
        );
        assert_eq!(
            iframe().attr("SRCDOC", page).render(),
            r#"<iframe SRCDOC=""></iframe>"#
        );
        assert_eq!(
            iframe().trusted_url("srcdoc", "<p>hi</p>").render(),
            r#"<iframe srcdoc="<p>hi</p>"></iframe>"#
        );
        assert_eq!(
            iframe().attr("srcdoc", "").render(),
            r#"<iframe srcdoc=""></iframe>"#
        );
    }

    #[test]
    fn srcset_is_checked_one_candidate_at_a_time() {
        assert_eq!(
            img()
                .attr("srcset", "/a.png 1x, javascript:alert(1) 2x")
                .render(),
            r##"<img srcset="#">"##
        );
        assert_eq!(
            img().attr("srcset", "/a.png 1x, /b.png 2x").render(),
            r#"<img srcset="/a.png 1x, /b.png 2x">"#
        );
        // A URL that merely contains the word is not a script URL.
        assert_eq!(
            img().attr("srcset", "/javascript:/a.png 1x").render(),
            r#"<img srcset="/javascript:/a.png 1x">"#
        );
    }

    #[test]
    fn the_other_navigating_attributes_are_covered() {
        for key in ["ping", "manifest", "longdesc"] {
            assert_eq!(
                div().attr(key, "javascript:alert(1)").render(),
                format!(r##"<div {key}="#"></div>"##),
                "{key}"
            );
        }
        assert_eq!(
            div().attr("ping", "https://e.com/p").render(),
            r#"<div ping="https://e.com/p"></div>"#
        );
    }

    use crate::core::Attribute;

    fn tag(name: &str, attribute: &Attribute) -> String {
        let mut out = format!("<{name}");
        attribute.write_into(&mut out);
        out.push_str("></");
        out.push_str(name);
        out.push('>');
        out
    }

    /// Ports `ThreeZeroTests.scriptURLsAreNeutralised`.
    #[test]
    fn script_urls_are_neutralised() {
        for url in [
            "javascript:alert(1)",
            "JaVaScRiPt:alert(1)",
            " javascript:x",
            "java\tscript:x",
            "java\nscript:x",
            "\u{0001}javascript:x",
            "vbscript:msgbox",
            "data:text/html,<b>x</b>",
        ] {
            assert_eq!(
                tag("a", &Attribute::new("href", url)),
                r##"<a href="#"></a>"##,
                "{url:?}"
            );
        }
    }

    /// Ports `ThreeZeroTests.otherURLsAndAttributesAreUntouched`.
    #[test]
    fn other_urls_and_attributes_are_untouched() {
        assert_eq!(
            tag(
                "a",
                &Attribute::new("href", "https://swift.org/javascript:")
            ),
            r#"<a href="https://swift.org/javascript:"></a>"#
        );
        assert_eq!(
            tag("img", &Attribute::new("src", "data:image/png;base64,AAA")),
            r#"<img src="data:image/png;base64,AAA"></img>"#
        );
        assert_eq!(
            tag("div", &Attribute::new("title", "javascript:fine")),
            r#"<div title="javascript:fine"></div>"#
        );
        assert_eq!(
            tag("form", &Attribute::new("ACTION", "javascript:x")),
            r##"<form ACTION="#"></form>"##
        );
        assert_eq!(
            tag("button", &Attribute::boolean("formaction")),
            "<button formaction></button>"
        );
        assert_eq!(
            tag("div", &Attribute::new("data-href", "javascript:x")),
            r#"<div data-href="javascript:x"></div>"#
        );
    }

    /// Ports `ThreeZeroTests.trustedURLsRenderAsGiven`.
    #[test]
    fn trusted_urls_render_as_given() {
        assert_eq!(
            tag("a", &Attribute::trusted_url("href", "javascript:print()")),
            r#"<a href="javascript:print()"></a>"#
        );
        assert_eq!(
            tag("a", &Attribute::trusted_url("href", "javascript:x")),
            r#"<a href="javascript:x"></a>"#
        );
    }

    /// Ports `ThreeZeroTests.characterReferencesCannotSpellAScriptURL`.
    #[test]
    fn character_references_cannot_spell_a_script_url() {
        for url in [
            "javascript&#58;alert(1)",
            "javascript&#x3A;x",
            "javascript&#X3a;x",
            "javascript&#0058;x",
            "javascript&#58x",
            "javascript&colon;x",
            "java&Tab;script:x",
            "java&NewLine;script:x",
            "&#x6A;avascript:x",
            "&#106;avascript:x",
            "java&#9;script:x",
            "vbscript&#58;x",
        ] {
            assert_eq!(
                tag("a", &Attribute::raw("href", url)),
                r##"<a href="#"></a>"##,
                "{url:?}"
            );
        }
    }

    /// Ports `ThreeZeroTests.characterReferencesInSafeURLsAreKept`.
    #[test]
    fn character_references_in_safe_urls_are_kept() {
        assert_eq!(
            tag("a", &Attribute::new("href", "javascript&#58;x")),
            r#"<a href="javascript&amp;#58;x"></a>"#
        );
        assert_eq!(
            tag("a", &Attribute::raw("href", "/search?q=a&amp;b=&#58;")),
            r#"<a href="/search?q=a&amp;b=&#58;"></a>"#
        );
        assert_eq!(
            tag("a", &Attribute::raw("href", "java&COLON;x")),
            r#"<a href="java&COLON;x"></a>"#
        );
    }

    #[test]
    fn every_url_attribute_is_covered_and_its_name_is_kept() {
        for key in [
            "href",
            "src",
            "action",
            "formaction",
            "poster",
            "cite",
            "background",
            "xlink:href",
            "data",
        ] {
            let rendered = tag("a", &Attribute::new(key, "javascript:x"));
            assert_eq!(rendered, format!(r##"<a {key}="#"></a>"##), "{key}");
        }
    }

    #[test]
    fn a_name_that_could_escape_its_quotes_is_not_emitted() {
        for key in [
            "a\" onclick=\"x",
            "a'b",
            "a=b",
            "a<b",
            "a>b",
            "a/b",
            "a b",
            "a\nb",
            "a\u{007f}b",
            "",
        ] {
            let mut out = String::from("<div");
            Attribute::raw(key, "1").write_into(&mut out);
            Attribute::new("id", "ok").write_into(&mut out);
            out.push_str("></div>");
            assert_eq!(out, r#"<div id="ok"></div>"#, "{key:?}");
        }
    }
}
