# Node example

Generates a two-page static site from Node, through the WebAssembly build.

```bash
./scripts/build-wasm.sh          # produces pkg/node/
node examples/node/build.mjs     # writes examples/node/dist/
```

The Rust equivalent is `cargo run --example static_site`. Both produce the same markup,
because both drive the same renderer — the bindings wrap the real node tree rather than
concatenating strings.

For the stricter version of this check, see `a_document_matches_the_rust_equivalent_byte_for_byte`
in `tests/wasm.rs`, which builds the same document through both APIs and compares the bytes.
