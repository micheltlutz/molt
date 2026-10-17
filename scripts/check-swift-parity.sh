#!/usr/bin/env bash
# Checks the `Ports `XTests.yyy`` cross-references against the Winged-Swift suite.
#
# Those comments are the only thing tying this test suite to the one it was ported from, and
# they rot silently: a reference written from memory rather than from the source points at
# nothing, and nobody finds out. This reads the Swift suite and says so.
#
# Winged-Swift is a separate local checkout, not a dependency, so this is a local tool rather than a
# CI gate — it exits 0 with a notice when the checkout is not there.
set -uo pipefail

cd "$(dirname "$0")/.."

SWIFT="${WINGED_SWIFT:-../WingedSwiftProjects/Winged-Swift}/Tests/WingedSwiftTests"

if [ ! -d "$SWIFT" ]; then
    echo "Winged-Swift not found at $SWIFT — skipping."
    echo "Point WINGED_SWIFT at a checkout to run this."
    exit 0
fi

# Suites deliberately not ported, per issue #30:
#   DeprecatedAPITests       nothing is deprecated in 1.0
#   BuilderInitCoverageTests proves Swift's generated initialisers are reachable, which the
#                            define_elements! macro makes a non-question
SKIPPED_SUITES="DeprecatedAPITests BuilderInitCoverageTests"

stale=0
missing=0

# A reference is any `SuiteTests.caseName` in backticks. Matching on the "Ports " prefix
# instead would miss the second name in a two-line comment, which is where they wrap.
references() {
    # `SomeTests.swift` in prose is a file name, not a case reference.
    grep -rhoE '`[A-Za-z0-9]+Tests\.[A-Za-z0-9_]+`' src tests |
        tr -d '`' |
        grep -v '\.swift$' |
        sort -u
}

echo "==> Cross-references that name a Swift test"
while read -r reference; do
    suite="${reference%%.*}"
    name="${reference#*.}"
    if [ ! -f "$SWIFT/$suite.swift" ]; then
        echo "  stale  $reference — no such suite"
        stale=$((stale + 1))
    elif ! grep -q "func $name\b" "$SWIFT/$suite.swift"; then
        echo "  stale  $reference — suite has no such test"
        stale=$((stale + 1))
    fi
done < <(references)

[ "$stale" -eq 0 ] && echo "  all references resolve"

echo "==> Swift tests with no counterpart here"
for file in "$SWIFT"/*.swift; do
    suite=$(basename "$file" .swift)
    case " $SKIPPED_SUITES " in *" $suite "*) continue ;; esac

    while read -r name; do
        if ! references | grep -qx "$suite.$name"; then
            echo "  missing  $suite.$name"
            missing=$((missing + 1))
        fi
    done < <(grep -oE '@Test +func +[a-zA-Z0-9_]+' "$file" | awk '{print $NF}')
done

[ "$missing" -eq 0 ] && echo "  every Swift test is accounted for"

echo
echo "stale references: $stale"
echo "unported tests:   $missing"

# Only the stale references fail: a missing port is tracked work (#30), while a reference
# that points at nothing is actively misleading.
[ "$stale" -eq 0 ]
