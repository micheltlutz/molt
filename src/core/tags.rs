//! The two tag sets the renderer branches on.
//!
//! Ports `HTMLTag.selfClosingTags` and `HTMLTag.whitespaceSensitiveTags` from
//! `Winged-Swift/Sources/WingedSwift/core/HTMLTag.swift`.

/// Elements that have no closing tag. Content and children on one of these are dropped.
///
/// Exactly the 13 entries in Winged-Swift's `HTMLTag.selfClosingTags`. Kept sorted so
/// [`is_void`] can binary-search it.
///
/// A slice, not an array, so the length is not part of the type: adding a tag is not a
/// semver break.
pub const VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

/// Elements whose text content is significant, so pretty printing must not indent inside them.
pub const WHITESPACE_SENSITIVE_TAGS: &[&str] = &["pre", "code", "textarea"];

/// Elements whose content the HTML parser reads as raw text rather than as markup.
///
/// Inside one of these, the first `</tag>` ends the element no matter where it appears —
/// including inside a quoted attribute value of a descendant. `noscript` is on the list
/// because browsers parse it as raw text whenever scripting is enabled, which is the case
/// that matters (a tracking `<img>` in `<noscript>` is common).
const RAW_TEXT_TAGS: [&str; 8] = [
    "iframe",
    "noembed",
    "noframes",
    "noscript",
    "plaintext",
    "script",
    "style",
    "xmp",
];

/// Whether the parser reads the content of `tag` as raw text.
pub(crate) fn is_raw_text(tag: &str) -> bool {
    RAW_TEXT_TAGS.contains(&tag)
}

/// Whether `tag` is safe to write between `<` and `>`.
///
/// Attribute names were already checked at write time; a tag name is written just as
/// verbatim, so `Element::new("img src=x onerror=alert(1)")` would otherwise inject an
/// attribute and `Element::new("div><script>…")` would inject an element. The accepted
/// shape is the HTML "tag name" production narrowed to ASCII: a letter, then letters,
/// digits, `-`, `_`, `.` or `:`. That admits custom elements (`my-card`) and prefixed names
/// (`svg:rect`), and nothing that can end a tag.
///
/// The renderer skips an element whose tag fails this check, children included.
///
/// # Examples
/// ```
/// use molt::core::tags::is_valid_tag_name;
/// assert!(is_valid_tag_name("div"));
/// assert!(is_valid_tag_name("my-card"));
/// assert!(!is_valid_tag_name("img src=x onerror=alert(1)"));
/// assert!(!is_valid_tag_name("div><script>"));
/// assert!(!is_valid_tag_name(""));
/// ```
#[must_use]
pub fn is_valid_tag_name(tag: &str) -> bool {
    let mut bytes = tag.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
}

/// Whether `tag` is a void element.
///
/// # Examples
/// ```
/// use molt::core::tags::is_void;
/// assert!(is_void("img"));
/// assert!(!is_void("div"));
/// ```
#[must_use]
pub fn is_void(tag: &str) -> bool {
    VOID_TAGS.binary_search(&tag).is_ok()
}

/// Whether `tag` renders its whitespace literally.
///
/// # Examples
/// ```
/// use molt::core::tags::is_whitespace_sensitive;
/// assert!(is_whitespace_sensitive("pre"));
/// assert!(!is_whitespace_sensitive("p"));
/// ```
#[must_use]
pub fn is_whitespace_sensitive(tag: &str) -> bool {
    WHITESPACE_SENSITIVE_TAGS.contains(&tag)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_void_set_matches_winged_swift() {
        for tag in [
            "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source",
            "track", "wbr",
        ] {
            assert!(is_void(tag), "{tag} should be void");
        }
        for tag in ["div", "p", "span", "table", "section"] {
            assert!(!is_void(tag), "{tag} should not be void");
        }
    }

    #[test]
    fn only_pre_code_and_textarea_are_whitespace_sensitive() {
        assert!(is_whitespace_sensitive("pre"));
        assert!(is_whitespace_sensitive("code"));
        assert!(is_whitespace_sensitive("textarea"));
        assert!(!is_whitespace_sensitive("div"));
    }
}
