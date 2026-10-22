#!/usr/bin/env bash
#
# Short fuzz run. Not part of scripts/verify.sh or the pull-request CI: libFuzzer needs
# nightly, and a red build there must not block a merge. The nightly workflow
# (.github/workflows/fuzz.yml) calls this.
#
#   FUZZ_SECONDS=30 ./scripts/fuzz.sh
#
# Every target runs even if one fails, so a night that finds a crash in one target still
# exercises the others; the exit status is non-zero if any of them failed.

set -uo pipefail

cd "$(dirname "$0")/.."

if ! command -v cargo-fuzz >/dev/null 2>&1; then
  echo "cargo-fuzz is not installed." >&2
  echo "  cargo install cargo-fuzz" >&2
  exit 1
fi

seconds="${FUZZ_SECONDS:-30}"
failed=""

for target in escape_text escape_attribute url_policy json_script; do
  echo "==> $target (${seconds}s)"
  if ! cargo +nightly fuzz run "$target" -- -max_total_time="$seconds"; then
    failed="$failed $target"
  fi
done

if [ -n "$failed" ]; then
  echo "fuzz found a problem in:$failed" >&2
  echo "reproduce with: cargo +nightly fuzz run <target> fuzz/artifacts/<target>/<file>" >&2
  exit 1
fi
