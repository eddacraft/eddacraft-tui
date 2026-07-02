/**
 * OPA Executor - Execute OPA binary and parse results
 */

import { spawn } from 'node:child_process';
import { writeFile, mkdir, mkdtemp } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir, platform } from 'node:os';
import { createHash } from 'node:crypto';
import type { LoadedPolicy } from './policy-loader.js';
import { createDebugger } from './utils/debug.js';

const debug = createDebugger('policy');

/**
 * OPA evaluation input structure
 */
export interface OPAInput {
  /** The APS plan being evaluated */
  plan: {
    id: string;
    hash: string;
    intent: string;
    schema_version: string;
    proposed_changes: Array<{
      type: string;
      path: string;
      description?: string;
      metadata?: Record<string, unknown>;
      extension?: string;
      directory?: string;
    }>;
    provenance?: {
      author?: string;
      source?: string;
      repository?: string;
      branch?: string;
    };
    validations?: {
      required_checks?: string[];
      skip_checks?: string[];
    };
    tags?: string[];
    change_count?: number;
    affected_directories?: string[];
  };
  /** Execution context */
  context: {
    workspace_root: string;
    timestamp: number;
    /** Test coverage data if available */
    coverage?: {
      lines?: number;
      functions?: number;
      branches?: number;
      statements?: number;
    };
    /** Git context for repository-aware policies */
    git?: {
      branch?: string;
      base_branch?: string;
      commit_sha?: string;
      author?: string;
      author_email?: string;
      commit_count?: number;
      files_changed?: string[];
      lines_added?: number;
      lines_removed?: number;
    };
    /** CI/CD context for pipeline-aware policies */
    ci?: {
      provider?: 'github' | 'gitlab' | 'jenkins' | 'azure' | 'local';
      build_id?: string;
      pr_number?: string;
      pr_author?: string;
      pr_reviewers?: string[];
      labels?: string[];
    };
  };
  /** Architecture context from ArchitectureCheck (for architecture-aware policies) */
  architecture?: {
    layers?: Record<string, string[]>;
    boundaries?: Array<{ from: string; to: string }>;
    dependencies?: Record<string, string[]>;
    summary?: {
      total_modules: number;
      total_violations: number;
      new_violations: number;
      circular_count: number;
      orphan_count: number;
      layer_violation_count: number;
      error_count: number;
      warn_count: number;
      baseline_loaded: boolean;
    };
    violations?: Array<{
      from: string;
      to: string;
      rule: string;
      severity: string;
      is_circular: boolean;
      is_new: boolean;
      from_layer: string | null;
      to_layer: string | null;
    }>;
  };
  /** Policy-specific configuration */
  config?: Record<string, unknown>;
}

/**
 * Category of policy violation for grouping and filtering
 */
export type ViolationCategory =
  | 'security'
  | 'architecture'
  | 'coverage'
  | 'scope'
  | 'quality'
  | 'compliance'
  | 'custom';

/**
 * A single policy violation
 */
export interface PolicyViolation {
  /** Rule that was violated */
  rule: string;
  /** Severity level */
  severity: 'error' | 'warning' | 'info';
  /** Human-readable message */
  message: string;
  /** File path if applicable */
  path?: string;
  /** Policy that generated this violation */
  policy?: string;
  /** Category for grouping violations */
  category?: ViolationCategory;
  /** Stable fingerprint for deduplication across runs (hash of policy+rule+path+message) */
  fingerprint?: string;
  /** URL to documentation about this violation and how to fix it */
  documentation_url?: string;
}

/**
 * Result of OPA evaluation
 */
export interface OPAEvaluationResult {
  /** Whether evaluation succeeded */
  success: boolean;
  /** Policy violations found */
  violations: PolicyViolation[];
  /** Execution metadata */
  metadata: {
    policy_count: number;
    execution_time_ms: number;
    opa_version?: string;
  };
  /** Raw OPA output (for debugging) */
  raw_output?: unknown;
  /** Error message if evaluation failed */
  error?: string;
}

/**
 * Configuration for OPA executor
 */
export interface OPAExecutorConfig {
  /** Timeout for OPA execution in milliseconds */
  timeout?: number;
  /** Whether to include raw output in results */
  includeRawOutput?: boolean;
  /** Custom query (default: data.anvil.policies) */
  query?: string;
}

const DEFAULT_QUERY = 'data.anvil.policies';
const DEFAULT_TIMEOUT = 30000;

/**
 * Network-capable and runtime-sensitive OPA built-ins removed from the
 * capabilities profile used for every evaluation (CIB-108).
 *
 * Workspace policies are untrusted input; these built-ins would let a policy
 * make outbound requests (`http.send`), resolve DNS (`net.lookup_ip_addr`),
 * or read the daemon's process environment (`opa.runtime`) from developer and
 * CI machines.
 */
export const OPA_DENIED_BUILTINS: ReadonlySet<string> = new Set([
  'http.send',
  'net.lookup_ip_addr',
  'opa.runtime',
]);

/**
 * Error prefix used when the restricted capabilities profile cannot be
 * derived. Evaluation fails closed rather than running unrestricted.
 */
const CAPABILITIES_FAILURE =
  'Failed to derive a restricted OPA capabilities profile; ' +
  'refusing to evaluate policies without built-in restrictions';

interface SpawnResult {
  stdout: string;
  stderr: string;
  code: number | null;
}

function spawnAsync(
  command: string,
  args: string[],
  options: { timeout?: number; maxBuffer?: number; cwd?: string }
): Promise<SpawnResult> {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd: options.cwd,
      stdio: ['pipe', 'pipe', 'pipe'],
      shell: platform() === 'win32',
    });

    let stdout = '';
    let stderr = '';
    let killed = false;

    const timer = options.timeout
      ? setTimeout(() => {
          killed = true;
          child.kill('SIGTERM');
          reject(new Error(`Command timed out after ${options.timeout}ms`));
        }, options.timeout)
      : null;

    child.stdout.on('data', (data: Buffer) => {
      stdout += data.toString('utf8');
      if (options.maxBuffer && stdout.length > options.maxBuffer) {
        child.kill('SIGTERM');
        reject(new Error('stdout maxBuffer exceeded'));
      }
    });

    child.stderr.on('data', (data: Buffer) => {
      stderr += data.toString('utf8');
      if (options.maxBuffer && stderr.length > options.maxBuffer) {
        child.kill('SIGTERM');
        reject(new Error('stderr maxBuffer exceeded'));
      }
    });

    child.on('error', (error: Error) => {
      if (timer) clearTimeout(timer);
      reject(error);
    });

    child.on('close', (code: number | null) => {
      if (timer) clearTimeout(timer);
      if (killed) return;
      resolve({ stdout, stderr, code });
    });
  });
}

/**
 * Executes OPA binary and parses results
 */
export class OPAExecutor {
  private readonly binaryPath: string;
  private readonly timeout: number;
  private readonly includeRawOutput: boolean;
  private readonly query: string;
  /** Memoised restricted capabilities JSON, derived once per executor. */
  private restrictedCapabilities?: Promise<string>;

  constructor(binaryPath: string, config: OPAExecutorConfig = {}) {
    this.binaryPath = binaryPath;
    this.timeout = config.timeout ?? DEFAULT_TIMEOUT;
    this.includeRawOutput = config.includeRawOutput ?? false;
    this.query = config.query ?? DEFAULT_QUERY;
    debug('OPAExecutor created', { binaryPath, timeout: this.timeout, query: this.query });
  }

  /**
   * Evaluate policies against input
   */
  async evaluate(policies: LoadedPolicy[], input: OPAInput): Promise<OPAEvaluationResult> {
    const startTime = Date.now();
    debug('evaluating policies', { policyCount: policies.length, planId: input.plan.id });

    if (policies.length === 0) {
      debug('no policies to evaluate, returning empty result');
      return {
        success: true,
        violations: [],
        metadata: {
          policy_count: 0,
          execution_time_ms: Date.now() - startTime,
        },
      };
    }

    // Create temporary directory for evaluation (using mkdtemp for secure unique dir)
    const tempDir = await mkdtemp(join(tmpdir(), 'anvil-opa-'));

    try {
      await this.setupTempDirectory(tempDir, policies, input);
      const result = await this.runOPA(tempDir);
      const violations = this.parseViolations(result, policies);

      debug('evaluation complete', {
        violationCount: violations.length,
        elapsed: Date.now() - startTime,
      });

      return {
        success: true,
        violations,
        metadata: {
          policy_count: policies.length,
          execution_time_ms: Date.now() - startTime,
        },
        raw_output: this.includeRawOutput ? result : undefined,
      };
    } catch (error) {
      debug('evaluation failed', error instanceof Error ? error : undefined);
      return {
        success: false,
        violations: [],
        metadata: {
          policy_count: policies.length,
          execution_time_ms: Date.now() - startTime,
        },
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    } finally {
      // Clean up temp directory
      await this.cleanupTempDirectory(tempDir);
    }
  }

  /**
   * Validate Rego syntax for a policy
   */
  async validateSyntax(policyContent: string): Promise<{
    valid: boolean;
    errors: string[];
  }> {
    const tempDir = await mkdtemp(join(tmpdir(), 'anvil-opa-validate-'));

    try {
      const policyPath = join(tempDir, 'policy.rego');
      await writeFile(policyPath, policyContent, 'utf-8');

      const result = await spawnAsync(this.binaryPath, ['check', policyPath], {
        timeout: this.timeout,
      });

      if (result.code === 0) {
        return { valid: true, errors: result.stderr ? [result.stderr] : [] };
      }

      const match = result.stderr.match(/error:.*$/m);
      return {
        valid: false,
        errors: match ? [match[0]] : [result.stderr || `OPA check failed with code ${result.code}`],
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      return {
        valid: false,
        errors: [errorMessage],
      };
    } finally {
      await this.cleanupTempDirectory(tempDir);
    }
  }

  /**
   * Run OPA tests for policies
   */
  async runTests(
    policies: LoadedPolicy[],
    testFiles: string[]
  ): Promise<{
    passed: number;
    failed: number;
    errors: string[];
    details: Array<{ name: string; passed: boolean; message?: string }>;
  }> {
    if (testFiles.length === 0) {
      return { passed: 0, failed: 0, errors: [], details: [] };
    }

    const tempDir = await mkdtemp(join(tmpdir(), 'anvil-opa-test-'));

    try {
      await this.setupTempDirectory(tempDir, policies, undefined, testFiles);
      const capabilitiesPath = await this.writeCapabilitiesFile(tempDir);

      const spawnResult = await spawnAsync(
        this.binaryPath,
        ['test', tempDir, '--capabilities', capabilitiesPath, '--format', 'json'],
        {
          timeout: this.timeout,
        }
      );

      const results = JSON.parse(spawnResult.stdout);
      const details: Array<{ name: string; passed: boolean; message?: string }> = [];
      let passed = 0;
      let failed = 0;

      for (const result of results) {
        const isPassed = result.fail !== true;
        details.push({
          name: result.name || 'unknown',
          passed: isPassed,
          message: result.fail ? result.error?.message : undefined,
        });
        if (isPassed) {
          passed++;
        } else {
          failed++;
        }
      }

      return {
        passed,
        failed,
        errors: spawnResult.stderr ? [spawnResult.stderr] : [],
        details,
      };
    } catch (error) {
      return {
        passed: 0,
        failed: 0,
        errors: [error instanceof Error ? error.message : 'Unknown error'],
        details: [],
      };
    } finally {
      await this.cleanupTempDirectory(tempDir);
    }
  }

  /**
   * Set up temporary directory with policies and input
   */
  private async setupTempDirectory(
    tempDir: string,
    policies: LoadedPolicy[],
    input?: OPAInput,
    testFiles?: string[]
  ): Promise<void> {
    if (!existsSync(tempDir)) {
      await mkdir(tempDir, { recursive: true });
    }

    // Write policies
    for (const policy of policies) {
      const policyPath = join(tempDir, `${policy.name}.rego`);
      await writeFile(policyPath, policy.content, 'utf-8');
    }

    // Write test files if provided
    if (testFiles) {
      const { readFile } = await import('node:fs/promises');
      for (let i = 0; i < testFiles.length; i++) {
        const content = await readFile(testFiles[i], 'utf-8');
        const testPath = join(tempDir, `_anvil_test_${i}.rego`);
        await writeFile(testPath, content, 'utf-8');
      }
    }

    // Write input if provided
    if (input) {
      const inputPath = join(tempDir, 'input.json');
      await writeFile(inputPath, JSON.stringify(input, null, 2), 'utf-8');
    }
  }

  private async runOPA(tempDir: string): Promise<unknown> {
    const inputPath = join(tempDir, 'input.json');
    const capabilitiesPath = await this.writeCapabilitiesFile(tempDir);

    const result = await spawnAsync(
      this.binaryPath,
      [
        'eval',
        '--data',
        tempDir,
        '--input',
        inputPath,
        '--capabilities',
        capabilitiesPath,
        '--format',
        'json',
        this.query,
      ],
      {
        timeout: this.timeout,
        maxBuffer: 10 * 1024 * 1024,
      }
    );

    if (result.code !== 0) {
      throw new Error(this.describeEvalFailure(result));
    }

    return JSON.parse(result.stdout);
  }

  /**
   * Derive the restricted capabilities profile from the configured binary
   * (`opa capabilities --current`), so the profile always matches the
   * installed OPA version, with the denied built-ins removed and `allow_net`
   * emptied as defence in depth. Fails closed: any derivation failure aborts
   * evaluation instead of falling back to unrestricted built-ins.
   */
  private getRestrictedCapabilities(): Promise<string> {
    this.restrictedCapabilities ??= this.deriveRestrictedCapabilities();
    return this.restrictedCapabilities;
  }

  private async deriveRestrictedCapabilities(): Promise<string> {
    let result: SpawnResult;
    try {
      result = await spawnAsync(this.binaryPath, ['capabilities', '--current'], {
        timeout: this.timeout,
        maxBuffer: 16 * 1024 * 1024,
      });
    } catch (error) {
      const detail = error instanceof Error ? error.message : 'unknown error';
      throw new Error(`${CAPABILITIES_FAILURE}: ${detail}`, { cause: error });
    }

    if (result.code !== 0) {
      const detail = result.stderr.trim() || `opa capabilities exited with code ${result.code}`;
      throw new Error(`${CAPABILITIES_FAILURE}: ${detail}`);
    }

    let parsed: unknown;
    try {
      parsed = JSON.parse(result.stdout);
    } catch (error) {
      throw new Error(`${CAPABILITIES_FAILURE}: opa capabilities returned invalid JSON`, {
        cause: error,
      });
    }

    if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
      throw new Error(`${CAPABILITIES_FAILURE}: unexpected opa capabilities output`);
    }

    const capabilities = parsed as { builtins?: unknown; allow_net?: unknown };
    if (!Array.isArray(capabilities.builtins)) {
      throw new Error(`${CAPABILITIES_FAILURE}: opa capabilities output has no builtins list`);
    }

    capabilities.builtins = capabilities.builtins.filter(
      (builtin) => !OPA_DENIED_BUILTINS.has(String((builtin as { name?: unknown })?.name))
    );
    // Defence in depth: even if a network-capable built-in slipped through,
    // an empty allowlist denies every host.
    capabilities.allow_net = [];

    return JSON.stringify(capabilities);
  }

  /**
   * Write the restricted capabilities profile next to (not inside) the temp
   * directory: `--data <tempDir>` and `opa test <tempDir>` load every JSON
   * file in the directory into the data document.
   */
  private capabilitiesPathFor(tempDir: string): string {
    return `${tempDir}-capabilities.json`;
  }

  private async writeCapabilitiesFile(tempDir: string): Promise<string> {
    const capabilities = await this.getRestrictedCapabilities();
    const capabilitiesPath = this.capabilitiesPathFor(tempDir);
    await writeFile(capabilitiesPath, capabilities, 'utf-8');
    return capabilitiesPath;
  }

  /**
   * Map an eval failure to a message; a policy that requires a denied
   * built-in gets an explicit "not permitted" explanation instead of a bare
   * compiler error.
   */
  private describeEvalFailure(result: SpawnResult): string {
    const detail = result.stderr || `OPA eval failed with code ${result.code}`;
    const denied = [...OPA_DENIED_BUILTINS].find(
      (name) =>
        detail.includes(`undefined function ${name}`) ||
        result.stdout.includes(`undefined function ${name}`)
    );
    if (denied) {
      return (
        `Policy requires the OPA built-in "${denied}", which is not permitted: ` +
        `network-capable and runtime-sensitive built-ins are disabled during ` +
        `policy evaluation (CIB-108). ${detail}`
      );
    }
    return detail;
  }

  /**
   * Parse OPA output into violations
   */
  private parseViolations(opaOutput: unknown, _policies: LoadedPolicy[]): PolicyViolation[] {
    const violations: PolicyViolation[] = [];

    // OPA eval output format:
    // { result: [{ expressions: [{ value: { policy_name: { violation: [...] } } }] }] }

    try {
      const output = opaOutput as {
        result?: Array<{
          expressions?: Array<{
            value?: Record<string, unknown>;
          }>;
        }>;
      };

      const expressions = output?.result?.[0]?.expressions;
      if (!expressions || expressions.length === 0) {
        return violations;
      }

      const value = expressions[0]?.value;
      if (!value || typeof value !== 'object') {
        return violations;
      }

      // Iterate through each policy's results
      for (const [policyName, policyResult] of Object.entries(value)) {
        if (typeof policyResult !== 'object' || policyResult === null) {
          continue;
        }

        const result = policyResult as Record<string, unknown>;

        // Look for violation/violations array
        const violationArray = (result.violation || result.violations) as unknown[];
        if (Array.isArray(violationArray)) {
          for (const v of violationArray) {
            const violation = this.normaliseViolation(v, policyName);
            if (violation) {
              violations.push(violation);
            }
          }
        }

        // Also check for deny/denies (common OPA convention)
        const denyArray = (result.deny || result.denies) as unknown[];
        if (Array.isArray(denyArray)) {
          for (const d of denyArray) {
            const violation = this.normaliseViolation(d, policyName, 'error');
            if (violation) {
              violations.push(violation);
            }
          }
        }

        // Check for warn/warnings
        const warnArray = (result.warn || result.warnings) as unknown[];
        if (Array.isArray(warnArray)) {
          for (const w of warnArray) {
            const violation = this.normaliseViolation(w, policyName, 'warning');
            if (violation) {
              violations.push(violation);
            }
          }
        }
      }
    } catch {
      // If parsing fails, return empty violations
      // The raw output can be inspected if includeRawOutput is true
    }

    return violations;
  }

  /**
   * Normalise a violation from various formats
   */
  private normaliseViolation(
    violation: unknown,
    policyName: string,
    defaultSeverity: PolicyViolation['severity'] = 'error'
  ): PolicyViolation | null {
    // String violation (simple message)
    if (typeof violation === 'string') {
      const result: PolicyViolation = {
        rule: policyName,
        severity: defaultSeverity,
        message: violation,
        policy: policyName,
        category: this.inferCategory(policyName),
      };
      result.fingerprint = this.generateFingerprint(result);
      return result;
    }

    // Object violation (structured)
    if (typeof violation === 'object' && violation !== null) {
      const v = violation as Record<string, unknown>;
      const result: PolicyViolation = {
        rule: (v.rule as string) || policyName,
        severity: this.parseSeverity(v.severity, defaultSeverity),
        message: (v.message as string) || (v.msg as string) || JSON.stringify(v),
        path: v.path as string | undefined,
        policy: policyName,
        category: this.parseCategory(v.category) || this.inferCategory(policyName),
        documentation_url: v.documentation_url as string | undefined,
      };
      result.fingerprint = this.generateFingerprint(result);
      return result;
    }

    return null;
  }

  /**
   * Generate a stable fingerprint for deduplication
   */
  private generateFingerprint(violation: PolicyViolation): string {
    const content = JSON.stringify({
      policy: violation.policy || '',
      rule: violation.rule || '',
      path: violation.path || '',
      message: violation.message || '',
    });

    return createHash('sha256').update(content).digest('hex').substring(0, 16);
  }

  /**
   * Infer category from policy name
   */
  private inferCategory(policyName: string): ViolationCategory {
    const name = policyName.toLowerCase();
    const hasWord = (word: string) => new RegExp(`(^|_|-)${word}($|_|-)`).test(name);

    if (hasWord('security') || hasWord('secret') || hasWord('auth')) {
      return 'security';
    }
    if (hasWord('architecture') || hasWord('layer') || hasWord('boundary')) {
      return 'architecture';
    }
    if (hasWord('coverage') || hasWord('test')) {
      return 'coverage';
    }
    if (hasWord('scope') || hasWord('change') || hasWord('size')) {
      return 'scope';
    }
    if (hasWord('lint') || hasWord('quality') || hasWord('style')) {
      return 'quality';
    }
    if (hasWord('compliance') || hasWord('license') || hasWord('audit')) {
      return 'compliance';
    }

    return 'custom';
  }

  /**
   * Parse category from violation object
   */
  private parseCategory(value: unknown): ViolationCategory | undefined {
    if (typeof value !== 'string') return undefined;

    const validCategories: ViolationCategory[] = [
      'security',
      'architecture',
      'coverage',
      'scope',
      'quality',
      'compliance',
      'custom',
    ];

    const lower = value.toLowerCase() as ViolationCategory;
    return validCategories.includes(lower) ? lower : undefined;
  }

  /**
   * Parse severity from various formats
   */
  private parseSeverity(
    severity: unknown,
    defaultValue: PolicyViolation['severity']
  ): PolicyViolation['severity'] {
    if (typeof severity !== 'string') {
      return defaultValue;
    }

    const lower = severity.toLowerCase();
    if (lower === 'error' || lower === 'err') return 'error';
    if (lower === 'warning' || lower === 'warn') return 'warning';
    if (lower === 'info') return 'info';

    return defaultValue;
  }

  /**
   * Clean up temporary directory
   */
  private async cleanupTempDirectory(tempDir: string): Promise<void> {
    try {
      const { rm } = await import('node:fs/promises');
      if (existsSync(tempDir)) {
        await rm(tempDir, { recursive: true, force: true });
      }
      await rm(this.capabilitiesPathFor(tempDir), { force: true });
    } catch {
      // Ignore cleanup errors
    }
  }
}
