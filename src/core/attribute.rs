//! HTML attributes.
//!
//! Ports `Winged-Swift/Sources/WingedSwift/core/Attribute.swift`.

use std::borrow::Cow;

use crate::core::escape::write_escaped_attribute;
use crate::core::url_policy;

/// A single HTML attribute.
///
/// Values are escaped when the attribute is built, not when it is rendered — see
/// [`crate::core::escape`].
///
/// # Examples
/// ```
/// use molt::core::Attribute;
/// assert_eq!(Attribute::new("href", "/a?x=1&y=2").to_string(), r#" href="/a?x=1&amp;y=2""#);
/// assert_eq!(Attribute::boolean("required").to_string(), " required");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Attribute {
    // `Cow` so the keys this crate writes itself (`class`, `id`, `lang`, …) are borrowed
    // literals rather than a fresh `String` per attribute. The public constructors still
    // take `impl Into<String>`, because a caller's key is rarely `'static`.
    key: Cow<'static, str>,
    value: String,
    is_boolean: bool,
    /// When true, a URL attribute is written as stored. The URL policy reads this flag.
    is_trusted_url: bool,
}

impl Attribute {
    /// Creates an attribute, escaping the value.
    pub fn new(key: impl Into<String>, value: impl AsRef<str>) -> Self {
        Self::with_key(Cow::Owned(key.into()), value)
    }

    /// As [`Self::new`] for a key known at compile time, which costs no allocation.
    pub(crate) fn new_static(key: &'static str, value: impl AsRef<str>) -> Self {
        Self::with_key(Cow::Borrowed(key), value)
    }

    fn with_key(key: Cow<'static, str>, value: impl AsRef<str>) -> Self {
        let raw = value.as_ref();
        let mut escaped = String::with_capacity(raw.len());
        write_escaped_attribute(&mut escaped, raw);
        Self {
            key,
            value: escaped,
            is_boolean: false,
            is_trusted_url: false,
        }
    }

    /// Appends ` word` to the stored value, escaping only the new part.
    ///
    /// What `add_class` needs: the stored value is already escaped, so rebuilding the
    /// attribute through [`Self::raw`] copied the whole value and reallocated the key.
    pub(crate) fn push_escaped_word(&mut self, word: &str) {
        self.value.push(' ');
        write_escaped_attribute(&mut self.value, word);
    }

    /// A URL attribute rendered as given, even when the value is a script URL.
    ///
    /// `href`, `src`, `action` and the other URL attributes render as `#` when the value
    /// would run script. Use this only for a URL you wrote yourself. The value is still
    /// attribute-escaped.
    ///
    /// # Examples
    /// ```
    /// use molt::prelude::*;
    /// let link = a().add_attribute(Attribute::trusted_url("href", "javascript:print()"));
    /// assert_eq!(link.render(), "<a href=\"javascript:print()\"></a>");
    /// ```
    #[must_use]
    pub fn trusted_url(key: impl Into<String>, value: impl AsRef<str>) -> Self {
        let mut attribute = Self::new(key, value);
        attribute.is_trusted_url = true;
        attribute
    }

    /// Creates an attribute **without** escaping the value.
    ///
    /// Winged-Swift uses this for the keys it controls itself — `Meta`'s `name`,
    /// `property`, `charset` and `http-equiv` are all inserted with `escape: false`.
    /// Never pass user data here.
    pub fn raw(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: Cow::Owned(key.into()),
            value: value.into(),
            is_boolean: false,
            is_trusted_url: false,
        }
    }

    /// Creates a boolean attribute, which renders as a bare key.
    ///
    /// `hidden`, `checked`, `required`, `open`, `controls`, `autoplay`, `muted`.
    pub fn boolean(key: impl Into<String>) -> Self {
        Self {
            key: Cow::Owned(key.into()),
            value: String::new(),
            is_boolean: true,
            is_trusted_url: false,
        }
    }

    /// The attribute name.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The escaped attribute value. Empty for boolean attributes.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Whether this attribute renders as a bare key.
    #[must_use]
    pub fn is_boolean(&self) -> bool {
        self.is_boolean
    }

    /// Appends ` key="value"`, or ` key` for a boolean attribute, to a buffer.
    ///
    /// A name that could break out of the attribute — space, quotes, `=`, `<`, `>`, `/`
    /// or a control character, and the empty name — is omitted. URL attributes are then
    /// checked by [`url_policy`](crate::core::url_policy); a blocked value is written as `#`. The check sees the
    /// stored value, so [`Self::raw`] is covered and [`Self::trusted_url`] is not.
    pub(crate) fn write_into(&self, out: &mut String) {
        self.write_into_in(out, false);
    }

    /// As [`Self::write_into`], for an attribute whose element sits inside a raw-text
    /// ancestor (`noscript`, `iframe`, `xmp`, `script`, `style`, …).
    ///
    /// Attribute escaping leaves `<` and `>` alone because neither can end a quoted value —
    /// and Winged-Swift's fixtures depend on that. Under a raw-text ancestor, though, the
    /// parser is not reading attributes at all: a `</noscript>` inside `alt="…"` ends the
    /// ancestor and what follows is markup. So there, and only there, `<` and `>` are
    /// written as references. The stored value never contained a `<` reference, so this
    /// cannot double-escape.
    pub(crate) fn write_into_in(&self, out: &mut String, in_raw_text: bool) {
        if !is_safe_attribute_name(&self.key) {
            return;
        }
        out.push(' ');
        out.push_str(&self.key);
        if self.is_boolean {
            return;
        }
        out.push_str("=\"");
        if self.is_trusted_url || !url_policy::is_blocked(&self.key, &self.value) {
            if in_raw_text {
                for c in self.value.chars() {
                    match c {
                        '<' => out.push_str("&lt;"),
                        '>' => out.push_str("&gt;"),
                        _ => out.push(c),
                    }
                }
            } else {
                out.push_str(&self.value);
            }
        } else {
            out.push_str(url_policy::REPLACEMENT);
        }
        out.push('"');
    }
}

/// An attribute name is emitted only when it cannot close the quotes or start a new
/// attribute. An empty name is omitted for the same reason: ` ="..."` is not a name.
fn is_safe_attribute_name(key: &str) -> bool {
    !key.is_empty()
        && !key.chars().any(|scalar| {
            scalar.is_control() || matches!(scalar, ' ' | '"' | '\'' | '=' | '<' | '>' | '/')
        })
}

impl std::fmt::Display for Attribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = String::new();
        self.write_into(&mut out);
        f.write_str(&out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_is_escaped_at_construction() {
        let attr = Attribute::new("title", r#"a "quoted" & <tagged> value"#);
        assert_eq!(attr.value(), "a &quot;quoted&quot; &amp; <tagged> value");
    }

    /// Ports the boolean-attribute cases in `HTML14FeaturesTests`.
    #[test]
    fn boolean_attributes_render_as_a_bare_key() {
        assert_eq!(Attribute::boolean("required").to_string(), " required");
        assert!(Attribute::boolean("open").is_boolean());
    }

    #[test]
    fn raw_attributes_are_not_escaped() {
        assert_eq!(Attribute::raw("property", "og:title").value(), "og:title");
    }

    #[test]
    fn non_boolean_attributes_render_with_a_quoted_value() {
        assert_eq!(
            Attribute::new("class", "card p-4").to_string(),
            r#" class="card p-4""#
        );
    }

    #[test]
    fn a_quote_in_the_value_cannot_break_out_of_the_attribute() {
        let attr = Attribute::new("class", r#"a" onload="alert(1)"#);
        assert!(!attr.value().contains('"'));
    }
}
