#!/usr/bin/env bash
#
# One command that is a superset of CI. Run it before reporting that a change is done.
#
# Ported from Winged-Swift/Scripts/verify.sh, including its two best ideas: accumulate
# failures instead of stopping at the first one, so a single run reports every problem;
# and finish with an end-to-end smoke test that builds a throwaway crate against this
# checkout, proving the library is actually consumable.

set -uo pipefail

RED=$'\033[0;31m'; GREEN=$'\033[0;32m'; BLUE=$'\033[0;34m'; RESET=$'\033[0m'
FAILURES=0
# A private log file: a fixed path in /tmp is one another user on a shared machine can
# pre-create as a symlink to a file of yours.
LOG=$(mktemp "${TMPDIR:-/tmp}/molt-verify.XXXXXX")
trap 'rm -f "$LOG"' EXIT

step() { printf '\n%s==> %s%s\n' "$BLUE" "$1" "$RESET"; }
ok()   { printf '%s  ok%s  %s\n' "$GREEN" "$RESET" "$1"; }
fail() { printf '%s  FAIL%s %s\n' "$RED" "$RESET" "$1"; FAILURES=$((FAILURES + 1)); }

run() {
  local label="$1"; shift
  if "$@" > "$LOG" 2>&1; then
    ok "$label"
  else
    fail "$label"
    sed 's/^/      /' "$LOG" | tail -40
  fi
}

cd "$(dirname "$0")/.."

step "1/8  Build"
run "cargo build --all-features" cargo build --all-features
run "cargo build --no-default-features" cargo build --no-default-features
run "wasm32 target" cargo build --features wasm --target wasm32-unknown-unknown

step "2/8  Test"
run "cargo test --all-features" cargo test --locked --all-features

step "3/8  Lint"
run "cargo fmt --check" cargo fmt --all --check
run "cargo clippy" cargo clippy --locked --all-targets --all-features -- -D warnings
if command -v cargo-deny >/dev/null 2>&1; then
  run "cargo deny" cargo deny check advisories bans licenses sources
else
  fail "cargo-deny is not installed (cargo install cargo-deny, or the CI install-action)"
fi

step "4/8  Tag catalog freshness"
if [ -x scripts/generate-tag-catalog.sh ]; then
  run "tag catalog is current" ./scripts/generate-tag-catalog.sh --check
else
  fail "scripts/generate-tag-catalog.sh is missing or not executable"
fi

step "5/8  Golden fixtures"
run "fixtures reproduce byte-for-byte" cargo test --test golden

step "6/8  End-to-end smoke test"
# A throwaway crate that depends on this checkout by path. This is what catches a library
# that compiles in-tree but cannot actually be consumed — a broken feature gate, a type
# that is not public, a doc example that only works with dev-dependencies in scope.
SMOKE_DIR="$(mktemp -d)"
trap 'rm -rf "$SMOKE_DIR"' EXIT

mkdir -p "$SMOKE_DIR/src"
cat > "$SMOKE_DIR/Cargo.toml" <<TOML
[package]
name = "molt-smoke"
version = "0.0.0"
edition = "2024"

[dependencies]
molt-framework = { path = "$PWD" }

[workspace]
TOML

cat > "$SMOKE_DIR/src/main.rs" <<'RS'
use molt::prelude::*;
use molt::Document;

fn main() {
    let page = Document::new(Some("pt-BR"))
        .head_children([title().text("Smoke")])
        .body_children([h1().text("It renders"), p().text("<ok>")]);
    println!("{}", page.render());
}
RS

if (cd "$SMOKE_DIR" && cargo run --quiet > output.html 2>/tmp/molt-smoke.log); then
  SMOKE_OUT="$SMOKE_DIR/output.html"
  for expected in '<!DOCTYPE html>' '<h1>It renders</h1>' '&lt;ok&gt;'; do
    if grep -qF -- "$expected" "$SMOKE_OUT"; then
      ok "rendered output contains ${expected}"
    else
      fail "rendered output is missing ${expected}"
      sed 's/^/      /' "$SMOKE_OUT"
    fi
  done
else
  fail "the smoke crate did not build"
  sed 's/^/      /' /tmp/molt-smoke.log | tail -40
fi

step "7/8  Winged-Swift cross-references"

# Skips itself when the sibling checkout is absent, which is why it can sit in the gate:
# CI never has Winged-Swift, and a contributor who does gets told when a `Ports` comment
# names a test that no longer exists.
if PARITY=$(./scripts/check-swift-parity.sh 2>&1); then
  printf '%s\n' "$PARITY" | tail -3 | sed 's/^/      /'
  ok "cross-references resolve"
else
  fail "a Ports comment names a Swift test that does not exist"
  printf '%s\n' "$PARITY" | grep stale | sed 's/^/      /'
fi

step "8/8  Agent knowledge base"
# Every lesson in .claude/skills/molt names the tests or files that guard it. A guard that
# was renamed or deleted is a claim nothing checks any more.
if [ -x scripts/check-knowledge-base.sh ]; then
  run "knowledge-base guards resolve" ./scripts/check-knowledge-base.sh
else
  fail "scripts/check-knowledge-base.sh is missing or not executable"
fi

printf '\n'
if [ "$FAILURES" -eq 0 ]; then
  printf '%sAll checks passed.%s\n' "$GREEN" "$RESET"
  exit 0
fi

printf '%s%d check(s) failed.%s\n\n' "$RED" "$FAILURES" "$RESET"
printf 'Environment:\n'
printf '  uname:  %s\n' "$(uname -a)"
printf '  rustc:  %s\n' "$(rustc --version 2>&1)"
printf '  cargo:  %s\n' "$(cargo --version 2>&1)"
exit 1
