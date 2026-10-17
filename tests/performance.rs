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

/// `Document::render_with` used to build `<html>` from clones of head and body. `Clone` is
/// recursive, so a deep body aborted the process before the writer ran.
#[test]
fn a_document_with_a_very_deep_body_renders_in_both_modes() {
    let page = molt::Document::new(Some("en"))
        .head_children([title().text("deep")])
        .body_children([deep_tree(VERY_DEEP)]);

    assert_eq!(
        page.render_compact().matches("<div>").count(),
        VERY_DEEP,
        "compact"
    );
    let pretty = page.render_with(&RenderOptions::pretty().with_indent(""));
    assert_eq!(pretty.lines().count(), VERY_DEEP * 2 + 9, "pretty");

    let mut streamed = Vec::new();
    page.render_to(&mut streamed, &RenderOptions::compact())
        .expect("written");
    assert_eq!(streamed, page.render_compact().into_bytes());
}

/// `audit` recursed once per level, so it aborted on the tree the renderer handles.
#[test]
fn auditing_a_very_deep_tree_does_not_abort() {
    use molt::accessibility::audit;

    let mut node = Node::from(img().attr("src", "/a.png"));
    for _ in 0..VERY_DEEP {
        node = Node::from(div().child(node));
    }
    node = Node::fragment([node, Node::from(img().attr("src", "/b.png"))]);

    let issues = audit(&node);
    assert_eq!(issues.len(), 2);
    assert!(!node.is_empty());
}

#[test]
fn a_chain_of_empty_fragments_is_empty_without_recursing() {
    let mut node = Node::text("");
    for _ in 0..VERY_DEEP {
        node = Node::fragment([node]);
    }
    assert!(node.is_empty());
    // Freed on a thread with a large stack: a bare fragment chain is dropped recursively
    // (documented in SECURITY.md), which is not what this test is about.
    std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(move || drop(node))
        .expect("spawned")
        .join()
        .expect("dropped");
}

/// `Clone`, `PartialEq` and `Debug` were derived, so each recursed once per level. A deep
/// tree rendered fine and then aborted in `assert_eq!` or `.clone()`.
// `assert!(a == b)` on purpose: a failing `assert_eq!` would format both trees with the
// derived, recursive `Debug`, and overflow the stack in the very case under test.
#[allow(clippy::manual_assert_eq)]
#[test]
fn cloning_and_comparing_a_very_deep_tree_does_not_abort() {
    let tree = deep_tree(VERY_DEEP);
    let copy = tree.clone();
    assert!(tree == copy);
    assert_eq!(copy.render().matches("<div>").count(), VERY_DEEP);

    // A difference at the very bottom is found, and a shorter tree is not equal.
    let mut different = deep_tree(VERY_DEEP);
    different = div().child(different);
    assert!(tree != different);
    let other_leaf = {
        let mut node = p().text("other");
        for _ in 0..VERY_DEEP {
            node = div().child(node);
        }
        node
    };
    assert!(tree != other_leaf);
}

#[allow(clippy::manual_assert_eq)]
#[test]
fn cloning_and_comparing_deep_fragments_does_not_abort() {
    let mut node = Node::from(p().text("bottom"));
    for _ in 0..VERY_DEEP {
        node = Node::fragment([node, Node::text("x")]);
    }
    // Wrapped in an element so teardown is the iterative one.
    let root = div().child(node);
    let copy = root.clone();
    assert!(root == copy);
    assert_eq!(copy.render(), root.render());
}
