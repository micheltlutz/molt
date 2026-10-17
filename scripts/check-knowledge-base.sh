#!/usr/bin/env bash
#
# Fails when the agent knowledge base points at something that no longer exists.
#
# Every lesson in .claude/skills/molt/references/lessons-learned.md ends with a "Guard" line
# naming the tests or files that enforce it. A lesson whose guard was renamed or deleted is
# a claim nothing checks any more, so this script turns that into a failure instead of
# letting the knowledge rot quietly. It also checks that every reference SKILL.md links to
# exists.
#
# Run by scripts/verify.sh. Usage: ./scripts/check-knowledge-base.sh

set -uo pipefail
cd "$(dirname "$0")/.."

KB=.claude/skills/molt
LESSONS=$KB/references/lessons-learned.md
FAILURES=0
CHECKED=0

fail() { printf '  missing: %s\n' "$1"; FAILURES=$((FAILURES + 1)); }

[ -f "$KB/SKILL.md" ] || { echo "no knowledge base at $KB"; exit 1; }
[ -f "$LESSONS" ] || { echo "missing $LESSONS"; exit 1; }

# 1. Every reference linked from SKILL.md exists.
for ref in $(grep -o '(references/[A-Za-z0-9_.-]*\.md)' "$KB/SKILL.md" | tr -d '()' | sort -u); do
  CHECKED=$((CHECKED + 1))
  [ -f "$KB/$ref" ] || fail "$KB/$ref (linked from SKILL.md)"
done

# 2. Every identifier or path named by a Guard line exists. A Guard runs from the line holding
#    "**Guard.**" to the next blank line, since long ones wrap.
guards=$(awk '
  /\*\*Guard\.\*\*/ { on = 1 }
  on && /^[[:space:]]*$/ { on = 0 }
  on { print }
' "$LESSONS")

for token in $(printf '%s\n' "$guards" | grep -o '`[^`]*`' | tr -d '`' | sort -u); do
  CHECKED=$((CHECKED + 1))
  case "$token" in
    */* | *.md | *.rs | *.toml | *.sh)
      if [ -e "$token" ] || [ -e "$KB/references/$token" ]; then continue; fi
      fail "$token (path)"
      ;;
    *)
      if grep -rqF -- "$token" src tests benches fuzz examples 2>/dev/null; then continue; fi
      fail "$token (no test, function or item with this name)"
      ;;
  esac
done

if [ "$FAILURES" -gt 0 ]; then
  printf 'knowledge base: %d of %d guards are stale\n' "$FAILURES" "$CHECKED"
  exit 1
fi
printf 'knowledge base: %d guards resolve\n' "$CHECKED"
