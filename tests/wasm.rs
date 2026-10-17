//! WebAssembly binding tests.
//!
//! Run with `wasm-pack test --node -- --features wasm`.
//!
//! The point of these is not that the bindings compile — it is that JavaScript and Rust
//! produce the *same bytes*. The bindings wrap the real tree rather than accumulating
//! pre-rendered strings, so there is one renderer; these tests prove the wrapper does not
//! quietly change its output.

#![cfg(all(feature = "wasm", target_arch = "wasm32"))]

use molt::Document;
use molt::prelude::*;
use molt::wasm::{WDocument, WElement, WSeo, element};
use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn an_element_renders_the_same_through_both_apis() {
    let through_wasm = element("div")
        .add_class("card")
        .set_id("hero")
        .child(element("h1").text("Hi"))
        .render();
    let through_rust = div()
        .add_class("card")
        .set_id("hero")
        .child(h1().text("Hi"))
        .render();

    assert_eq!(through_wasm, through_rust);
}

#[wasm_bindgen_test]
fn text_is_escaped_across_the_boundary() {
    assert_eq!(
        element("p").text("<script>alert('x')</script>").render(),
        "<p>&lt;script&gt;alert(&#x27;x&#x27;)&lt;/script&gt;</p>"
    );
}

#[wasm_bindgen_test]
fn raw_text_is_not_escaped_across_the_boundary() {
    assert_eq!(
        element("p").raw_text("<b>x</b>").render(),
        "<p><b>x</b></p>"
    );
}

/// The capability the spec's string-accumulating design would have made impossible.
#[wasm_bindgen_test]
fn pretty_printing_works_from_javascript() {
    let rendered = element("div")
        .child(element("p").text("deep"))
        .render_pretty();
    assert_eq!(rendered, "<div>\n  <p>deep</p>\n</div>");
}

#[wasm_bindgen_test]
fn a_document_owns_the_doctype_and_defaults_to_pretty() {
    let page = WDocument::new(Some("pt-BR".into())).add_body(element("h1").text("Hi"));
    let rendered = page.render();

    assert!(rendered.starts_with("<!DOCTYPE html>\n<html lang=\"pt-BR\">"));
    assert!(rendered.contains('\n'));
    assert!(
        !page
            .render_compact()
            .trim_start_matches("<!DOCTYPE html>\n")
            .contains('\n')
    );
}

#[wasm_bindgen_test]
fn a_document_matches_the_rust_equivalent_byte_for_byte() {
    let through_wasm = WDocument::new(Some("pt-BR".into()))
        .add_head(element("title").text("RideKeeper"))
        .add_body(element("h1").text("Track every service"))
        .render();

    let through_rust = Document::new(Some("pt-BR"))
        .head_children([title().text("RideKeeper")])
        .body_children([h1().text("Track every service")])
        .render();

    assert_eq!(through_wasm, through_rust);
}

#[wasm_bindgen_test]
fn boolean_and_prefixed_attributes_survive_the_boundary() {
    let rendered = element("input")
        .attr("type", "email")
        .data_attr("field", "email")
        .aria_attr("label", "E-mail")
        .bool_attr("required")
        .render();

    assert_eq!(
        rendered,
        r#"<input type="email" data-field="email" aria-label="E-mail" required>"#
    );
}

#[wasm_bindgen_test]
fn void_elements_drop_children_from_javascript_too() {
    let rendered = element("img")
        .attr("src", "/a.png")
        .child(element("span").text("x"))
        .render();
    assert_eq!(rendered, r#"<img src="/a.png">"#);
}

#[wasm_bindgen_test]
fn comments_cannot_close_early() {
    let rendered = element("div").comment("a --> b").render();
    assert_eq!(rendered.matches("-->").count(), 1);
}

#[wasm_bindgen_test]
fn the_seo_builder_is_reachable_from_javascript() {
    let rendered = WSeo::new("RideKeeper", "Motorcycle maintenance companion")
        .image("https://ridekeeper.example/og.jpg")
        .url("https://ridekeeper.example")
        .twitter_site("@micheltlutz")
        .render();

    assert!(rendered.contains(r#"<meta property="og:title" content="RideKeeper">"#));
    assert!(rendered.contains(r#"<meta name="twitter:site" content="@micheltlutz">"#));
}

#[wasm_bindgen_test]
fn a_seo_block_goes_into_the_head_of_a_document() {
    let through_wasm = WDocument::new(Some("en".into()))
        .add_seo(WSeo::new("T", "D"))
        .add_body(element("h1").text("Hi"))
        .render_compact();
    assert!(through_wasm.contains(r#"<meta property="og:title" content="T">"#));
    assert!(through_wasm.contains("<head>"));
}

#[wasm_bindgen_test]
fn the_extra_element_methods_match_the_rust_builder() {
    let through_wasm = element("div")
        .add_classes(vec!["a".into(), "b".into()])
        .raw_child("<hr>")
        .child(element("span").self_closing(true))
        .render();
    let through_rust = div()
        .add_classes(["a", "b"])
        .child(Node::raw("<hr>"))
        .child(span().self_closing(true))
        .render();
    assert_eq!(through_wasm, through_rust);
}

#[wasm_bindgen_test]
fn render_options_reach_javascript() {
    let tree = || {
        element("div")
            .child(element("br"))
            .child(element("p").text("x"))
    };
    assert_eq!(
        tree().render_with_options(true, "\t", true),
        "<div>\n\t<br />\n\t<p>x</p>\n</div>"
    );
    assert_eq!(tree().render_bytes(), tree().render().into_bytes());
}

#[wasm_bindgen_test]
fn the_version_is_reported() {
    assert!(!molt::wasm::version().is_empty());
}

/// `WElement` must stay constructible the idiomatic JS way as well as through `element()`.
#[wasm_bindgen_test]
fn both_constructors_agree() {
    assert_eq!(
        WElement::new("span").text("x").render(),
        element("span").text("x").render()
    );
}
