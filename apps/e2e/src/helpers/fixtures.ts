/**
 * Shared Fixtures and Test Data Factories
 *
 * Provides deterministic test data for use across all E2E suites.
 * Every factory produces valid data by default and accepts overrides
 * for testing edge cases.
 */

import {
  type APSPlan,
  type Change,
  type GateConfig,
  type GateCheck,
  APS_SCHEMA_VERSION,
  createPlan,
  generateHash,
} from '@eddacraft/anvil-core';

// ─── Plan Factories ─────────────────────────────────────────────

/** Counter for deterministic IDs within a test run */
let planCounter = 0;

/**
 * Reset the plan counter between test suites
 */
export function resetFixtures(): void {
  planCounter = 0;
}

/**
 * Create a minimal valid APS plan via createPlan() from @eddacraft/anvil-core.
 * Hash is computed from the final plan (after overrides) via generateHash().
 */
export function makePlan(overrides: Partial<APSPlan> = {}): APSPlan {
  planCounter++;
  const hexCounter = planCounter.toString(16).padStart(8, '0');
  const plan = createPlan({
    id: overrides.id ?? `aps-${hexCounter}`,
    intent: overrides.intent ?? `E2E test plan #${planCounter}`,
    provenance: overrides.provenance ?? {
      timestamp: new Date().toISOString(),
      author: 'e2e-harness',
      source: 'cli' as const,
      version: '0.1.0',
    },
    changes: overrides.proposed_changes ?? [makeChange()],
    validations: overrides.validations ?? {
      required_checks: ['lint', 'test', 'coverage', 'secrets'],
      skip_checks: [],
    },
  });
  const { hash: _overrideHash, ...nonHashOverrides } = overrides;
  const merged = { ...plan, ...nonHashOverrides };
  const hash = overrides.hash ?? generateHash(merged);
  return { ...merged, hash } as APSPlan;
}

/**
 * Create a single proposed change entry.
 */
export function makeChange(overrides: Partial<Change> & { file?: string } = {}): Change {
  const { file, ...rest } = overrides;
  return {
    path: file ?? rest.path ?? 'src/example.ts',
    type: rest.type ?? 'file_update',
    description: rest.description ?? 'Update implementation',
    ...rest,
  } as Change;
}

// ─── Gate Config Factories ──────────────────────────────────────

/**
 * Create a gate configuration with sensible defaults.
 */
export function makeGateConfig(overrides: Partial<GateConfig> = {}): GateConfig {
  return {
    version: 1,
    checks: overrides.checks ?? [
      makeGateCheck({ name: 'lint', enabled: true }),
      makeGateCheck({ name: 'test', enabled: true }),
      makeGateCheck({ name: 'coverage', enabled: true }),
      makeGateCheck({ name: 'secrets', enabled: true }),
    ],
    thresholds: overrides.thresholds ?? { overall_score: 80 },
    ...overrides,
  } as GateConfig;
}

/**
 * Create an individual gate check entry.
 */
export function makeGateCheck(overrides: Partial<GateCheck> = {}): GateCheck {
  return {
    name: overrides.name ?? 'check',
    description: overrides.description ?? 'Test gate check',
    enabled: true,
    ...overrides,
  };
}

// ─── File Content Factories ─────────────────────────────────────

/**
 * Create a minimal TypeScript source file.
 */
export function makeTsSource(name = 'example'): string {
  return `// ${name}.ts\nexport function ${name}(): string {\n  return '${name}';\n}\n`;
}

/**
 * Create a SpecKit-format markdown document.
 *
 * Matches the SpecKit adapter's detection indicators (Specification header,
 * Intent, Overview, Goals, Changes sections). See
 * `packages/adapters/src/speckit/format-adapter.ts`.
 */
export function makeSpecKitDoc(title = 'Test Spec'): string {
  return [
    '# Specification',
    '',
    `> ${title}`,
    '',
    '## Intent',
    '',
    `Document the ${title} requirement in SpecKit format.`,
    '',
    '## Overview',
    '',
    'A test specification document produced by the E2E harness.',
    '',
    '## Goals',
    '',
    '- Verify SpecKit adapter detection',
    '- Exercise the format roundtrip pipeline',
    '',
    '## Changes',
    '',
    '- Modify `src/example.ts` — update implementation',
    '',
    '## Metadata',
    '',
    '| Key     | Value       |',
    '| ------- | ----------- |',
    '| Author  | e2e-harness |',
    '| Version | 0.1.0       |',
    '',
  ].join('\n');
}

/**
 * Create an APS-format markdown planning document.
 *
 * Produces an APS leaf spec matching the detection indicators in
 * `packages/adapters/src/aps-markdown/adapter.ts` — H1 title, ID field,
 * Tasks section with SCOPE-NNN headings, Intent/Confidence/Owner/Priority
 * fields. See also: existing specs under `plans/modules/*.aps.md`.
 */
export function makeAPSMarkdown(intent = 'E2E test plan'): string {
  return [
    '---',
    `schema_version: "${APS_SCHEMA_VERSION}"`,
    `intent: "${intent}"`,
    '---',
    '',
    `# ${intent}`,
    '',
    '| ID    | Owner       | Status |',
    '| ----- | ----------- | ------ |',
    '| E2E-1 | e2e-harness | Draft  |',
    '',
    '## Purpose',
    '',
    `Fixture plan for the ${intent} suite.`,
    '',
    '## Tasks',
    '',
    '### E2E-001: initial change',
    '',
    '- **Intent:** update implementation of `src/example.ts`',
    '- **Owner:** e2e-harness',
    '- **Priority:** medium',
    '- **Confidence:** high',
    '- **Files:** `src/example.ts`',
    '',
    '### E2E-002: follow-up change',
    '',
    '- **Intent:** verify the update compiles',
    '- **Owner:** e2e-harness',
    '- **Priority:** low',
    '- **Confidence:** high',
    '',
  ].join('\n');
}

/**
 * Create source content that deliberately triggers multiple anti-patterns
 * from `packages/anvil/core/src/antipattern/patterns.ts`:
 *
 * - AP-004 `@ts-ignore`
 * - AP-003 explicit `any`
 * - AP-006 empty catch block
 * - AP-007 `console.*` in production-shaped code (only when
 *   `includeOptIn: true` is enabled)
 *
 * Intentionally contains no secret-shaped strings — the antipattern catalogue
 * has no secret-detection rule (see `patterns.ts`), so a fixture that claimed
 * to exercise "secret detection" would be lying about coverage.
 */
export function makeSourceWithAntipatterns(): string {
  return [
    '// config.ts',
    '// @ts-ignore',
    'const DB_HOST: any = process.env.DB_HOST;',
    '',
    'export function load(): void {',
    '  try {',
    '    console.log("loading", DB_HOST);',
    '  } catch (e) {',
    '  }',
    '}',
    '',
  ].join('\n');
}

/**
 * Create source content with a known architecture violation
 * (importing from a layer it shouldn't).
 */
export function makeSourceWithBoundaryViolation(): string {
  return [
    '// domain/service.ts',
    "import { dbQuery } from '../infrastructure/database';",
    '',
    'export function getUser(id: string) {',
    '  return dbQuery(`SELECT * FROM users WHERE id = ${id}`);',
    '}',
    '',
  ].join('\n');
}
