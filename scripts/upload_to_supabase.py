#!/usr/bin/env python3
"""
Upload Contract Test Results to Supabase

Reads JSON from stdin or file and uploads to Supabase.
"""

import json
import sys
import os
import urllib.request
import urllib.error


# Supabase credentials from environment or config
SUPABASE_URL = os.environ.get('NEXT_PUBLIC_SUPABASE_URL', 'https://poyclapxmvulvboiebxq.supabase.co')
SUPABASE_KEY = os.environ.get('SUPABASE_SERVICE_ROLE_KEY', 'sbp_4678ce7a0b136fb6bd15366ec4f203625bdc1f88')


def upload_to_supabase(record):
    """Upload run and contract results to Supabase"""

    # Extract run data
    match_run = {
        "id": record["run_id"],
        "created_at": record["created_at"],
        "branch": record["branch"],
        "commit_sha": record["commit_sha"],
        "build_id": record["build_id"],
        "seed": record["seed"],
        "scenario_id": record["scenario_id"],
        "status": record["status"],
        "duration_ms": record["duration_ms"],
        "contract_pass": record["contract_pass"],
        "contract_fail_count": record["contract_fail_count"],
        "meta": {
            "rust_version": "1.75.0",
            "test_env": "local"
        }
    }

    # Upload match_run
    print(f"📤 Uploading match_run {match_run['id']}...", file=sys.stderr)

    url = f"{SUPABASE_URL}/rest/v1/match_runs"
    headers = {
        'apikey': SUPABASE_KEY,
        'Authorization': f'Bearer {SUPABASE_KEY}',
        'Content-Type': 'application/json',
        'Prefer': 'return=representation'
    }

    data = json.dumps(match_run).encode('utf-8')
    req = urllib.request.Request(url, data=data, headers=headers, method='POST')

    try:
        with urllib.request.urlopen(req) as response:
            result = json.loads(response.read().decode('utf-8'))
            print(f"✅ match_run inserted: {match_run['id'][:8]}...", file=sys.stderr)
    except urllib.error.HTTPError as e:
        error_body = e.read().decode('utf-8')
        print(f"❌ Error inserting match_run: {e.code} {e.reason}", file=sys.stderr)
        print(f"   {error_body}", file=sys.stderr)
        return False

    # Upload contract_results
    print(f"📤 Uploading {len(record['contracts'])} contract results...", file=sys.stderr)

    for i, contract in enumerate(record['contracts']):
        contract_result = {
            "run_id": record["run_id"],
            "contract_key": contract["contract_key"],
            "pass": contract["pass"],
            "severity": contract["severity"],
            "details_json": contract.get("details")
        }

        url = f"{SUPABASE_URL}/rest/v1/contract_results"
        data = json.dumps(contract_result).encode('utf-8')
        req = urllib.request.Request(url, data=data, headers=headers, method='POST')

        try:
            with urllib.request.urlopen(req) as response:
                result = json.loads(response.read().decode('utf-8'))
                status = "✅" if contract["pass"] else "❌"
                print(f"   {status} {contract['contract_key']}", file=sys.stderr)
        except urllib.error.HTTPError as e:
            error_body = e.read().decode('utf-8')
            print(f"   ❌ Error: {contract['contract_key']}: {e.code}", file=sys.stderr)
            continue

    return True


def main():
    # Read JSON from stdin or file
    if len(sys.argv) > 1:
        with open(sys.argv[1], 'r') as f:
            record = json.load(f)
    else:
        record = json.load(sys.stdin)

    # Validate record
    required_fields = ["run_id", "created_at", "branch", "commit_sha", "status", "contracts"]
    for field in required_fields:
        if field not in record:
            print(f"Error: Missing required field: {field}", file=sys.stderr)
            sys.exit(1)

    # Upload
    print(f"\n🚀 Uploading to Supabase ({SUPABASE_URL})...", file=sys.stderr)
    success = upload_to_supabase(record)

    if success:
        print(f"\n✅ Upload complete!", file=sys.stderr)
        print(f"   Run ID: {record['run_id']}", file=sys.stderr)
        print(f"   Status: {record['status']}", file=sys.stderr)
        print(f"   Passed: {6 - record['contract_fail_count']}/6", file=sys.stderr)
        print(f"\n🔗 View in Supabase:", file=sys.stderr)
        print(f"   https://supabase.com/dashboard/project/poyclapxmvulvboiebxq/editor", file=sys.stderr)
    else:
        print(f"\n❌ Upload failed", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
