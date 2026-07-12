import type { components } from '@/api/generated/openapi';

export const protectionOverviewFixture = {
  schema_version: 'anvil.dashboard.protection.v1',
  data_state: 'partial',
  source_message: 'Deterministic development protection evidence',
  claim: null,
  assurance: null,
  save_time: { state: 'attached', active: true, failure_count: 0 },
  observed_at_unix: 1_748_419_930,
  latest_run: {
    id: 'run-143207',
    result: 'issues',
    label: 'Completed with issues',
    score: 72,
    warning_count: 12,
    duration_seconds: 18.4,
  },
  next_attention: {
    title: 'Hard-coded API key detected',
    detail: 'src/services/payment/gateway.ts:27',
    evidence_id: 'anvil://evidence/8f2e3c7d-7b2a-4a1d-9c9a-2f3b6e9a1d55',
  },
  warnings_state: 'partial',
  warnings: [],
  affected_files_state: 'partial',
  affected_files: [],
  gaps: [],
} satisfies components['schemas']['ProtectionOverview'];
