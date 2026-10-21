#!/usr/bin/env bash
#
# Fails when the agent knowledge base points at something that no longer exists.
#
# Every lesson in .claude/skills/molt/references/lessons-learned.md ends with a "Guard" line
# naming the tests or files that enforce it. A lesson whose guard was renamed or deleted is
# a claim nothing checks any more, so this script turns that into a failure instead of
# letting the knowledge rot quietly. It also checks that every relative link under .claude/
# resolves, that every rule's `paths:` glob still matches a tracked file, and that AGENTS.md
# lists every rule, command and agent, so none of them can go stale unnoticed.
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

# Relative markdown links in one file, resolved against its own directory. Absolute URLs and
# in-page anchors are skipped.
check_links() {
  local file="$1" dir target
  dir=$(dirname "$file")
  for target in $(grep -o '](\([^)#]*\)' "$file" | sed 's/^](//; s/)$//' | grep -v '^https\?://' | grep -v '^$' | sort -u); do
    CHECKED=$((CHECKED + 1))
    [ -e "$dir/$target" ] || fail "$dir/$target (linked from $file)"
  done
}

# 1. Every reference linked from SKILL.md exists.
for ref in $(grep -o '(references/[A-Za-z0-9_.-]*\.md)' "$KB/SKILL.md" | tr -d '()' | sort -u); do
  CHECKED=$((CHECKED + 1))
  [ -f "$KB/$ref" ] || fail "$KB/$ref (linked from SKILL.md)"
done

# 1b. Every relative link in any skill's SKILL.md resolves, so a moved file cannot leave a
#     skill pointing at nothing.
for skill in .claude/skills/*/SKILL.md; do
  check_links "$skill"
done

# 1c. The same for every rule, command and agent, which link into the skill's references.
for file in .claude/rules/*.md .claude/commands/*.md .claude/agents/*.md; do
  [ -f "$file" ] || continue
  check_links "$file"
done

# 1d. Every `paths:` glob in a rule matches at least one tracked file. A rule scoped to a path
#     that no longer exists never loads again, which is the quiet way for a rule to die.
#     Only the block-list form is read (one `  - "glob"` per line), which is the form the
#     rules use; git's own glob matching stands in for Claude Code's, `**` included.
for rule in .claude/rules/*.md; do
  [ -f "$rule" ] || continue
  # Read line by line: an unquoted `$(…)` would word-split the output and then expand each
  # glob against the working tree, so `.claude/**` would turn into a list of directories
  # and a glob matching nothing would never be seen (L39).
  while IFS= read -r glob; do
    [ -n "$glob" ] || continue
    CHECKED=$((CHECKED + 1))
    if [ -z "$(git ls-files -- ":(glob)$glob" | head -n 1)" ]; then
      fail "$glob (paths: glob in $rule matches no tracked file)"
    fi
  done < <(awk '
    NR == 1 && $0 != "---" { exit }
    NR > 1 && $0 == "---" { exit }
    /^paths:/ { on = 1; next }
    on && /^  - / { sub(/^  - /, ""); gsub(/"/, ""); print; next }
    on { on = 0 }
  ' "$rule")
done

# 1e. AGENTS.md is the map an agent reads first; a rule, command or agent it does not list
#     is one nobody is told about.
for file in .claude/rules/*.md .claude/commands/*.md .claude/agents/*.md; do
  [ -f "$file" ] || continue
  CHECKED=$((CHECKED + 1))
  case "$file" in
    .claude/commands/*) name="/$(basename "$file" .md)" ;;
    *) name="$file" ;;
  esac
  grep -qF -- "$name" AGENTS.md || fail "$name (not listed in AGENTS.md)"
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
