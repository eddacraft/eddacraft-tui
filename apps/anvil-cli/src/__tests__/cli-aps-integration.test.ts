/**
 * CLI + APS Integration Tests
 *
 * Tests the integration between CLI commands and APS Core functionality
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdirSync, rmSync, existsSync, writeFileSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import {
  generatePlanId,
  generateHash,
  validateAPSPlan,
  isValidPlanId,
  isValidHash,
  type APSPlan,
  APS_SCHEMA_VERSION,
} from '@anvil/core';
import { loadPlan, savePlan, findPlanById, getWorkspaceRoot } from '../utils/file-io.js';

describe('CLI + APS Integration Tests', () => {
  let tempDir: string;
  let plansDir: string;

  beforeEach(() => {
    // Create temporary workspace
    tempDir = join(tmpdir(), 'anvil-cli-test', Math.random().toString(36).substring(7));
    plansDir = join(tempDir, '.anvil', 'plans');
    mkdirSync(plansDir, { recursive: true });

    // Create package.json to make it look like a workspace
    writeFileSync(
      join(tempDir, 'package.json'),
      JSON.stringify({ name: 'test-workspace', version: '1.0.0' }),
      'utf-8'
    );

    // Set working directory to temp dir for tests
    process.chdir(tempDir);
  });

  afterEach(() => {
    // Clean up
    if (existsSync(tempDir)) {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  describe('Plan Creation', () => {
    it('should create a valid APS plan with minimal data', () => {
      const planId = generatePlanId();
      const intent = 'Add basic logging functionality to the application';

      const planData = {
        id: planId,
        schema_version: APS_SCHEMA_VERSION,
        intent,
        proposed_changes: [],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'test-user',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: ['lint', 'test', 'coverage', 'secrets'],
          skip_checks: [],
        },
      };

      const hash = generateHash(planData);
      const plan: APSPlan = { ...planData, hash } as APSPlan;

      // Verify plan structure
      expect(isValidPlanId(plan.id)).toBe(true);
      expect(isValidHash(plan.hash)).toBe(true);
      expect(plan.intent).toBe(intent);
      expect(plan.schema_version).toBe(APS_SCHEMA_VERSION);
    });

    it('should create a plan with changes', () => {
      const planId = generatePlanId();

      const planData = {
        id: planId,
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Add authentication module',
        proposed_changes: [
          {
            type: 'file_create' as const,
            path: 'src/auth.ts',
            description: 'Create authentication module',
            content: 'export function authenticate() { return true; }',
          },
          {
            type: 'dependency_add' as const,
            path: 'package.json',
            description: 'Add JWT library',
            metadata: { package: 'jsonwebtoken', version: '^9.0.0' },
          },
        ],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'test-user',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: ['lint', 'test'],
          skip_checks: [],
        },
      };

      const hash = generateHash(planData);
      const plan: APSPlan = { ...planData, hash } as APSPlan;

      expect(plan.proposed_changes).toHaveLength(2);
      expect(plan.proposed_changes[0].type).toBe('file_create');
      expect(plan.proposed_changes[1].type).toBe('dependency_add');
    });

    it('should generate unique plan IDs', () => {
      const id1 = generatePlanId();
      const id2 = generatePlanId();
      const id3 = generatePlanId();

      expect(id1).not.toBe(id2);
      expect(id2).not.toBe(id3);
      expect(id1).not.toBe(id3);

      expect(isValidPlanId(id1)).toBe(true);
      expect(isValidPlanId(id2)).toBe(true);
      expect(isValidPlanId(id3)).toBe(true);
    });

    it('should generate deterministic hashes', () => {
      const planData = {
        id: 'aps-12345678',
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Test plan',
        proposed_changes: [],
        provenance: {
          timestamp: '2024-01-01T00:00:00.000Z',
          author: 'test',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: [],
          skip_checks: [],
        },
      };

      const hash1 = generateHash(planData);
      const hash2 = generateHash(planData);

      expect(hash1).toBe(hash2);
      expect(isValidHash(hash1)).toBe(true);
    });
  });

  describe('Plan Validation', () => {
    it('should validate a correct plan', async () => {
      const planData = {
        id: generatePlanId(),
        schema_version: APS_SCHEMA_VERSION,
        intent: 'This is a valid plan with proper structure',
        proposed_changes: [],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'test-user',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: ['lint'],
          skip_checks: [],
        },
      };

      const plan: APSPlan = {
        ...planData,
        hash: generateHash(planData),
      } as APSPlan;

      const result = await validateAPSPlan(plan);

      expect(result.valid).toBe(true);
      expect(result.data).toBeDefined();
      expect(result.issues).toBeUndefined();
    });

    it('should reject plan with invalid ID format', async () => {
      const planData = {
        id: 'invalid-id',
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Test plan with invalid ID',
        proposed_changes: [],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'test',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: [],
          skip_checks: [],
        },
        hash: '0000000000000000000000000000000000000000000000000000000000000000',
      };

      const result = await validateAPSPlan(planData);

      expect(result.valid).toBe(false);
      expect(result.issues).toBeDefined();
      expect(result.issues?.some((issue) => issue.path === 'id')).toBe(true);
    });

    it('should reject plan with short intent', async () => {
      const planData = {
        id: generatePlanId(),
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Short',
        proposed_changes: [],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'test',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: [],
          skip_checks: [],
        },
        hash: '0000000000000000000000000000000000000000000000000000000000000000',
      };

      const result = await validateAPSPlan(planData);

      expect(result.valid).toBe(false);
      expect(result.issues).toBeDefined();
      expect(result.issues?.some((issue) => issue.path === 'intent')).toBe(true);
    });

    it('should reject plan with invalid change type', async () => {
      const planData = {
        id: generatePlanId(),
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Test plan with invalid change type',
        proposed_changes: [
          {
            type: 'invalid_type',
            path: 'test.ts',
            description: 'Invalid change',
          },
        ],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'test',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: [],
          skip_checks: [],
        },
        hash: '0000000000000000000000000000000000000000000000000000000000000000',
      };

      const result = await validateAPSPlan(planData);

      expect(result.valid).toBe(false);
      expect(result.issues).toBeDefined();
    });

    it('should reject plan with wrong schema version', async () => {
      const planData = {
        id: generatePlanId(),
        schema_version: '0.0.1',
        intent: 'Test plan with wrong version',
        proposed_changes: [],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'test',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: [],
          skip_checks: [],
        },
        hash: '0000000000000000000000000000000000000000000000000000000000000000',
      };

      const result = await validateAPSPlan(planData);

      expect(result.valid).toBe(false);
      expect(result.issues?.some((issue) => issue.path === 'schema_version')).toBe(true);
    });
  });

  describe('File I/O Operations', () => {
    it('should save and load a plan', async () => {
      const planData = {
        id: generatePlanId(),
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Test plan for save and load',
        proposed_changes: [
          {
            type: 'file_create' as const,
            path: 'test.ts',
            description: 'Create test file',
          },
        ],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'test-user',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: ['lint'],
          skip_checks: [],
        },
      };

      const plan: APSPlan = {
        ...planData,
        hash: generateHash(planData),
      } as APSPlan;

      // Save plan
      const planPath = join(plansDir, `${plan.id}.json`);
      savePlan(plan, planPath);

      // Verify file exists
      expect(existsSync(planPath)).toBe(true);

      // Load plan
      const loadedPlan = await loadPlan(planPath);

      // Verify loaded plan matches original
      expect(loadedPlan.id).toBe(plan.id);
      expect(loadedPlan.intent).toBe(plan.intent);
      expect(loadedPlan.hash).toBe(plan.hash);
      expect(loadedPlan.proposed_changes).toHaveLength(1);
    });

    it('should find plan by ID', () => {
      const planId = generatePlanId();
      const planData = {
        id: planId,
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Test plan for ID lookup',
        proposed_changes: [],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'test',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: [],
          skip_checks: [],
        },
      };

      const plan: APSPlan = {
        ...planData,
        hash: generateHash(planData),
      } as APSPlan;

      // Save plan
      const planPath = join(plansDir, `${planId}.json`);
      savePlan(plan, planPath);

      // Find by ID
      const foundPath = findPlanById(planId, tempDir);

      expect(foundPath).not.toBeNull();
      expect(foundPath).toBe(planPath);
    });

    it('should return null for non-existent plan ID', () => {
      const nonExistentId = 'aps-99999999';
      const foundPath = findPlanById(nonExistentId, tempDir);

      expect(foundPath).toBeNull();
    });

    it('should throw error when loading non-existent file', async () => {
      const nonExistentPath = join(plansDir, 'nonexistent.json');

      await expect(loadPlan(nonExistentPath)).rejects.toThrow();
    });

    it('should throw error when loading invalid JSON', async () => {
      const invalidPath = join(plansDir, 'invalid.json');
      writeFileSync(invalidPath, 'not valid json{', 'utf-8');

      await expect(loadPlan(invalidPath)).rejects.toThrow();
    });

    it('should throw error when loading plan with invalid schema', async () => {
      const invalidPlan = {
        id: 'bad-id',
        intent: 'Bad',
        // Missing required fields
      };

      const invalidPath = join(plansDir, 'invalid-schema.json');
      writeFileSync(invalidPath, JSON.stringify(invalidPlan), 'utf-8');

      await expect(loadPlan(invalidPath)).rejects.toThrow('Invalid plan');
    });
  });

  describe('Workspace Detection', () => {
    it('should detect workspace root with package.json', () => {
      const root = getWorkspaceRoot();
      expect(root).toBe(tempDir);
      expect(existsSync(join(root, 'package.json'))).toBe(true);
    });

    it('should detect workspace root with .git directory', () => {
      // Remove package.json
      rmSync(join(tempDir, 'package.json'));

      // Create .git directory
      mkdirSync(join(tempDir, '.git'));

      const root = getWorkspaceRoot();
      expect(root).toBe(tempDir);
    });
  });

  describe('End-to-End Plan Workflow', () => {
    it('should complete full plan lifecycle: create → save → load → validate', async () => {
      // 1. Create plan
      const planId = generatePlanId();
      const planData = {
        id: planId,
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Complete end-to-end workflow test',
        proposed_changes: [
          {
            type: 'file_create' as const,
            path: 'src/feature.ts',
            description: 'Add new feature',
            content: 'export const feature = "new";',
          },
        ],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'e2e-test',
          source: 'cli' as const,
          version: '1.0.0',
          repository: 'https://github.com/test/repo',
          branch: 'main',
          commit: 'abc123',
        },
        validations: {
          required_checks: ['lint', 'test'],
          skip_checks: [],
        },
      };

      const plan: APSPlan = {
        ...planData,
        hash: generateHash(planData),
      } as APSPlan;

      // 2. Validate created plan
      const createValidation = await validateAPSPlan(plan);
      expect(createValidation.valid).toBe(true);

      // 3. Save plan
      const planPath = join(plansDir, `${planId}.json`);
      savePlan(plan, planPath);
      expect(existsSync(planPath)).toBe(true);

      // 4. Load plan from disk
      const loadedPlan = await loadPlan(planPath);
      expect(loadedPlan.id).toBe(planId);

      // 5. Validate loaded plan
      const loadValidation = await validateAPSPlan(loadedPlan);
      expect(loadValidation.valid).toBe(true);

      // 6. Find plan by ID
      const foundPath = findPlanById(planId, tempDir);
      expect(foundPath).toBe(planPath);

      // 7. Verify plan content integrity
      expect(loadedPlan.hash).toBe(plan.hash);
      expect(loadedPlan.intent).toBe(plan.intent);
      expect(loadedPlan.proposed_changes).toHaveLength(1);
      expect(loadedPlan.provenance.repository).toBe('https://github.com/test/repo');
    });

    it('should handle plan with multiple changes through full workflow', async () => {
      const planId = generatePlanId();
      const planData = {
        id: planId,
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Multi-change workflow test with comprehensive changes',
        proposed_changes: [
          {
            type: 'file_create' as const,
            path: 'src/models/user.ts',
            description: 'Create user model',
          },
          {
            type: 'file_create' as const,
            path: 'src/services/auth.ts',
            description: 'Create auth service',
          },
          {
            type: 'dependency_add' as const,
            path: 'package.json',
            description: 'Add bcrypt',
            metadata: { package: 'bcrypt', version: '^5.1.0' },
          },
          {
            type: 'config_update' as const,
            path: 'tsconfig.json',
            description: 'Update TypeScript config',
          },
          {
            type: 'script_execute' as const,
            path: 'scripts/migrate.sh',
            description: 'Run database migration',
          },
        ],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'multi-change-test',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: ['lint', 'test', 'coverage', 'secrets'],
          skip_checks: [],
        },
        tags: ['authentication', 'security', 'database'],
        metadata: {
          priority: 'high',
          estimatedHours: 8,
        },
      };

      const plan: APSPlan = {
        ...planData,
        hash: generateHash(planData),
      } as APSPlan;

      // Validate
      const validation = await validateAPSPlan(plan);
      expect(validation.valid).toBe(true);

      // Save
      const planPath = join(plansDir, `${planId}.json`);
      savePlan(plan, planPath);

      // Load
      const loadedPlan = await loadPlan(planPath);

      // Verify all changes preserved
      expect(loadedPlan.proposed_changes).toHaveLength(5);
      expect(loadedPlan.tags).toEqual(['authentication', 'security', 'database']);
      expect(loadedPlan.metadata?.priority).toBe('high');

      // Verify change types
      const changeTypes = loadedPlan.proposed_changes.map((c) => c.type);
      expect(changeTypes).toContain('file_create');
      expect(changeTypes).toContain('dependency_add');
      expect(changeTypes).toContain('config_update');
      expect(changeTypes).toContain('script_execute');
    });
  });

  describe('Plan ID and Hash Utilities', () => {
    it('should validate correct plan ID format', () => {
      expect(isValidPlanId('aps-12345678')).toBe(true);
      expect(isValidPlanId('aps-abcdef00')).toBe(true);
      expect(isValidPlanId('aps-ffffffff')).toBe(true);
    });

    it('should reject invalid plan ID formats', () => {
      expect(isValidPlanId('aps-123')).toBe(false); // Too short
      expect(isValidPlanId('aps-123456789')).toBe(false); // Too long
      expect(isValidPlanId('aps-ABCDEF00')).toBe(false); // Uppercase
      expect(isValidPlanId('plan-12345678')).toBe(false); // Wrong prefix
      expect(isValidPlanId('12345678')).toBe(false); // No prefix
    });

    it('should validate correct hash format', () => {
      const validHash = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef';
      expect(isValidHash(validHash)).toBe(true);
    });

    it('should reject invalid hash formats', () => {
      expect(isValidHash('short')).toBe(false);
      expect(isValidHash('0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF')).toBe(
        false
      ); // Uppercase
      expect(isValidHash('0123456789abcdef')).toBe(false); // Too short
    });
  });

  describe('Error Handling', () => {
    it('should handle corrupted plan files gracefully', async () => {
      const corruptedPath = join(plansDir, 'corrupted.json');
      writeFileSync(corruptedPath, '{"invalid": json content}', 'utf-8');

      await expect(loadPlan(corruptedPath)).rejects.toThrow();
    });

    it('should handle missing required fields', async () => {
      const incompletePlan = {
        id: generatePlanId(),
        intent: 'Incomplete plan',
        // Missing schema_version, provenance, etc.
      };

      const result = await validateAPSPlan(incompletePlan);
      expect(result.valid).toBe(false);
      expect(result.issues).toBeDefined();
      expect(result.issues!.length).toBeGreaterThan(0);
    });

    it('should preserve exact plan content through save/load cycle', async () => {
      const planData = {
        id: generatePlanId(),
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Test content preservation through save/load cycle',
        proposed_changes: [
          {
            type: 'file_create' as const,
            path: 'src/test.ts',
            description: 'Test file with special characters: "quotes" and \'apostrophes\'',
            content: 'const obj = { key: "value", num: 123 };',
          },
        ],
        provenance: {
          timestamp: '2024-01-15T10:30:00.000Z',
          author: 'test@example.com',
          source: 'cli' as const,
          version: '1.2.3',
        },
        validations: {
          required_checks: ['lint', 'test'],
          skip_checks: ['secrets'],
        },
      };

      const plan: APSPlan = {
        ...planData,
        hash: generateHash(planData),
      } as APSPlan;

      // Save
      const planPath = join(plansDir, `${plan.id}.json`);
      savePlan(plan, planPath);

      // Load
      const loaded = await loadPlan(planPath);

      // Deep comparison
      expect(loaded).toEqual(plan);
      expect(loaded.proposed_changes[0].description).toBe(
        'Test file with special characters: "quotes" and \'apostrophes\''
      );
    });
  });
});
