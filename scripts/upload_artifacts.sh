#!/bin/bash
set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== RunOps: Uploading Artifacts to Supabase Storage ===${NC}"

# Check required environment variables
if [ -z "$SUPABASE_URL" ]; then
    echo -e "${YELLOW}Warning: SUPABASE_URL not set. Skipping upload.${NC}"
    exit 0
fi

if [ -z "$SUPABASE_SERVICE_KEY" ]; then
    echo -e "${YELLOW}Warning: SUPABASE_SERVICE_KEY not set. Skipping upload.${NC}"
    exit 0
fi

# Get run_id from argument or environment
RUN_ID="${1:-$RUNOPS_RUN_ID}"

if [ -z "$RUN_ID" ]; then
    echo -e "${RED}Error: RUN_ID required${NC}"
    echo "Usage: $0 <run_id> [artifact_path...]"
    exit 1
fi

echo "Run ID: $RUN_ID"

# Get artifact paths from remaining arguments
shift 2>/dev/null || true
ARTIFACT_PATHS=("$@")

# If no paths provided, find common artifact files
if [ ${#ARTIFACT_PATHS[@]} -eq 0 ]; then
    echo "Searching for artifacts..."

    # Common artifact patterns
    PATTERNS=(
        "*.replay"
        "*.log"
        "test_output.txt"
        "godot_test_output.txt"
        "*.bin"
        "*.dat"
    )

    for pattern in "${PATTERNS[@]}"; do
        while IFS= read -r -d $'\0' file; do
            ARTIFACT_PATHS+=("$file")
        done < <(find . -maxdepth 2 -name "$pattern" -type f -print0 2>/dev/null)
    done
fi

if [ ${#ARTIFACT_PATHS[@]} -eq 0 ]; then
    echo -e "${YELLOW}No artifacts found to upload${NC}"
    exit 0
fi

echo "Found ${#ARTIFACT_PATHS[@]} artifact(s)"

# Upload each artifact
UPLOAD_COUNT=0
FAIL_COUNT=0

for ARTIFACT_PATH in "${ARTIFACT_PATHS[@]}"; do
    if [ ! -f "$ARTIFACT_PATH" ]; then
        echo -e "${YELLOW}  ✗ Skipping (not found): $ARTIFACT_PATH${NC}"
        continue
    fi

    FILENAME=$(basename "$ARTIFACT_PATH")
    STORAGE_PATH="$RUN_ID/$FILENAME"

    echo -n "  Uploading $FILENAME..."

    # Get file size
    FILE_SIZE=$(stat -c%s "$ARTIFACT_PATH" 2>/dev/null || stat -f%z "$ARTIFACT_PATH" 2>/dev/null || echo "0")

    # Upload to Supabase Storage
    RESPONSE=$(curl -s -w "\nHTTP_STATUS:%{http_code}" \
        -X POST \
        -H "apikey: ${SUPABASE_SERVICE_KEY}" \
        -H "Authorization: Bearer ${SUPABASE_SERVICE_KEY}" \
        -F "file=@${ARTIFACT_PATH}" \
        "${SUPABASE_URL}/storage/v1/object/test-artifacts/${STORAGE_PATH}")

    # Extract HTTP status
    HTTP_STATUS=$(echo "$RESPONSE" | grep "HTTP_STATUS:" | cut -d':' -f2)
    BODY=$(echo "$RESPONSE" | sed '/HTTP_STATUS:/d')

    if [ "$HTTP_STATUS" == "200" ] || [ "$HTTP_STATUS" == "201" ]; then
        echo -e " ${GREEN}✓${NC} ($FILE_SIZE bytes)"
        UPLOAD_COUNT=$((UPLOAD_COUNT + 1))
    else
        echo -e " ${RED}✗${NC} (HTTP $HTTP_STATUS)"
        echo "  Response: $BODY"
        FAIL_COUNT=$((FAIL_COUNT + 1))
    fi
done

# Summary
echo ""
echo -e "${GREEN}=== Upload Summary ===${NC}"
echo "Uploaded:  $UPLOAD_COUNT file(s)"
echo "Failed:    $FAIL_COUNT file(s)"

if [ $FAIL_COUNT -gt 0 ]; then
    echo -e "${YELLOW}Some artifacts failed to upload${NC}"
    exit 1
fi

echo -e "${GREEN}✓ All artifacts uploaded successfully${NC}"
