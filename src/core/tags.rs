//! The two tag sets the renderer branches on.
//!
//! Ports `HTMLTag.selfClosingTags` and `HTMLTag.whitespaceSensitiveTags` from
//! `Winged-Swift/Sources/WingedSwift/core/HTMLTag.swift`.

/// Elements that have no closing tag. Content and children on one of these are dropped.
///
/// Exactly the 13 entries in Winged-Swift's `HTMLTag.selfClosingTags`. Kept sorted so
/// [`is_void`] can binary-search it.
pub const VOID_TAGS: [&str; 13] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

/// Elements whose text content is significant, so pretty printing must not indent inside them.
pub const WHITESPACE_SENSITIVE_TAGS: [&str; 3] = ["pre", "code", "textarea"];

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
