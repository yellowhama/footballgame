#!/bin/bash
set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== RunOps: Uploading Test Results to Supabase ===${NC}"

# Generate run ID
RUN_ID=$(uuidgen)
echo "Run ID: $RUN_ID"

# Get environment variables
BRANCH="${GITHUB_REF_NAME:-main}"
COMMIT_SHA="${GITHUB_SHA:-unknown}"
BUILD_ID="${GITHUB_RUN_ID:-0}"

# Get test results from environment or defaults
TOTAL_TESTS="${TOTAL_TESTS:-0}"
PASSED_TESTS="${PASSED_TESTS:-0}"
FAILED_TESTS="${FAILED_TESTS:-0}"
TEST_EXIT_CODE="${TEST_EXIT_CODE:-1}"

# Determine overall status
if [ "$TEST_EXIT_CODE" == "0" ]; then
    STATUS="success"
else
    STATUS="fail"
fi

# Determine contract_pass
if [ "$FAILED_TESTS" == "0" ] && [ "$TOTAL_TESTS" != "0" ]; then
    CONTRACT_PASS="true"
else
    CONTRACT_PASS="false"
fi

echo "Branch: $BRANCH"
echo "Commit: $COMMIT_SHA"
echo "Build ID: $BUILD_ID"
echo "Total Tests: $TOTAL_TESTS"
echo "Passed: $PASSED_TESTS"
echo "Failed: $FAILED_TESTS"
echo "Status: $STATUS"
echo "Contract Pass: $CONTRACT_PASS"

# Check required environment variables
if [ -z "$SUPABASE_URL" ]; then
    echo -e "${YELLOW}Warning: SUPABASE_URL not set. Skipping upload.${NC}"
    exit 0
fi

if [ -z "$SUPABASE_SERVICE_KEY" ]; then
    echo -e "${YELLOW}Warning: SUPABASE_SERVICE_KEY not set. Skipping upload.${NC}"
    exit 0
fi

# Create match_run entry
echo -e "${GREEN}Creating match_run entry...${NC}"

RESPONSE=$(curl -s -w "\nHTTP_STATUS:%{http_code}" \
  -X POST \
  -H "Content-Type: application/json" \
  -H "apikey: ${SUPABASE_SERVICE_KEY}" \
  -H "Authorization: Bearer ${SUPABASE_SERVICE_KEY}" \
  -H "Prefer: return=minimal" \
  "${SUPABASE_URL}/rest/v1/match_runs" \
  -d "{
    \"id\": \"${RUN_ID}\",
    \"branch\": \"${BRANCH}\",
    \"commit_sha\": \"${COMMIT_SHA}\",
    \"build_id\": \"${BUILD_ID}\",
    \"seed\": 42,
    \"status\": \"${STATUS}\",
    \"contract_pass\": ${CONTRACT_PASS},
    \"contract_fail_count\": ${FAILED_TESTS}
  }")

# Extract HTTP status
HTTP_STATUS=$(echo "$RESPONSE" | grep "HTTP_STATUS:" | cut -d':' -f2)
BODY=$(echo "$RESPONSE" | sed '/HTTP_STATUS:/d')

if [ "$HTTP_STATUS" == "201" ] || [ "$HTTP_STATUS" == "200" ]; then
    echo -e "${GREEN}✓ Successfully created match_run${NC}"
else
    echo -e "${RED}✗ Failed to create match_run (HTTP $HTTP_STATUS)${NC}"
    echo "Response: $BODY"
    exit 1
fi

# Parse test output and upload individual contract results
if [ -f "test_output.txt" ]; then
    echo -e "${GREEN}Parsing contract results...${NC}"

    # Extract contract test results
    # Format: test engine_contracts::tests::test_name ... ok/FAILED

    while IFS= read -r line; do
        if echo "$line" | grep -q "^test engine_contracts::"; then
            # Extract contract key and result
            CONTRACT_KEY=$(echo "$line" | sed -E 's/test engine_contracts::(tests::)?([a-z_]+).*/\2/')

            if echo "$line" | grep -q "... ok$"; then
                PASS="true"
                SEVERITY="info"
            elif echo "$line" | grep -q "... FAILED$"; then
                PASS="false"
                SEVERITY="critical"
            else
                continue
            fi

            # Upload contract result
            curl -s \
              -X POST \
              -H "Content-Type: application/json" \
              -H "apikey: ${SUPABASE_SERVICE_KEY}" \
              -H "Authorization: Bearer ${SUPABASE_SERVICE_KEY}" \
              -H "Prefer: return=minimal" \
              "${SUPABASE_URL}/rest/v1/contract_results" \
              -d "{
                \"run_id\": \"${RUN_ID}\",
                \"contract_key\": \"${CONTRACT_KEY}\",
                \"pass\": ${PASS},
                \"severity\": \"${SEVERITY}\"
              }" > /dev/null

            if [ "$PASS" == "true" ]; then
                echo -e "  ${GREEN}✓${NC} $CONTRACT_KEY"
            else
                echo -e "  ${RED}✗${NC} $CONTRACT_KEY"
            fi
        fi
    done < test_output.txt

    echo -e "${GREEN}✓ Contract results uploaded${NC}"
else
    echo -e "${YELLOW}Warning: test_output.txt not found. Skipping contract results.${NC}"
fi

# Create a system log entry
echo -e "${GREEN}Creating system log entry...${NC}"

curl -s \
  -X POST \
  -H "Content-Type: application/json" \
  -H "apikey: ${SUPABASE_SERVICE_KEY}" \
  -H "Authorization: Bearer ${SUPABASE_SERVICE_KEY}" \
  -H "Prefer: return=minimal" \
  "${SUPABASE_URL}/rest/v1/run_logs" \
  -d "{
    \"run_id\": \"${RUN_ID}\",
    \"source\": \"ci\",
    \"level\": \"info\",
    \"message\": \"Contract tests completed: ${PASSED_TESTS}/${TOTAL_TESTS} passed\",
    \"meta_json\": {
      \"ci_provider\": \"github_actions\",
      \"workflow\": \"contract-tests\",
      \"build_id\": \"${BUILD_ID}\"
    }
  }" > /dev/null

echo -e "${GREEN}✓ System log created${NC}"

# Output run_id for GitHub Actions
echo "run_id=$RUN_ID" >> $GITHUB_OUTPUT

echo -e "${GREEN}=== Upload Complete ===${NC}"
echo "View results: https://runops-console.vercel.app/runs/${RUN_ID}"
