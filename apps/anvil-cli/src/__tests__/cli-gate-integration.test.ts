/**
 * CLI + Gate Integration Tests
 *
 * Tests the integration between CLI gate command and Gate functionality
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkdirSync, rmSync, existsSync, writeFileSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import {
  generatePlanId,
  generateHash,
  GateRunner,
  GateConfigManager,
  type APSPlan,
  type Evidence,
  APS_SCHEMA_VERSION,
} from '@anvil/core';
import { savePlan, loadPlan } from '../utils/file-io.js';

describe('CLI + Gate Integration Tests', () => {
  let tempDir: string;
  let plansDir: string;
  let gateRunner: GateRunner;
  let configManager: GateConfigManager;

  beforeEach(() => {
    // Create temporary workspace
    tempDir = join(tmpdir(), 'anvil-gate-test', Math.random().toString(36).substring(7));
    plansDir = join(tempDir, '.anvil', 'plans');
    mkdirSync(plansDir, { recursive: true });

    // Create package.json
    writeFileSync(
      join(tempDir, 'package.json'),
      JSON.stringify({ name: 'test-workspace', version: '1.0.0' }),
      'utf-8'
    );

    // Initialize gate infrastructure
    gateRunner = new GateRunner();
    configManager = new GateConfigManager(tempDir);

    process.chdir(tempDir);
  });

  afterEach(() => {
    if (existsSync(tempDir)) {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  describe('Gate Configuration', () => {
    it('should create default gate configuration', () => {
      const config = {
        version: 1,
        checks: [
          {
            name: 'secret',
            description: 'Secret scanning',
            enabled: true,
            config: {},
          },
        ],
        thresholds: {
          overall_score: 80,
        },
      };

      configManager.saveConfig(config);

      const loaded = configManager.loadConfig();
      expect(loaded).toBeDefined();
      expect(loaded.version).toBe(1);
      expect(loaded.checks).toHaveLength(1);
      expect(loaded.checks[0].name).toBe('secret');
    });

    it('should enable/disable checks in configuration', () => {
      const config = {
        version: 1,
        checks: [
          {
            name: 'secret',
            description: 'Secret scanning',
            enabled: true,
            config: {},
          },
          {
            name: 'lint',
            description: 'ESLint checking',
            enabled: false,
            config: {},
          },
        ],
        thresholds: {
          overall_score: 80,
        },
      };

      configManager.saveConfig(config);
      const loaded = configManager.loadConfig();

      expect(loaded.checks.find((c) => c.name === 'secret')?.enabled).toBe(true);
      expect(loaded.checks.find((c) => c.name === 'lint')?.enabled).toBe(false);
    });
  });

  describe('Gate Execution', () => {
    it('should run gate checks on a clean plan', async () => {
      const planData = {
        id: generatePlanId(),
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Add clean functionality without secrets',
        proposed_changes: [
          {
            type: 'file_create' as const,
            path: 'src/utils.ts',
            description: 'Create utility functions',
            content: 'export function add(a: number, b: number) { return a + b; }',
          },
        ],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'test-user',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: ['secret'],
          skip_checks: [],
        },
      };

      const plan: APSPlan = {
        ...planData,
        hash: generateHash(planData),
      } as APSPlan;

      // Create the file referenced in the plan
      const srcDir = join(tempDir, 'src');
      mkdirSync(srcDir, { recursive: true });
      writeFileSync(join(srcDir, 'utils.ts'), planData.proposed_changes[0].content!, 'utf-8');

      const config = {
        version: 1,
        checks: [
          {
            name: 'secret',
            description: 'Secret scanning',
            enabled: true,
            config: {},
          },
        ],
        thresholds: {
          overall_score: 80,
        },
      };

      const result = await gateRunner.runGate(plan, config, tempDir);

      expect(result).toBeDefined();
      expect(result.overall).toBe(true);
      expect(result.checks).toHaveLength(1);
      expect(result.checks[0].check).toBe('secret');
      expect(result.checks[0].passed).toBe(true);
    });

    it('should detect secrets in plan changes', async () => {
      const planData = {
        id: generatePlanId(),
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Add configuration with potential secret',
        proposed_changes: [
          {
            type: 'file_create' as const,
            path: 'src/config.ts',
            description: 'Create config file',
            content: 'export const apiKey = "sk-1234567890abcdef1234567890abcdef";',
          },
        ],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'test-user',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: ['secret'],
          skip_checks: [],
        },
      };

      const plan: APSPlan = {
        ...planData,
        hash: generateHash(planData),
      } as APSPlan;

      // Create the file with secret
      const srcDir = join(tempDir, 'src');
      mkdirSync(srcDir, { recursive: true });
      writeFileSync(join(srcDir, 'config.ts'), planData.proposed_changes[0].content!, 'utf-8');

      const config = {
        version: 1,
        checks: [
          {
            name: 'secret',
            description: 'Secret scanning',
            enabled: true,
            config: {},
          },
        ],
        thresholds: {
          overall_score: 80,
        },
      };

      const result = await gateRunner.runGate(plan, config, tempDir);

      expect(result.overall).toBe(false);
      expect(result.checks[0].passed).toBe(false);
      expect(result.checks[0].details?.findings).toBeDefined();
      expect(result.checks[0].details?.findings.length).toBeGreaterThan(0);
    });

    it('should skip disabled checks', async () => {
      const planData = {
        id: generatePlanId(),
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Test plan with disabled check',
        proposed_changes: [],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'test-user',
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

      const config = {
        version: 1,
        checks: [
          {
            name: 'secret',
            description: 'Secret scanning',
            enabled: false,
            config: {},
          },
        ],
        thresholds: {
          overall_score: 80,
        },
      };

      const result = await gateRunner.runGate(plan, config, tempDir);

      expect(result.overall).toBe(true);
      expect(result.checks[0].skipped).toBe(true);
    });

    it('should calculate overall score correctly', async () => {
      const planData = {
        id: generatePlanId(),
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Test overall score calculation',
        proposed_changes: [
          {
            type: 'file_create' as const,
            path: 'src/test.ts',
            description: 'Create test file',
            content: 'export const test = true;',
          },
        ],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'test-user',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: ['secret'],
          skip_checks: [],
        },
      };

      const plan: APSPlan = {
        ...planData,
        hash: generateHash(planData),
      } as APSPlan;

      // Create clean file
      const srcDir = join(tempDir, 'src');
      mkdirSync(srcDir, { recursive: true });
      writeFileSync(join(srcDir, 'test.ts'), planData.proposed_changes[0].content!, 'utf-8');

      const config = {
        version: 1,
        checks: [
          {
            name: 'secret',
            description: 'Secret scanning',
            enabled: true,
            config: {},
          },
        ],
        thresholds: {
          overall_score: 50,
        },
      };

      const result = await gateRunner.runGate(plan, config, tempDir);

      expect(result.score).toBe(100);
      expect(result.overall).toBe(true);
    });
  });

  describe('Gate Evidence Integration', () => {
    it('should add evidence to plan after gate execution', async () => {
      const planData = {
        id: generatePlanId(),
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Test evidence tracking',
        proposed_changes: [
          {
            type: 'file_create' as const,
            path: 'src/feature.ts',
            description: 'Add feature',
            content: 'export const feature = "implemented";',
          },
        ],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'test-user',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: ['secret'],
          skip_checks: [],
        },
      };

      let plan: APSPlan = {
        ...planData,
        hash: generateHash(planData),
      } as APSPlan;

      // Save initial plan
      const planPath = join(plansDir, `${plan.id}.json`);
      savePlan(plan, planPath);

      // Create file
      const srcDir = join(tempDir, 'src');
      mkdirSync(srcDir, { recursive: true });
      writeFileSync(join(srcDir, 'feature.ts'), planData.proposed_changes[0].content!, 'utf-8');

      // Run gate
      const config = {
        version: 1,
        checks: [
          {
            name: 'secret',
            description: 'Secret scanning',
            enabled: true,
            config: {},
          },
        ],
        thresholds: {
          overall_score: 80,
        },
      };

      const gateResult = await gateRunner.runGate(plan, config, tempDir);

      // Create evidence from gate result
      const evidence: Evidence = {
        gate_version: '1.0.0',
        timestamp: new Date().toISOString(),
        overall_status: gateResult.overall ? 'passed' : 'failed',
        checks: gateResult.checks.map((check) => ({
          check: check.check,
          status: check.passed ? ('passed' as const) : ('failed' as const),
          timestamp: new Date().toISOString(),
          message: check.message,
          details: check.details,
        })),
        summary: gateResult.message,
      };

      // Update plan with evidence
      plan = {
        ...plan,
        evidence: [evidence],
      };

      // Save updated plan
      savePlan(plan, planPath);

      // Reload and verify
      const loadedPlan = await loadPlan(planPath);
      expect(loadedPlan.evidence).toBeDefined();
      expect(loadedPlan.evidence).toHaveLength(1);
      expect(loadedPlan.evidence![0].overall_status).toBe('passed');
      expect(loadedPlan.evidence![0].checks).toHaveLength(1);
      expect(loadedPlan.evidence![0].checks[0].check).toBe('secret');
    });

    it('should append multiple evidence entries', async () => {
      const planData = {
        id: generatePlanId(),
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Test multiple evidence entries',
        proposed_changes: [],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'test-user',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: [],
          skip_checks: [],
        },
      };

      let plan: APSPlan = {
        ...planData,
        hash: generateHash(planData),
        evidence: [],
      } as APSPlan;

      // First evidence entry
      const evidence1: Evidence = {
        gate_version: '1.0.0',
        timestamp: '2024-01-01T10:00:00.000Z',
        overall_status: 'passed',
        checks: [
          {
            check: 'secret',
            status: 'passed',
            timestamp: '2024-01-01T10:00:00.000Z',
          },
        ],
      };

      plan = {
        ...plan,
        evidence: [evidence1],
      };

      // Second evidence entry
      const evidence2: Evidence = {
        gate_version: '1.0.0',
        timestamp: '2024-01-01T11:00:00.000Z',
        overall_status: 'passed',
        checks: [
          {
            check: 'lint',
            status: 'passed',
            timestamp: '2024-01-01T11:00:00.000Z',
          },
        ],
      };

      plan = {
        ...plan,
        evidence: [...(plan.evidence || []), evidence2],
      };

      expect(plan.evidence).toHaveLength(2);
      expect(plan.evidence![0].timestamp).toBe('2024-01-01T10:00:00.000Z');
      expect(plan.evidence![1].timestamp).toBe('2024-01-01T11:00:00.000Z');
    });
  });

  describe('End-to-End Gate Workflow', () => {
    it('should complete full gate workflow: create plan → save → run gate → add evidence → save', async () => {
      // 1. Create plan
      const planId = generatePlanId();
      const planData = {
        id: planId,
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Complete gate workflow with evidence tracking',
        proposed_changes: [
          {
            type: 'file_create' as const,
            path: 'src/module.ts',
            description: 'Create new module',
            content: 'export class Module { constructor() {} }',
          },
        ],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'e2e-test',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: ['secret'],
          skip_checks: [],
        },
      };

      let plan: APSPlan = {
        ...planData,
        hash: generateHash(planData),
      } as APSPlan;

      // 2. Save initial plan
      const planPath = join(plansDir, `${planId}.json`);
      savePlan(plan, planPath);
      expect(existsSync(planPath)).toBe(true);

      // 3. Create files referenced in plan
      const srcDir = join(tempDir, 'src');
      mkdirSync(srcDir, { recursive: true });
      writeFileSync(join(srcDir, 'module.ts'), planData.proposed_changes[0].content!, 'utf-8');

      // 4. Set up gate configuration
      const config = {
        version: 1,
        checks: [
          {
            name: 'secret',
            description: 'Secret scanning',
            enabled: true,
            config: {},
          },
        ],
        thresholds: {
          overall_score: 80,
        },
      };

      configManager.saveConfig(config);

      // 5. Load configuration
      const loadedConfig = configManager.loadConfig();
      expect(loadedConfig.checks).toHaveLength(1);

      // 6. Run gate checks
      const gateResult = await gateRunner.runGate(plan, loadedConfig, tempDir);
      expect(gateResult.overall).toBe(true);
      expect(gateResult.checks[0].passed).toBe(true);

      // 7. Create evidence from gate result
      const evidence: Evidence = {
        gate_version: '1.0.0',
        timestamp: new Date().toISOString(),
        overall_status: gateResult.overall ? 'passed' : 'failed',
        checks: gateResult.checks.map((check) => ({
          check: check.check,
          status: check.passed ? ('passed' as const) : ('failed' as const),
          timestamp: new Date().toISOString(),
          message: check.message,
          details: check.details,
        })),
        summary: gateResult.message,
      };

      // 8. Update plan with evidence
      plan = {
        ...plan,
        evidence: [evidence],
      };

      // 9. Save updated plan
      savePlan(plan, planPath);

      // 10. Reload and verify complete plan
      const finalPlan = await loadPlan(planPath);

      expect(finalPlan.id).toBe(planId);
      expect(finalPlan.evidence).toBeDefined();
      expect(finalPlan.evidence).toHaveLength(1);
      expect(finalPlan.evidence![0].overall_status).toBe('passed');
      expect(finalPlan.evidence![0].checks[0].check).toBe('secret');
      expect(finalPlan.evidence![0].checks[0].status).toBe('passed');
      expect(finalPlan.proposed_changes).toHaveLength(1);
    });

    it('should handle failed gate checks and record failure evidence', async () => {
      const planId = generatePlanId();
      const planData = {
        id: planId,
        schema_version: APS_SCHEMA_VERSION,
        intent: 'Test failed gate check workflow',
        proposed_changes: [
          {
            type: 'file_create' as const,
            path: 'src/secrets.ts',
            description: 'Config with secrets (will fail)',
            content: 'const secret = "sk-1234567890abcdef1234567890abcdef";',
          },
        ],
        provenance: {
          timestamp: new Date().toISOString(),
          author: 'test-user',
          source: 'cli' as const,
          version: '1.0.0',
        },
        validations: {
          required_checks: ['secret'],
          skip_checks: [],
        },
      };

      let plan: APSPlan = {
        ...planData,
        hash: generateHash(planData),
      } as APSPlan;

      // Save plan
      const planPath = join(plansDir, `${planId}.json`);
      savePlan(plan, planPath);

      // Create file with secret
      const srcDir = join(tempDir, 'src');
      mkdirSync(srcDir, { recursive: true });
      writeFileSync(join(srcDir, 'secrets.ts'), planData.proposed_changes[0].content!, 'utf-8');

      // Run gate
      const config = {
        version: 1,
        checks: [
          {
            name: 'secret',
            description: 'Secret scanning',
            enabled: true,
            config: {},
          },
        ],
        thresholds: {
          overall_score: 80,
        },
      };

      const gateResult = await gateRunner.runGate(plan, config, tempDir);

      // Gate should fail
      expect(gateResult.overall).toBe(false);
      expect(gateResult.checks[0].passed).toBe(false);

      // Create failure evidence
      const evidence: Evidence = {
        gate_version: '1.0.0',
        timestamp: new Date().toISOString(),
        overall_status: 'failed',
        checks: gateResult.checks.map((check) => ({
          check: check.check,
          status: check.passed ? ('passed' as const) : ('failed' as const),
          timestamp: new Date().toISOString(),
          message: check.message,
          details: check.details,
        })),
        summary: 'Gate failed due to secret detection',
      };

      // Update and save plan
      plan = {
        ...plan,
        evidence: [evidence],
      };

      savePlan(plan, planPath);

      // Verify failure is recorded
      const loadedPlan = await loadPlan(planPath);
      expect(loadedPlan.evidence![0].overall_status).toBe('failed');
      expect(loadedPlan.evidence![0].checks[0].status).toBe('failed');
      expect(loadedPlan.evidence![0].checks[0].details?.findings).toBeDefined();
    });
  });
});
