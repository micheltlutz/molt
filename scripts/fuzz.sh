#!/usr/bin/env bash
#
# Short fuzz run. Not part of scripts/verify.sh or the pull-request CI: libFuzzer needs
# nightly, and a red build there must not block a merge. The nightly workflow calls this.
#
#   FUZZ_SECONDS=30 ./scripts/fuzz.sh

set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v cargo-fuzz >/dev/null 2>&1; then
  echo "cargo-fuzz is not installed." >&2
  echo "  cargo install cargo-fuzz" >&2
  exit 1
fi

seconds="${FUZZ_SECONDS:-30}"

for target in escape_text escape_attribute url_policy; do
  echo "==> $target (${seconds}s)"
  cargo +nightly fuzz run "$target" -- -max_total_time="$seconds"
done
