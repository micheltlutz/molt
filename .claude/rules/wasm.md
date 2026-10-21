---
paths:
  - "src/wasm.rs"
  - "tests/wasm.rs"
  - "examples/web/**"
  - "examples/node/**"
  - "scripts/build-wasm.sh"
---

# WebAssembly bindings

- The `wasm` feature is gated by **feature**, never by `target_arch`, so native tests
  exercise it. `ssg` is the only module compiled out on `wasm32`, and `rayon` never enters
  the wasm dependency graph.
- Arguments cross the boundary **by value**: they consume the JS handle. Taking `&T` forces
  a clone of the whole tree on every call (L12). Where an owned argument is the only form
  wasm-bindgen can marshal, the `#[allow(clippy::needless_pass_by_value)]` sits under a
  comment saying so.
- `WElement` and `WDocument` apply the same validation and policy as the Rust API: the tag
  name in `new WElement`, the indent string in the render options. A binding is never a
  bypass.
- `cargo check` is not a test. Run `wasm-pack test --node -- --features wasm`; `verify.sh`
  does when `wasm-pack` is installed and says when it is not, and CI always does.
- `scripts/build-wasm.sh` builds the three targets (`web`, `node`, `bundler`) and fails
  above the gzipped size budget. `panic = "abort"` in the release profile is part of that
  budget; do not change it for the server case.
- The `examples/web` and `examples/node` READMEs are run, not only read, when the surface
  changes. A guide that compiles natively still has to compile for `wasm32` (L31).
