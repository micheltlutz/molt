//! The [`html!`](crate::html) declarative macro.
//!
//! Rust's answer to Winged-Swift's `@HTMLBuilder` / `@HTMLFragmentBuilder` result builders
//! (`Sources/WingedSwift/functions/HTMLFunctions.swift`). It is **sugar over the builder
//! API, never a second code path** — every form here expands to the same `Element` and
//! `Node` calls you would write by hand, so there is exactly one renderer to keep correct.

use crate::core::{Element, Node};

/// Builds a node tree from nested markup-like syntax.
///
/// # Syntax
///
/// | form | meaning |
/// | --- | --- |
/// | `div { … }` | an element with children |
/// | `div(class = "card", id = "x") { … }` | attributes, then children |
/// | `input(type = "email", required)` | a bare name is a boolean attribute |
/// | `"text"` | escaped text |
/// | `(expr)` | an escaped `Display` expression |
/// | `raw(expr)` | unescaped markup |
/// | `@if cond { … } @else { … }` | conditional, empty branch renders to nothing |
/// | `@for pat in iter { … }` | repetition |
///
/// # Examples
///
/// ```
/// use molt::{html, prelude::*};
///
/// let page = html! {
///     div(class = "card", id = "hero") {
///         h1 { "Welcome" }
///         p { "Fuel, tyres & chain." }
///     }
/// };
///
/// assert_eq!(
///     page.render(),
///     r#"<div class="card" id="hero"><h1>Welcome</h1><p>Fuel, tyres &amp; chain.</p></div>"#
/// );
/// ```
///
/// Conditionals and loops, matching `buildOptional` / `buildEither` / `buildArray`:
///
/// ```
/// use molt::{html, prelude::*};
///
/// let names = ["Ana", "Bruno"];
/// let logged_in = false;
///
/// let list = html! {
///     ul {
///         @for name in names { li { (name) } }
///         @if logged_in { li { "Sign out" } }
///     }
/// };
///
/// assert_eq!(list.render(), "<ul><li>Ana</li><li>Bruno</li></ul>");
/// ```
///
/// The expansion is the builder API, so the two are interchangeable:
///
/// ```
/// use molt::{html, prelude::*};
/// assert_eq!(
///     html! { p(class = "lead") { "hi" } }.render(),
///     Node::from(p().add_class("lead").child(Node::text("hi"))).render()
/// );
/// ```
#[macro_export]
macro_rules! html {
    // A single root node.
    ($($body:tt)*) => {{
        let mut __nodes = $crate::macros::node_buffer();
        $crate::html_nodes!(__nodes, $($body)*);
        if __nodes.len() == 1 {
            __nodes.pop().unwrap_or_else(|| $crate::Node::Fragment(::std::vec::Vec::new()))
        } else {
            $crate::Node::Fragment(__nodes)
        }
    }};
}

/// Accumulates sibling nodes into a `Vec`. Not part of the public API.
#[doc(hidden)]
#[macro_export]
macro_rules! html_nodes {
    // Terminal.
    ($out:ident,) => {};

    // @if and @for hand off to a token muncher. A `$cond:expr` fragment cannot be
    // used before a brace: the expression parser would read `show { … }` as a struct
    // literal and macro_rules cannot backtrack once a fragment has been consumed.
    ($out:ident, @if $($tail:tt)*) => { $crate::html_if!($out, [] $($tail)*); };
    ($out:ident, @for $($tail:tt)*) => { $crate::html_for!($out, [] $($tail)*); };

    // raw(expr) — unescaped markup.
    ($out:ident, raw($value:expr) $($rest:tt)*) => {
        $out.push($crate::Node::raw(::std::string::ToString::to_string(&$value)));
        $crate::html_nodes!($out, $($rest)*);
    };

    // Element with attributes and children.
    ($out:ident, $tag:ident ( $($attrs:tt)* ) { $($children:tt)* } $($rest:tt)*) => {
        {
            let __el = $crate::elements::$tag();
            let __el = $crate::html_attrs!(__el, $($attrs)*);
            let mut __kids = $crate::macros::node_buffer();
            $crate::html_nodes!(__kids, $($children)*);
            $out.push($crate::Node::Element(
                $crate::macros::element_with_children(__el, __kids)));
        }
        $crate::html_nodes!($out, $($rest)*);
    };

    // Element with attributes only.
    ($out:ident, $tag:ident ( $($attrs:tt)* ) $($rest:tt)*) => {
        {
            let __el = $crate::elements::$tag();
            $out.push($crate::Node::Element($crate::html_attrs!(__el, $($attrs)*)));
        }
        $crate::html_nodes!($out, $($rest)*);
    };

    // Element with children only.
    ($out:ident, $tag:ident { $($children:tt)* } $($rest:tt)*) => {
        {
            let mut __kids = $crate::macros::node_buffer();
            $crate::html_nodes!(__kids, $($children)*);
            $out.push($crate::Node::Element(
                $crate::macros::element_with_children($crate::elements::$tag(), __kids)));
        }
        $crate::html_nodes!($out, $($rest)*);
    };

    // Interpolated expression, escaped.
    ($out:ident, ($value:expr) $($rest:tt)*) => {
        $out.push($crate::macros::display_text(&$value));
        $crate::html_nodes!($out, $($rest)*);
    };

    // Literal text, escaped.
    ($out:ident, $text:literal $($rest:tt)*) => {
        $out.push($crate::Node::text($text));
        $crate::html_nodes!($out, $($rest)*);
    };

    // A bare element with no attributes and no children.
    ($out:ident, $tag:ident $($rest:tt)*) => {
        $out.push($crate::Node::Element($crate::elements::$tag()));
        $crate::html_nodes!($out, $($rest)*);
    };
}

/// Applies an attribute list to an element. Not part of the public API.
#[doc(hidden)]
#[macro_export]
macro_rules! html_attrs {
    ($el:expr,) => { $el };
    ($el:expr) => { $el };

    // "key" = value — a string key, for names that are Rust keywords such as `type`.
    ($el:expr, $key:literal = $value:expr, $($rest:tt)*) => {
        $crate::html_attrs!(
            $crate::macros::attr_display($el, $key, &$value), $($rest)*)
    };
    ($el:expr, $key:literal = $value:expr) => {
        $crate::macros::attr_display($el, $key, &$value)
    };

    // key = value
    ($el:expr, $key:ident = $value:expr, $($rest:tt)*) => {
        $crate::html_attrs!(
            $crate::macros::attr_display($el, stringify!($key), &$value), $($rest)*)
    };
    ($el:expr, $key:ident = $value:expr) => {
        $crate::macros::attr_display($el, stringify!($key), &$value)
    };

    // A bare name is a boolean attribute.
    ($el:expr, $key:literal, $($rest:tt)*) => {
        $crate::html_attrs!($el.bool_attr($key), $($rest)*)
    };
    ($el:expr, $key:literal) => {
        $el.bool_attr($key)
    };
    ($el:expr, $key:ident, $($rest:tt)*) => {
        $crate::html_attrs!($el.bool_attr(stringify!($key)), $($rest)*)
    };
    ($el:expr, $key:ident) => {
        $el.bool_attr(stringify!($key))
    };
}

/// A text node from a `Display` value, escaped as it is formatted. Not public API.
#[doc(hidden)]
#[must_use]
pub fn display_text(value: &impl std::fmt::Display) -> Node {
    Node::text_display(value)
}

/// Adds `key = value` to an element, formatting the value straight into escaped form. The
/// key is always a literal in the macro, so it is borrowed, not allocated. Not public API.
#[doc(hidden)]
#[must_use]
pub fn attr_display(
    element: Element,
    key: &'static str,
    value: &impl std::fmt::Display,
) -> Element {
    element.add_attribute(crate::core::Attribute::from_display(key, value))
}

/// An empty sibling buffer for the macro to push into.
///
/// A function rather than an inline `Vec::new()` so that `clippy::vec_init_then_push` does
/// not fire inside every `html!` expansion in every downstream crate. The lint is about
/// hand-written style; this code is generated.
#[doc(hidden)]
#[must_use]
pub fn node_buffer() -> Vec<Node> {
    Vec::new()
}

/// Attaches macro-collected children to an element.
///
/// When every child is text, it becomes the element's **content** rather than a list of
/// child nodes. That is what the builder API produces for `p().text("x")`, and the two
/// render differently once pretty printing is on: content stays on the element's line,
/// while a child text node gets its own indented line. Collapsing here is what keeps the
/// macro true sugar over the builder rather than a second, subtly different code path.
#[doc(hidden)]
#[must_use]
pub fn element_with_children(element: Element, children: Vec<Node>) -> Element {
    let all_text = !children.is_empty() && children.iter().all(|c| matches!(c, Node::Text(_)));

    if all_text {
        // The first text node's string is reused as the buffer, so the common single-text
        // child (`li { (name) }`) is moved, not copied.
        let mut texts = children.into_iter().filter_map(|child| match child {
            Node::Text(text) => Some(text.into_string()),
            _ => None,
        });
        let mut content = texts.next().unwrap_or_default();
        for text in texts {
            content.push_str(&text);
        }
        // The text was escaped when each node was built, so it goes in verbatim.
        return element.raw_text(content);
    }

    element.children_from(children)
}

/// Token muncher for `@if`. Collects condition tokens until the body block. Not public.
#[doc(hidden)]
#[macro_export]
macro_rules! html_if {
    ($out:ident, [$($cond:tt)+] { $($body:tt)* } @else { $($alt:tt)* } $($rest:tt)*) => {
        if $($cond)+ { $crate::html_nodes!($out, $($body)*); }
        else { $crate::html_nodes!($out, $($alt)*); }
        $crate::html_nodes!($out, $($rest)*);
    };
    ($out:ident, [$($cond:tt)+] { $($body:tt)* } $($rest:tt)*) => {
        if $($cond)+ { $crate::html_nodes!($out, $($body)*); }
        $crate::html_nodes!($out, $($rest)*);
    };
    ($out:ident, [$($cond:tt)*] $next:tt $($rest:tt)*) => {
        $crate::html_if!($out, [$($cond)* $next] $($rest)*);
    };
}

/// Token muncher for `@for`. Collects `pat in iter` until the body block. Not public.
#[doc(hidden)]
#[macro_export]
macro_rules! html_for {
    ($out:ident, [$($head:tt)+] { $($body:tt)* } $($rest:tt)*) => {
        for $($head)+ { $crate::html_nodes!($out, $($body)*); }
        $crate::html_nodes!($out, $($rest)*);
    };
    ($out:ident, [$($head:tt)*] $next:tt $($rest:tt)*) => {
        $crate::html_for!($out, [$($head)* $next] $($rest)*);
    };
}

#[cfg(test)]
mod tests {
    /// A `Display` impl that writes in several pieces, each holding a special character.
    /// Escaping per piece must equal escaping the whole.
    struct Pieces;

    impl std::fmt::Display for Pieces {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a<")?;
            f.write_str("b&")?;
            write!(f, "{}\"c'", 7)
        }
    }

    #[test]
    fn a_display_value_is_escaped_as_it_is_formatted() {
        let text = crate::html! { p { (Pieces) } };
        assert_eq!(text.render(), "<p>a&lt;b&amp;7&quot;c&#x27;</p>");
        let attribute = crate::html! { p(title = Pieces) { "x" } };
        assert_eq!(
            attribute.render(),
            "<p title=\"a<b&amp;7&quot;c&#x27;\">x</p>"
        );
        assert_eq!(
            text.render(),
            Node::from(crate::elements::p().text(Pieces.to_string())).render()
        );
    }

    #[test]
    fn numbers_and_keyword_keys_still_work_as_attributes() {
        let image = crate::html! { img(width = 100, "type" = 'x', hidden) };
        assert_eq!(image.render(), r#"<img width="100" type="x" hidden>"#);
    }

    #[test]
    fn several_text_children_are_joined_into_one_content_string() {
        let paragraph = crate::html! { p { "a " (1) " <b>" } };
        assert_eq!(paragraph.render(), "<p>a 1 &lt;b&gt;</p>");
    }

    use crate::prelude::*;

    #[test]
    fn a_bare_element_renders_empty() {
        assert_eq!(html! { div }.render(), "<div></div>");
    }

    #[test]
    fn text_children_are_escaped() {
        assert_eq!(html! { p { "a & b" } }.render(), "<p>a &amp; b</p>");
    }

    #[test]
    fn raw_children_are_not_escaped() {
        assert_eq!(html! { p { raw("<b>x</b>") } }.render(), "<p><b>x</b></p>");
    }

    #[test]
    fn attributes_and_children_combine() {
        assert_eq!(
            html! { div(class = "card", id = "hero") { p { "hi" } } }.render(),
            r#"<div class="card" id="hero"><p>hi</p></div>"#
        );
    }

    #[test]
    fn a_bare_attribute_name_is_a_boolean_attribute() {
        assert_eq!(
            html! { input("type" = "email", name = "email", required) }.render(),
            r#"<input type="email" name="email" required>"#
        );
    }

    #[test]
    fn expressions_interpolate_escaped() {
        let name = "Tom & Jerry";
        assert_eq!(
            html! { span { (name) } }.render(),
            "<span>Tom &amp; Jerry</span>"
        );
    }

    /// Matches `buildArray` in Winged-Swift's `HTMLBuilder`.
    #[test]
    fn for_loops_expand_to_siblings() {
        let names = ["Ana", "Bruno"];
        assert_eq!(
            html! { ul { @for n in names { li { (n) } } } }.render(),
            "<ul><li>Ana</li><li>Bruno</li></ul>"
        );
    }

    /// Matches `buildOptional`: a false branch renders to nothing at all.
    #[test]
    fn a_false_condition_contributes_no_node() {
        let show = false;
        assert_eq!(
            html! { div { @if show { p { "x" } } } }.render(),
            "<div></div>"
        );
    }

    /// Matches `buildEither`.
    #[test]
    fn if_else_picks_one_branch() {
        let logged_in = true;
        let markup = html! {
            nav { @if logged_in { a { "Sign out" } } @else { a { "Sign in" } } }
        };
        assert_eq!(markup.render(), "<nav><a>Sign out</a></nav>");
    }

    #[test]
    fn several_roots_become_a_fragment() {
        assert_eq!(html! { p { "a" } p { "b" } }.render(), "<p>a</p><p>b</p>");
    }

    /// The macro is sugar: it must produce exactly what the builder produces.
    #[test]
    fn macro_output_equals_the_builder_output() {
        let from_macro = html! { div(class = "card") { h1 { "T" } p { "B" } } };
        let from_builder = Node::from(
            div()
                .add_class("card")
                .child(h1().text("T"))
                .child(p().text("B")),
        );
        assert_eq!(from_macro.render(), from_builder.render());
        assert_eq!(from_macro.render_pretty(), from_builder.render_pretty());
    }

    #[test]
    fn nesting_survives_pretty_printing() {
        let markup = html! { div { section { p { "deep" } } } };
        assert_eq!(
            markup.render_pretty(),
            "<div>\n  <section>\n    <p>deep</p>\n  </section>\n</div>"
        );
    }

    // Ports `BuilderInitTests`, whose thesis is that Swift's result-builder initialiser is
    // *exactly* equivalent to the array one. `html!` makes the same promise here — rule 9
    // of AGENTS.md: sugar over the builder, never a second code path — so each of these
    // asserts the macro and the builder agree, and then pins the markup.

    /// Ports `BuilderInitTests.plainContainer`.
    #[test]
    fn the_macro_and_the_builder_agree_on_a_plain_container() {
        let macro_built = html! { div { p { "Hi" } } };
        let builder_built = div().child(p().text("Hi"));

        assert_eq!(macro_built.render(), builder_built.render());
        assert_eq!(macro_built.render(), "<div><p>Hi</p></div>");
    }

    /// Ports `BuilderInitTests.containerWithRequiredAttribute`.
    #[test]
    fn the_macro_and_the_builder_agree_on_a_required_attribute() {
        let macro_built = html! { a(href = "/docs") { span { "Docs" } } };
        let builder_built = link_to("/docs").child(span().text("Docs"));

        assert_eq!(macro_built.render(), builder_built.render());
        assert_eq!(
            macro_built.render(),
            r#"<a href="/docs"><span>Docs</span></a>"#
        );
    }

    /// Ports `BuilderInitTests.containerWithAttributesAndContent`.
    #[test]
    fn attributes_and_children_render_together() {
        assert_eq!(
            html! { div(id = "main") { p { "Hi" } } }.render(),
            r#"<div id="main"><p>Hi</p></div>"#
        );
    }

    /// Ports `BuilderInitTests.booleanFlagIsPreserved`.
    #[test]
    fn a_boolean_flag_survives_the_macro() {
        assert_eq!(
            html! { details(open) { summary { "More" } } }.render(),
            "<details open><summary>More</summary></details>"
        );
    }

    /// Ports `BuilderInitTests.tableFamily`.
    #[test]
    fn the_table_family_nests() {
        let markup = html! {
            table {
                thead { tr { th { "A" } } }
                tbody { tr { td { "1" } } }
            }
        };

        assert_eq!(
            markup.render(),
            "<table><thead><tr><th>A</th></tr></thead><tbody><tr><td>1</td></tr></tbody></table>"
        );
    }

    /// Ports `BuilderInitTests.formFamily`.
    ///
    /// `for` is a Rust keyword, so the attribute name is quoted — the one place the macro
    /// asks for punctuation the Swift builder does not.
    #[test]
    fn the_form_family_nests() {
        let markup = html! {
            form {
                fieldset {
                    legend { "Account" }
                    label("for" = "email") { "Email" }
                }
            }
        };

        assert_eq!(
            markup.render(),
            concat!(
                "<form><fieldset><legend>Account</legend>",
                r#"<label for="email">Email</label></fieldset></form>"#,
            )
        );
    }

    /// Ports `BuilderInitTests.mediaFamily`.
    #[test]
    fn the_media_family_nests() {
        let markup = html! {
            picture {
                source(srcset = "a.webp", "type" = "image/webp")
                img(src = "a.jpg", alt = "A")
            }
        };

        assert_eq!(
            markup.render(),
            concat!(
                r#"<picture><source srcset="a.webp" type="image/webp">"#,
                r#"<img src="a.jpg" alt="A"></picture>"#,
            )
        );
    }

    /// Ports `BuilderInitTests.loopsInsideTheBuilder`.
    #[test]
    fn a_loop_inside_the_macro_repeats_its_body() {
        let markup = html! { ul { @for name in ["a", "b", "c"] { li { (name) } } } };

        assert_eq!(markup.render(), "<ul><li>a</li><li>b</li><li>c</li></ul>");
    }

    /// Ports `BuilderInitTests.conditionsInsideTheBuilder`.
    #[test]
    fn a_false_condition_inside_the_macro_renders_nothing() {
        let is_admin = false;
        let markup = html! {
            nav {
                a(href = "/") { "Home" }
                @if is_admin { a(href = "/admin") { "Admin" } }
            }
        };

        assert_eq!(markup.render(), r#"<nav><a href="/">Home</a></nav>"#);
    }

    /// Ports `BuilderInitTests.mapInsideTheBuilder`.
    ///
    /// Swift drops a `map` straight into the builder. The macro takes an iterator through
    /// `@for` instead, and `children_from` is the builder's spelling of the same thing —
    /// the test is that both land on identical markup.
    #[test]
    fn a_mapped_sequence_matches_the_macro_loop() {
        let macro_built = html! { ol { @for item in ["x", "y"] { li { (item) } } } };
        let builder_built = ol().children_from(["x", "y"].map(|item| li().text(item)));

        assert_eq!(macro_built.render(), builder_built.render());
        assert_eq!(macro_built.render(), "<ol><li>x</li><li>y</li></ol>");
    }

    /// Ports `BuilderInitTests.emptyBuilderProducesAnEmptyElement`.
    #[test]
    fn an_empty_body_produces_an_empty_element() {
        assert_eq!(html! { div {} }.render(), "<div></div>");
    }

    /// Ports `BuilderInitTests.untypedElementSupportsTheBuilder`.
    ///
    /// The macro resolves a tag name to its generated constructor, and there is no
    /// `hgroup()` among the 93 — Winged-Swift has no `Hgroup` type either, which is why its
    /// own test reaches for the untyped `HTMLTag`. `Element::new` is that escape hatch here,
    /// and it composes with macro-built children.
    #[test]
    fn an_untyped_element_takes_macro_built_children() {
        let group = Element::new("hgroup")
            .child(html! { h1 { "Title" } })
            .child(html! { p { "Subtitle" } });

        assert_eq!(
            group.render(),
            "<hgroup><h1>Title</h1><p>Subtitle</p></hgroup>"
        );
    }

    /// Ports `BuilderInitTests.chainingStillPreservesTheType`.
    ///
    /// Swift's builder initialiser returns the concrete tag type, so `.addClass` chains off
    /// it. `html!` returns a [`Node`], which is the union of every shape a child can take
    /// and has no builder methods — so chaining happens on the builder, and the macro
    /// supplies the children. Both spellings render the same.
    #[test]
    fn chaining_happens_on_the_builder_side() {
        let chained = div().child(html! { p { "Hi" } }).add_class("card");

        assert_eq!(chained.render(), r#"<div class="card"><p>Hi</p></div>"#);
    }

    /// Ports `BuilderInitTests.nestingIsArbitrarilyDeep`.
    #[test]
    fn nesting_goes_as_deep_as_it_is_written() {
        let page = html! {
            body { main_tag { section { article { h2 { "Title" } } } } }
        };

        assert_eq!(
            page.render(),
            "<body><main><section><article><h2>Title</h2></article></section></main></body>"
        );
    }

    // Ports `HTMLBuilderTests`, which exercises Swift's result-builder plumbing:
    // `buildOptional`, `buildEither` and `buildExpression`. Those are compiler hooks with
    // no counterpart here — `html!` lowers `@if`/`@else`/`@for` straight onto the builder —
    // so each port keeps the *behaviour* the Swift test describes. Several of those tests
    // reach into `document.children[0].name`; these assert markup instead, which is rule 11
    // of AGENTS.md and the reason Winged-Swift's own suite survived its 2.0 rewrite.

    /// Ports `HTMLBuilderTests.testHTMLBuilderCreatesRootHTMLTag`.
    #[test]
    fn a_root_html_element_keeps_its_children_in_order() {
        let document = html_tag().child(body()).child(head());

        assert_eq!(document.render(), "<html><body></body><head></head></html>");
    }

    /// Ports `HTMLBuilderTests.testHTMLBuilderHandlesOptional`.
    ///
    /// `Option` is an iterator of zero or one item, so `children_from` is all the plumbing
    /// an optional child needs.
    #[test]
    fn an_optional_child_is_included_when_present() {
        let present: Option<Element> = Some(Element::new("optional"));
        let absent: Option<Element> = None;

        assert_eq!(
            html_tag().children_from(present).render(),
            "<html><optional></optional></html>"
        );
        assert_eq!(html_tag().children_from(absent).render(), "<html></html>");
    }

    /// Ports `HTMLBuilderTests.testHTMLBuilderWrapsASingleNonGroupedComponent`.
    #[test]
    fn a_single_child_gains_no_wrapper() {
        let document = html_tag().child(body().child(h1().text("Hi")));

        assert_eq!(document.render(), "<html><body><h1>Hi</h1></body></html>");
    }

    /// Ports `HTMLBuilderTests.testHTMLBuilderTakesBothBranchesOfAnIfElse`.
    #[test]
    fn an_if_else_renders_whichever_branch_holds() {
        fn page(logged_in: bool) -> Element {
            html_tag().child(html! {
                @if logged_in { nav { "Sign out" } } @else { nav { "Sign in" } }
            })
        }

        assert_eq!(page(true).render(), "<html><nav>Sign out</nav></html>");
        assert_eq!(page(false).render(), "<html><nav>Sign in</nav></html>");
    }

    /// Ports `HTMLBuilderTests.testHTMLBuilderAcceptsAnArrayExpression`.
    #[test]
    fn a_sequence_of_children_flattens_into_siblings() {
        let document = html_tag().children_from(["a", "b"].map(|text| p().text(text)));

        assert_eq!(document.render(), "<html><p>a</p><p>b</p></html>");
    }

    /// Ports `HTMLBuilderTests.testHTMLBuilderHandlesEitherFirst`.
    ///
    /// Swift names the branches with the untyped `HTMLTag("first")`; the macro resolves a
    /// tag name to its generated constructor, so real tags stand in.
    #[test]
    fn the_first_branch_of_a_condition_renders_alone() {
        let document = html_tag().child(html! {
            @if true { header } @else { footer }
        });

        assert_eq!(document.render(), "<html><header></header></html>");
    }

    /// Ports `HTMLBuilderTests.testHTMLBuilderHandlesEitherSecond`.
    #[test]
    fn the_second_branch_of_a_condition_renders_alone() {
        let document = html_tag().child(html! {
            @if false { header } @else { footer }
        });

        assert_eq!(document.render(), "<html><footer></footer></html>");
    }
}
