//! Render benchmarks.
//!
//! Phase 6 of `MOLT_SPEC.md` §7: 10,000 nodes, compact and pretty.
//!
//! These run under `cargo bench` and are **not** a CI gate — criterion timings are too
//! noisy for that. The cheap regression guard that does gate CI lives in
//! `tests/performance.rs`, which asserts on output size rather than time, borrowing the
//! instinct from `Winged-Swift/Tests/WingedSwiftTests/RenderPerformanceTests.swift`.

#![allow(missing_docs)]

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
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
        group.bench_function(format!("compact/{count}"), |b| {
            b.iter_batched(
                || wide_tree(count),
                |tree| black_box(tree.render()),
                BatchSize::SmallInput,
            );
        });
        group.bench_function(format!("pretty/{count}"), |b| {
            b.iter_batched(
                || wide_tree(count),
                |tree| black_box(tree.render_pretty()),
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_deep(c: &mut Criterion) {
    c.bench_function("deep/1000/pretty", |b| {
        b.iter_batched(
            || deep_tree(1_000),
            |tree| black_box(tree.render_pretty()),
            BatchSize::SmallInput,
        );
    });
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

    let mut group = c.benchmark_group("escape");
    group.bench_function("hostile/20k", |b| {
        b.iter(|| black_box(escape_text(&hostile)));
    });
    group.bench_function("plain/20k", |b| {
        b.iter(|| black_box(escape_text(&plain)));
    });
    group.finish();
}

fn bench_document(c: &mut Criterion) {
    c.bench_function("document/50-cards/pretty", |b| {
        b.iter_batched(
            realistic_page,
            |page| black_box(page.render()),
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_wide,
    bench_deep,
    bench_build_vs_render,
    bench_escaping,
    bench_document
);
criterion_main!(benches);
