#!/usr/bin/env bash
#
# Fails when a public item added since the last release is missing from `## [Unreleased]` in
# CHANGELOG.md.
#
# An entry can be lost without anyone noticing: in the 2.0.0 cycle four of them vanished in
# conflict resolutions and the build stayed green. This asks the code instead of the history.
# It lists the names of the `pub fn`, `struct`, `enum`, `trait`, `const`, `type`, `static` and
# `mod` lines added under src/ since the last tag, drops the ones that already existed under that
# name at the tag, and requires each remaining name to appear as a word in [Unreleased].
# A name that a contributor deliberately leaves out (a helper that is public only because of a
# module layout) is a sign that it should be `pub(crate)`.
#
# Needs the tags: a checkout without them (`git fetch --depth 1`) skips the check, and says so.
#
#   ./scripts/check-changelog.sh

set -uo pipefail
cd "$(dirname "$0")/.."

if ! tag=$(git describe --tags --abbrev=0 2>/dev/null); then
  echo "no release tag to compare with; skipped"
  exit 0
fi

KIND='(const[[:space:]]+fn|fn|struct|enum|trait|const|type|static|mod)'
unreleased=$(awk '/^## \[Unreleased\]/{found=1; next} /^## \[/{found=0} found' CHANGELOG.md)

# Added lines only. `pub(crate)` has a parenthesis after `pub`, so it never matches.
names=$(git diff "$tag" -U0 -- src \
  | grep -E "^\\+[[:space:]]*pub[[:space:]]+$KIND[[:space:]]+[A-Za-z_]" \
  | sed -E "s/^\\+[[:space:]]*pub[[:space:]]+$KIND[[:space:]]+([A-Za-z_][A-Za-z0-9_]*).*/\\2/" \
  | sort -u)

missing=""
checked=0
while IFS= read -r name; do
  [ -z "$name" ] && continue
  # Already public under this name at the tag: not new, whatever moved around it.
  if git grep -qE "pub[[:space:]]+$KIND[[:space:]]+$name([^A-Za-z0-9_]|\$)" "$tag" -- src; then
    continue
  fi
  checked=$((checked + 1))
  if ! printf '%s\n' "$unreleased" | grep -qw -- "$name"; then
    missing="$missing $name"
  fi
done <<EOF2
$names
EOF2

if [ -n "$missing" ]; then
  echo "public items added since $tag are not mentioned in [Unreleased]:" >&2
  for name in $missing; do echo "  $name" >&2; done
  exit 1
fi
echo "$checked new public item(s) since $tag, all mentioned in [Unreleased]"
