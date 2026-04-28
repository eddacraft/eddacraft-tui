import { describe, it, expect } from 'vitest';
import {
  DriftSnapshotSchema,
  SnapshotViolationSchema,
  SnapshotAntiPatternSchema,
  SnapshotSuppressionSchema,
  SnapshotMetricsSchema,
  SNAPSHOT_SCHEMA_VERSION,
  generateSnapshotFilename,
  generateNamedSnapshotFilename,
  parseSnapshotFilename,
  createEmptySnapshot,
  validateSnapshot,
  type DriftSnapshot,
} from './snapshot-schema.js';

describe('DriftSnapshot Schema', () => {
  describe('SnapshotViolationSchema', () => {
    it('should validate a valid violation', () => {
      const violation = {
        id: 'src_api_handler-ts_src_db_query-ts_42',
        type: 'boundary',
        from_file: 'src/api/handler.ts',
        to_file: 'src/db/query.ts',
        from_layer: 'presentation',
        to_layer: 'infrastructure',
        line: 42,
        rule: 'no-presentation-to-infrastructure',
        message: 'Presentation layer should not access infrastructure directly',
      };

      const result = SnapshotViolationSchema.safeParse(violation);
      expect(result.success).toBe(true);
    });

    it('should allow null layers', () => {
      const violation = {
        id: 'test-id',
        type: 'boundary',
        from_file: 'src/unknown/file.ts',
        to_file: 'src/other/file.ts',
        from_layer: null,
        to_layer: null,
        line: 1,
      };

      const result = SnapshotViolationSchema.safeParse(violation);
      expect(result.success).toBe(true);
    });
  });

  describe('SnapshotAntiPatternSchema', () => {
    it('should validate a valid anti-pattern', () => {
      const antipattern = {
        id: 'AP-003',
        file: 'src/api/handler.ts',
        line: 15,
        pattern: 'explicit-any',
        severity: 'warning',
      };

      const result = SnapshotAntiPatternSchema.safeParse(antipattern);
      expect(result.success).toBe(true);
    });
  });

  describe('SnapshotSuppressionSchema', () => {
    it('should validate a valid suppression', () => {
      const suppression = {
        id: 'src/api/handler.ts:10:AP-003',
        pattern_id: 'AP-003',
        file: 'src/api/handler.ts',
        line: 10,
        reason: 'Third-party SDK requires any',
        scope: 'statement',
      };

      const result = SnapshotSuppressionSchema.safeParse(suppression);
      expect(result.success).toBe(true);
    });

    it('should validate suppression with expiry', () => {
      const suppression = {
        id: 'test-id',
        pattern_id: 'AP-004',
        file: 'src/legacy/code.ts',
        line: 20,
        reason: 'Temporary workaround',
        scope: 'line',
        expires_at: '2025-06-01T00:00:00.000Z',
        is_expired: false,
      };

      const result = SnapshotSuppressionSchema.safeParse(suppression);
      expect(result.success).toBe(true);
    });
  });

  describe('SnapshotMetricsSchema', () => {
    it('should validate valid metrics', () => {
      const metrics = {
        boundary_violations: 15,
        antipattern_count: 42,
        suppression_count: 8,
        expired_suppressions: 2,
        files_analysed: 150,
      };

      const result = SnapshotMetricsSchema.safeParse(metrics);
      expect(result.success).toBe(true);
    });

    it('should reject negative values', () => {
      const metrics = {
        boundary_violations: -1,
        antipattern_count: 0,
        suppression_count: 0,
        expired_suppressions: 0,
        files_analysed: 0,
      };

      const result = SnapshotMetricsSchema.safeParse(metrics);
      expect(result.success).toBe(false);
    });
  });

  describe('DriftSnapshotSchema', () => {
    it('should validate a complete snapshot', () => {
      const snapshot: DriftSnapshot = {
        schema_version: '1.0.0',
        created_at: '2025-01-31T10:00:00.000Z',
        name: 'release-1.0',
        metrics: {
          boundary_violations: 15,
          antipattern_count: 42,
          suppression_count: 8,
          expired_suppressions: 1,
          files_analysed: 150,
        },
        antipattern_breakdown: {
          'AP-003': 20,
          'AP-004': 12,
          'AP-006': 10,
        },
        hotspots: [
          {
            path: 'src/legacy/',
            violation_count: 8,
            types: ['AP-003', 'AP-006'],
          },
        ],
        violations: [
          {
            id: 'test-violation-1',
            type: 'boundary',
            from_file: 'src/api/handler.ts',
            to_file: 'src/db/query.ts',
            from_layer: 'presentation',
            to_layer: 'infrastructure',
            line: 42,
          },
        ],
        antipatterns: [
          {
            id: 'AP-003',
            file: 'src/legacy/code.ts',
            line: 10,
            pattern: 'explicit-any',
            severity: 'warning',
          },
        ],
        suppressions: [
          {
            id: 'supp-1',
            pattern_id: 'AP-003',
            file: 'src/compat/shim.ts',
            line: 5,
            reason: 'Legacy compatibility layer',
            scope: 'file',
          },
        ],
        baseline_hash: 'abc123def456',
        git_ref: 'main',
      };

      const result = DriftSnapshotSchema.safeParse(snapshot);
      expect(result.success).toBe(true);
    });

    it('should validate a minimal snapshot', () => {
      const snapshot = {
        schema_version: '1.0.0',
        created_at: '2025-01-31T10:00:00.000Z',
        metrics: {
          boundary_violations: 0,
          antipattern_count: 0,
          suppression_count: 0,
          expired_suppressions: 0,
          files_analysed: 0,
        },
        violations: [],
        antipatterns: [],
        suppressions: [],
      };

      const result = DriftSnapshotSchema.safeParse(snapshot);
      expect(result.success).toBe(true);
    });
  });
});

describe('Snapshot Filename Utilities', () => {
  describe('generateSnapshotFilename', () => {
    it('should generate timestamped filename', () => {
      const date = new Date('2025-01-31T10:30:45.123Z');
      const filename = generateSnapshotFilename(date);

      expect(filename).toMatch(/^snapshot-2025-01-31T10-30-45-123\.json$/);
    });
  });

  describe('generateNamedSnapshotFilename', () => {
    it('should generate named filename', () => {
      const filename = generateNamedSnapshotFilename('release-1.0');
      expect(filename).toBe('snapshot-release-1-0.json');
    });

    it('should sanitise special characters', () => {
      const filename = generateNamedSnapshotFilename('My Release v2.0!');
      expect(filename).toBe('snapshot-my-release-v2-0-.json');
    });
  });

  describe('parseSnapshotFilename', () => {
    it('should parse timestamped filename', () => {
      const result = parseSnapshotFilename('snapshot-2025-01-31T10-30-45-123.json');
      expect(result.isNamed).toBe(false);
      expect(result.nameOrTimestamp).toBe('2025-01-31T10-30-45-123');
    });

    it('should parse named filename', () => {
      const result = parseSnapshotFilename('snapshot-release-1-0.json');
      expect(result.isNamed).toBe(true);
      expect(result.nameOrTimestamp).toBe('release-1-0');
    });

    it('should throw on invalid filename', () => {
      expect(() => parseSnapshotFilename('invalid.json')).toThrow();
    });
  });
});

describe('Snapshot Factory Functions', () => {
  describe('createEmptySnapshot', () => {
    it('should create valid empty snapshot', () => {
      const snapshot = createEmptySnapshot();

      expect(snapshot.schema_version).toBe(SNAPSHOT_SCHEMA_VERSION);
      expect(snapshot.metrics.boundary_violations).toBe(0);
      expect(snapshot.violations).toEqual([]);
      expect(snapshot.antipatterns).toEqual([]);
      expect(snapshot.suppressions).toEqual([]);

      const result = DriftSnapshotSchema.safeParse(snapshot);
      expect(result.success).toBe(true);
    });

    it('should create snapshot with name', () => {
      const snapshot = createEmptySnapshot({ name: 'test-snapshot' });
      expect(snapshot.name).toBe('test-snapshot');
    });

    it('should create snapshot with baseline hash', () => {
      const snapshot = createEmptySnapshot({ baselineHash: 'abc123' });
      expect(snapshot.baseline_hash).toBe('abc123');
    });

    it('should create snapshot with git ref', () => {
      const snapshot = createEmptySnapshot({ gitRef: 'feature/test' });
      expect(snapshot.git_ref).toBe('feature/test');
    });
  });

  describe('validateSnapshot', () => {
    it('should validate valid snapshot', () => {
      const snapshot = createEmptySnapshot();
      const result = validateSnapshot(snapshot);

      expect(result.success).toBe(true);
      expect(result.data).toBeDefined();
    });

    it('should reject invalid snapshot', () => {
      const invalid = { not: 'a snapshot' };
      const result = validateSnapshot(invalid);

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });
  });
});
