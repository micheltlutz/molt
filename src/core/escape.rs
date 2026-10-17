//! HTML and XML escaping.
//!
//! Winged-Swift has three distinct escapers and the golden fixtures depend on the
//! differences between them. `MOLT_SPEC.md` §3.3 shows only one; following the
//! spec here would break parity.
//!
//! | function | escapes | notes |
//! | --- | --- | --- |
//! | [`escape_text`] | `&` `<` `>` `"` `'` | `&` first; `'` becomes `&#x27;` |
//! | [`escape_attribute`] | `&` `"` `'` | **not** `<` or `>` |
//! | [`escape_xml`] | `&` `<` `>` `"` `'` | `'` becomes `&apos;`, the XML spelling |
//!
//! Escaping is applied **once, when content enters the tree** — never at render time.
//! That mirrors Winged-Swift, where `HTMLTag.init` stores already-escaped content, and it
//! is why [`crate::core::Node::Raw`] exists as the documented escape hatch.

/// Escapes text for use as HTML element content.
///
/// Set `escape_slashes` to also turn `/` into `&#x2F;`. It defaults to off in
/// [`escape_text`] because `&`, `<`, `>`, `"` and `'` already close the XSS surface, and
/// escaping every slash turns dates, paths and "and/or" into unreadable entities.
///
/// # Examples
/// ```
/// use molt::core::escape::escape_text;
/// assert_eq!(
///     escape_text("<script>alert('XSS')</script>"),
///     "&lt;script&gt;alert(&#x27;XSS&#x27;)&lt;/script&gt;"
/// );
/// ```
#[must_use]
pub fn escape_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    write_escaped_text(&mut out, input, false);
    out
}

/// Escapes text for HTML content, optionally escaping `/` as well.
///
/// # Examples
/// ```
/// use molt::core::escape::escape_text_with_slashes;
/// assert_eq!(escape_text_with_slashes("a/b"), "a&#x2F;b");
/// ```
#[must_use]
pub fn escape_text_with_slashes(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    write_escaped_text(&mut out, input, true);
    out
}

/// Escapes a value for use inside a double-quoted HTML attribute.
///
/// Deliberately narrower than [`escape_text`]: `<` and `>` are left alone, because they
/// cannot terminate a quoted attribute value. Winged-Swift's `HTMLEscape.escapeAttribute`
/// does the same, and `marketing-pretty.html` contains attribute values that prove it.
///
/// # Examples
/// ```
/// use molt::core::escape::escape_attribute;
/// assert_eq!(escape_attribute(r#"a "b" & <c>"#), "a &quot;b&quot; &amp; <c>");
/// ```
#[must_use]
pub fn escape_attribute(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    write_escaped_attribute(&mut out, input);
    out
}

/// Escapes text for XML content — used by the sitemap and RSS generators.
///
/// Differs from [`escape_text`] in one character: `'` becomes `&apos;`, which is an XML
/// entity, rather than `&#x27;`.
///
/// # Examples
/// ```
/// use molt::core::escape::escape_xml;
/// assert_eq!(escape_xml("Tom & Jerry's <show>"), "Tom &amp; Jerry&apos;s &lt;show&gt;");
/// ```
#[must_use]
pub fn escape_xml(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Appends escaped HTML text to an existing buffer.
///
/// The whole renderer writes into one buffer; allocating a fresh `String` per escape
/// would defeat that.
pub(crate) fn write_escaped_text(out: &mut String, input: &str, escape_slashes: bool) {
    out.reserve(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            '/' if escape_slashes => out.push_str("&#x2F;"),
            _ => out.push(c),
        }
    }
}

/// Appends an escaped attribute value to an existing buffer.
pub(crate) fn write_escaped_attribute(out: &mut String, input: &str) {
    out.reserve(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// No single Swift counterpart: this covers in one assertion what `testEscapeAmpersand`
    /// and `testEscapeQuotes` check separately, plus the angle brackets.
    #[test]
    fn escapes_the_five_html_characters() {
        assert_eq!(escape_text("<>&\"'"), "&lt;&gt;&amp;&quot;&#x27;");
    }

    /// Ports `HTMLEscapeTests.testEscapeBasicHTML`.
    #[test]
    fn escapes_a_script_payload() {
        assert_eq!(
            escape_text("<script>alert('XSS')</script>"),
            "&lt;script&gt;alert(&#x27;XSS&#x27;)&lt;/script&gt;"
        );
    }

    /// The ampersand must be replaced first, or every other entity gets double-escaped.
    #[test]
    fn ampersand_is_escaped_before_the_entities_it_introduces() {
        assert_eq!(escape_text("a & b < c"), "a &amp; b &lt; c");
    }

    /// Ports `HTMLEscapeTests.testSlashesAreKeptByDefault` and
    /// `HTMLEscapeTests.testSlashesCanBeEscapedExplicitly`.
    #[test]
    fn slashes_are_only_escaped_on_request() {
        assert_eq!(escape_text("2026/09/16"), "2026/09/16");
        assert_eq!(
            escape_text_with_slashes("2026/09/16"),
            "2026&#x2F;09&#x2F;16"
        );
    }

    /// Ports `HTMLEscapeTests.testAttributeEscape`. Attribute context leaves `<` and `>`
    /// alone — they cannot terminate a quoted value.
    #[test]
    fn attribute_escaping_leaves_angle_brackets_alone() {
        assert_eq!(
            escape_attribute(r#"a "b" & <c>"#),
            "a &quot;b&quot; &amp; <c>"
        );
    }

    #[test]
    fn xml_uses_the_apos_entity_where_html_uses_a_numeric_reference() {
        assert_eq!(escape_xml("it's"), "it&apos;s");
        assert_eq!(escape_text("it's"), "it&#x27;s");
    }

    /// Escaping is applied exactly once, by the caller, at construction. Applying it
    /// twice double-escapes — the same behaviour Winged-Swift has, and the reason the
    /// builder never re-escapes an existing attribute value.
    #[test]
    fn escaping_is_not_idempotent_by_design() {
        assert_eq!(escape_text(&escape_text("a & b")), "a &amp;amp; b");
    }

    /// Nothing survives that could open a tag or start an entity.
    #[test]
    fn output_never_contains_a_bare_angle_bracket() {
        for input in ["<", "<<>>", "a<b>c", "&<>\"'"] {
            let escaped = escape_text(input);
            assert!(!escaped.contains('<'), "{input:?} produced {escaped:?}");
            assert!(!escaped.contains('>'), "{input:?} produced {escaped:?}");
        }
    }

    #[test]
    fn non_ascii_passes_through_untouched() {
        assert_eq!(escape_text("R$ 9,90/mês — ação"), "R$ 9,90/mês — ação");
    }

    #[test]
    fn empty_input_produces_empty_output() {
        assert_eq!(escape_text(""), "");
        assert_eq!(escape_attribute(""), "");
        assert_eq!(escape_xml(""), "");
    }

    /// Ports `HTMLEscapeTests.testEscapeAmpersand`.
    #[test]
    fn an_ampersand_becomes_an_entity() {
        assert_eq!(escape_text("Tom & Jerry"), "Tom &amp; Jerry");
    }

    /// Ports `HTMLEscapeTests.testEscapeQuotes`.
    #[test]
    fn double_quotes_become_entities() {
        assert_eq!(
            escape_text(r#"He said "Hello""#),
            "He said &quot;Hello&quot;"
        );
    }
}
