// @vitest-environment node
import { describe, it, expect, vi, afterEach } from 'vitest';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { InMemoryTransport } from '@modelcontextprotocol/sdk/inMemory.js';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { registerBaselineResource } from './baseline.resource.js';
import { registerBoundariesResource } from './boundaries.resource.js';
import { registerPatternsResource } from './patterns.resource.js';
import { registerSuppressionsResource } from './suppressions.resource.js';
import { registerConfigResource } from './config.resource.js';
import { registerConstraintsResource } from './constraints.resource.js';
import { registerDriftResource } from './drift.resource.js';
import { registerFileWarningsResource } from './file-warnings.resource.js';

// ---------------------------------------------------------------------------
// Mocks
// ---------------------------------------------------------------------------

const mockBaselineExists = vi.fn();
const mockLoadBaseline = vi.fn();
const mockStoreLoad = vi.fn().mockResolvedValue(undefined);
const mockStoreGetAll = vi.fn().mockReturnValue([]);
const mockStoreGetExpired = vi.fn().mockReturnValue([]);

vi.mock('@eddacraft/anvil-core', () => {
  return {
    baselineExists: (...args: unknown[]) => mockBaselineExists(...args),
    loadBaseline: (...args: unknown[]) => mockLoadBaseline(...args),
    PATTERNS: [
      {
        id: 'AP-001',
        name: 'Broad eslint-disable',
        category: 'escape-hatch',
        severity: 'warning',
        confidence: 'high',
        title: 'Broad eslint-disable added',
        explanation: 'Disabling all ESLint rules hides legitimate issues.',
        suggestion: 'Disable specific rules instead.',
        enabled: true,
        optIn: false,
        allowlist: undefined,
      },
      {
        id: 'AP-003',
        name: 'Explicit any type',
        category: 'type-safety',
        severity: 'warning',
        confidence: 'high',
        title: 'Explicit any type usage',
        explanation: 'Using any defeats TypeScript type checking.',
        suggestion: 'Use unknown or define a proper type.',
        enabled: true,
        optIn: false,
        allowlist: ['*.d.ts'],
      },
    ],
    SuppressionStore: class MockSuppressionStore {
      load = mockStoreLoad;
      getAll = mockStoreGetAll;
      getExpired = mockStoreGetExpired;
    },
    getLatestSnapshot: vi.fn().mockResolvedValue(null),
    listSnapshots: vi.fn().mockResolvedValue([]),
    loadSnapshot: vi.fn().mockResolvedValue(null),
    compareSnapshots: vi.fn().mockReturnValue({
      before: { created_at: '2025-01-01T00:00:00Z' },
      after: { created_at: '2025-01-02T00:00:00Z' },
      duration_days: 1,
      metrics: {},
      net_change: { violations: 0, antipatterns: 0, suppressions: 0 },
      overall_trend: 'stable',
    }),
  };
});

const mockLoadConfigWithDetails = vi.fn();
const mockAnalyzeFiles = vi.fn();

vi.mock('@eddacraft/anvil-runtime', () => ({
  GateConfigManager: class MockGateConfigManager {
    loadConfigWithDetails = mockLoadConfigWithDetails;
  },
  GateRunner: class MockGateRunner {
    analyzeFiles = mockAnalyzeFiles;
  },
  collectConstraints: vi.fn().mockResolvedValue({
    boundaries: [],
    layers: [],
    antiPatterns: [
      {
        id: 'AP-001',
        name: 'Broad eslint-disable',
        category: 'escape-hatch',
        explanation: 'Disabling all ESLint rules hides issues.',
        suggestion: 'Disable specific rules.',
        severity: 'warning',
        enabled: true,
      },
    ],
    conventions: [
      {
        category: 'spelling',
        description: 'Use UK English spelling',
      },
    ],
    metadata: {
      collectedAt: '2025-01-01T00:00:00Z',
      workspaceRoot: '/tmp/test',
      hasBaseline: false,
    },
  }),
}));

vi.mock('node:path', async () => {
  const actual = await vi.importActual<typeof import('node:path')>('node:path');
  return { ...actual, default: actual };
});

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const cleanupFns: Array<() => Promise<void>> = [];
const WORKSPACE_ROOT = '/tmp/test-project';

function getWorkspaceRoot(): string {
  return WORKSPACE_ROOT;
}

async function createServerWithResource(
  registerFn: (server: McpServer, getRoot: () => string) => void
) {
  const server = new McpServer({ name: 'test-resources', version: '0.0.1' });
  registerFn(server, getWorkspaceRoot);

  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
  await server.connect(serverTransport);

  const client = new Client({ name: 'test-client', version: '1.0.0' });
  await client.connect(clientTransport);

  cleanupFns.push(async () => {
    await client.close();
    await server.close();
  });

  return { server, client };
}

async function createServerWithStaticResource(registerFn: (server: McpServer) => void) {
  const server = new McpServer({ name: 'test-resources', version: '0.0.1' });
  registerFn(server);

  const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
  await server.connect(serverTransport);

  const client = new Client({ name: 'test-client', version: '1.0.0' });
  await client.connect(clientTransport);

  cleanupFns.push(async () => {
    await client.close();
    await server.close();
  });

  return { server, client };
}

function parseResourceText(result: { contents: Array<{ text?: string }> }): unknown {
  const text = result.contents[0]?.text;
  if (!text) throw new Error('No text content in resource response');
  return JSON.parse(text);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('MCP Resources', () => {
  afterEach(async () => {
    for (const fn of cleanupFns) {
      await fn();
    }
    cleanupFns.length = 0;
    vi.restoreAllMocks();
  });

  // =========================================================================
  // anvil://baseline
  // =========================================================================
  describe('anvil://baseline', () => {
    it('registers the baseline resource', async () => {
      const { client } = await createServerWithResource(registerBaselineResource);
      const { resources } = await client.listResources();

      const baseline = resources.find((r) => r.uri === 'anvil://baseline');
      expect(baseline).toBeDefined();
      expect(baseline!.name).toBe('baseline');
      expect(baseline!.mimeType).toBe('application/json');
    });

    it('returns baseline JSON when baseline exists', async () => {
      const mockBaseline = {
        schema_version: '0.1.0',
        created_at: '2025-01-01T00:00:00Z',
        updated_at: '2025-01-01T00:00:00Z',
        entry_points: [],
        layers: {
          domain: { patterns: ['src/domain/**'], depends_on: [], description: 'Domain layer' },
        },
        boundaries: [],
        baseline_snapshot: { module_count: 10, timestamp: '2025-01-01T00:00:00Z', violations: [] },
      };

      mockBaselineExists.mockReturnValue(true);
      mockLoadBaseline.mockReturnValue(mockBaseline);

      const { client } = await createServerWithResource(registerBaselineResource);
      const result = await client.readResource({ uri: 'anvil://baseline' });

      const parsed = parseResourceText(result) as Record<string, unknown>;
      expect(parsed.schema_version).toBe('0.1.0');
      expect(parsed.layers).toBeDefined();
      expect(parsed.baseline_snapshot).toBeDefined();
    });

    it('returns error message when no baseline exists', async () => {
      mockBaselineExists.mockReturnValue(false);

      const { client } = await createServerWithResource(registerBaselineResource);
      const result = await client.readResource({ uri: 'anvil://baseline' });

      const parsed = parseResourceText(result) as Record<string, unknown>;
      expect(parsed.error).toBe('no-baseline');
      expect(parsed.message).toContain('anvil init');
    });

    it('returns error when baseline exists but cannot be loaded', async () => {
      mockBaselineExists.mockReturnValue(true);
      mockLoadBaseline.mockReturnValue(null);

      const { client } = await createServerWithResource(registerBaselineResource);
      const result = await client.readResource({ uri: 'anvil://baseline' });

      const parsed = parseResourceText(result) as Record<string, unknown>;
      expect(parsed.error).toBe('baseline-load-failed');
    });
  });

  // =========================================================================
  // anvil://boundaries
  // =========================================================================
  describe('anvil://boundaries', () => {
    it('registers the boundaries resource', async () => {
      const { client } = await createServerWithResource(registerBoundariesResource);
      const { resources } = await client.listResources();

      const boundaries = resources.find((r) => r.uri === 'anvil://boundaries');
      expect(boundaries).toBeDefined();
      expect(boundaries!.name).toBe('boundaries');
    });

    it('returns layers and boundaries when baseline exists', async () => {
      mockBaselineExists.mockReturnValue(true);
      mockLoadBaseline.mockReturnValue({
        layers: {
          presentation: {
            patterns: ['src/ui/**'],
            depends_on: ['application', 'shared'],
            description: 'UI layer',
          },
          application: {
            patterns: ['src/app/**'],
            depends_on: ['domain', 'shared'],
          },
        },
        boundaries: [
          {
            name: 'no-ui-in-domain',
            from: 'domain',
            to: 'presentation',
            severity: 'error',
            message: 'Domain must not depend on presentation.',
          },
        ],
      });

      const { client } = await createServerWithResource(registerBoundariesResource);
      const result = await client.readResource({ uri: 'anvil://boundaries' });

      const parsed = parseResourceText(result) as {
        layers: Array<{ name: string; depends_on: string[] }>;
        boundaries: Array<{ name: string; from: string; to: string }>;
      };

      expect(parsed.layers).toHaveLength(2);
      expect(parsed.layers[0].name).toBe('presentation');
      expect(parsed.layers[0].depends_on).toContain('application');
      expect(parsed.boundaries).toHaveLength(1);
      expect(parsed.boundaries[0].from).toBe('domain');
    });

    it('returns error when no baseline exists', async () => {
      mockBaselineExists.mockReturnValue(false);

      const { client } = await createServerWithResource(registerBoundariesResource);
      const result = await client.readResource({ uri: 'anvil://boundaries' });

      const parsed = parseResourceText(result) as Record<string, unknown>;
      expect(parsed.error).toBe('no-baseline');
    });
  });

  // =========================================================================
  // anvil://patterns
  // =========================================================================
  describe('anvil://patterns', () => {
    it('registers the patterns resource', async () => {
      const { client } = await createServerWithStaticResource(registerPatternsResource);
      const { resources } = await client.listResources();

      const patterns = resources.find((r) => r.uri === 'anvil://patterns');
      expect(patterns).toBeDefined();
      expect(patterns!.name).toBe('patterns');
    });

    it('returns the anti-pattern catalogue', async () => {
      const { client } = await createServerWithStaticResource(registerPatternsResource);
      const result = await client.readResource({ uri: 'anvil://patterns' });

      const parsed = parseResourceText(result) as {
        patterns: Array<{ id: string; name: string; category: string }>;
        count: number;
      };

      expect(parsed.count).toBe(2);
      expect(parsed.patterns).toHaveLength(2);
      expect(parsed.patterns[0].id).toBe('AP-001');
      expect(parsed.patterns[0].name).toBe('Broad eslint-disable');
      expect(parsed.patterns[1].id).toBe('AP-003');
    });

    it('includes explanation and suggestion for each pattern', async () => {
      const { client } = await createServerWithStaticResource(registerPatternsResource);
      const result = await client.readResource({ uri: 'anvil://patterns' });

      const parsed = parseResourceText(result) as {
        patterns: Array<{ explanation: string; suggestion: string }>;
      };

      expect(parsed.patterns[0].explanation).toContain('ESLint');
      expect(parsed.patterns[0].suggestion).toContain('specific rules');
    });
  });

  // =========================================================================
  // anvil://suppressions
  // =========================================================================
  describe('anvil://suppressions', () => {
    it('registers the suppressions resource', async () => {
      const { client } = await createServerWithResource(registerSuppressionsResource);
      const { resources } = await client.listResources();

      const suppressions = resources.find((r) => r.uri === 'anvil://suppressions');
      expect(suppressions).toBeDefined();
      expect(suppressions!.name).toBe('suppressions');
    });

    it('returns empty suppressions when store is empty', async () => {
      const { client } = await createServerWithResource(registerSuppressionsResource);
      const result = await client.readResource({ uri: 'anvil://suppressions' });

      const parsed = parseResourceText(result) as {
        suppressions: unknown[];
        summary: { total: number; active: number; expired: number };
      };

      expect(parsed.suppressions).toEqual([]);
      expect(parsed.summary.total).toBe(0);
      expect(parsed.summary.active).toBe(0);
      expect(parsed.summary.expired).toBe(0);
    });

    it('returns suppressions with expiry information', async () => {
      mockStoreGetAll.mockReturnValue([
        {
          id: 'src/app.ts:10:AP-003',
          pattern_id: 'AP-003',
          file: 'src/app.ts',
          line: 10,
          reason: 'Legacy code',
          scope: 'line',
          expires_at: '2025-06-01T00:00:00Z',
        },
      ]);
      mockStoreGetExpired.mockReturnValue([]);

      const { client } = await createServerWithResource(registerSuppressionsResource);
      const result = await client.readResource({ uri: 'anvil://suppressions' });

      const parsed = parseResourceText(result) as {
        suppressions: Array<{ id: string; pattern_id: string; isExpired: boolean }>;
        summary: { total: number; active: number; expired: number };
      };

      expect(parsed.suppressions).toHaveLength(1);
      expect(parsed.suppressions[0].pattern_id).toBe('AP-003');
      expect(parsed.suppressions[0].isExpired).toBe(false);
      expect(parsed.summary.total).toBe(1);
      expect(parsed.summary.active).toBe(1);
    });
  });

  // =========================================================================
  // anvil://config
  // =========================================================================
  describe('anvil://config', () => {
    it('registers the config resource', async () => {
      const { client } = await createServerWithResource(registerConfigResource);
      const { resources } = await client.listResources();

      const config = resources.find((r) => r.uri === 'anvil://config');
      expect(config).toBeDefined();
      expect(config!.name).toBe('config');
    });

    it('returns gate configuration with details', async () => {
      mockLoadConfigWithDetails.mockReturnValue({
        config: {
          version: 1,
          checks: [
            { name: 'eslint', enabled: true, config: {} },
            { name: 'coverage', enabled: false, config: {} },
          ],
          thresholds: { overall_score: 80 },
        },
        path: '/tmp/test-project/.anvilrc',
        isDefault: false,
        errors: [],
      });

      const { client } = await createServerWithResource(registerConfigResource);
      const result = await client.readResource({ uri: 'anvil://config' });

      const parsed = parseResourceText(result) as {
        config: { checks: Array<{ name: string; enabled: boolean }> };
        source: string;
        isDefault: boolean;
        errors: string[];
      };

      expect(parsed.config.checks).toHaveLength(2);
      expect(parsed.source).toBe('/tmp/test-project/.anvilrc');
      expect(parsed.isDefault).toBe(false);
      expect(parsed.errors).toEqual([]);
    });

    it('returns default config when no config file found', async () => {
      mockLoadConfigWithDetails.mockReturnValue({
        config: {
          version: 1,
          checks: [],
          thresholds: { overall_score: 80 },
        },
        path: null,
        isDefault: true,
        errors: [],
      });

      const { client } = await createServerWithResource(registerConfigResource);
      const result = await client.readResource({ uri: 'anvil://config' });

      const parsed = parseResourceText(result) as {
        source: string | null;
        isDefault: boolean;
      };

      expect(parsed.isDefault).toBe(true);
      expect(parsed.source).toBeNull();
    });

    it('returns error when config loading throws', async () => {
      mockLoadConfigWithDetails.mockImplementation(() => {
        throw new Error('Permission denied');
      });

      const { client } = await createServerWithResource(registerConfigResource);
      const result = await client.readResource({ uri: 'anvil://config' });

      const parsed = parseResourceText(result) as { error: string };
      expect(parsed.error).toBe('Permission denied');
    });
  });

  // =========================================================================
  // anvil://constraints
  // =========================================================================
  describe('anvil://constraints', () => {
    it('registers the constraints resource', async () => {
      const { client } = await createServerWithResource(registerConstraintsResource);
      const { resources } = await client.listResources();

      const constraints = resources.find((r) => r.uri === 'anvil://constraints');
      expect(constraints).toBeDefined();
      expect(constraints!.name).toBe('constraints');
    });

    it('returns aggregated constraints', async () => {
      const { client } = await createServerWithResource(registerConstraintsResource);
      const result = await client.readResource({ uri: 'anvil://constraints' });

      const parsed = parseResourceText(result) as {
        antiPatterns: Array<{ id: string }>;
        conventions: Array<{ category: string }>;
        metadata: { workspaceRoot: string };
      };

      expect(parsed.antiPatterns).toHaveLength(1);
      expect(parsed.antiPatterns[0].id).toBe('AP-001');
      expect(parsed.conventions).toHaveLength(1);
      expect(parsed.conventions[0].category).toBe('spelling');
    });
  });

  // =========================================================================
  // anvil://drift
  // =========================================================================
  describe('anvil://drift', () => {
    it('registers the drift resource', async () => {
      const { client } = await createServerWithResource(registerDriftResource);
      const { resources } = await client.listResources();

      const drift = resources.find((r) => r.uri === 'anvil://drift');
      expect(drift).toBeDefined();
      expect(drift!.name).toBe('drift');
    });

    it('returns no-snapshots status when no snapshots exist', async () => {
      const { client } = await createServerWithResource(registerDriftResource);
      const result = await client.readResource({ uri: 'anvil://drift' });

      const parsed = parseResourceText(result) as { status: string; snapshotCount: number };
      expect(parsed.status).toBe('no-snapshots');
      expect(parsed.snapshotCount).toBe(0);
    });

    it('returns single-snapshot status with metrics when one snapshot exists', async () => {
      const core = await import('@eddacraft/anvil-core');
      const snapshot = {
        name: 'initial',
        created_at: '2025-01-01T00:00:00Z',
        metrics: {
          boundary_violations: 2,
          antipattern_count: 5,
          suppression_count: 1,
          expired_suppressions: 0,
          files_analysed: 10,
        },
        hotspots: [{ path: 'src/core', violation_count: 3, types: ['boundary'] }],
      };

      (core.listSnapshots as ReturnType<typeof vi.fn>).mockResolvedValue([
        { filename: 'snapshot-initial.json', name: 'initial', created_at: '2025-01-01T00:00:00Z' },
      ]);
      (core.getLatestSnapshot as ReturnType<typeof vi.fn>).mockResolvedValue(snapshot);

      const { client } = await createServerWithResource(registerDriftResource);
      const result = await client.readResource({ uri: 'anvil://drift' });

      const parsed = parseResourceText(result) as {
        status: string;
        snapshotCount: number;
        latest: { metrics: { boundary_violations: number } };
      };

      expect(parsed.status).toBe('single-snapshot');
      expect(parsed.snapshotCount).toBe(1);
      expect(parsed.latest.metrics.boundary_violations).toBe(2);
    });

    it('returns comparison when two snapshots exist', async () => {
      const core = await import('@eddacraft/anvil-core');

      (core.listSnapshots as ReturnType<typeof vi.fn>).mockResolvedValue([
        { filename: 'snapshot-second.json', name: 'second', created_at: '2025-01-02T00:00:00Z' },
        { filename: 'snapshot-first.json', name: 'first', created_at: '2025-01-01T00:00:00Z' },
      ]);
      (core.getLatestSnapshot as ReturnType<typeof vi.fn>).mockResolvedValue({
        name: 'second',
        created_at: '2025-01-02T00:00:00Z',
        metrics: {
          boundary_violations: 1,
          antipattern_count: 3,
          suppression_count: 1,
          expired_suppressions: 0,
          files_analysed: 10,
        },
        violations: [],
        antipatterns: [],
        suppressions: [],
      });
      (core.loadSnapshot as ReturnType<typeof vi.fn>).mockResolvedValue({
        name: 'first',
        created_at: '2025-01-01T00:00:00Z',
        metrics: {
          boundary_violations: 2,
          antipattern_count: 5,
          suppression_count: 1,
          expired_suppressions: 0,
          files_analysed: 10,
        },
        violations: [],
        antipatterns: [],
        suppressions: [],
      });

      const { client } = await createServerWithResource(registerDriftResource);
      const result = await client.readResource({ uri: 'anvil://drift' });

      const parsed = parseResourceText(result) as {
        status: string;
        snapshotCount: number;
        comparison: { overall_trend: string };
      };

      expect(parsed.status).toBe('ok');
      expect(parsed.snapshotCount).toBe(2);
      expect(parsed.comparison).toBeDefined();
      expect(parsed.comparison.overall_trend).toBe('stable');
    });
  });

  // =========================================================================
  // anvil://file/{path}/warnings
  // =========================================================================
  describe('anvil://file/{path}/warnings', () => {
    it('registers the file-warnings resource template', async () => {
      const { client } = await createServerWithResource(registerFileWarningsResource);
      const { resourceTemplates } = await client.listResourceTemplates();

      const fileWarnings = resourceTemplates.find(
        (t) => t.uriTemplate === 'anvil://file/{path}/warnings'
      );
      expect(fileWarnings).toBeDefined();
      expect(fileWarnings!.name).toBe('file-warnings');
    });

    it('returns warnings for a file', async () => {
      mockAnalyzeFiles.mockResolvedValue({
        warnings: {
          warnings: [
            {
              id: 'AP-001',
              severity: 'warning',
              title: 'Broad eslint-disable',
              message: 'All rules disabled',
              suggestion: 'Disable specific rules',
              location: { file: 'src/app.ts', line: 5 },
              category: 'escape-hatch',
            },
          ],
          summary: { total: 1, errors: 0, warnings: 1, info: 0, suppressed: 0 },
        },
        checksRun: ['antipattern'],
        hasBlockingWarnings: false,
      });

      const { client } = await createServerWithResource(registerFileWarningsResource);
      const result = await client.readResource({
        uri: 'anvil://file/src%2Fapp.ts/warnings',
      });

      const parsed = parseResourceText(result) as {
        file: string;
        warnings: Array<{ id: string; severity: string }>;
        summary: { total: number };
        hasBlockingWarnings: boolean;
      };

      expect(parsed.file).toBe('src/app.ts');
      expect(parsed.warnings).toHaveLength(1);
      expect(parsed.warnings[0].id).toBe('AP-001');
      expect(parsed.summary.total).toBe(1);
      expect(parsed.hasBlockingWarnings).toBe(false);
    });

    it('returns empty warnings for a clean file', async () => {
      mockAnalyzeFiles.mockResolvedValue({
        warnings: {
          warnings: [],
          summary: { total: 0, errors: 0, warnings: 0, info: 0, suppressed: 0 },
        },
        checksRun: ['architecture', 'antipattern'],
        hasBlockingWarnings: false,
      });

      const { client } = await createServerWithResource(registerFileWarningsResource);
      const result = await client.readResource({
        uri: 'anvil://file/src%2Fclean.ts/warnings',
      });

      const parsed = parseResourceText(result) as {
        warnings: unknown[];
        hasBlockingWarnings: boolean;
      };

      expect(parsed.warnings).toEqual([]);
      expect(parsed.hasBlockingWarnings).toBe(false);
    });

    it('returns error when analysis throws', async () => {
      mockAnalyzeFiles.mockRejectedValue(new Error('File not found'));

      const { client } = await createServerWithResource(registerFileWarningsResource);
      const result = await client.readResource({
        uri: 'anvil://file/nonexistent.ts/warnings',
      });

      const parsed = parseResourceText(result) as { error: string };
      expect(parsed.error).toBe('File not found');
    });

    it('rejects path traversal with ../ segments', async () => {
      const { client } = await createServerWithResource(registerFileWarningsResource);
      const result = await client.readResource({
        uri: 'anvil://file/..%2F..%2F..%2Fetc%2Fpasswd/warnings',
      });

      const parsed = parseResourceText(result) as { error: string };
      expect(parsed.error).toContain('outside workspace root');
    });
  });

  // =========================================================================
  // All resources registered together (integration)
  // =========================================================================
  describe('all resources registered together', () => {
    it('lists all 7 static resources and 1 template', async () => {
      mockLoadConfigWithDetails.mockReturnValue({
        config: { version: 1, checks: [], thresholds: { overall_score: 80 } },
        path: null,
        isDefault: true,
        errors: [],
      });

      const server = new McpServer({ name: 'test-all-resources', version: '0.0.1' });

      registerBaselineResource(server, getWorkspaceRoot);
      registerBoundariesResource(server, getWorkspaceRoot);
      registerPatternsResource(server);
      registerSuppressionsResource(server, getWorkspaceRoot);
      registerConfigResource(server, getWorkspaceRoot);
      registerConstraintsResource(server, getWorkspaceRoot);
      registerDriftResource(server, getWorkspaceRoot);
      registerFileWarningsResource(server, getWorkspaceRoot);

      const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
      await server.connect(serverTransport);

      const client = new Client({ name: 'test-client', version: '1.0.0' });
      await client.connect(clientTransport);

      cleanupFns.push(async () => {
        await client.close();
        await server.close();
      });

      const { resources } = await client.listResources();
      const { resourceTemplates } = await client.listResourceTemplates();

      const staticUris = resources.map((r) => r.uri).sort();
      expect(staticUris).toEqual([
        'anvil://baseline',
        'anvil://boundaries',
        'anvil://config',
        'anvil://constraints',
        'anvil://drift',
        'anvil://patterns',
        'anvil://suppressions',
      ]);

      expect(resourceTemplates).toHaveLength(1);
      expect(resourceTemplates[0].uriTemplate).toBe('anvil://file/{path}/warnings');
    });
  });
});
