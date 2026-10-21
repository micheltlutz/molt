---
paths:
  - "benches/**"
  - "tests/allocations.rs"
  - "tests/performance.rs"
  - "src/core/render.rs"
  - "src/core/escape.rs"
---

# Performance

A claim needs a number. "Faster" needs `cargo bench --bench render` before and after;
"fewer allocations" needs `tests/allocations.rs`. Report the regressions you caused in the
same PR as the wins (L16).

- Measure the build you ship: the bench profile can measure the wrong one (L14). Calibrate
  the experiment before trusting its verdict (L34).
- Count allocations instead of arguing about them (L15). The counting allocator is a
  dev-dependency, its test is the single `#[test]` in `tests/allocations.rs`, and its
  budget needs at least nine measurements to be stable.
- Hot loops are generic over the entity function so the compiler inlines the match; an
  opt-in counter (depth) is monomorphised away when off (L28).
- Do not optimise away a safety property. The scratch-buffer render of each child (rule 6)
  is what keeps empty children from leaving a blank line, and the byte scan in the escapers
  is correct only because every escaped character is ASCII.
- Refused on purpose: SIMD, `mimalloc`, a global render switch, attribute sorting. The
  reasons are in [`decision-log.md`](../skills/molt/references/decision-log.md).
- "Analysed, not applied" is a result worth writing down (L24).

Method and baseline numbers:
[`performance.md`](../skills/molt/references/performance.md).
