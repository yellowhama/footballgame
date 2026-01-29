#!/usr/bin/env node
/**
 * Regression Detection Script
 *
 * Detects contract regressions (PASS → FAIL) and sends Slack notifications.
 *
 * Usage:
 *   node check_regressions.ts <run_id>
 *   tsx check_regressions.ts <run_id>
 *
 * Environment variables:
 *   - SUPABASE_URL: Supabase project URL
 *   - SUPABASE_SERVICE_KEY: Service role key
 *   - SLACK_WEBHOOK_URL: Slack incoming webhook URL
 */

interface Regression {
  contract_key: string;
  branch: string;
  current_commit: string;
  previous_commit: string;
  regression_time: string;
}

interface ContractResult {
  contract_key: string;
  pass: boolean;
}

interface MatchRun {
  id: string;
  branch: string | null;
  commit_sha: string | null;
  created_at: string;
}

const SUPABASE_URL = process.env.SUPABASE_URL;
const SUPABASE_SERVICE_KEY = process.env.SUPABASE_SERVICE_KEY;
const SLACK_WEBHOOK_URL = process.env.SLACK_WEBHOOK_URL;

async function fetchJson(url: string, options: RequestInit = {}) {
  const response = await fetch(url, {
    ...options,
    headers: {
      'apikey': SUPABASE_SERVICE_KEY!,
      'Authorization': `Bearer ${SUPABASE_SERVICE_KEY}`,
      'Content-Type': 'application/json',
      ...options.headers,
    },
  });

  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: ${await response.text()}`);
  }

  return response.json();
}

async function detectRegressions(runId: string): Promise<Regression[]> {
  console.log(`Checking for regressions in run: ${runId}`);

  // Get current run's contracts
  const currentContracts: ContractResult[] = await fetchJson(
    `${SUPABASE_URL}/rest/v1/contract_results?run_id=eq.${runId}&select=contract_key,pass`
  );

  if (!currentContracts || currentContracts.length === 0) {
    console.log('No contract results found for this run');
    return [];
  }

  // Get current run metadata
  const currentRunData = await fetchJson(
    `${SUPABASE_URL}/rest/v1/match_runs?id=eq.${runId}&select=branch,created_at,commit_sha`
  );

  const currentRun: MatchRun | undefined = currentRunData[0];

  if (!currentRun) {
    console.log('Run metadata not found');
    return [];
  }

  // Get previous run on same branch
  const previousRunData = await fetchJson(
    `${SUPABASE_URL}/rest/v1/match_runs?branch=eq.${currentRun.branch}&created_at=lt.${currentRun.created_at}&select=id,commit_sha&order=created_at.desc&limit=1`
  );

  const previousRun: MatchRun | undefined = previousRunData[0];

  if (!previousRun) {
    console.log('No previous run found on this branch');
    return [];
  }

  // Get previous run's contracts
  const previousContracts: ContractResult[] = await fetchJson(
    `${SUPABASE_URL}/rest/v1/contract_results?run_id=eq.${previousRun.id}&select=contract_key,pass`
  );

  // Find regressions
  const regressions: Regression[] = [];

  for (const current of currentContracts) {
    const previous = previousContracts.find(
      (p) => p.contract_key === current.contract_key
    );

    if (previous && previous.pass && !current.pass) {
      regressions.push({
        contract_key: current.contract_key,
        branch: currentRun.branch || 'unknown',
        current_commit: currentRun.commit_sha || runId,
        previous_commit: previousRun.commit_sha || previousRun.id,
        regression_time: currentRun.created_at,
      });
    }
  }

  console.log(`Found ${regressions.length} regression(s)`);
  return regressions;
}

async function sendSlackNotification(regressions: Regression[], runId: string) {
  if (!SLACK_WEBHOOK_URL) {
    console.log('SLACK_WEBHOOK_URL not set, skipping notification');
    return;
  }

  if (regressions.length === 0) {
    console.log('No regressions to report');
    return;
  }

  const consoleUrl = process.env.RUNOPS_CONSOLE_URL || 'http://localhost:3000';

  const blocks = [
    {
      type: 'header',
      text: {
        type: 'plain_text',
        text: `🚨 ${regressions.length} Contract Regression${regressions.length > 1 ? 's' : ''} Detected`,
        emoji: true,
      },
    },
    {
      type: 'section',
      text: {
        type: 'mrkdwn',
        text: regressions
          .map((r) => `• *${r.contract_key}* on \`${r.branch}\`\n  _${r.previous_commit.substring(0, 7)} → ${r.current_commit.substring(0, 7)}_`)
          .join('\n\n'),
      },
    },
    {
      type: 'divider',
    },
    {
      type: 'actions',
      elements: [
        {
          type: 'button',
          text: {
            type: 'plain_text',
            text: 'View in RunOps Console',
            emoji: true,
          },
          url: `${consoleUrl}/runs/${runId}`,
          style: 'danger',
        },
        {
          type: 'button',
          text: {
            type: 'plain_text',
            text: 'View All Runs',
            emoji: true,
          },
          url: `${consoleUrl}/runs`,
        },
      ],
    },
  ];

  const payload = {
    blocks,
    text: `${regressions.length} contract regression(s) detected`,
  };

  console.log('Sending Slack notification...');

  const response = await fetch(SLACK_WEBHOOK_URL, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(payload),
  });

  if (!response.ok) {
    throw new Error(`Slack API error: ${response.status} ${await response.text()}`);
  }

  console.log('✓ Slack notification sent');
}

// Main execution
async function main() {
  const runId = process.argv[2];

  if (!runId) {
    console.error('Usage: node check_regressions.ts <run_id>');
    process.exit(1);
  }

  if (!SUPABASE_URL || !SUPABASE_SERVICE_KEY) {
    console.error('Error: SUPABASE_URL and SUPABASE_SERVICE_KEY must be set');
    process.exit(1);
  }

  try {
    const regressions = await detectRegressions(runId);
    await sendSlackNotification(regressions, runId);

    if (regressions.length > 0) {
      console.log('\n⚠️  Regressions detected:');
      regressions.forEach((r) => {
        console.log(`  - ${r.contract_key} (${r.previous_commit.substring(0, 7)} → ${r.current_commit.substring(0, 7)})`);
      });
    } else {
      console.log('\n✓ No regressions detected');
    }
  } catch (error) {
    console.error('Error:', error);
    process.exit(1);
  }
}

main();
