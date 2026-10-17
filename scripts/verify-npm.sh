#!/usr/bin/env bash
#
# Verifies the NPM package the way a consumer would receive it.
#
# The important part is that it installs a **packed tarball** into a throwaway directory
# rather than linking the workspace. A workspace link resolves files that `npm pack` would
# never ship, so it hides exactly the mistakes this is meant to catch: a wrong `files`
# allowlist, a broken `exports` map, a missing type-marker package.json.

set -euo pipefail

cd "$(dirname "$0")/.."

if [ ! -f npm/node/molt.js ]; then
  echo "==> Building the WebAssembly package first"
  ./scripts/build-wasm.sh
fi

echo "==> tsc --noEmit against the generated typings"
npx --yes --package typescript@5 -- tsc --project npm/tsconfig.json

echo "==> npm pack"
TARBALL="$(cd npm && npm pack --silent)"
TARBALL_PATH="$PWD/npm/$TARBALL"
trap 'rm -f "$TARBALL_PATH"' EXIT
echo "    $TARBALL"

CONSUMER="$(mktemp -d)"
trap 'rm -f "$TARBALL_PATH"; rm -rf "$CONSUMER"' EXIT

echo "==> Installing the tarball into a fresh consumer project"
(
  cd "$CONSUMER"
  printf '{ "name": "consumer", "private": true, "type": "module" }\n' > package.json
  npm install --silent --no-audit --no-fund "$TARBALL_PATH"
)

echo "==> Rendering the golden page through the installed package"
MOLT_MODULE="$CONSUMER/node_modules/molt/node/molt.js" node npm/smoke.mjs

echo
echo "NPM package verified."
