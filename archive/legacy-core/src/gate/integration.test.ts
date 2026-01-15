import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { GateRunner } from './gate-runner.js';
import { GateConfigManager } from './gate-config.js';
import { writeFileSync, mkdirSync, rmSync, existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';

describe('Gate Integration Tests', () => {
  let gateRunner: GateRunner;
  let configManager: GateConfigManager;
  let tempDir: string;

  beforeEach(() => {
    tempDir = join(tmpdir(), 'anvil-test', Math.random().toString(36));
    mkdirSync(tempDir, { recursive: true });

    gateRunner = new GateRunner();
    configManager = new GateConfigManager(tempDir);
  });

  afterEach(() => {
    if (existsSync(tempDir)) {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it('should run complete gate workflow', async () => {
    // Create a test plan
    const plan = {
      id: 'aps-test123',
      schema_version: '0.1.0',
      hash: 'test-hash',
      intent: 'Add TypeScript support',
      proposed_changes: [
        {
          type: 'file_create',
          path: 'src/index.ts',
          description: 'Create index file',
          content: 'console.log("Hello, TypeScript!");',
        },
        {
          type: 'file_create',
          path: 'src/utils.ts',
          description: 'Create utils file',
          content: 'export function helper() { return "help"; }',
        },
      ],
      provenance: {
        timestamp: '2024-01-01T00:00:00Z',
        author: 'test@example.com',
        source: 'cli',
        version: '1.0.0',
      },
      validations: {
        required_checks: [],
        skip_checks: [],
      },
      evidence: [],
      executions: [],
    };

    // Create the files referenced in the plan
    mkdirSync(join(tempDir, 'src'), { recursive: true });
    writeFileSync(join(tempDir, 'src', 'index.ts'), 'console.log("Hello, TypeScript!");');
    writeFileSync(join(tempDir, 'src', 'utils.ts'), 'export function helper() { return "help"; }');

    // Create a basic config
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

    // Run the gate
    const result = await gateRunner.runGate(plan, config, tempDir);

    expect(result).toBeDefined();
    expect(result.overall).toBe(true);
    expect(result.checks).toHaveLength(1);
    expect(result.checks[0].check).toBe('secret');
    expect(result.checks[0].passed).toBe(true);
  });

  it('should handle mixed check results', async () => {
    const plan = {
      id: 'aps-test123',
      schema_version: '0.1.0',
      hash: 'test-hash',
      intent: 'Test plan with secrets',
      proposed_changes: [
        {
          type: 'file_create',
          path: 'src/secret.js',
          description: 'Create file with secret',
          content: 'const apiKey = "sk-1234567890abcdef";',
        },
      ],
      provenance: {
        timestamp: '2024-01-01T00:00:00Z',
        author: 'test@example.com',
        source: 'cli',
        version: '1.0.0',
      },
      validations: {
        required_checks: [],
        skip_checks: [],
      },
      evidence: [],
      executions: [],
    };

    mkdirSync(join(tempDir, 'src'), { recursive: true });
    writeFileSync(join(tempDir, 'src', 'secret.js'), 'const apiKey = "sk-1234567890abcdef";');

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
    expect(result.checks[0].details?.findings).toHaveLength(1);
  });

  it('should handle disabled checks', async () => {
    const plan = {
      id: 'aps-test123',
      schema_version: '0.1.0',
      hash: 'test-hash',
      intent: 'Test plan',
      proposed_changes: [],
      provenance: {
        timestamp: '2024-01-01T00:00:00Z',
        author: 'test@example.com',
        source: 'cli',
        version: '1.0.0',
      },
      validations: {
        required_checks: [],
        skip_checks: [],
      },
      evidence: [],
      executions: [],
    };

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

  it('should calculate scores correctly with multiple checks', async () => {
    const plan = {
      id: 'aps-test123',
      schema_version: '0.1.0',
      hash: 'test-hash',
      intent: 'Test plan',
      proposed_changes: [
        {
          type: 'file_create',
          path: 'src/clean.js',
          description: 'Create clean file',
          content: 'console.log("clean code");',
        },
      ],
      provenance: {
        timestamp: '2024-01-01T00:00:00Z',
        author: 'test@example.com',
        source: 'cli',
        version: '1.0.0',
      },
      validations: {
        required_checks: [],
        skip_checks: [],
      },
      evidence: [],
      executions: [],
    };

    mkdirSync(join(tempDir, 'src'), { recursive: true });
    writeFileSync(join(tempDir, 'src', 'clean.js'), 'console.log("clean code");');

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
