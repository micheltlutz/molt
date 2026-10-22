---
paths:
  - "src/core/**"
  - "src/document.rs"
  - "src/layout.rs"
---

# Walking the tree

`Node` and `Element` are recursive, and a stack overflow in Rust is an abort, not an error.
The release profile is `panic = "abort"` on top of that.

- Any traversal uses an explicit stack (`Vec<Step>`), never recursion. Children are queued
  in order with `iter().rev()` onto the LIFO stack.
- Never `derive` `Clone`, `PartialEq`, `Eq`, `Hash` or `Drop` on a recursive type: the
  derive recurses. Hand-write them on the existing `clone_tree`/`trees_equal`/`hash_tree`
  pattern and keep `Drop` iterative. `Debug` is hand-written too, but bounded rather than
  iterative: it writes `...` below `DEBUG_MAX_DEPTH` (128) levels, so its recursion is
  constant (L6, #79).
- Cloning a tree is O(n) and a reviewer's BLOCK. `Document::root()` cloning head and body
  was a real abort (L6). Replace a clone with a borrow, then prove the bytes did not move
  (L7).
- A feature that handles untrusted nesting is not done until a 100,000-level test exists
  (`tests/performance.rs` has the existing ones). In it, prefer `assert!(a == b)`: since 2.0.0
  a failing `assert_eq!` panics with a bounded message instead of aborting, but it still
  formats two 100,000-level trees for nothing.
- Depth is bounded only on request: `RenderOptions::with_max_depth`. The counter lives in
  the `LIMITED` monomorphisation and costs nothing otherwise.
- Do not add interior mutability or `Rc` to the tree. It is `Send + Sync`, the `parallel`
  feature relies on it, and a test asserts it.

Lessons L6 and L7 in [`lessons-learned.md`](../skills/molt/references/lessons-learned.md).
