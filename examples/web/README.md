# Browser example

A single page that loads the WebAssembly build and renders markup in the browser, with a
live-editable builder snippet.

```bash
./scripts/build-wasm.sh          # produces npm/web/
python3 -m http.server           # from the repository root
# then open http://localhost:8000/examples/web/
```

No bundler and no build step beyond `wasm-pack`. `file://` will not work — a module script
needs a real origin.

## What it demonstrates

- The same builder API as the Rust crate, driving the same renderer.
- `render()` on a `WDocument` is pretty-printed and includes the doctype.
- Escaping happens inside the renderer: type `<script>` into the code and watch it come out
  as `&lt;script&gt;` in the markup pane.
