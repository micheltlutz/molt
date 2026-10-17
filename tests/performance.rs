//! A cheap render-performance guard that is safe to gate CI on.
//!
//! Ports the instinct behind
//! `Winged-Swift/Tests/WingedSwiftTests/RenderPerformanceTests.swift`, which asserts on
//! **output size** rather than elapsed time. A size assertion is deterministic across
//! machines and CI runners; a timing assertion is not, which is why the criterion suite in
//! `benches/render.rs` is not part of `cargo test`.
//!
//! What this catches: an accidental change that starts emitting extra whitespace, repeats
//! an attribute, or silently drops children — all of which move the byte count long before
//! anyone notices them in a diff.

use molt::prelude::*;

fn wide_tree(count: usize) -> Element {
    ul().children_from((0..count).map(|i| li().add_class("item").text(format!("Item {i}"))))
}

/// Ports `RenderPerformanceTests.outputSizeIsWhatWeExpect`, which is the whole of Swift's
/// performance suite that can be asserted deterministically.
#[test]
fn compact_output_size_is_stable() {
    let rendered = wide_tree(1_000).render();

    // <ul> + </ul> = 9 bytes; each item is `<li class="item">Item N</li>`.
    let expected: usize = 9
        + (0..1_000)
            .map(|i| format!(r#"<li class="item">Item {i}</li>"#).len())
            .sum::<usize>();

    assert_eq!(
        rendered.len(),
        expected,
        "compact render emitted unexpected bytes"
    );
}

#[test]
fn pretty_output_adds_exactly_one_line_per_node() {
    let rendered = wide_tree(1_000).render_pretty();
    // One line for `<ul>`, one per item, one for `</ul>`.
    assert_eq!(rendered.lines().count(), 1_002);
}

#[test]
fn rendering_is_stable_across_repeated_calls() {
    let tree = wide_tree(500);
    let first = tree.render();
    for _ in 0..10 {
        assert_eq!(tree.render(), first, "render is not deterministic");
    }
}

/// Depth is no longer bounded by the stack.
///
/// The writer walks an explicit work stack instead of recursing, and `Element` tears its
/// subtree down the same way, so nesting costs heap — bounded by a tree that is already in
/// memory — rather than stack, which aborts the process when it runs out.
///
/// 100,000 levels is far past anything a browser renders: Chrome and Firefox both flatten
/// nesting beyond roughly 512 elements. The figure is here to prove the ceiling is gone,
/// not to suggest markup like this is reasonable.
const VERY_DEEP: usize = 100_000;

fn deep_tree(depth: usize) -> Element {
    let mut node = p().text("bottom");
    for _ in 0..depth {
        node = div().child(node);
    }
    node
}

#[test]
fn very_deep_nesting_renders_compact() {
    assert_eq!(
        deep_tree(VERY_DEEP).render().matches("<div>").count(),
        VERY_DEEP
    );
}

/// Pretty mode, with the indent turned off.
///
/// Indentation makes pretty output quadratic in depth — every one of the 200,001 lines
/// carries one indent string per level above it, which at this depth is some 20 GB of
/// whitespace and nothing to do with the stack. An empty indent walks exactly the same
/// arms of the writer (newlines, the empty-child rollback, the deferred close tag) in
/// linear output.
#[test]
fn very_deep_nesting_renders_pretty() {
    let options = RenderOptions::pretty().with_indent("");
    let rendered = deep_tree(VERY_DEEP).render_with(&options);

    // One line to open each `<div>`, one to close it, and one for the `<p>` at the bottom.
    assert_eq!(rendered.lines().count(), VERY_DEEP * 2 + 1);
}

/// Freeing the tree used to recurse too, so a tree deep enough to render could still abort
/// on the way out of scope.
#[test]
fn dropping_a_very_deep_tree_does_not_abort() {
    drop(deep_tree(VERY_DEEP));
}

/// Fragments nest through a different arm of the writer than elements do, and through a
/// different arm of `Element`'s teardown.
#[test]
fn very_deep_fragments_render_and_drop() {
    let mut node = Node::from(p().text("bottom"));
    for _ in 0..VERY_DEEP {
        node = Node::fragment([node]);
    }

    // Wrapped in an element on purpose: a bare `Node` chain is still freed recursively,
    // because giving `Node` its own `Drop` would forbid `match node { Node::Element(e) =>
    // e }` — a partial move this crate's own `static_site` example performs.
    let root = div().child(node);
    assert!(root.render().ends_with("<p>bottom</p></div>"));
    drop(root);
}

/// The guaranteed depth this crate documents, kept as the cheap regression guard.
#[test]
fn nesting_to_the_documented_depth_renders() {
    let node = deep_tree(256);
    assert!(node.render().len() > 256);
    assert!(node.render_pretty().lines().count() > 256);
}
