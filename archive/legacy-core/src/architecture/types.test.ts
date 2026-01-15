/**
 * Tests for architecture types and schemas
 */

import { describe, it, expect } from 'vitest';
import {
  EntryPointSchema,
  LayerSchema,
  BoundarySchema,
  BaselineViolationSchema,
  BaselineSnapshotSchema,
  ArchitectureBaselineSchema,
  LayerAssignmentSchema,
  DependencyEdgeSchema,
  BoundaryViolationSchema,
  createViolationId,
  isExistingViolation,
  createDefaultLayers,
  createDefaultBoundaries,
  type BoundaryViolation,
  type BaselineSnapshot,
} from './types.js';

describe('EntryPoint Schema', () => {
  it('should validate a complete entry point', () => {
    const entryPoint = {
      path: 'src/index.ts',
      type: 'package',
      confidence: 'high',
      exports: ['default', 'foo'],
    };

    const result = EntryPointSchema.safeParse(entryPoint);
    expect(result.success).toBe(true);
  });

  it('should validate entry point without optional exports', () => {
    const entryPoint = {
      path: 'src/main.ts',
      type: 'application',
      confidence: 'medium',
    };

    const result = EntryPointSchema.safeParse(entryPoint);
    expect(result.success).toBe(true);
  });

  it('should reject invalid entry point type', () => {
    const entryPoint = {
      path: 'src/index.ts',
      type: 'invalid-type',
      confidence: 'high',
    };

    const result = EntryPointSchema.safeParse(entryPoint);
    expect(result.success).toBe(false);
  });

  it('should validate all entry point types', () => {
    const types = ['package', 'application', 'http', 'api', 'cli', 'worker', 'test', 'unknown'];

    for (const type of types) {
      const result = EntryPointSchema.safeParse({
        path: 'src/file.ts',
        type,
        confidence: 'high',
      });
      expect(result.success).toBe(true);
    }
  });
});

describe('Layer Schema', () => {
  it('should validate a complete layer definition', () => {
    const layer = {
      patterns: ['src/controllers/**', 'src/routes/**'],
      depends_on: ['application', 'shared'],
      description: 'Presentation layer',
    };

    const result = LayerSchema.safeParse(layer);
    expect(result.success).toBe(true);
  });

  it('should validate layer without optional description', () => {
    const layer = {
      patterns: ['src/services/**'],
      depends_on: ['domain'],
    };

    const result = LayerSchema.safeParse(layer);
    expect(result.success).toBe(true);
  });

  it('should reject layer without patterns', () => {
    const layer = {
      depends_on: ['domain'],
    };

    const result = LayerSchema.safeParse(layer);
    expect(result.success).toBe(false);
  });
});

describe('Boundary Schema', () => {
  it('should validate a complete boundary', () => {
    const boundary = {
      name: 'no-presentation-to-infrastructure',
      from: 'presentation',
      to: 'infrastructure',
      severity: 'error',
      message: 'Presentation layer must not directly access infrastructure',
    };

    const result = BoundarySchema.safeParse(boundary);
    expect(result.success).toBe(true);
  });

  it('should validate all severity levels', () => {
    const severities = ['error', 'warning', 'info'];

    for (const severity of severities) {
      const result = BoundarySchema.safeParse({
        name: 'test-boundary',
        from: 'layer-a',
        to: 'layer-b',
        severity,
        message: 'Test message',
      });
      expect(result.success).toBe(true);
    }
  });
});

describe('BaselineViolation Schema', () => {
  it('should validate a complete violation', () => {
    const violation = {
      id: 'src_controllers_user_ts:src_repositories_user_repo_ts:5',
      from_layer: 'presentation',
      to_layer: 'infrastructure',
      from_file: 'src/controllers/user.ts',
      to_file: 'src/repositories/user-repo.ts',
      import_line: 5,
    };

    const result = BaselineViolationSchema.safeParse(violation);
    expect(result.success).toBe(true);
  });

  it('should reject violation with non-positive line number', () => {
    const violation = {
      id: 'test-id',
      from_layer: 'presentation',
      to_layer: 'infrastructure',
      from_file: 'src/a.ts',
      to_file: 'src/b.ts',
      import_line: 0,
    };

    const result = BaselineViolationSchema.safeParse(violation);
    expect(result.success).toBe(false);
  });
});

describe('BaselineSnapshot Schema', () => {
  it('should validate a complete snapshot', () => {
    const snapshot = {
      module_count: 127,
      timestamp: '2025-01-15T10:30:00Z',
      violations: [
        {
          id: 'v-001',
          from_layer: 'presentation',
          to_layer: 'infrastructure',
          from_file: 'src/controllers/legacy.ts',
          to_file: 'src/repositories/user-repo.ts',
          import_line: 5,
        },
      ],
    };

    const result = BaselineSnapshotSchema.safeParse(snapshot);
    expect(result.success).toBe(true);
  });

  it('should validate snapshot with empty violations', () => {
    const snapshot = {
      module_count: 50,
      timestamp: '2025-01-15T10:30:00Z',
      violations: [],
    };

    const result = BaselineSnapshotSchema.safeParse(snapshot);
    expect(result.success).toBe(true);
  });
});

describe('ArchitectureBaseline Schema', () => {
  it('should validate a complete baseline', () => {
    const baseline = {
      schema_version: '0.1.0',
      created_at: '2025-01-15T10:00:00Z',
      updated_at: '2025-01-15T10:30:00Z',
      entry_points: [
        {
          path: 'src/index.ts',
          type: 'package',
          confidence: 'high',
        },
      ],
      layers: {
        presentation: {
          patterns: ['src/controllers/**'],
          depends_on: ['application', 'shared'],
        },
        application: {
          patterns: ['src/services/**'],
          depends_on: ['domain', 'shared'],
        },
      },
      boundaries: [
        {
          name: 'no-presentation-to-infrastructure',
          from: 'presentation',
          to: 'infrastructure',
          severity: 'error',
          message: 'Not allowed',
        },
      ],
      baseline_snapshot: {
        module_count: 100,
        timestamp: '2025-01-15T10:30:00Z',
        violations: [],
      },
    };

    const result = ArchitectureBaselineSchema.safeParse(baseline);
    expect(result.success).toBe(true);
  });

  it('should reject baseline with wrong schema version', () => {
    const baseline = {
      schema_version: '0.2.0', // Wrong version
      created_at: '2025-01-15T10:00:00Z',
      updated_at: '2025-01-15T10:30:00Z',
      entry_points: [],
      layers: {},
      boundaries: [],
      baseline_snapshot: {
        module_count: 0,
        timestamp: '2025-01-15T10:30:00Z',
        violations: [],
      },
    };

    const result = ArchitectureBaselineSchema.safeParse(baseline);
    expect(result.success).toBe(false);
  });
});

describe('LayerAssignment Schema', () => {
  it('should validate assignment with layer', () => {
    const assignment = {
      file: 'src/controllers/user.ts',
      layer: 'presentation',
      confidence: 'high',
      matched_pattern: '**/controllers/**',
    };

    const result = LayerAssignmentSchema.safeParse(assignment);
    expect(result.success).toBe(true);
  });

  it('should validate assignment without layer (unassigned)', () => {
    const assignment = {
      file: 'src/unknown/file.ts',
      layer: null,
      confidence: 'low',
    };

    const result = LayerAssignmentSchema.safeParse(assignment);
    expect(result.success).toBe(true);
  });
});

describe('DependencyEdge Schema', () => {
  it('should validate a complete edge', () => {
    const edge = {
      from: 'src/controllers/user.ts',
      to: 'src/services/user-service.ts',
      from_layer: 'presentation',
      to_layer: 'application',
      line: 5,
      type: 'import',
    };

    const result = DependencyEdgeSchema.safeParse(edge);
    expect(result.success).toBe(true);
  });

  it('should validate all import types', () => {
    const types = ['import', 'require', 'dynamic'];

    for (const type of types) {
      const result = DependencyEdgeSchema.safeParse({
        from: 'a.ts',
        to: 'b.ts',
        from_layer: null,
        to_layer: null,
        line: 1,
        type,
      });
      expect(result.success).toBe(true);
    }
  });
});

describe('BoundaryViolation Schema', () => {
  it('should validate a new violation', () => {
    const violation = {
      edge: {
        from: 'src/controllers/user.ts',
        to: 'src/repositories/user-repo.ts',
        from_layer: 'presentation',
        to_layer: 'infrastructure',
        line: 10,
        type: 'import',
      },
      is_new: true,
    };

    const result = BoundaryViolationSchema.safeParse(violation);
    expect(result.success).toBe(true);
  });

  it('should validate an existing violation with baseline_id', () => {
    const violation = {
      edge: {
        from: 'src/controllers/user.ts',
        to: 'src/repositories/user-repo.ts',
        from_layer: 'presentation',
        to_layer: 'infrastructure',
        line: 10,
        type: 'import',
      },
      boundary: {
        name: 'no-presentation-to-infrastructure',
        from: 'presentation',
        to: 'infrastructure',
        severity: 'error',
        message: 'Not allowed',
      },
      is_new: false,
      baseline_id: 'v-001',
    };

    const result = BoundaryViolationSchema.safeParse(violation);
    expect(result.success).toBe(true);
  });
});

describe('Utility Functions', () => {
  describe('createViolationId', () => {
    it('should create deterministic IDs', () => {
      const id1 = createViolationId('src/a.ts', 'src/b.ts', 10);
      const id2 = createViolationId('src/a.ts', 'src/b.ts', 10);

      expect(id1).toBe(id2);
    });

    it('should create different IDs for different inputs', () => {
      const id1 = createViolationId('src/a.ts', 'src/b.ts', 10);
      const id2 = createViolationId('src/a.ts', 'src/b.ts', 11);
      const id3 = createViolationId('src/a.ts', 'src/c.ts', 10);

      expect(id1).not.toBe(id2);
      expect(id1).not.toBe(id3);
    });

    it('should sanitise special characters', () => {
      const id = createViolationId('src/path/to/file.ts', 'src/other/file.ts', 5);

      // Should not contain characters that could break IDs
      expect(id).not.toContain('/');
      expect(id).toContain('_');
    });
  });

  describe('isExistingViolation', () => {
    it('should return true for existing violations', () => {
      const violation: BoundaryViolation = {
        edge: {
          from: 'src/a.ts',
          to: 'src/b.ts',
          from_layer: 'presentation',
          to_layer: 'infrastructure',
          line: 10,
          type: 'import',
        },
        is_new: false,
      };

      const baseline: BaselineSnapshot = {
        module_count: 100,
        timestamp: '2025-01-15T10:30:00Z',
        violations: [
          {
            id: createViolationId('src/a.ts', 'src/b.ts', 10),
            from_layer: 'presentation',
            to_layer: 'infrastructure',
            from_file: 'src/a.ts',
            to_file: 'src/b.ts',
            import_line: 10,
          },
        ],
      };

      expect(isExistingViolation(violation, baseline)).toBe(true);
    });

    it('should return false for new violations', () => {
      const violation: BoundaryViolation = {
        edge: {
          from: 'src/new.ts',
          to: 'src/b.ts',
          from_layer: 'presentation',
          to_layer: 'infrastructure',
          line: 10,
          type: 'import',
        },
        is_new: true,
      };

      const baseline: BaselineSnapshot = {
        module_count: 100,
        timestamp: '2025-01-15T10:30:00Z',
        violations: [],
      };

      expect(isExistingViolation(violation, baseline)).toBe(false);
    });
  });

  describe('createDefaultLayers', () => {
    it('should create all standard layers', () => {
      const layers = createDefaultLayers();

      expect(layers).toHaveProperty('presentation');
      expect(layers).toHaveProperty('application');
      expect(layers).toHaveProperty('domain');
      expect(layers).toHaveProperty('infrastructure');
      expect(layers).toHaveProperty('shared');
    });

    it('should have valid patterns for each layer', () => {
      const layers = createDefaultLayers();

      for (const [_name, layer] of Object.entries(layers)) {
        expect(layer.patterns.length).toBeGreaterThan(0);
        expect(Array.isArray(layer.depends_on)).toBe(true);
        expect(layer.description).toBeDefined();
      }
    });

    it('should have correct dependency rules', () => {
      const layers = createDefaultLayers();

      // Presentation should depend on application and shared
      expect(layers.presentation.depends_on).toContain('application');
      expect(layers.presentation.depends_on).toContain('shared');

      // Domain should only depend on shared
      expect(layers.domain.depends_on).toEqual(['shared']);

      // Shared should have no dependencies
      expect(layers.shared.depends_on).toEqual([]);
    });
  });

  describe('createDefaultBoundaries', () => {
    it('should create boundaries from layers', () => {
      const layers = createDefaultLayers();
      const boundaries = createDefaultBoundaries(layers);

      expect(boundaries.length).toBeGreaterThan(0);
    });

    it('should not create self-referencing boundaries', () => {
      const layers = createDefaultLayers();
      const boundaries = createDefaultBoundaries(layers);

      for (const boundary of boundaries) {
        expect(boundary.from).not.toBe(boundary.to);
      }
    });

    it('should create boundary for presentation to infrastructure', () => {
      const layers = createDefaultLayers();
      const boundaries = createDefaultBoundaries(layers);

      const presentationToInfra = boundaries.find(
        (b) => b.from === 'presentation' && b.to === 'infrastructure'
      );

      expect(presentationToInfra).toBeDefined();
      expect(presentationToInfra?.severity).toBe('error');
    });

    it('should not create boundary for allowed dependencies', () => {
      const layers = createDefaultLayers();
      const boundaries = createDefaultBoundaries(layers);

      // Presentation is allowed to depend on application
      const presentationToApp = boundaries.find(
        (b) => b.from === 'presentation' && b.to === 'application'
      );

      expect(presentationToApp).toBeUndefined();
    });
  });
});
