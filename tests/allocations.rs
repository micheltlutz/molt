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

/// How many times each measurement is taken. See [`allocations`].
const ATTEMPTS: usize = 9;

/// Heap allocations and reallocations that `work` makes, and what it returned.
///
/// A `Vec` or `String` that grows by doubling costs a reallocation each time, which is as
/// much a cost as a fresh allocation, so both count.
///
/// **This is the minimum over several runs, not one run.** The allocator counts every thread,
/// and the test harness has its own: libtest's main thread prints and receives events while
/// the test runs, and allocates when it does. One of those allocations landing inside a
/// measurement made `div()` "allocate twice" on a CI runner, in a build whose only change was
/// a README. The cost of deterministic code is the same on every run, and other threads can
/// only *add* to it, so the smallest figure is the true one. It does mean `work` runs
/// [`ATTEMPTS`] times, so it must be repeatable.
fn allocations<T>(work: impl Fn() -> T) -> (usize, T) {
    let (mut fewest, output) = measure(&work);
    for _ in 1..ATTEMPTS {
        fewest = fewest.min(measure(&work).0);
    }
    (fewest, output)
}

/// One measurement: allocations plus reallocations while `work` runs, and its result.
fn measure<T>(work: &impl Fn() -> T) -> (usize, T) {
    let region = Region::new(GLOBAL);
    let output = work();
    let change = region.change();
    (change.allocations + change.reallocations, output)
}

/// A closure whose first calls allocate extra, as if another thread had interfered, still
/// measures as its true cost. Deterministic: the "noise" comes from a counter, not from timing.
///
/// Called from the one test below and **not** a `#[test]` itself: a second test would run on
/// another thread at the same time and put its own allocations into every figure here.
fn a_measurement_ignores_allocations_made_by_other_threads() {
    let calls = std::cell::Cell::new(0_usize);
    let noisy = || {
        calls.set(calls.get() + 1);
        // The first four calls drop an extra allocation inside the window, as a stray thread
        // would; every call also makes the two allocations that are the work itself.
        if calls.get() <= 4 {
            std::hint::black_box(Vec::<u8>::with_capacity(8));
            std::hint::black_box(Vec::<u8>::with_capacity(8));
        }
        (
            std::hint::black_box(Vec::<u8>::with_capacity(8)),
            std::hint::black_box(Vec::<u8>::with_capacity(8)),
        )
    };
    let (measured, _) = allocations(noisy);
    assert_eq!(
        measured, 2,
        "the least of {ATTEMPTS} runs is the work alone"
    );
    assert_eq!(calls.get(), ATTEMPTS);
}

#[test]
fn the_hot_paths_stay_within_their_allocation_budgets() {
    a_measurement_ignores_allocations_made_by_other_threads();

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

    // Replacing an attribute compares keys by reference; it used to copy the key first.
    check(
        "a().trusted_url(href)",
        allocations(|| a().trusted_url("href", "/x")).0,
        3,
    );

    // `html!`: a `Display` value is escaped as it is formatted, not copied through
    // `to_string()`, and a text-only element moves its text instead of rebuilding it.
    let name = String::from("Ana");
    check(
        "html! li { (name) }",
        allocations(|| molt::html! { li { (name) } }).0,
        3,
    );
    check(
        "html! li(class = \"item\") { (name) }",
        allocations(|| molt::html! { li(class = "item") { (name) } }).0,
        5,
    );

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
        1,
    );

    // Cloning, which has to allocate every owned string once.
    check("clone, 100 items", allocations(|| tree.clone()).0, 302);

    // Not a speed check: a ceiling on how large the tree types may grow. `Node` is as large as
    // its largest variant, `Element` (104 bytes), so every text node in a `Vec<Node>` costs 104
    // bytes. Boxing the element would shrink a text node to about 32 bytes but add one
    // allocation per element child, the cost batch C spent effort removing, and change the
    // public shape of `Node`. Nothing measured shows memory pressure, so it was not done; this
    // notices if the types grow past what was measured. Kept inside this test because the
    // allocator counts globally and a second test thread would corrupt the figures above.
    {
        use std::mem::size_of;
        check("size_of::<Node>()", size_of::<Node>(), 104);
        check("size_of::<Element>()", size_of::<Element>(), 104);
        check("size_of::<Attribute>()", size_of::<Attribute>(), 56);
    }

    eprintln!("{}", report.join("\n"));
}
