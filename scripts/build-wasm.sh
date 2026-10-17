#!/usr/bin/env bash
#
# Builds the WebAssembly bindings for all three JavaScript environments and reports the
# gzipped size of each.
#
# A WebAssembly library nobody can afford to load is not a shipped library, so the size is
# printed on every build and checked against a budget in CI.

set -euo pipefail

cd "$(dirname "$0")/.."

BUDGET_KB="${MOLT_WASM_BUDGET_KB:-120}"
OUT="pkg"

for target in web nodejs bundler; do
  case "$target" in
    nodejs) dir="node" ;;
    *)      dir="$target" ;;
  esac

  echo "==> wasm-pack build --target $target"
  wasm-pack build \
    --release \
    --target "$target" \
    --out-dir "$OUT/$dir" \
    --out-name molt \
    --no-pack \
    -- --features wasm
done

echo
echo "==> Sizes"
FAILED=0
for dir in web node bundler; do
  wasm="$OUT/$dir/molt_bg.wasm"
  [ -f "$wasm" ] || continue

  raw_kb=$(( $(wc -c < "$wasm") / 1024 ))
  gz_kb=$(( $(gzip -c "$wasm" | wc -c) / 1024 ))
  printf '  %-8s %4d KB raw   %4d KB gzipped\n' "$dir" "$raw_kb" "$gz_kb"

  if [ "$gz_kb" -gt "$BUDGET_KB" ]; then
    printf '  FAIL: %s is %d KB gzipped, over the %d KB budget\n' "$dir" "$gz_kb" "$BUDGET_KB"
    FAILED=1
  fi
done

exit "$FAILED"
