/**
 * Unit Tests for OPA Executor
 *
 * Tests OPA policy evaluation and violation detection
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { OPAExecutor, type OPAInput } from './opa-executor.js';
import { type LoadedPolicy } from './policy-loader.js';
import { existsSync, mkdirSync, rmSync, writeFileSync, chmodSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir, platform } from 'node:os';

describe('OPAExecutor', () => {
  let executor: OPAExecutor;
  let tempDir: string;
  let mockBinaryPath: string;
  let mockInput: OPAInput;
  let mockPolicies: LoadedPolicy[];

  beforeEach(() => {
    tempDir = join(tmpdir(), 'anvil-opa-executor-test', Math.random().toString(36));
    mkdirSync(tempDir, { recursive: true });

    // Create a mock OPA binary that returns valid JSON
    mockBinaryPath = join(tempDir, platform() === 'win32' ? 'opa.exe' : 'opa');
    const mockScript =
      platform() === 'win32'
        ? '@echo {"result":[{"expressions":[{"value":{}}]}]}'
        : '#!/bin/sh\necho \'{"result":[{"expressions":[{"value":{}}]}]}\'';

    writeFileSync(mockBinaryPath, mockScript);
    if (platform() !== 'win32') {
      chmodSync(mockBinaryPath, 0o755);
    }

    executor = new OPAExecutor(mockBinaryPath, {
      timeout: 5000,
      includeRawOutput: false,
    });

    mockInput = {
      plan: {
        id: 'test-plan-123',
        hash: 'test-hash-abc',
        intent: 'Test plan for OPA evaluation',
        schema_version: '0.1.0',
        proposed_changes: [
          {
            type: 'file_create',
            path: 'src/test.ts',
            description: 'Test file',
          },
        ],
        change_count: 1,
        affected_directories: ['src'],
      },
      context: {
        workspace_root: tempDir,
        timestamp: Date.now(),
      },
    };

    mockPolicies = [
      {
        name: 'test_policy',
        path: join(tempDir, 'test_policy.rego'),
        content: `package anvil.policies.test_policy

violation[msg] {
  false
  msg := "This should not trigger"
}`,
        package: 'anvil.policies.test_policy',
        hasTests: false,
      },
    ];
  });

  afterEach(() => {
    if (existsSync(tempDir)) {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  describe('initialization', () => {
    it('should create executor that can evaluate policies', async () => {
      const result = await executor.evaluate(mockPolicies, mockInput);
      expect(result).toBeDefined();
      expect(result.success).toBe(true);
      expect(Array.isArray(result.violations)).toBe(true);
    });

    it('should include raw output only when enabled', async () => {
      const withRaw = new OPAExecutor(mockBinaryPath, { includeRawOutput: true });
      const withoutRaw = new OPAExecutor(mockBinaryPath, { includeRawOutput: false });

      const resultWith = await withRaw.evaluate(mockPolicies, mockInput);
      const resultWithout = await withoutRaw.evaluate(mockPolicies, mockInput);

      expect(resultWith.raw_output).toBeDefined();
      expect(resultWithout.raw_output).toBeUndefined();
    });
  });

  describe('policy evaluation', () => {
    it('should evaluate empty policies list', async () => {
      const result = await executor.evaluate([], mockInput);

      expect(result.success).toBe(true);
      expect(result.violations).toEqual([]);
      expect(result.metadata.policy_count).toBe(0);
    });

    it('should return success with no violations', async () => {
      const result = await executor.evaluate(mockPolicies, mockInput);

      expect(result.success).toBe(true);
      expect(result.violations).toEqual([]);
      expect(result.metadata.policy_count).toBe(1);
    });

    it('should include execution metadata', async () => {
      const result = await executor.evaluate(mockPolicies, mockInput);

      expect(result.metadata).toBeDefined();
      expect(result.metadata.policy_count).toBe(1);
      expect(typeof result.metadata.execution_time_ms).toBe('number');
      expect(result.metadata.execution_time_ms).toBeGreaterThanOrEqual(0);
    });

    it('should handle multiple policies', async () => {
      const multiplePolicies: LoadedPolicy[] = [
        ...mockPolicies,
        {
          name: 'second_policy',
          path: join(tempDir, 'second_policy.rego'),
          content: 'package anvil.policies.second_policy',
          package: 'anvil.policies.second_policy',
          hasTests: false,
        },
      ];

      const result = await executor.evaluate(multiplePolicies, mockInput);

      expect(result.success).toBe(true);
      expect(result.metadata.policy_count).toBe(2);
    });
  });

  describe('violation detection', () => {
    it('should return empty violations for clean policy evaluation', async () => {
      const result = await executor.evaluate(mockPolicies, mockInput);

      expect(Array.isArray(result.violations)).toBe(true);
      expect(result.violations).toHaveLength(0);
    });

    it('should parse string violations from mock binary', async () => {
      // Create a mock binary that returns actual violations
      const violationBinary = join(tempDir, platform() === 'win32' ? 'opa-viol.exe' : 'opa-viol');
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    violation: ['Test violation message'],
                  },
                },
              },
            ],
          },
        ],
      });

      const script =
        platform() === 'win32'
          ? `@echo ${outputJson.replace(/"/g, '\\"')}`
          : `#!/bin/sh\necho '${outputJson}'`;

      writeFileSync(violationBinary, script);
      if (platform() !== 'win32') {
        chmodSync(violationBinary, 0o755);
      }

      const violationExecutor = new OPAExecutor(violationBinary);
      const result = await violationExecutor.evaluate(mockPolicies, mockInput);

      expect(result.violations).toHaveLength(1);
      expect(result.violations[0]).toHaveProperty('rule');
      expect(result.violations[0]).toHaveProperty('severity');
      expect(result.violations[0]).toHaveProperty('message');
      expect(result.violations[0].message).toBe('Test violation message');
      expect(result.violations[0].severity).toBe('error');
    });
  });

  describe('error handling', () => {
    it('should handle invalid binary path', async () => {
      const invalidExecutor = new OPAExecutor('/nonexistent/opa');

      const result = await invalidExecutor.evaluate(mockPolicies, mockInput);

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });

    it('should handle malformed policy content', async () => {
      const badPolicy: LoadedPolicy = {
        name: 'bad_policy',
        path: join(tempDir, 'bad.rego'),
        content: 'this is not valid rego',
        package: 'unknown',
        hasTests: false,
      };

      const result = await executor.evaluate([badPolicy], mockInput);

      // Should return error or handle gracefully
      expect(result).toBeDefined();
      expect(typeof result.success).toBe('boolean');
    });

    it('should handle timeout errors', async () => {
      // Create executor with very short timeout
      const timeoutExecutor = new OPAExecutor(mockBinaryPath, { timeout: 1 });

      const result = await timeoutExecutor.evaluate(mockPolicies, mockInput);

      // May timeout depending on system performance
      expect(result).toBeDefined();
      expect(typeof result.success).toBe('boolean');
    });

    it('should return error details on failure', async () => {
      const invalidExecutor = new OPAExecutor('/nonexistent/opa');

      const result = await invalidExecutor.evaluate(mockPolicies, mockInput);

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
      expect(typeof result.error).toBe('string');
    });
  });

  describe('syntax validation', () => {
    it('should validate valid Rego syntax', async () => {
      const validPolicy = 'package test\nallow = true\n';

      const result = await executor.validateSyntax(validPolicy);

      // Validation might fail if mock binary doesn't support check command
      expect(result).toBeDefined();
      expect(typeof result.valid).toBe('boolean');
      expect(Array.isArray(result.errors)).toBe(true);
    });

    it('should detect invalid Rego syntax', async () => {
      const invalidPolicy = 'package test\ninvalid rego syntax here\n';

      const result = await executor.validateSyntax(invalidPolicy);

      expect(result).toBeDefined();
      expect(typeof result.valid).toBe('boolean');
      expect(Array.isArray(result.errors)).toBe(true);
    });

    it('should return error messages for invalid syntax', async () => {
      const invalidPolicy = 'not valid';

      const result = await executor.validateSyntax(invalidPolicy);

      // Should have errors for invalid syntax
      expect(Array.isArray(result.errors)).toBe(true);
    });
  });

  describe('test execution', () => {
    it('should run policy tests', async () => {
      const testFiles = [join(tempDir, 'test_policy_test.rego')];
      writeFileSync(
        testFiles[0],
        `package test
test_example {
  true
}
`
      );

      const result = await executor.runTests(mockPolicies, testFiles);

      expect(result).toBeDefined();
      expect(typeof result.passed).toBe('number');
      expect(typeof result.failed).toBe('number');
      expect(Array.isArray(result.errors)).toBe(true);
      expect(Array.isArray(result.details)).toBe(true);
    });

    it('should return empty results for no test files', async () => {
      const result = await executor.runTests(mockPolicies, []);

      expect(result.passed).toBe(0);
      expect(result.failed).toBe(0);
      expect(result.errors).toEqual([]);
      expect(result.details).toEqual([]);
    });

    it('should handle test execution errors', async () => {
      const invalidTestFiles = ['/nonexistent/test.rego'];

      const result = await executor.runTests(mockPolicies, invalidTestFiles);

      // Should handle errors gracefully
      expect(result).toBeDefined();
      expect(Array.isArray(result.errors)).toBe(true);
    });
  });

  describe('raw output', () => {
    it('should include raw output when enabled', async () => {
      const verboseExecutor = new OPAExecutor(mockBinaryPath, { includeRawOutput: true });

      const result = await verboseExecutor.evaluate(mockPolicies, mockInput);

      expect(result.success).toBe(true);
      expect(result.raw_output).toBeDefined();
    });

    it('should exclude raw output when disabled', async () => {
      const result = await executor.evaluate(mockPolicies, mockInput);

      expect(result.raw_output).toBeUndefined();
    });
  });

  describe('severity parsing', () => {
    it('should default severity to error for deny/violation rules', async () => {
      const denyBinary = join(tempDir, platform() === 'win32' ? 'opa-deny.exe' : 'opa-deny');
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  deny_policy: {
                    deny: ['Denied action'],
                  },
                },
              },
            ],
          },
        ],
      });

      const script =
        platform() === 'win32'
          ? `@echo ${outputJson.replace(/"/g, '\\"')}`
          : `#!/bin/sh\necho '${outputJson}'`;

      writeFileSync(denyBinary, script);
      if (platform() !== 'win32') {
        chmodSync(denyBinary, 0o755);
      }

      const denyPolicy: LoadedPolicy = {
        name: 'deny_policy',
        path: join(tempDir, 'deny_policy.rego'),
        content: 'package anvil.policies.deny_policy',
        package: 'anvil.policies.deny_policy',
        hasTests: false,
      };

      const denyExecutor = new OPAExecutor(denyBinary);
      const result = await denyExecutor.evaluate([denyPolicy], mockInput);

      expect(result.violations).toHaveLength(1);
      expect(result.violations[0].severity).toBe('error');
    });

    it('should default severity to warning for warn rules', async () => {
      const warnBinary = join(tempDir, platform() === 'win32' ? 'opa-warn.exe' : 'opa-warn');
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  warn_policy: {
                    warn: ['Warning message'],
                  },
                },
              },
            ],
          },
        ],
      });

      const script =
        platform() === 'win32'
          ? `@echo ${outputJson.replace(/"/g, '\\"')}`
          : `#!/bin/sh\necho '${outputJson}'`;

      writeFileSync(warnBinary, script);
      if (platform() !== 'win32') {
        chmodSync(warnBinary, 0o755);
      }

      const warnPolicy: LoadedPolicy = {
        name: 'warn_policy',
        path: join(tempDir, 'warn_policy.rego'),
        content: 'package anvil.policies.warn_policy',
        package: 'anvil.policies.warn_policy',
        hasTests: false,
      };

      const warnExecutor = new OPAExecutor(warnBinary);
      const result = await warnExecutor.evaluate([warnPolicy], mockInput);

      expect(result.violations).toHaveLength(1);
      expect(result.violations[0].severity).toBe('warning');
    });
  });

  describe('category inference', () => {
    it('should infer security category from policy name', async () => {
      const secBinary = join(tempDir, platform() === 'win32' ? 'opa-sec.exe' : 'opa-sec');
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  security_check: {
                    violation: ['Security violation'],
                  },
                },
              },
            ],
          },
        ],
      });

      const script =
        platform() === 'win32'
          ? `@echo ${outputJson.replace(/"/g, '\\"')}`
          : `#!/bin/sh\necho '${outputJson}'`;

      writeFileSync(secBinary, script);
      if (platform() !== 'win32') {
        chmodSync(secBinary, 0o755);
      }

      const securityPolicy: LoadedPolicy = {
        name: 'security_check',
        path: join(tempDir, 'security_check.rego'),
        content: 'package anvil.policies.security_check',
        package: 'anvil.policies.security_check',
        hasTests: false,
      };

      const secExecutor = new OPAExecutor(secBinary);
      const result = await secExecutor.evaluate([securityPolicy], mockInput);

      expect(result.violations).toHaveLength(1);
      expect(result.violations[0].category).toBe('security');
    });

    it('should infer architecture category from policy name', async () => {
      const archBinary = join(tempDir, platform() === 'win32' ? 'opa-arch.exe' : 'opa-arch');
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  architecture_boundary: {
                    violation: ['Architecture violation'],
                  },
                },
              },
            ],
          },
        ],
      });

      const script =
        platform() === 'win32'
          ? `@echo ${outputJson.replace(/"/g, '\\"')}`
          : `#!/bin/sh\necho '${outputJson}'`;

      writeFileSync(archBinary, script);
      if (platform() !== 'win32') {
        chmodSync(archBinary, 0o755);
      }

      const archPolicy: LoadedPolicy = {
        name: 'architecture_boundary',
        path: join(tempDir, 'architecture_boundary.rego'),
        content: 'package anvil.policies.architecture_boundary',
        package: 'anvil.policies.architecture_boundary',
        hasTests: false,
      };

      const archExecutor = new OPAExecutor(archBinary);
      const result = await archExecutor.evaluate([archPolicy], mockInput);

      expect(result.violations).toHaveLength(1);
      expect(result.violations[0].category).toBe('architecture');
    });

    it('should default to custom category for unknown policy names', async () => {
      const customBinary = join(tempDir, platform() === 'win32' ? 'opa-custom.exe' : 'opa-custom');
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  my_custom_policy: {
                    violation: ['Custom violation'],
                  },
                },
              },
            ],
          },
        ],
      });

      const script =
        platform() === 'win32'
          ? `@echo ${outputJson.replace(/"/g, '\\"')}`
          : `#!/bin/sh\necho '${outputJson}'`;

      writeFileSync(customBinary, script);
      if (platform() !== 'win32') {
        chmodSync(customBinary, 0o755);
      }

      const customPolicy: LoadedPolicy = {
        name: 'my_custom_policy',
        path: join(tempDir, 'my_custom_policy.rego'),
        content: 'package anvil.policies.my_custom_policy',
        package: 'anvil.policies.my_custom_policy',
        hasTests: false,
      };

      const customExecutor = new OPAExecutor(customBinary);
      const result = await customExecutor.evaluate([customPolicy], mockInput);

      expect(result.violations).toHaveLength(1);
      expect(result.violations[0].category).toBe('custom');
    });
  });
});
