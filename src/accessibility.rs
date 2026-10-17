//! Accessibility helpers and a debug-time audit.
//!
//! Winged-Swift has no accessibility module: its whole a11y surface is `ariaAttribute`,
//! `ariaAttributes` and `setRole` in `AttributeHelpers.swift`, plus two structural
//! affordances — `Iframe` requires a `title:`, and `Img` takes an `alt:`.
//!
//! This module adds the linter its `ROADMAP.md` has asked for since 1.5 and never shipped.

use crate::core::{Element, Node};

/// A landmark or widget role, for [`Element::set_role`](crate::Element::set_role).
///
/// Winged-Swift accepts any string. This enum catches the typos while
/// [`Role::Custom`] keeps the door open for roles it does not know.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Role {
    /// `role="banner"` — the page header.
    Banner,
    /// `role="navigation"`.
    Navigation,
    /// `role="main"`.
    Main,
    /// `role="complementary"` — a sidebar.
    Complementary,
    /// `role="contentinfo"` — the page footer.
    Contentinfo,
    /// `role="search"`.
    Search,
    /// `role="form"`.
    Form,
    /// `role="region"`.
    Region,
    /// `role="button"`.
    Button,
    /// `role="dialog"`.
    Dialog,
    /// `role="alert"`.
    Alert,
    /// `role="status"`.
    Status,
    /// Any other role, passed through verbatim.
    Custom(String),
}

impl Role {
    /// The attribute value for this role.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Banner => "banner",
            Self::Navigation => "navigation",
            Self::Main => "main",
            Self::Complementary => "complementary",
            Self::Contentinfo => "contentinfo",
            Self::Search => "search",
            Self::Form => "form",
            Self::Region => "region",
            Self::Button => "button",
            Self::Dialog => "dialog",
            Self::Alert => "alert",
            Self::Status => "status",
            Self::Custom(role) => role,
        }
    }
}

/// Something in the tree that will be hard or impossible to use with assistive technology.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct A11yIssue {
    /// The tag the problem was found on.
    pub tag: String,
    /// What is wrong, in one sentence.
    pub message: String,
}

/// Finds accessibility problems in a subtree.
///
/// The rules are the ones Winged-Swift's `ROADMAP.md` lists under "Quality":
///
/// - `<img>` without an `alt` attribute
/// - `<button>` with neither text nor an `aria-label`
/// - `<iframe>` without a `title`
/// - `<a>` with no accessible name
/// - `<input>` with neither an `aria-label` nor an `id` a `<label>` could point at
///
/// This is a linter, not a guarantee: it cannot tell whether an `alt` is *useful*, only
/// whether it exists.
///
/// # Examples
/// ```
/// use molt::prelude::*;
/// use molt::accessibility::audit;
///
/// let good = image("/a.png", "A cat");
/// assert_eq!(audit(&good.clone().into()), Vec::new());
///
/// let bad = Node::from(img().attr("src", "/a.png"));
/// assert_eq!(audit(&bad).len(), 1);
/// ```
#[must_use]
pub fn audit(node: &Node) -> Vec<A11yIssue> {
    let mut issues = Vec::new();
    walk(node, &mut issues);
    issues
}

/// Pre-order, with an explicit stack: the renderer is iterative so that a 100,000-level tree
/// is safe to render, and an audit that recursed would abort the process on the same tree.
fn walk(root: &Node, issues: &mut Vec<A11yIssue>) {
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        match node {
            Node::Element(element) => {
                check(element, issues);
                pending.extend(element.children().iter().rev());
            }
            Node::Fragment(children) => pending.extend(children.iter().rev()),
            Node::Text(_) | Node::Raw(_) | Node::Comment(_) => {}
        }
    }
}

fn check(element: &Element, issues: &mut Vec<A11yIssue>) {
    let has = |key: &str| element.attributes().iter().any(|a| a.key() == key);
    let mut report = |message: &str| {
        issues.push(A11yIssue {
            tag: element.tag().to_string(),
            message: message.to_string(),
        });
    };

    match element.tag() {
        "img" if !has("alt") => {
            report("an image needs an alt attribute; pass an empty one if it is decorative");
        }
        "iframe" if !has("title") => {
            report("an iframe needs a title describing its content");
        }
        "button" if !has_accessible_name(element) => {
            report("a button needs text content or an aria-label");
        }
        "a" if !has_accessible_name(element) => {
            report("a link needs text content or an aria-label");
        }
        "input" if !has("aria-label") && !has("id") && !has("aria-labelledby") => {
            report("an input needs an id a label can point at, or an aria-label");
        }
        _ => {}
    }
}

/// Whether an element has a name a screen reader can announce.
fn has_accessible_name(element: &Element) -> bool {
    if element
        .attributes()
        .iter()
        .any(|a| a.key() == "aria-label" || a.key() == "aria-labelledby")
    {
        return true;
    }
    if element.content().is_some_and(|c| !c.trim().is_empty()) {
        return true;
    }
    element.children().iter().any(|child| !child.is_empty())
}

/// Panics in debug builds if the subtree has accessibility problems.
///
/// Compiled away entirely in release builds, so it costs nothing in production.
///
/// # Panics
/// In debug builds, if [`audit`] reports any issue.
pub fn debug_assert_accessible(node: &Node) {
    if cfg!(debug_assertions) {
        let issues = audit(node);
        assert!(
            issues.is_empty(),
            "accessibility audit found {} issue(s): {:?}",
            issues.len(),
            issues
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elements::{a, button, div, iframe, iframe_titled, image, img, input, link_to};

    #[test]
    fn an_image_without_alt_is_reported() {
        let issues = audit(&img().attr("src", "/a.png").into());
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].tag, "img");
    }

    #[test]
    fn an_image_with_an_empty_alt_is_accepted_as_decorative() {
        assert_eq!(audit(&image("/a.png", "").into()), Vec::new());
    }

    #[test]
    fn an_iframe_without_a_title_is_reported() {
        assert_eq!(audit(&iframe().attr("src", "/e").into()).len(), 1);
        assert_eq!(audit(&iframe_titled("/e", "A map").into()), Vec::new());
    }

    #[test]
    fn a_button_needs_text_or_a_label() {
        assert_eq!(audit(&button().into()).len(), 1);
        assert_eq!(audit(&button().text("Send").into()), Vec::new());
        assert_eq!(
            audit(&button().aria_attr("label", "Send").into()),
            Vec::new()
        );
    }

    #[test]
    fn a_link_needs_an_accessible_name() {
        assert_eq!(audit(&a().attr("href", "/x").into()).len(), 1);
        assert_eq!(audit(&link_to("/x").text("Home").into()), Vec::new());
        assert_eq!(
            audit(&link_to("/x").child(image("/i.png", "Home")).into()),
            Vec::new()
        );
    }

    #[test]
    fn an_input_needs_something_a_label_can_attach_to() {
        assert_eq!(audit(&input().attr("type", "email").into()).len(), 1);
        assert_eq!(audit(&input().attr("id", "email").into()), Vec::new());
        assert_eq!(
            audit(&input().aria_attr("label", "E-mail").into()),
            Vec::new()
        );
    }

    #[test]
    fn the_audit_descends_into_children_and_fragments() {
        let tree = div().child(div().child(img().attr("src", "/a.png")));
        assert_eq!(audit(&tree.into()).len(), 1);

        let fragment = Node::fragment([img().attr("src", "/a.png").into(), button().into()]);
        assert_eq!(audit(&fragment).len(), 2);
    }

    #[test]
    fn a_clean_page_reports_nothing() {
        let tree = div()
            .child(image("/a.png", "A cat"))
            .child(button().text("Send"))
            .child(link_to("/x").text("Home"));
        assert_eq!(audit(&tree.into()), Vec::new());
    }

    #[test]
    fn roles_render_their_attribute_value() {
        assert_eq!(Role::Navigation.as_str(), "navigation");
        assert_eq!(Role::Custom("tooltip".into()).as_str(), "tooltip");
    }
}
