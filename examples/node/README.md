# Node example

Generates a two-page static site from Node, through the WebAssembly build.

```bash
./scripts/build-wasm.sh          # produces npm/node/
node examples/node/build.mjs     # writes examples/node/dist/
```

The Rust equivalent is `cargo run --example static_site`. Both produce the same markup,
because both drive the same renderer — the bindings wrap the real node tree rather than
concatenating strings.

For the stricter version of this check, see `npm/smoke.mjs`, which renders the golden
fixture page through WebAssembly and diffs it byte for byte against the file the Rust tests
use.
