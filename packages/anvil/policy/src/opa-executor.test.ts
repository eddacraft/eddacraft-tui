/**
 * Unit Tests for OPA Executor
 *
 * Tests OPA policy evaluation and violation detection
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { OPAExecutor, type OPAInput } from './opa-executor.js';
import { type LoadedPolicy } from './policy-loader.js';
import { mkdtempSync, writeFileSync, chmodSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir, platform } from 'node:os';
import { safeCleanup } from '../../../../tools/test-utils/safe-cleanup.js';

/**
 * Minimal capabilities document served by mock binaries for
 * `opa capabilities --current` (CIB-108). Content only needs to be parseable
 * and carry a builtins list; enforcement itself is covered by
 * opa-executor.capabilities.test.ts.
 */
const MOCK_CAPABILITIES = '{"builtins":[{"name":"eq"},{"name":"count"},{"name":"http.send"}]}';

/**
 * Build a mock `opa` script that answers the `capabilities` subcommand and
 * echoes `outputJson` for every other invocation (eval/test).
 */
function mockOpaScript(outputJson: string): string {
  if (platform() === 'win32') {
    return [
      '@echo off',
      'if "%1"=="capabilities" goto caps',
      `echo ${outputJson}`,
      'exit /b 0',
      ':caps',
      `echo ${MOCK_CAPABILITIES}`,
      'exit /b 0',
      '',
    ].join('\r\n');
  }
  return [
    '#!/bin/sh',
    'if [ "$1" = "capabilities" ]; then',
    `  echo '${MOCK_CAPABILITIES}'`,
    '  exit 0',
    'fi',
    `echo '${outputJson}'`,
    '',
  ].join('\n');
}

function writeMockOpa(path: string, outputJson: string): void {
  writeFileSync(path, mockOpaScript(outputJson));
  if (platform() !== 'win32') {
    chmodSync(path, 0o755);
  }
}

describe('OPAExecutor', () => {
  let executor: OPAExecutor;
  let tempDir: string;
  let mockBinaryPath: string;
  let mockInput: OPAInput;
  let mockPolicies: LoadedPolicy[];

  beforeEach(() => {
    tempDir = mkdtempSync(join(tmpdir(), 'anvil-opa-executor-test-'));

    // Create a mock OPA binary that returns valid JSON
    mockBinaryPath = join(tempDir, platform() === 'win32' ? 'opa.cmd' : 'opa');
    writeMockOpa(mockBinaryPath, '{"result":[{"expressions":[{"value":{}}]}]}');

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

  afterEach(async () => {
    await safeCleanup(tempDir);
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
    it('should parse string violations from violation field', async () => {
      // Create a mock binary that returns string violations
      const violationBinary = join(tempDir, platform() === 'win32' ? 'opa-viol.cmd' : 'opa-viol');
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    violation: ['Simple string violation', 'Another violation'],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(violationBinary, outputJson);

      const violationExecutor = new OPAExecutor(violationBinary);
      const result = await violationExecutor.evaluate(mockPolicies, mockInput);

      expect(result.success).toBe(true);
      expect(result.violations).toHaveLength(2);
      expect(result.violations[0].message).toBe('Simple string violation');
      expect(result.violations[1].message).toBe('Another violation');
      expect(result.violations[0].severity).toBe('error');
    });

    it('should parse string violations from violations field', async () => {
      const violationsBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-viols.cmd' : 'opa-viols'
      );
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    violations: ['Violation from violations field'],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(violationsBinary, outputJson);

      const violationsExecutor = new OPAExecutor(violationsBinary);
      const result = await violationsExecutor.evaluate(mockPolicies, mockInput);

      expect(result.success).toBe(true);
      expect(result.violations).toHaveLength(1);
      expect(result.violations[0].message).toBe('Violation from violations field');
    });

    it('should parse structured violation objects', async () => {
      const violationBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-struct.cmd' : 'opa-struct'
      );
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    violation: [
                      {
                        rule: 'test-rule',
                        severity: 'warning',
                        message: 'Structured violation',
                        path: 'src/test.ts',
                        category: 'quality',
                      },
                    ],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(violationBinary, outputJson);

      const violationExecutor = new OPAExecutor(violationBinary);
      const result = await violationExecutor.evaluate(mockPolicies, mockInput);

      expect(result.violations).toHaveLength(1);
      const violation = result.violations[0];
      expect(violation.rule).toBe('test-rule');
      expect(violation.severity).toBe('warning');
      expect(violation.message).toBe('Structured violation');
      expect(violation.path).toBe('src/test.ts');
      expect(violation.category).toBe('quality');
    });

    it('should parse deny arrays', async () => {
      const denyBinary = join(tempDir, platform() === 'win32' ? 'opa-deny.cmd' : 'opa-deny');
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    deny: ['Denied action', 'Another denied action'],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(denyBinary, outputJson);

      const denyExecutor = new OPAExecutor(denyBinary);
      const result = await denyExecutor.evaluate(mockPolicies, mockInput);

      expect(result.violations).toHaveLength(2);
      expect(result.violations[0].severity).toBe('error');
      expect(result.violations[1].severity).toBe('error');
    });

    it('should parse denies arrays', async () => {
      const deniesBinary = join(tempDir, platform() === 'win32' ? 'opa-denies.cmd' : 'opa-denies');
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    denies: ['Denied via denies field'],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(deniesBinary, outputJson);

      const deniesExecutor = new OPAExecutor(deniesBinary);
      const result = await deniesExecutor.evaluate(mockPolicies, mockInput);

      expect(result.violations).toHaveLength(1);
      expect(result.violations[0].severity).toBe('error');
    });

    it('should parse warn arrays', async () => {
      const warnBinary = join(tempDir, platform() === 'win32' ? 'opa-warn.cmd' : 'opa-warn');
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    warn: ['Warning message', 'Another warning'],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(warnBinary, outputJson);

      const warnExecutor = new OPAExecutor(warnBinary);
      const result = await warnExecutor.evaluate(mockPolicies, mockInput);

      expect(result.violations).toHaveLength(2);
      expect(result.violations[0].severity).toBe('warning');
      expect(result.violations[1].severity).toBe('warning');
    });

    it('should parse warnings arrays', async () => {
      const warningsBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-warnings.cmd' : 'opa-warnings'
      );
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    warnings: ['Warning via warnings field'],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(warningsBinary, outputJson);

      const warningsExecutor = new OPAExecutor(warningsBinary);
      const result = await warningsExecutor.evaluate(mockPolicies, mockInput);

      expect(result.violations).toHaveLength(1);
      expect(result.violations[0].severity).toBe('warning');
    });

    it('should include violation fingerprints', async () => {
      const violationBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-finger.cmd' : 'opa-finger'
      );
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    violation: ['Test violation'],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(violationBinary, outputJson);

      const fingerprintExecutor = new OPAExecutor(violationBinary);
      const result = await fingerprintExecutor.evaluate(mockPolicies, mockInput);

      expect(result.violations[0].fingerprint).toBeDefined();
      expect(typeof result.violations[0].fingerprint).toBe('string');
      expect(result.violations[0].fingerprint?.length).toBe(16);
    });

    it('should generate consistent fingerprints', async () => {
      const violationBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-consist.cmd' : 'opa-consist'
      );
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    violation: [
                      {
                        rule: 'same-rule',
                        message: 'Same message',
                        path: 'same/path.ts',
                      },
                      {
                        rule: 'same-rule',
                        message: 'Same message',
                        path: 'same/path.ts',
                      },
                    ],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(violationBinary, outputJson);

      const consistentExecutor = new OPAExecutor(violationBinary);
      const result = await consistentExecutor.evaluate(mockPolicies, mockInput);

      expect(result.violations).toHaveLength(2);
      expect(result.violations[0].fingerprint).toBe(result.violations[1].fingerprint);
    });

    it('should infer security category from policy names', async () => {
      const securityBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-security.cmd' : 'opa-security'
      );
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  security_check: {
                    violation: ['Security issue found'],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(securityBinary, outputJson);

      const securityPolicy: LoadedPolicy = {
        name: 'security_check',
        path: join(tempDir, 'security_check.rego'),
        content: 'package anvil.policies.security_check',
        package: 'anvil.policies.security_check',
        hasTests: false,
      };

      const securityExecutor = new OPAExecutor(securityBinary);
      const result = await securityExecutor.evaluate([securityPolicy], mockInput);

      expect(result.violations[0].category).toBe('security');
    });

    it('should infer architecture category from policy names', async () => {
      const archBinary = join(tempDir, platform() === 'win32' ? 'opa-arch.cmd' : 'opa-arch');
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

      writeMockOpa(archBinary, outputJson);

      const archPolicy: LoadedPolicy = {
        name: 'architecture_boundary',
        path: join(tempDir, 'architecture_boundary.rego'),
        content: 'package anvil.policies.architecture_boundary',
        package: 'anvil.policies.architecture_boundary',
        hasTests: false,
      };

      const archExecutor = new OPAExecutor(archBinary);
      const result = await archExecutor.evaluate([archPolicy], mockInput);

      expect(result.violations[0].category).toBe('architecture');
    });

    it('should infer coverage category from policy names', async () => {
      const coverageBinary = join(tempDir, platform() === 'win32' ? 'opa-cov.cmd' : 'opa-cov');
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  coverage_min: {
                    violation: ['Coverage too low'],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(coverageBinary, outputJson);

      const coveragePolicy: LoadedPolicy = {
        name: 'coverage_min',
        path: join(tempDir, 'coverage_min.rego'),
        content: 'package anvil.policies.coverage_min',
        package: 'anvil.policies.coverage_min',
        hasTests: false,
      };

      const coverageExecutor = new OPAExecutor(coverageBinary);
      const result = await coverageExecutor.evaluate([coveragePolicy], mockInput);

      expect(result.violations[0].category).toBe('coverage');
    });

    it('should default to custom category for unknown types', async () => {
      const customBinary = join(tempDir, platform() === 'win32' ? 'opa-custom.cmd' : 'opa-custom');
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

      writeMockOpa(customBinary, outputJson);

      const customPolicy: LoadedPolicy = {
        name: 'my_custom_policy',
        path: join(tempDir, 'my_custom_policy.rego'),
        content: 'package anvil.policies.my_custom_policy',
        package: 'anvil.policies.my_custom_policy',
        hasTests: false,
      };

      const customExecutor = new OPAExecutor(customBinary);
      const result = await customExecutor.evaluate([customPolicy], mockInput);

      expect(result.violations[0].category).toBe('custom');
    });

    it('should parse msg field as message', async () => {
      const msgBinary = join(tempDir, platform() === 'win32' ? 'opa-msg.cmd' : 'opa-msg');
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    violation: [
                      {
                        msg: 'Message from msg field',
                      },
                    ],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(msgBinary, outputJson);

      const msgExecutor = new OPAExecutor(msgBinary);
      const result = await msgExecutor.evaluate(mockPolicies, mockInput);

      expect(result.violations[0].message).toBe('Message from msg field');
    });

    it('should include documentation_url in violations', async () => {
      const docBinary = join(tempDir, platform() === 'win32' ? 'opa-doc.cmd' : 'opa-doc');
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    violation: [
                      {
                        message: 'Test violation',
                        documentation_url: 'https://example.com/docs/violation',
                      },
                    ],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(docBinary, outputJson);

      const docExecutor = new OPAExecutor(docBinary);
      const result = await docExecutor.evaluate(mockPolicies, mockInput);

      expect(result.violations[0].documentation_url).toBe('https://example.com/docs/violation');
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
      // Create a binary that sleeps longer than timeout
      const slowBinary = join(tempDir, platform() === 'win32' ? 'opa-slow.cmd' : 'opa-slow');
      const script =
        platform() === 'win32'
          ? '@echo off\nping -n 11 127.0.0.1 >nul\necho {"result":[]}'
          : '#!/bin/sh\nsleep 10\necho \'{"result":[]}\'';

      writeFileSync(slowBinary, script);
      if (platform() !== 'win32') {
        chmodSync(slowBinary, 0o755);
      }

      const timeoutExecutor = new OPAExecutor(slowBinary, { timeout: 100 });

      const result = await timeoutExecutor.evaluate(mockPolicies, mockInput);

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
      expect(result.error).toContain('timed out');
    });

    it('should return error details on failure', async () => {
      const invalidExecutor = new OPAExecutor('/nonexistent/opa');

      const result = await invalidExecutor.evaluate(mockPolicies, mockInput);

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
      expect(typeof result.error).toBe('string');
    });

    it('should handle OPA binary that returns non-zero exit code', async () => {
      const errorBinary = join(tempDir, platform() === 'win32' ? 'opa-error.cmd' : 'opa-error');
      const script = platform() === 'win32' ? '@echo off\nexit /b 1' : '#!/bin/sh\nexit 1';

      writeFileSync(errorBinary, script);
      if (platform() !== 'win32') {
        chmodSync(errorBinary, 0o755);
      }

      const errorExecutor = new OPAExecutor(errorBinary);
      const result = await errorExecutor.evaluate(mockPolicies, mockInput);

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });

    it('should handle OPA binary that outputs invalid JSON', async () => {
      const badJsonBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-badjson.cmd' : 'opa-badjson'
      );
      writeMockOpa(badJsonBinary, '{invalid json}');

      const badJsonExecutor = new OPAExecutor(badJsonBinary);
      const result = await badJsonExecutor.evaluate(mockPolicies, mockInput);

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });

    it('should handle stderr output', async () => {
      const stderrBinary = join(tempDir, platform() === 'win32' ? 'opa-stderr.cmd' : 'opa-stderr');
      const script =
        platform() === 'win32'
          ? '@echo off\necho Error message 1>&2\nexit /b 1'
          : '#!/bin/sh\necho "Error message" >&2\nexit 1';

      writeFileSync(stderrBinary, script);
      if (platform() !== 'win32') {
        chmodSync(stderrBinary, 0o755);
      }

      const stderrExecutor = new OPAExecutor(stderrBinary);
      const result = await stderrExecutor.evaluate(mockPolicies, mockInput);

      expect(result.success).toBe(false);
      expect(result.error).toContain('Error message');
    });

    it('should handle empty OPA output', async () => {
      const emptyBinary = join(tempDir, platform() === 'win32' ? 'opa-empty.cmd' : 'opa-empty');
      const script = platform() === 'win32' ? '@echo off\necho.' : '#!/bin/sh\necho ""';

      writeFileSync(emptyBinary, script);
      if (platform() !== 'win32') {
        chmodSync(emptyBinary, 0o755);
      }

      const emptyExecutor = new OPAExecutor(emptyBinary);
      const result = await emptyExecutor.evaluate(mockPolicies, mockInput);

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });

    it('should handle malformed OPA output structure', async () => {
      const malformedBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-malformed.cmd' : 'opa-malformed'
      );
      writeMockOpa(malformedBinary, '{"unexpected":"structure"}');

      const malformedExecutor = new OPAExecutor(malformedBinary);
      const result = await malformedExecutor.evaluate(mockPolicies, mockInput);

      // Should succeed but with no violations since output doesn't match expected format
      expect(result.success).toBe(true);
      expect(result.violations).toEqual([]);
    });

    it('should handle empty expressions array', async () => {
      const emptyExprBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-emptyexpr.cmd' : 'opa-emptyexpr'
      );
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [],
          },
        ],
      });

      writeMockOpa(emptyExprBinary, outputJson);

      const emptyExprExecutor = new OPAExecutor(emptyExprBinary);
      const result = await emptyExprExecutor.evaluate(mockPolicies, mockInput);

      expect(result.success).toBe(true);
      expect(result.violations).toEqual([]);
    });

    it('should handle expressions with non-object value', async () => {
      const nonObjValBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-nonobjval.cmd' : 'opa-nonobjval'
      );
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: null,
              },
            ],
          },
        ],
      });

      writeMockOpa(nonObjValBinary, outputJson);

      const nonObjValExecutor = new OPAExecutor(nonObjValBinary);
      const result = await nonObjValExecutor.evaluate(mockPolicies, mockInput);

      expect(result.success).toBe(true);
      expect(result.violations).toEqual([]);
    });

    it('should handle non-object policy result', async () => {
      const nonObjBinary = join(tempDir, platform() === 'win32' ? 'opa-nonobj.cmd' : 'opa-nonobj');
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: 'not an object',
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(nonObjBinary, outputJson);

      const nonObjExecutor = new OPAExecutor(nonObjBinary);
      const result = await nonObjExecutor.evaluate(mockPolicies, mockInput);

      expect(result.success).toBe(true);
      expect(result.violations).toEqual([]);
    });

    it('should handle non-string, non-object violations gracefully', async () => {
      const badViolBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-badviol.cmd' : 'opa-badviol'
      );
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    violation: [123, true, null],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(badViolBinary, outputJson);

      const badViolExecutor = new OPAExecutor(badViolBinary);
      const result = await badViolExecutor.evaluate(mockPolicies, mockInput);

      // Should ignore invalid violation types (numbers, booleans, null)
      // Arrays are converted to objects and stringified
      expect(result.success).toBe(true);
      expect(result.violations).toEqual([]);
    });

    it('should convert array violations to JSON strings', async () => {
      const arrayViolBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-arrayviol.cmd' : 'opa-arrayviol'
      );
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    violation: [['item1', 'item2']],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(arrayViolBinary, outputJson);

      const arrayViolExecutor = new OPAExecutor(arrayViolBinary);
      const result = await arrayViolExecutor.evaluate(mockPolicies, mockInput);

      expect(result.success).toBe(true);
      expect(result.violations).toHaveLength(1);
      expect(result.violations[0].message).toBe('["item1","item2"]');
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

    it('should validate with warnings in stderr', async () => {
      // Create binary that returns 0 but has stderr
      const warnBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-checkwarn.cmd' : 'opa-checkwarn'
      );
      const script =
        platform() === 'win32'
          ? '@echo off\necho Warning: unused variable 1>&2\nexit /b 0'
          : '#!/bin/sh\necho "Warning: unused variable" >&2\nexit 0';

      writeFileSync(warnBinary, script);
      if (platform() !== 'win32') {
        chmodSync(warnBinary, 0o755);
      }

      const warnExecutor = new OPAExecutor(warnBinary);
      const result = await warnExecutor.validateSyntax('package test\nallow = true');

      expect(result.valid).toBe(true);
      expect(result.errors).toHaveLength(1);
      expect(result.errors[0]).toContain('Warning');
    });

    it('should detect invalid Rego syntax with error regex', async () => {
      // Create binary that returns syntax error
      const errorBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-checkerr.cmd' : 'opa-checkerr'
      );
      const script =
        platform() === 'win32'
          ? '@echo off\necho policy.rego:2: error: syntax error 1>&2\nexit /b 1'
          : '#!/bin/sh\necho "policy.rego:2: error: syntax error" >&2\nexit 1';

      writeFileSync(errorBinary, script);
      if (platform() !== 'win32') {
        chmodSync(errorBinary, 0o755);
      }

      const errorExecutor = new OPAExecutor(errorBinary);
      const result = await errorExecutor.validateSyntax('package test\ninvalid syntax');

      expect(result.valid).toBe(false);
      expect(result.errors).toHaveLength(1);
      expect(result.errors[0]).toContain('error:');
    });

    it('should handle validation failure without error regex match', async () => {
      // Create binary that returns non-zero without matching error format
      const failBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-checkfail.cmd' : 'opa-checkfail'
      );
      const script =
        platform() === 'win32'
          ? '@echo off\necho Something went wrong 1>&2\nexit /b 1'
          : '#!/bin/sh\necho "Something went wrong" >&2\nexit 1';

      writeFileSync(failBinary, script);
      if (platform() !== 'win32') {
        chmodSync(failBinary, 0o755);
      }

      const failExecutor = new OPAExecutor(failBinary);
      const result = await failExecutor.validateSyntax('package test');

      expect(result.valid).toBe(false);
      expect(result.errors).toHaveLength(1);
      // Should use fallback error message
      expect(result.errors[0]).toBeTruthy();
    });

    it('should handle validation timeout', async () => {
      // Create binary that sleeps
      const slowBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-checkslow.cmd' : 'opa-checkslow'
      );
      const script =
        platform() === 'win32'
          ? '@echo off\nping -n 11 127.0.0.1 >nul\nexit /b 0'
          : '#!/bin/sh\nsleep 10\nexit 0';

      writeFileSync(slowBinary, script);
      if (platform() !== 'win32') {
        chmodSync(slowBinary, 0o755);
      }

      const slowExecutor = new OPAExecutor(slowBinary, { timeout: 100 });
      const result = await slowExecutor.validateSyntax('package test');

      expect(result.valid).toBe(false);
      expect(result.errors).toHaveLength(1);
      expect(result.errors[0]).toContain('timed out');
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

    it('should handle failed tests', async () => {
      // Create a mock binary that returns test failures
      const testBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-testfail.cmd' : 'opa-testfail'
      );
      const outputJson = JSON.stringify([
        {
          name: 'test_passes',
          fail: false,
        },
        {
          name: 'test_fails',
          fail: true,
          error: {
            message: 'Test assertion failed',
          },
        },
      ]);

      writeMockOpa(testBinary, outputJson);

      const testExecutor = new OPAExecutor(testBinary);
      const testFiles = [join(tempDir, 'test.rego')];
      writeFileSync(testFiles[0], 'package test\ntest_example { true }');

      const result = await testExecutor.runTests(mockPolicies, testFiles);

      expect(result.passed).toBe(1);
      expect(result.failed).toBe(1);
      expect(result.details).toHaveLength(2);
      expect(result.details[0].passed).toBe(true);
      expect(result.details[1].passed).toBe(false);
      expect(result.details[1].message).toBe('Test assertion failed');
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
      // Create a mock binary that returns violations from a deny rule
      const denyBinary = join(tempDir, platform() === 'win32' ? 'opa-deny.cmd' : 'opa-deny');
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

      writeMockOpa(denyBinary, outputJson);

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
      // Create a mock binary that returns violations from a warn rule
      const warnBinary = join(tempDir, platform() === 'win32' ? 'opa-warn.cmd' : 'opa-warn');
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

      writeMockOpa(warnBinary, outputJson);

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
    it('should infer scope category from policy name', async () => {
      const scopeBinary = join(tempDir, platform() === 'win32' ? 'opa-scope.cmd' : 'opa-scope');
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  scope_check: {
                    violation: ['Scope violation'],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(scopeBinary, outputJson);

      const scopePolicy: LoadedPolicy = {
        name: 'scope_check',
        path: join(tempDir, 'scope_check.rego'),
        content: 'package anvil.policies.scope_check',
        package: 'anvil.policies.scope_check',
        hasTests: false,
      };

      const scopeExecutor = new OPAExecutor(scopeBinary);
      const result = await scopeExecutor.evaluate([scopePolicy], mockInput);

      expect(result.violations[0].category).toBe('scope');
    });

    it('should infer quality category from policy name', async () => {
      const qualityBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-quality.cmd' : 'opa-quality'
      );
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  lint_rules: {
                    violation: ['Quality issue'],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(qualityBinary, outputJson);

      const qualityPolicy: LoadedPolicy = {
        name: 'lint_rules',
        path: join(tempDir, 'lint_rules.rego'),
        content: 'package anvil.policies.lint_rules',
        package: 'anvil.policies.lint_rules',
        hasTests: false,
      };

      const qualityExecutor = new OPAExecutor(qualityBinary);
      const result = await qualityExecutor.evaluate([qualityPolicy], mockInput);

      expect(result.violations[0].category).toBe('quality');
    });

    it('should infer compliance category from policy name', async () => {
      const complianceBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-compliance.cmd' : 'opa-compliance'
      );
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  compliance_check: {
                    violation: ['Compliance issue'],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(complianceBinary, outputJson);

      const compliancePolicy: LoadedPolicy = {
        name: 'compliance_check',
        path: join(tempDir, 'compliance_check.rego'),
        content: 'package anvil.policies.compliance_check',
        package: 'anvil.policies.compliance_check',
        hasTests: false,
      };

      const complianceExecutor = new OPAExecutor(complianceBinary);
      const result = await complianceExecutor.evaluate([compliancePolicy], mockInput);

      expect(result.violations[0].category).toBe('compliance');
    });
  });

  describe('severity parsing', () => {
    it('should parse error severity', async () => {
      const errorBinary = join(tempDir, platform() === 'win32' ? 'opa-err.cmd' : 'opa-err');
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    violation: [
                      { severity: 'error', message: 'Error 1' },
                      { severity: 'err', message: 'Error 2' },
                    ],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(errorBinary, outputJson);

      const errorExecutor = new OPAExecutor(errorBinary);
      const result = await errorExecutor.evaluate(mockPolicies, mockInput);

      expect(result.violations[0].severity).toBe('error');
      expect(result.violations[1].severity).toBe('error');
    });

    it('should parse warning severity', async () => {
      const warnBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-warnparse.cmd' : 'opa-warnparse'
      );
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    violation: [
                      { severity: 'warning', message: 'Warning 1' },
                      { severity: 'warn', message: 'Warning 2' },
                    ],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(warnBinary, outputJson);

      const warnExecutor = new OPAExecutor(warnBinary);
      const result = await warnExecutor.evaluate(mockPolicies, mockInput);

      expect(result.violations[0].severity).toBe('warning');
      expect(result.violations[1].severity).toBe('warning');
    });

    it('should parse info severity', async () => {
      const infoBinary = join(tempDir, platform() === 'win32' ? 'opa-info.cmd' : 'opa-info');
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    violation: [{ severity: 'info', message: 'Info message' }],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(infoBinary, outputJson);

      const infoExecutor = new OPAExecutor(infoBinary);
      const result = await infoExecutor.evaluate(mockPolicies, mockInput);

      expect(result.violations[0].severity).toBe('info');
    });

    it('should use default severity for invalid values', async () => {
      const invalidBinary = join(
        tempDir,
        platform() === 'win32' ? 'opa-invalidsev.cmd' : 'opa-invalidsev'
      );
      const outputJson = JSON.stringify({
        result: [
          {
            expressions: [
              {
                value: {
                  test_policy: {
                    violation: [{ severity: 'invalid', message: 'Test' }],
                  },
                },
              },
            ],
          },
        ],
      });

      writeMockOpa(invalidBinary, outputJson);

      const invalidExecutor = new OPAExecutor(invalidBinary);
      const result = await invalidExecutor.evaluate(mockPolicies, mockInput);

      expect(result.violations[0].severity).toBe('error');
    });
  });

  describe('input handling', () => {
    it('should accept input with git context', async () => {
      const gitInput: OPAInput = {
        ...mockInput,
        context: {
          ...mockInput.context,
          git: {
            branch: 'main',
            commit_sha: 'abc123',
            author: 'Test User',
            files_changed: ['src/test.ts'],
          },
        },
      };

      const result = await executor.evaluate(mockPolicies, gitInput);

      expect(result.success).toBe(true);
    });

    it('should accept input with CI context', async () => {
      const ciInput: OPAInput = {
        ...mockInput,
        context: {
          ...mockInput.context,
          ci: {
            provider: 'github',
            build_id: 'build-123',
            pr_number: '42',
          },
        },
      };

      const result = await executor.evaluate(mockPolicies, ciInput);

      expect(result.success).toBe(true);
    });

    it('should accept input with coverage context', async () => {
      const coverageInput: OPAInput = {
        ...mockInput,
        context: {
          ...mockInput.context,
          coverage: {
            lines: 85,
            functions: 90,
            branches: 75,
            statements: 85,
          },
        },
      };

      const result = await executor.evaluate(mockPolicies, coverageInput);

      expect(result.success).toBe(true);
    });

    it('should accept input with architecture context', async () => {
      const archInput: OPAInput = {
        ...mockInput,
        architecture: {
          layers: {
            ui: ['src/components'],
            business: ['src/services'],
          },
          boundaries: [{ from: 'ui', to: 'business' }],
          summary: {
            total_modules: 10,
            total_violations: 2,
            new_violations: 1,
            circular_count: 0,
            orphan_count: 0,
            layer_violation_count: 2,
            error_count: 2,
            warn_count: 0,
            baseline_loaded: true,
          },
        },
      };

      const result = await executor.evaluate(mockPolicies, archInput);

      expect(result.success).toBe(true);
    });

    it('should accept input with config', async () => {
      const configInput: OPAInput = {
        ...mockInput,
        config: {
          customOption: true,
          threshold: 80,
        },
      };

      const result = await executor.evaluate(mockPolicies, configInput);

      expect(result.success).toBe(true);
    });
  });

  describe('custom query', () => {
    it('should use custom query when specified', async () => {
      const customExecutor = new OPAExecutor(mockBinaryPath, {
        query: 'data.custom.query',
      });

      const result = await customExecutor.evaluate(mockPolicies, mockInput);

      expect(result.success).toBe(true);
    });
  });
});
