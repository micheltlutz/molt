//! Allocation budgets for the hot builder and render paths.
//!
//! A timing assertion is noisy on a CI runner; an allocation *count* is exact. These pin how
//! many heap allocations the common operations make, so a change that quietly adds one per
//! element shows up here instead of in a benchmark nobody ran. Each figure is a ceiling:
//! lowering it after an optimisation is the point, raising it needs a reason in the commit.
//!
//! Everything is one `#[test]` on purpose. The allocator counts globally, so a second test
//! running on another thread would add its own allocations to the figure.

#![cfg(not(target_arch = "wasm32"))]

use std::alloc::System;

use molt::prelude::*;
use stats_alloc::{INSTRUMENTED_SYSTEM, Region, StatsAlloc};

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

/// Heap allocations and reallocations made while `work` runs, and what it returned.
///
/// A `Vec` or `String` that grows by doubling costs a reallocation each time, which is as
/// much a cost as a fresh allocation, so both count.
fn allocations<T>(work: impl FnOnce() -> T) -> (usize, T) {
    let region = Region::new(GLOBAL);
    let out = work();
    let change = region.change();
    (change.allocations + change.reallocations, out)
}

#[test]
fn the_hot_paths_stay_within_their_allocation_budgets() {
    let mut report = Vec::new();
    let mut check = |name: &str, measured: usize, budget: usize| {
        report.push(format!("{name}: {measured} (budget {budget})"));
        assert!(
            measured <= budget,
            "{name} made {measured} allocations, budget {budget}"
        );
    };

    // Construction.
    check("div()", allocations(div).0, 0);
    check(
        "div().add_class",
        allocations(|| div().add_class("card")).0,
        2,
    );
    check(
        "li().add_class().text()",
        allocations(|| li().add_class("item").text("x")).0,
        3,
    );
    check(
        "a().attr(href)",
        allocations(|| a().attr("href", "/x")).0,
        3,
    );
    check("p().text()", allocations(|| p().text("hello")).0, 1);
    check("Node::text", allocations(|| Node::text("hello")).0, 1);

    // Rendering a tree that already exists: the output string, and nothing else.
    let tree = ul().children_from((0..100).map(|i| li().add_class("item").text(format!("n{i}"))));
    check(
        "render compact, 100 items",
        allocations(|| tree.render()).0,
        6,
    );
    check(
        "render pretty, 100 items",
        allocations(|| tree.render_pretty()).0,
        7,
    );

    // Escaping.
    check(
        "escape_text clean",
        allocations(|| molt::core::escape::escape_text("plain")).0,
        0,
    );
    check(
        "escape_text dirty",
        allocations(|| molt::core::escape::escape_text("a<b")).0,
        2,
    );

    // Cloning, which has to allocate every owned string once.
    check("clone, 100 items", allocations(|| tree.clone()).0, 302);

    eprintln!("{}", report.join("\n"));
}
