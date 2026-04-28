import { describe, it, expect } from 'vitest';
import {
  captureSnapshot,
  SnapshotCaptureService,
  type CaptureContext,
} from './snapshot-capture.js';
import { DriftSnapshotSchema } from './snapshot-schema.js';
import type { ArchitectureBaseline } from '../architecture/index.js';
import type { ScanResult } from '../antipattern/index.js';
import type { FileSuppressions } from '../suppression/index.js';

function createMockBaseline(): ArchitectureBaseline {
  return {
    schema_version: '0.1.0',
    created_at: '2025-01-01T00:00:00.000Z',
    updated_at: '2025-01-01T00:00:00.000Z',
    entry_points: [],
    layers: {},
    boundaries: [],
    baseline_snapshot: {
      module_count: 10,
      timestamp: '2025-01-01T00:00:00.000Z',
      violations: [
        {
          id: 'violation-1',
          from_layer: 'presentation',
          to_layer: 'infrastructure',
          from_file: 'src/api/handler.ts',
          to_file: 'src/db/query.ts',
          import_line: 5,
          rule: 'no-presentation-to-infrastructure',
        },
      ],
    },
  };
}

function createMockScanResults(): ScanResult[] {
  return [
    {
      file: 'src/api/handler.ts',
      warnings: [
        {
          id: 'AP-003',
          category: 'anti-pattern',
          severity: 'warning',
          confidence: 'high',
          title: 'Explicit any type',
          message: 'Found explicit-any at line 10',
          explanation: 'Using any defeats type safety',
          suggestion: 'Use a proper type instead',
          location: { file: 'src/api/handler.ts', line: 10 },
          pattern: 'AP-003',
        },
        {
          id: 'AP-004',
          category: 'anti-pattern',
          severity: 'warning',
          confidence: 'high',
          title: 'TS-ignore directive',
          message: 'Found ts-ignore at line 15',
          explanation: 'Ignores type errors',
          suggestion: 'Fix the type error instead',
          location: { file: 'src/api/handler.ts', line: 15 },
          pattern: 'AP-004',
        },
      ],
      patternsChecked: ['AP-003', 'AP-004'],
    },
    {
      file: 'src/utils/helpers.ts',
      warnings: [
        {
          id: 'AP-003',
          category: 'anti-pattern',
          severity: 'warning',
          confidence: 'high',
          title: 'Explicit any type',
          message: 'Found explicit-any at line 20',
          explanation: 'Using any defeats type safety',
          suggestion: 'Use a proper type instead',
          location: { file: 'src/utils/helpers.ts', line: 20 },
          pattern: 'AP-003',
        },
      ],
      patternsChecked: ['AP-003', 'AP-004'],
    },
  ];
}

function createMockSuppressions(): FileSuppressions[] {
  const futureDate = new Date();
  futureDate.setFullYear(futureDate.getFullYear() + 1);

  return [
    {
      file: 'src/legacy/compat.ts',
      suppressions: [
        {
          warningId: 'AP-003',
          reason: 'Legacy compatibility layer',
          line: 5,
          scope: 'statement',
          expiresAt: futureDate,
        },
      ],
    },
  ];
}

describe('SnapshotCapture', () => {
  describe('captureSnapshot', () => {
    it('should capture snapshot with all components', async () => {
      const context: CaptureContext = {
        workspaceRoot: '/test/project',
        files: ['src/api/handler.ts', 'src/utils/helpers.ts'],
        baseline: createMockBaseline(),
        scanResults: createMockScanResults(),
        suppressions: createMockSuppressions(),
        gitRef: 'abc123def456',
      };

      const snapshot = await captureSnapshot(context, { name: 'test-snapshot' });

      expect(snapshot.name).toBe('test-snapshot');
      expect(snapshot.git_ref).toBe('abc123def456');
      expect(snapshot.schema_version).toBe('1.0.0');

      const result = DriftSnapshotSchema.safeParse(snapshot);
      expect(result.success).toBe(true);
    });

    it('should capture boundary violations from baseline', async () => {
      const context: CaptureContext = {
        workspaceRoot: '/test/project',
        files: [],
        baseline: createMockBaseline(),
        scanResults: [],
        suppressions: [],
      };

      const snapshot = await captureSnapshot(context);

      expect(snapshot.violations).toHaveLength(1);
      expect(snapshot.violations[0].id).toBe('violation-1');
      expect(snapshot.violations[0].from_layer).toBe('presentation');
      expect(snapshot.violations[0].to_layer).toBe('infrastructure');
      expect(snapshot.metrics.boundary_violations).toBe(1);
    });

    it('should capture anti-patterns from scan results', async () => {
      const context: CaptureContext = {
        workspaceRoot: '/test/project',
        files: ['src/api/handler.ts', 'src/utils/helpers.ts'],
        baseline: null,
        scanResults: createMockScanResults(),
        suppressions: [],
      };

      const snapshot = await captureSnapshot(context);

      expect(snapshot.antipatterns).toHaveLength(3);
      expect(snapshot.metrics.antipattern_count).toBe(3);
    });

    it('should calculate anti-pattern breakdown', async () => {
      const context: CaptureContext = {
        workspaceRoot: '/test/project',
        files: [],
        baseline: null,
        scanResults: createMockScanResults(),
        suppressions: [],
      };

      const snapshot = await captureSnapshot(context);

      expect(snapshot.antipattern_breakdown).toBeDefined();
      expect(snapshot.antipattern_breakdown?.['AP-003']).toBe(2);
      expect(snapshot.antipattern_breakdown?.['AP-004']).toBe(1);
    });

    it('should capture suppressions', async () => {
      const context: CaptureContext = {
        workspaceRoot: '/test/project',
        files: [],
        baseline: null,
        scanResults: [],
        suppressions: createMockSuppressions(),
      };

      const snapshot = await captureSnapshot(context);

      expect(snapshot.suppressions).toHaveLength(1);
      expect(snapshot.suppressions[0].pattern_id).toBe('AP-003');
      expect(snapshot.suppressions[0].reason).toBe('Legacy compatibility layer');
      expect(snapshot.metrics.suppression_count).toBe(1);
    });

    it('should identify expired suppressions', async () => {
      const expiredSuppression: FileSuppressions[] = [
        {
          file: 'src/old.ts',
          suppressions: [
            {
              warningId: 'AP-003',
              reason: 'Expired suppression',
              line: 1,
              scope: 'line',
              expiresAt: new Date('2020-01-01T00:00:00.000Z'),
            },
          ],
        },
      ];

      const context: CaptureContext = {
        workspaceRoot: '/test/project',
        files: [],
        baseline: null,
        scanResults: [],
        suppressions: expiredSuppression,
      };

      const snapshot = await captureSnapshot(context);

      expect(snapshot.suppressions[0].is_expired).toBe(true);
      expect(snapshot.metrics.expired_suppressions).toBe(1);
      expect(snapshot.metrics.suppression_count).toBe(0);
    });

    it('should calculate hotspots', async () => {
      const context: CaptureContext = {
        workspaceRoot: '/test/project',
        files: [],
        baseline: null,
        scanResults: createMockScanResults(),
        suppressions: [],
      };

      const snapshot = await captureSnapshot(context);

      expect(snapshot.hotspots).toBeDefined();
      expect(snapshot.hotspots?.length).toBeGreaterThan(0);

      const apiHotspot = snapshot.hotspots?.find((h) => h.path === 'src/api');
      expect(apiHotspot).toBeDefined();
      expect(apiHotspot?.violation_count).toBe(2);
    });

    it('should not include suppressed warnings in antipatterns', async () => {
      const scanWithSuppressed: ScanResult[] = [
        {
          file: 'src/test.ts',
          warnings: [
            {
              id: 'AP-003',
              category: 'anti-pattern',
              severity: 'warning',
              confidence: 'high',
              title: 'Test',
              message: 'Test',
              explanation: 'Test',
              suggestion: 'Test',
              location: { file: 'src/test.ts', line: 1 },
              pattern: 'AP-003',
              suppressed: { reason: 'Intentional', scope: 'statement' },
            },
          ],
          patternsChecked: ['AP-003'],
        },
      ];

      const context: CaptureContext = {
        workspaceRoot: '/test/project',
        files: [],
        baseline: null,
        scanResults: scanWithSuppressed,
        suppressions: [],
      };

      const snapshot = await captureSnapshot(context);

      expect(snapshot.antipatterns).toHaveLength(0);
      expect(snapshot.metrics.antipattern_count).toBe(0);
    });

    it('should generate baseline hash when baseline exists', async () => {
      const context: CaptureContext = {
        workspaceRoot: '/test/project',
        files: [],
        baseline: createMockBaseline(),
        scanResults: [],
        suppressions: [],
      };

      const snapshot = await captureSnapshot(context);

      expect(snapshot.baseline_hash).toBeDefined();
      expect(snapshot.baseline_hash?.length).toBeGreaterThan(0);
    });

    it('should handle empty context', async () => {
      const context: CaptureContext = {
        workspaceRoot: '/test/project',
        files: [],
        baseline: null,
        scanResults: [],
        suppressions: [],
      };

      const snapshot = await captureSnapshot(context);

      expect(snapshot.violations).toEqual([]);
      expect(snapshot.antipatterns).toEqual([]);
      expect(snapshot.suppressions).toEqual([]);
      expect(snapshot.metrics.boundary_violations).toBe(0);
      expect(snapshot.metrics.antipattern_count).toBe(0);
      expect(snapshot.metrics.suppression_count).toBe(0);

      const result = DriftSnapshotSchema.safeParse(snapshot);
      expect(result.success).toBe(true);
    });
  });

  describe('SnapshotCaptureService', () => {
    it('should create service with workspace root', () => {
      const service = new SnapshotCaptureService('/test/project');
      expect(service).toBeDefined();
      expect(typeof service.capture).toBe('function');
      expect(typeof service.captureWithContext).toBe('function');
    });

    it('should capture with context', async () => {
      const service = new SnapshotCaptureService('/test/project');

      const snapshot = await service.captureWithContext({
        baseline: createMockBaseline(),
        scanResults: createMockScanResults(),
      });

      expect(snapshot.violations).toHaveLength(1);
      expect(snapshot.antipatterns).toHaveLength(3);
    });
  });
});
