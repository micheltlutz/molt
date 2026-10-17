# Performance: how to measure, what was measured, what not to do

Principle from `docs/plans/2026-10-17-seguranca-e-performance.md`: **measure first, and do not
optimise what the end-to-end numbers do not ask for.** In that plan's harness the Rust
`/api/render` endpoint sustained 4,000 req/s on one vCPU and did not saturate; the DSL is not
the bottleneck of a real request. Optimise to remove waste you can *show*, not to win a
micro-benchmark.

## Tools

| tool | answers | command |
| --- | --- | --- |
| criterion | "how long?" (and MiB/s) | `cargo bench --bench render` |
| criterion baseline | "faster or slower than before?" | `cargo bench --bench render -- --save-baseline before`, then `-- --baseline before` |
| `tests/allocations.rs` | "how many allocations?" exact, gates CI | `cargo test --test allocations -- --nocapture` |
| `tests/performance.rs` | "did output size or depth handling change?" | `cargo test --test performance` |
| `scripts/build-wasm.sh` | "does the WASM still fit 120 KB gzip?" | `./scripts/build-wasm.sh` |

Gotchas that produced wrong numbers (L14):

- `cargo bench -- --save-baseline x` forwards the flag to the **lib** test harness too and
  fails. Name the bench: `cargo bench --bench render -- --save-baseline x`.
- Criterion takes **one** filter, not several. To compare a subset, run the whole suite.
- `[profile.bench] opt-level = 3`. The release profile is `z` for WASM size.
- Use `iter_batched_ref` so freeing the input is not timed.
- Do not run anything else while a baseline is being measured. Re-run a surprising number
  before believing it; ±5% is noise on this machine, a repeatable 2× is not.

## Baseline (opt-level 3, the development machine, before batch C changes)

| bench | median |
| --- | --- |
| `wide/compact/10000` | 359 µs (≈ 820 MiB/s) |
| `wide/pretty/10000` | 382 µs (≈ 770 MiB/s) |
| `deep/1000/compact` | 49 µs |
| `deep/1000/pretty` | 1.57 ms (output is O(depth²): one indent per level per line) |
| `phases/build/10000` | 1.02 ms |
| `phases/render/10000` | 316 µs |
| `escape/hostile/20k` | 20.1 µs (every other char escaped) |
| `escape/plain/20k` | 5.7 µs |
| `document/50-cards` pretty / compact / `render_to` | 7.4 / 5.6 / 8.5 µs |
| `attributes/build/1000` / `render/1000` | 628 / 242 µs |
| `html_macro/build/1000` vs `builder/1000` | 178 vs 99 µs |

Allocation baseline: `div()` 0; `div().add_class()` 2; `li().add_class().text()` 3;
`a().attr("href", …)` 3; render compact of 100 items 6, pretty 7; `escape_text` of dirty input 2.

Reading it: rendering is already hundreds of MiB/s; **building** the tree (allocations) costs
more than rendering it. `html!` cost ~1.8× the equivalent builder although AGENTS.md rule 9
says it is only sugar — found by the baseline, fixed in PR #59 (below).

## What each change did (keep appending, with numbers)

| change | result |
| --- | --- |
| `Cow<'static, str>` for tag and attribute key (A3) | `div()` makes 0 allocations; asserted. Time effect: not separately measured |
| `add_class` appends in place (P2) | one fewer copy and key reallocation per second class; covered by the allocation test |
| `html!` formats `Display` straight into escaped form (PR #59) | build/1000 178 → 107 µs (−40%); `html! li { (name) }` 5 → 3 allocations; remaining gap to the builder ~14% |
| escapers copy clean runs with `push_str` (P4) | sparse text 21.8 → 10.3 µs; clean 5.7 → 5.2 µs; **dense 19.7 → 55.6 µs (worse)**; `escape_text` dirty 2 → 1 alloc |

## Rules that came out of measuring

1. **`fn` pointers do not inline.** Passing the byte→entity table to a hot loop as `fn(u8) ->
   Option<&str>` made the loop 2.5× slower. Pass `impl Fn(u8) -> …` so the match is inlined.
2. **Per-piece overhead beats per-byte savings on short pieces.** Copying runs with `push_str`
   wins when runs are long and loses when every other byte is special. State which input your
   win is on, and give the other.
3. **Do not "optimise away" a safety property.** AGENTS.md rule 6: the pretty writer rolls back
   an empty child's separator instead of testing for emptiness first. Rule 5: `<pre>`,
   `<code>`, `<textarea>` are never indented inside. Faster output that changes bytes is a bug.
4. **No `unsafe`.** `get_unchecked`, `from_utf8_unchecked` and friends are off the table
   (`unsafe_code = "forbid"`). Some micro-optimisations are therefore simply unavailable; say
   so rather than work around the lint.
5. **Not in scope** (reaffirmed): SIMD in the escaper, `mimalloc`, changing `panic = "abort"`,
   a custom allocator in the library, rewriting the Markdown parser.
6. **Pretty output size is quadratic in depth.** That is not a performance bug to "fix" in the
   writer; bound depth or render compact.
7. **WASM budget is 120 KB gzip** (`scripts/build-wasm.sh`; measured 22 KB). Opt-level `z` stays
   for the release profile; do not flip it to speed up native benchmarks.

## Candidates analysed and not applied (issues #34–#38; evidence is on each issue)

| candidate | verdict |
| --- | --- |
| `Box<Element>` in `Node` | `size_of` = 104 B confirmed, but +1 allocation per element child; not done, ceiling test added |
| initial stack capacity 128 | allocations 6 → 5, time −3% (noise), +5 KB per tiny render; not done |
| skip `DropIfEmpty` for `Element` | premise invalid since S2 (an element can now render empty) |
| cache `name_ok` / `is_url` per attribute | moves the cost, no gain for build-once/render-once; not done |
| O(depth) work stack | no benchmark shows the cost; not done |

Reopen any of them only with a benchmark that shows the cost. "Should be faster" is not one.

## Writing a performance PR

1. State the workload and the number you are trying to move.
2. `--save-baseline before` on a quiet machine, change, `--baseline before`.
3. Put the table (including any regression) in the PR body, with the input shape each row used.
4. Lower the matching ceiling in `tests/allocations.rs` if allocations dropped.
5. `./scripts/verify.sh` green; fixtures unchanged.
