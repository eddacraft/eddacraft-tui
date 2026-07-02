/**
 * Tests for CIB-108: restrict network-capable OPA built-ins during policy
 * evaluation.
 *
 * Untrusted workspace Rego policies must not be able to call `http.send`,
 * `net.lookup_ip_addr`, or `opa.runtime` (outbound network access and
 * process-environment disclosure). The executor derives a restricted
 * capabilities profile from the installed binary (`opa capabilities
 * --current`), removes the denied built-ins, and passes `--capabilities` on
 * every `opa eval` / `opa test` invocation. If the profile cannot be derived,
 * evaluation fails closed.
 *
 * Two layers:
 *   - mock-binary unit tests (always run) prove the flag is passed, the
 *     denied built-ins are filtered from the written profile, and derivation
 *     failure fails closed;
 *   - real-binary integration tests (skipped when `opa` is not on PATH and
 *     `ANVIL_OPA_PATH` is unset, mirroring opa-real.integration.test.ts)
 *     prove a policy using a denied built-in gets a deterministic error
 *     instead of making an outbound request.
 */

import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { OPAExecutor, OPA_DENIED_BUILTINS, type OPAInput } from './opa-executor.js';
import { type LoadedPolicy } from './policy-loader.js';
import { mkdtempSync, writeFileSync, chmodSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { tmpdir, platform } from 'node:os';
import { safeCleanup } from '../../../../tools/test-utils/safe-cleanup.js';

/** Capabilities document served by mocks — includes the denied built-ins so
 * the executor has something to filter out. */
const FULL_CAPABILITIES = JSON.stringify({
  builtins: [
    { name: 'eq' },
    { name: 'count' },
    { name: 'http.send' },
    { name: 'net.lookup_ip_addr' },
    { name: 'opa.runtime' },
  ],
});

const EVAL_JSON = JSON.stringify({
  result: [
    {
      expressions: [
        {
          value: {
            test_policy: {
              violation: ['capability-checked violation'],
            },
          },
        },
      ],
    },
  ],
});

const TEST_JSON = JSON.stringify([{ name: 'test_passes', fail: false }]);

/**
 * Mock `opa` that answers `capabilities --current`, then REQUIRES a
 * `--capabilities <file>` argument on eval/test calls and fails if any denied
 * built-in survived filtering (or if `allow_net` was not emptied). This makes
 * the tests red when the executor does not enforce the restriction.
 */
function enforcingScript(): string {
  if (platform() === 'win32') {
    // Positional args: eval --data <dir> --input <file> --capabilities <file> ...
    //                  test <dir> --capabilities <file> ...
    return [
      '@echo off',
      'if "%1"=="capabilities" goto caps',
      'if "%1"=="test" goto testcmd',
      'if not "%6"=="--capabilities" goto fail',
      'findstr /C:"http.send" "%7" >nul',
      'if not errorlevel 1 goto fail',
      `echo ${EVAL_JSON}`,
      'exit /b 0',
      ':testcmd',
      'if not "%3"=="--capabilities" goto fail',
      'findstr /C:"http.send" "%4" >nul',
      'if not errorlevel 1 goto fail',
      `echo ${TEST_JSON}`,
      'exit /b 0',
      ':caps',
      `echo ${FULL_CAPABILITIES}`,
      'exit /b 0',
      ':fail',
      'echo capabilities restriction not enforced 1>&2',
      'exit /b 1',
      '',
    ].join('\r\n');
  }
  return [
    '#!/bin/sh',
    'if [ "$1" = "capabilities" ]; then',
    `  echo '${FULL_CAPABILITIES}'`,
    '  exit 0',
    'fi',
    'caps=""',
    'prev=""',
    'for arg in "$@"; do',
    '  if [ "$prev" = "--capabilities" ]; then caps="$arg"; fi',
    '  prev="$arg"',
    'done',
    'if [ -z "$caps" ] || [ ! -f "$caps" ]; then',
    '  echo "invoked without --capabilities" >&2',
    '  exit 1',
    'fi',
    'for denied in http.send net.lookup_ip_addr opa.runtime; do',
    '  if grep -q "\\"$denied\\"" "$caps"; then',
    '    echo "denied built-in $denied still present in capabilities" >&2',
    '    exit 1',
    '  fi',
    'done',
    'if ! grep -q \'"allow_net":\\[\\]\' "$caps"; then',
    '  echo "allow_net not emptied in capabilities" >&2',
    '  exit 1',
    'fi',
    'if [ "$1" = "test" ]; then',
    `  echo '${TEST_JSON}'`,
    'else',
    `  echo '${EVAL_JSON}'`,
    'fi',
    '',
  ].join('\n');
}

/** Mock `opa` whose `capabilities` subcommand fails but whose eval succeeds —
 * evaluation must fail closed rather than run unrestricted. */
function brokenCapabilitiesScript(): string {
  if (platform() === 'win32') {
    return [
      '@echo off',
      'if "%1"=="capabilities" goto caps',
      `echo ${EVAL_JSON}`,
      'exit /b 0',
      ':caps',
      'echo capabilities subcommand unsupported 1>&2',
      'exit /b 1',
      '',
    ].join('\r\n');
  }
  return [
    '#!/bin/sh',
    'if [ "$1" = "capabilities" ]; then',
    '  echo "capabilities subcommand unsupported" >&2',
    '  exit 1',
    'fi',
    `echo '${EVAL_JSON}'`,
    '',
  ].join('\n');
}

/** Mock `opa` that reports a compile error for a denied built-in, mirroring
 * real `opa eval --capabilities` output. */
function deniedBuiltinErrorScript(): string {
  const stderrLine =
    '1 error occurred: policy.rego:4: rego_type_error: undefined function http.send';
  if (platform() === 'win32') {
    return [
      '@echo off',
      'if "%1"=="capabilities" goto caps',
      `echo ${stderrLine} 1>&2`,
      'exit /b 2',
      ':caps',
      `echo ${FULL_CAPABILITIES}`,
      'exit /b 0',
      '',
    ].join('\r\n');
  }
  return [
    '#!/bin/sh',
    'if [ "$1" = "capabilities" ]; then',
    `  echo '${FULL_CAPABILITIES}'`,
    '  exit 0',
    'fi',
    `echo '${stderrLine}' >&2`,
    'exit 2',
    '',
  ].join('\n');
}

function writeScript(path: string, content: string): void {
  writeFileSync(path, content);
  if (platform() !== 'win32') {
    chmodSync(path, 0o755);
  }
}

function baseInput(workspaceRoot: string): OPAInput {
  return {
    plan: {
      id: 'plan-cib-108',
      hash: 'h',
      intent: 'capabilities restriction test',
      schema_version: '0.1.0',
      proposed_changes: [{ type: 'file_create', path: 'src/a.ts' }],
    },
    context: {
      workspace_root: workspaceRoot,
      timestamp: 0,
    },
  };
}

function policy(name: string, content: string): LoadedPolicy {
  return {
    name,
    path: `${name}.rego`,
    content,
    package: `anvil.policies.${name}`,
    hasTests: false,
  };
}

describe('OPAExecutor capabilities restriction (CIB-108)', () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = mkdtempSync(join(tmpdir(), 'anvil-opa-caps-test-'));
  });

  afterEach(async () => {
    await safeCleanup(tempDir);
  });

  it('denies exactly the network-capable and runtime-sensitive built-ins', () => {
    expect([...OPA_DENIED_BUILTINS].sort()).toEqual([
      'http.send',
      'net.lookup_ip_addr',
      'opa.runtime',
    ]);
  });

  it('passes a restricted capabilities profile to opa eval', async () => {
    const binary = join(tempDir, platform() === 'win32' ? 'opa.cmd' : 'opa');
    writeScript(binary, enforcingScript());

    const executor = new OPAExecutor(binary, { timeout: 5000 });
    const result = await executor.evaluate(
      [policy('test_policy', 'package anvil.policies.test_policy')],
      baseInput(tempDir)
    );

    expect(result.error).toBeUndefined();
    expect(result.success).toBe(true);
    expect(result.violations).toHaveLength(1);
    expect(result.violations[0].message).toBe('capability-checked violation');
  });

  it('passes a restricted capabilities profile to opa test', async () => {
    const binary = join(tempDir, platform() === 'win32' ? 'opa.cmd' : 'opa');
    writeScript(binary, enforcingScript());

    const testFile = join(tempDir, 'policy_test.rego');
    writeFileSync(testFile, 'package test\n\ntest_passes { true }\n');

    const executor = new OPAExecutor(binary, { timeout: 5000 });
    const result = await executor.runTests(
      [policy('test_policy', 'package anvil.policies.test_policy')],
      [testFile]
    );

    expect(result.errors).toEqual([]);
    expect(result.passed).toBe(1);
    expect(result.failed).toBe(0);
  });

  it('fails closed when the capabilities profile cannot be derived', async () => {
    const binary = join(tempDir, platform() === 'win32' ? 'opa.cmd' : 'opa');
    writeScript(binary, brokenCapabilitiesScript());

    const executor = new OPAExecutor(binary, { timeout: 5000 });
    const result = await executor.evaluate(
      [policy('test_policy', 'package anvil.policies.test_policy')],
      baseInput(tempDir)
    );

    expect(result.success).toBe(false);
    expect(result.error).toMatch(/capabilities/i);
    expect(result.error).toMatch(/refusing to evaluate/i);
  });

  it('reports a clear error when a policy requires a denied built-in', async () => {
    const binary = join(tempDir, platform() === 'win32' ? 'opa.cmd' : 'opa');
    writeScript(binary, deniedBuiltinErrorScript());

    const executor = new OPAExecutor(binary, { timeout: 5000 });
    const result = await executor.evaluate(
      [policy('test_policy', 'package anvil.policies.test_policy')],
      baseInput(tempDir)
    );

    expect(result.success).toBe(false);
    expect(result.error).toContain('http.send');
    expect(result.error).toMatch(/not permitted/);
  });
});

// ---------------------------------------------------------------------------
// Real-binary integration — same discovery/skip pattern as
// opa-real.integration.test.ts.
// ---------------------------------------------------------------------------

function findOpaBinary(): string | null {
  const envPath = process.env.ANVIL_OPA_PATH;
  if (envPath) return envPath;
  const lookup = process.platform === 'win32' ? 'where' : 'which';
  const result = spawnSync(lookup, ['opa'], { encoding: 'utf-8' });
  if (result.status === 0) {
    return result.stdout.trim().split(/\r?\n/)[0] ?? null;
  }
  return null;
}

const opaPath = findOpaBinary();

// `import rego.v1` keeps the fixtures valid on both the 0.x line (>= 0.59)
// and the 1.x line pinned in CI (DEFAULT_OPA_VERSION).
const HTTP_SEND_POLICY = `package anvil.policies.exfil

import rego.v1

violation contains msg if {
  resp := http.send({"method": "get", "url": "http://127.0.0.1:9/exfil"})
  msg := sprintf("leaked %v", [resp.status_code])
}
`;

const NET_LOOKUP_POLICY = `package anvil.policies.dns_probe

import rego.v1

violation contains msg if {
  addrs := net.lookup_ip_addr("example.com")
  msg := sprintf("resolved %v", [addrs])
}
`;

const OPA_RUNTIME_POLICY = `package anvil.policies.env_leak

import rego.v1

violation contains msg if {
  rt := opa.runtime()
  msg := sprintf("env %v", [rt.env])
}
`;

const BENIGN_POLICY = `package anvil.policies.change_gate

import rego.v1

violation contains msg if {
  count(input.plan.proposed_changes) > 0
  msg := "plan proposes changes"
}
`;

describe.skipIf(!opaPath)('OPAExecutor capabilities restriction — real opa binary (CIB-108)', () => {
  const deniedCases: Array<{ builtin: string; name: string; content: string }> = [
    { builtin: 'http.send', name: 'exfil', content: HTTP_SEND_POLICY },
    { builtin: 'net.lookup_ip_addr', name: 'dns_probe', content: NET_LOOKUP_POLICY },
    { builtin: 'opa.runtime', name: 'env_leak', content: OPA_RUNTIME_POLICY },
  ];

  it.each(deniedCases)(
    'rejects a policy using $builtin instead of executing it',
    async ({ builtin, name, content }) => {
      const executor = new OPAExecutor(opaPath as string, { timeout: 15_000 });
      const result = await executor.evaluate([policy(name, content)], baseInput('/tmp'));

      expect(result.success).toBe(false);
      expect(result.error).toContain(builtin);
      expect(result.error).toMatch(/not permitted/);
      expect(result.violations).toEqual([]);
    },
    30_000
  );

  it(
    'still evaluates a policy that uses only permitted built-ins',
    async () => {
      const executor = new OPAExecutor(opaPath as string, { timeout: 15_000 });
      const result = await executor.evaluate(
        [policy('change_gate', BENIGN_POLICY)],
        baseInput('/tmp')
      );

      expect(result.error).toBeUndefined();
      expect(result.success).toBe(true);
      expect(result.violations).toHaveLength(1);
      expect(result.violations[0].message).toBe('plan proposes changes');
      expect(result.violations[0].policy).toBe('change_gate');
    },
    30_000
  );

  it(
    'refuses opa test runs that use a denied built-in',
    async () => {
      const executor = new OPAExecutor(opaPath as string, { timeout: 15_000 });
      const testDir = mkdtempSync(join(tmpdir(), 'anvil-opa-caps-real-'));
      const testFile = join(testDir, 'exfil_test.rego');
      writeFileSync(
        testFile,
        `package anvil.policies.exfil_test

import rego.v1

test_exfil if {
  resp := http.send({"method": "get", "url": "http://127.0.0.1:9/exfil"})
  resp.status_code == 200
}
`
      );

      try {
        const result = await executor.runTests(
          [policy('change_gate', BENIGN_POLICY)],
          [testFile]
        );

        expect(result.passed).toBe(0);
        expect(result.errors.length).toBeGreaterThan(0);
      } finally {
        await safeCleanup(testDir);
      }
    },
    30_000
  );
});
