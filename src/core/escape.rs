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

use std::borrow::Cow;

/// Each `*_entity` function below maps one byte to its entity, or `None` if the byte is
/// written as it is. They are passed as generic parameters, never as `fn` pointers, so the
/// compiler inlines the match into the scanning loop.
///
/// Every escaped character is ASCII, and no byte of a multi-byte UTF-8 sequence is, so a
/// scan over bytes finds exactly the characters a `char` loop would, and slicing the input
/// at a matched byte never splits a character.
fn text_entity(byte: u8) -> Option<&'static str> {
    match byte {
        b'&' => Some("&amp;"),
        b'<' => Some("&lt;"),
        b'>' => Some("&gt;"),
        b'"' => Some("&quot;"),
        b'\'' => Some("&#x27;"),
        _ => None,
    }
}

fn text_entity_with_slashes(byte: u8) -> Option<&'static str> {
    if byte == b'/' {
        Some("&#x2F;")
    } else {
        text_entity(byte)
    }
}

/// `<` and `>` are not here on purpose: see [`escape_attribute`].
fn attribute_entity(byte: u8) -> Option<&'static str> {
    match byte {
        b'&' => Some("&amp;"),
        b'"' => Some("&quot;"),
        b'\'' => Some("&#x27;"),
        _ => None,
    }
}

/// The only difference from [`text_entity`] is the apostrophe, which XML spells `&apos;`.
fn xml_entity(byte: u8) -> Option<&'static str> {
    if byte == b'\'' {
        Some("&apos;")
    } else {
        text_entity(byte)
    }
}

/// Appends `input` to `out`, replacing each byte `entity` names.
///
/// Copies the stretches between escapes with one `push_str` each instead of pushing a
/// `char` at a time, which is what makes mostly-clean text cheap.
fn write_runs(out: &mut String, input: &str, entity: impl Fn(u8) -> Option<&'static str>) {
    let mut start = 0;
    for (index, byte) in input.bytes().enumerate() {
        if let Some(replacement) = entity(byte) {
            if index > start {
                out.push_str(&input[start..index]);
            }
            out.push_str(replacement);
            start = index + 1;
        }
    }
    out.push_str(&input[start..]);
}

/// A [`std::fmt::Write`] that escapes what it receives as element text and appends it to a
/// buffer.
///
/// Lets `html!` format a `Display` value straight into its escaped form instead of going
/// through `to_string()` and escaping a second copy. Escaping is local to a character, so
/// escaping each piece a `Display` impl writes gives the same result as escaping the whole.
pub(crate) struct TextWriter<'a>(pub(crate) &'a mut String);

impl std::fmt::Write for TextWriter<'_> {
    fn write_str(&mut self, piece: &str) -> std::fmt::Result {
        write_escaped_text(self.0, piece, false);
        Ok(())
    }
}

/// As [`TextWriter`], with attribute-value escaping.
pub(crate) struct AttributeWriter<'a>(pub(crate) &'a mut String);

impl std::fmt::Write for AttributeWriter<'_> {
    fn write_str(&mut self, piece: &str) -> std::fmt::Result {
        write_escaped_attribute(self.0, piece);
        Ok(())
    }
}

/// Escapes `input` only if some byte needs it, and otherwise hands it back untouched.
///
/// Returning [`Cow`] means the common case — text with nothing to escape — costs no
/// allocation and no copy.
fn escape_with(input: &str, entity: impl Fn(u8) -> Option<&'static str> + Copy) -> Cow<'_, str> {
    let Some(first) = input.bytes().position(|byte| entity(byte).is_some()) else {
        return Cow::Borrowed(input);
    };
    // Room for a few entities up front, so a short dirty string does not grow once.
    let mut out = String::with_capacity(input.len() + input.len() / 4 + 16);
    out.push_str(&input[..first]);
    write_runs(&mut out, &input[first..], entity);
    Cow::Owned(out)
}

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
pub fn escape_text(input: &str) -> Cow<'_, str> {
    escape_with(input, text_entity)
}

/// Escapes text for HTML content, optionally escaping `/` as well.
///
/// # Examples
/// ```
/// use molt::core::escape::escape_text_with_slashes;
/// assert_eq!(escape_text_with_slashes("a/b"), "a&#x2F;b");
/// ```
#[must_use]
pub fn escape_text_with_slashes(input: &str) -> Cow<'_, str> {
    escape_with(input, text_entity_with_slashes)
}

/// Escapes a value for use inside a double-quoted HTML attribute.
///
/// Deliberately narrower than [`escape_text`]: `<` and `>` are left alone, because they
/// cannot terminate a quoted attribute value. Winged-Swift's `HTMLEscape.escapeAttribute`
/// does the same, and `marketing-pretty.html` contains attribute values that prove it.
///
/// That holds wherever the parser reads attributes. Under a raw-text ancestor (`noscript`,
/// `iframe`, `xmp`, `script`, `style`, …) it does not, so the renderer escapes `<` and `>`
/// itself there — this function cannot, because it never sees the ancestors. Divergence
/// from Winged-Swift, on purpose; see `SECURITY.md`.
///
/// # Examples
/// ```
/// use molt::core::escape::escape_attribute;
/// assert_eq!(escape_attribute(r#"a "b" & <c>"#), "a &quot;b&quot; &amp; <c>");
/// ```
#[must_use]
pub fn escape_attribute(input: &str) -> Cow<'_, str> {
    escape_with(input, attribute_entity)
}

/// Escapes text for XML content — used by the sitemap and RSS generators.
///
/// Differs from [`escape_text`] in one character: `'` becomes `&apos;`, which is an XML
/// entity, rather than `&#x27;`.
///
/// It also **drops** the characters XML 1.0 cannot represent at all, even escaped: the
/// control characters U+0000–U+0008, U+000B, U+000C and U+000E–U+001F, and the
/// non-characters U+FFFE and U+FFFF. One of them in a title makes the whole document
/// ill-formed, so a feed reader or crawler rejects every entry, not just that one. Tab,
/// newline and carriage return are legal and kept.
///
/// # Examples
/// ```
/// use molt::core::escape::escape_xml;
/// assert_eq!(escape_xml("Tom & Jerry's <show>"), "Tom &amp; Jerry&apos;s &lt;show&gt;");
/// assert_eq!(escape_xml("a\u{0}b\u{8}c\u{FFFE}d\te"), "abcd\te");
/// ```
#[must_use]
pub fn escape_xml(input: &str) -> Cow<'_, str> {
    if !input.chars().any(needs_xml_change) {
        return Cow::Borrowed(input);
    }
    let mut out = String::with_capacity(input.len() + input.len() / 4 + 16);
    for c in input.chars() {
        if is_xml_illegal(c) {
            continue;
        }
        match u8::try_from(c).ok().and_then(xml_entity) {
            Some(entity) => out.push_str(entity),
            None => out.push(c),
        }
    }
    Cow::Owned(out)
}

/// Whether XML 1.0 has no way to write `c`, escaped or not.
fn is_xml_illegal(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{8}' | '\u{B}' | '\u{C}' | '\u{E}'..='\u{1F}' | '\u{FFFE}' | '\u{FFFF}')
}

fn needs_xml_change(c: char) -> bool {
    is_xml_illegal(c) || u8::try_from(c).ok().and_then(xml_entity).is_some()
}

/// Appends escaped HTML text to an existing buffer.
///
/// The whole renderer writes into one buffer; allocating a fresh `String` per escape
/// would defeat that.
pub(crate) fn write_escaped_text(out: &mut String, input: &str, escape_slashes: bool) {
    out.reserve(input.len());
    write_runs(
        out,
        input,
        if escape_slashes {
            text_entity_with_slashes
        } else {
            text_entity
        },
    );
}

/// Appends an escaped attribute value to an existing buffer.
pub(crate) fn write_escaped_attribute(out: &mut String, input: &str) {
    out.reserve(input.len());
    write_runs(out, input, attribute_entity);
}

#[cfg(test)]
mod tests {
    /// The boundaries of XML 1.0's `Char` production, one on each side.
    #[test]
    fn xml_drops_exactly_the_characters_xml_1_0_cannot_hold() {
        for illegal in [
            '\u{0}', '\u{8}', '\u{B}', '\u{C}', '\u{E}', '\u{1F}', '\u{FFFE}', '\u{FFFF}',
        ] {
            assert_eq!(
                escape_xml(&format!("a{illegal}b")),
                "ab",
                "U+{:04X}",
                illegal as u32
            );
        }
        for legal in [
            '\t',
            '\n',
            '\r',
            ' ',
            '\u{7F}',
            '\u{80}',
            '\u{FFFD}',
            '\u{10000}',
            'é',
        ] {
            assert_eq!(
                escape_xml(&format!("a{legal}b")),
                format!("a{legal}b"),
                "U+{:04X}",
                legal as u32
            );
        }
        assert!(matches!(escape_xml("plain"), std::borrow::Cow::Borrowed(_)));
        assert_eq!(escape_xml("<\u{0}&"), "&lt;&amp;");
    }

    /// Text with nothing to escape comes back borrowed: no allocation, no copy.
    #[test]
    fn clean_input_is_returned_without_allocating() {
        use std::borrow::Cow;
        for clean in ["", "plain text", "ação — 日本語 ✓", "2026/09/16"] {
            assert!(matches!(escape_text(clean), Cow::Borrowed(_)), "{clean:?}");
            assert!(
                matches!(escape_attribute(clean), Cow::Borrowed(_)),
                "{clean:?}"
            );
            assert!(matches!(escape_xml(clean), Cow::Borrowed(_)), "{clean:?}");
        }
        assert!(matches!(escape_text("a & b"), Cow::Owned(_)));
        // `/` is only special when asked for, and `<` is not special to an attribute.
        assert!(matches!(escape_text_with_slashes("a/b"), Cow::Owned(_)));
        assert!(matches!(escape_attribute("a < b"), Cow::Borrowed(_)));
    }

    /// The byte scan must agree with the `char` loop for every special, next to multi-byte text.
    #[test]
    fn byte_scanning_finds_specials_beside_multibyte_characters() {
        assert_eq!(escape_text("é<é"), "é&lt;é");
        assert_eq!(escape_attribute("日\"本"), "日&quot;本");
        assert_eq!(escape_xml("✓'✓"), "✓&apos;✓");
    }

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
