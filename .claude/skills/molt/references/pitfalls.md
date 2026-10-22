# Pitfalls (for contributors)

The symptom tables for people who **use** Molt (wrong characters, missing output, the wrong
shape, compile errors, WASM) live in [`docs/pitfalls.md`](../../../../docs/pitfalls.md), because
that file ships in the published crate and this directory does not. Fix a row there, once. This
file keeps what only matters when you change the repository.

## Tests

| you see | cause | fix |
| --- | --- | --- |
| a deep-tree test aborts with a stack overflow in `assert_eq!` | before #79 the failure message formatted the tree with the recursive `Debug` | prefer `assert!(a == b)` (L6) |
| the allocation test is flaky | another test thread allocated | keep it as a single `#[test]` in its own file |
| a filesystem test collides across runs | shared temp name | include the pid in the directory name |
| golden test fails after my change | markup changed | decide: wrong change, or intended — then regenerate and say so in the PR |
| the Rust and JS pages differ | you changed the bindings without a matching test | `tests/wasm.rs` builds one document through both APIs and compares bytes |

## Process


- Do not `git push`, publish, delete or regenerate fixtures without being asked (see
  [`agent-governance.md`](agent-governance.md)).
- `./scripts/verify.sh` failing on `cargo-deny` alone means the tool is not installed locally.
  Report it; do not edit the script.
- After adding an element: `./scripts/generate-tag-catalog.sh`, and keep `VOID_TAGS` sorted.
