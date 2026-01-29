#!/bin/bash
# Gate C: Direction API Usage Check
# Prevents is_home from being used for direction calculations in match_sim/
#
# BAD patterns (direction should use attacks_right, not is_home):
#   if is_home { goal_x = 105.0 }
#   let goal_x = if is_home { 0.0 } else { 105.0 }
#   if is_home { x > 50 } else { x < 50 }
#
# GOOD patterns (team identity, not direction):
#   let team_range = if is_home { 0..11 } else { 11..22 }
#   if is_home { home_stats } else { away_stats }

set -e

SEARCH_PATH="crates/of_core/src/engine/match_sim"

echo "=== Gate C: Direction API Usage Check ==="
echo "Scanning: $SEARCH_PATH"
echo ""

# Pattern 1: is_home used with goal coordinates
PATTERN1='if is_home \{[^}]*(goal|GOAL|105\.0|0\.0)[^}]*\}'
HITS1=$(grep -rn "$PATTERN1" --include="*.rs" "$SEARCH_PATH" 2>/dev/null | grep -v "// OK:" | grep -v "test" || true)

# Pattern 2: is_home used with x-coordinate comparisons
PATTERN2='if is_home \{[^}]*x\s*[<>=][^}]*\}'
HITS2=$(grep -rn "$PATTERN2" --include="*.rs" "$SEARCH_PATH" 2>/dev/null | grep -v "// OK:" | grep -v "test" || true)

# Pattern 3: is_home assigned to attacks_right (parameter confusion)
PATTERN3='let attacks_right\s*=\s*is_home'
HITS3=$(grep -rn "$PATTERN3" --include="*.rs" "$SEARCH_PATH" 2>/dev/null | grep -v "// OK:" || true)

TOTAL_HITS=""
if [ -n "$HITS1" ]; then
    echo "❌ Pattern 1 hits (is_home with goal coordinates):"
    echo "$HITS1"
    echo ""
    TOTAL_HITS="$HITS1"
fi

if [ -n "$HITS2" ]; then
    echo "❌ Pattern 2 hits (is_home with x-coordinate comparisons):"
    echo "$HITS2"
    echo ""
    TOTAL_HITS="$TOTAL_HITS$HITS2"
fi

if [ -n "$HITS3" ]; then
    echo "❌ Pattern 3 hits (is_home assigned to attacks_right):"
    echo "$HITS3"
    echo ""
    TOTAL_HITS="$TOTAL_HITS$HITS3"
fi

if [ -n "$TOTAL_HITS" ]; then
    echo "=== GATE C FAILED ==="
    echo "Found is_home used for direction calculations."
    echo "Use attacks_right instead of is_home for:"
    echo "  - Goal coordinates"
    echo "  - Forward/backward direction"
    echo "  - Field position comparisons"
    exit 1
else
    echo "✅ Gate C PASSED - No is_home direction patterns found"
    exit 0
fi
