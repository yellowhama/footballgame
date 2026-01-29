#!/bin/bash
set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== RunOps: Running Godot Contract Tests ===${NC}"

# Check for Godot executable
GODOT_BIN="${GODOT_PATH:-godot}"

if [ ! -f "$GODOT_BIN" ] && ! command -v godot &> /dev/null; then
    echo -e "${RED}Error: Godot executable not found${NC}"
    echo "Set GODOT_PATH environment variable or ensure 'godot' is in PATH"
    exit 1
fi

echo "Using Godot: $GODOT_BIN"

# Set run ID from environment or generate
if [ -z "$RUNOPS_RUN_ID" ]; then
    export RUNOPS_RUN_ID=$(uuidgen)
fi

echo "Run ID: $RUNOPS_RUN_ID"

# Run Godot tests in headless mode
echo -e "${GREEN}Running Godot contract tests...${NC}"

OUTPUT_FILE="godot_test_output.txt"

set +e
$GODOT_BIN \
    --headless \
    --script ./scripts/tests/contract_runner.gd \
    -- --contract-tests \
    > "$OUTPUT_FILE" 2>&1
EXIT_CODE=$?
set -e

# Display output
cat "$OUTPUT_FILE"

# Parse results
TOTAL=$(grep "Total:" "$OUTPUT_FILE" | grep -oP '\d+' || echo "0")
PASSED=$(grep "Passed:" "$OUTPUT_FILE" | grep -oP '\d+' || echo "0")
FAILED=$(grep "Failed:" "$OUTPUT_FILE" | grep -oP '\d+' || echo "0")

echo ""
echo -e "${GREEN}=== Godot Test Summary ===${NC}"
echo "Total:  $TOTAL"
echo "Passed: $PASSED"
echo "Failed: $FAILED"

# Extract CONTRACT_RESULT lines for API upload
if [ -n "$SUPABASE_URL" ] && [ -n "$SUPABASE_SERVICE_KEY" ]; then
    echo -e "${GREEN}Uploading Godot test results to Supabase...${NC}"

    grep "CONTRACT_RESULT:" "$OUTPUT_FILE" | while read -r line; do
        # Extract JSON (everything after "CONTRACT_RESULT:")
        JSON=$(echo "$line" | sed 's/CONTRACT_RESULT://')

        # Post to API
        curl -s \
            -X POST \
            -H "Content-Type: application/json" \
            -H "apikey: ${SUPABASE_SERVICE_KEY}" \
            -H "Authorization: Bearer ${SUPABASE_SERVICE_KEY}" \
            -H "Prefer: return=minimal" \
            "${SUPABASE_URL}/rest/v1/run_logs" \
            -d "$JSON" > /dev/null

        echo -e "  ${GREEN}✓${NC} Uploaded result"
    done

    echo -e "${GREEN}✓ Godot results uploaded${NC}"
else
    echo -e "${YELLOW}Skipping Supabase upload (credentials not set)${NC}"
fi

# Exit with same code as Godot
if [ $EXIT_CODE -ne 0 ]; then
    echo -e "${RED}✗ Godot tests failed${NC}"
    exit $EXIT_CODE
else
    echo -e "${GREEN}✓ Godot tests passed${NC}"
    exit 0
fi
