//! Render benchmarks.
//!
//! Phase 6 of `MOLT_SPEC.md` §7: 10,000 nodes, compact and pretty.
//!
//! The bench profile is `opt-level = 3` (see `Cargo.toml`); the release profile is tuned for
//! WASM size and would measure the wrong thing. Routines that consume a tree use
//! `iter_batched_ref`, so freeing it is not part of the time.
//!
//! These run under `cargo bench` and are **not** a CI gate — criterion timings are too
//! noisy for that. The cheap regression guard that does gate CI lives in
//! `tests/performance.rs`, which asserts on output size rather than time, borrowing the
//! instinct from `Winged-Swift/Tests/WingedSwiftTests/RenderPerformanceTests.swift`.

#![allow(missing_docs)]

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

use molt::Document;
use molt::prelude::*;

/// 10,000 sibling list items — the spec's headline number.
fn wide_tree(count: usize) -> Element {
    ul().children_from((0..count).map(|i| li().add_class("item").text(format!("Item {i}"))))
}

/// A tree `depth` levels deep, which stresses recursion rather than iteration.
fn deep_tree(depth: usize) -> Element {
    let mut node = p().text("bottom");
    for level in 0..depth {
        node = div().add_class(format!("level-{level}")).child(node);
    }
    node
}

fn realistic_page() -> Document {
    Document::new(Some("pt-BR"))
        .head_children([
            meta().attr("charset", "UTF-8"),
            title().text("Benchmark"),
            stylesheet("/css/style.css"),
        ])
        .body_children([
            header().child(nav().child(link_to("/").text("Home"))),
            main_tag().children_from((0..50).map(|i| {
                article()
                    .add_class("card")
                    .child(h2().text(format!("Post {i}")))
                    .child(p().text("Fuel, tyres & chain — all in one place."))
            })),
            footer().child(p().text("© 2026")),
        ])
}

fn bench_wide(c: &mut Criterion) {
    let mut group = c.benchmark_group("wide");
    for count in [100, 1_000, 10_000] {
        let bytes = wide_tree(count).render().len() as u64;
        group.throughput(Throughput::Bytes(bytes));
        group.bench_function(format!("compact/{count}"), |b| {
            b.iter_batched_ref(
                || wide_tree(count),
                |tree| black_box(tree.render()),
                BatchSize::SmallInput,
            );
        });
        group.bench_function(format!("pretty/{count}"), |b| {
            b.iter_batched_ref(
                || wide_tree(count),
                |tree| black_box(tree.render_pretty()),
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_deep(c: &mut Criterion) {
    let mut group = c.benchmark_group("deep");
    group.bench_function("1000/pretty", |b| {
        b.iter_batched_ref(
            || deep_tree(1_000),
            |tree| black_box(tree.render_pretty()),
            BatchSize::SmallInput,
        );
    });
    group.bench_function("1000/compact", |b| {
        b.iter_batched_ref(
            || deep_tree(1_000),
            |tree| black_box(tree.render()),
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn bench_build_vs_render(c: &mut Criterion) {
    let mut group = c.benchmark_group("phases");
    group.bench_function("build/10000", |b| {
        b.iter(|| black_box(wide_tree(10_000)));
    });

    let tree = wide_tree(10_000);
    group.bench_function("render/10000", |b| {
        b.iter(|| black_box(tree.render()));
    });
    group.finish();
}

fn bench_escaping(c: &mut Criterion) {
    use molt::core::escape::escape_text;

    // Half the characters need escaping — the worst realistic case.
    let hostile: String = "a<b>c&d\"e'f".repeat(2_000);
    let plain: String = "abcdefghij".repeat(2_000);
    // Prose: one character in two hundred needs escaping, the realistic case. The hostile
    // string above is the adversarial one, where every other character does.
    let sparse: String = format!("{}&", "abcdefghij".repeat(20)).repeat(100);

    let mut group = c.benchmark_group("escape");
    group.bench_function("hostile/20k", |b| {
        b.iter(|| black_box(escape_text(&hostile)));
    });
    group.bench_function("plain/20k", |b| {
        b.iter(|| black_box(escape_text(&plain)));
    });
    group.bench_function("sparse/20k", |b| {
        b.iter(|| black_box(escape_text(&sparse)));
    });
    group.finish();
}

fn bench_document(c: &mut Criterion) {
    let mut group = c.benchmark_group("document");
    let bytes = realistic_page().render().len();
    group.throughput(Throughput::Bytes(bytes as u64));
    group.bench_function("50-cards/pretty", |b| {
        b.iter_batched_ref(
            realistic_page,
            |page| black_box(page.render()),
            BatchSize::SmallInput,
        );
    });
    group.bench_function("50-cards/compact", |b| {
        b.iter_batched_ref(
            realistic_page,
            |page| black_box(page.render_compact()),
            BatchSize::SmallInput,
        );
    });
    // The streaming path the static site generator uses.
    group.bench_function("50-cards/render_to", |b| {
        b.iter_batched_ref(
            realistic_page,
            |page| {
                let mut sink = Vec::with_capacity(bytes);
                // Writing into a `Vec` cannot fail, and a bench has no use for the `Result`.
                let _ = page.render_to(&mut sink, &RenderOptions::pretty());
                black_box(sink)
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

/// Every attribute is escaped on the way in and re-checked on the way out, so a tree that is
/// mostly attributes stresses a different path from one that is mostly text.
fn attribute_heavy(count: usize) -> Element {
    div().children_from((0..count).map(|i| {
        a().attr("href", format!("/posts/{i}?ref=home&utm=a"))
            .attr("title", "Read \"more\" & more")
            .attr("rel", "noopener")
            .attr("target", "_blank")
            .data_attr("index", i.to_string())
            .aria_attr("label", "Open the post")
            .add_class("link primary")
            .set_id(format!("post-{i}"))
            .text("Open")
    }))
}

fn bench_attributes(c: &mut Criterion) {
    let mut group = c.benchmark_group("attributes");
    group.bench_function("build/1000", |b| {
        b.iter(|| black_box(attribute_heavy(1_000)));
    });
    let tree = attribute_heavy(1_000);
    group.throughput(Throughput::Bytes(tree.render().len() as u64));
    group.bench_function("render/1000", |b| b.iter(|| black_box(tree.render())));
    group.finish();
}

fn bench_macro(c: &mut Criterion) {
    use molt::html;

    let items: Vec<String> = (0..1_000).map(|i| format!("Item {i}")).collect();
    let mut group = c.benchmark_group("html_macro");
    group.bench_function("build/1000", |b| {
        b.iter(|| {
            black_box(html! {
                ul(class = "list") {
                    @for item in &items { li(class = "item") { (item) } }
                }
            })
        });
    });
    group.bench_function("builder/1000", |b| {
        b.iter(|| {
            black_box(Node::from(
                ul().add_class("list").children_from(
                    items
                        .iter()
                        .map(|item| li().add_class("item").child(Node::text(item))),
                ),
            ))
        });
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_wide,
    bench_deep,
    bench_build_vs_render,
    bench_escaping,
    bench_document,
    bench_attributes,
    bench_macro
);
criterion_main!(benches);
