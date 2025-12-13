/**
 * OPA Executor - Execute OPA binary and parse results
 */

import { exec } from 'child_process';
import { promisify } from 'util';
import { writeFile, mkdir } from 'fs/promises';
import { existsSync } from 'fs';
import { join } from 'path';
import { tmpdir } from 'os';
import { randomUUID, createHash } from 'crypto';
import type { LoadedPolicy } from './policy-loader.js';

const execAsync = promisify(exec);

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
  /** Architecture context (for architecture policies) */
  architecture?: {
    layers?: Record<string, string[]>;
    boundaries?: Array<{ from: string; to: string }>;
    dependencies?: Record<string, string[]>;
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

/**
 * Default query to evaluate all Anvil policies
 */
const DEFAULT_QUERY = 'data.anvil.policies';

/**
 * Default timeout (30 seconds)
 */
const DEFAULT_TIMEOUT = 30000;

/**
 * Executes OPA binary and parses results
 */
export class OPAExecutor {
  private readonly binaryPath: string;
  private readonly timeout: number;
  private readonly includeRawOutput: boolean;
  private readonly query: string;

  constructor(binaryPath: string, config: OPAExecutorConfig = {}) {
    this.binaryPath = binaryPath;
    this.timeout = config.timeout ?? DEFAULT_TIMEOUT;
    this.includeRawOutput = config.includeRawOutput ?? false;
    this.query = config.query ?? DEFAULT_QUERY;
  }

  /**
   * Evaluate policies against input
   */
  async evaluate(policies: LoadedPolicy[], input: OPAInput): Promise<OPAEvaluationResult> {
    const startTime = Date.now();

    if (policies.length === 0) {
      return {
        success: true,
        violations: [],
        metadata: {
          policy_count: 0,
          execution_time_ms: Date.now() - startTime,
        },
      };
    }

    // Create temporary directory for evaluation
    const tempDir = join(tmpdir(), `anvil-opa-${randomUUID()}`);

    try {
      await this.setupTempDirectory(tempDir, policies, input);
      const result = await this.runOPA(tempDir);
      const violations = this.parseViolations(result, policies);

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
    const tempDir = join(tmpdir(), `anvil-opa-validate-${randomUUID()}`);

    try {
      if (!existsSync(tempDir)) {
        await mkdir(tempDir, { recursive: true });
      }

      const policyPath = join(tempDir, 'policy.rego');
      await writeFile(policyPath, policyContent, 'utf-8');

      const { stderr } = await execAsync(`"${this.binaryPath}" check "${policyPath}"`, {
        timeout: this.timeout,
      });

      // OPA check returns 0 for valid, non-zero for invalid
      return { valid: true, errors: stderr ? [stderr] : [] };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      // Extract meaningful error from OPA output
      const match = errorMessage.match(/error:.*$/m);
      return {
        valid: false,
        errors: match ? [match[0]] : [errorMessage],
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

    const tempDir = join(tmpdir(), `anvil-opa-test-${randomUUID()}`);

    try {
      await this.setupTempDirectory(tempDir, policies, undefined, testFiles);

      const { stdout, stderr } = await execAsync(
        `"${this.binaryPath}" test "${tempDir}" --format json`,
        { timeout: this.timeout }
      );

      const results = JSON.parse(stdout);
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
        errors: stderr ? [stderr] : [],
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
      const { readFile } = await import('fs/promises');
      for (const testFile of testFiles) {
        const content = await readFile(testFile, 'utf-8');
        const testPath = join(tempDir, `${randomUUID()}_test.rego`);
        await writeFile(testPath, content, 'utf-8');
      }
    }

    // Write input if provided
    if (input) {
      const inputPath = join(tempDir, 'input.json');
      await writeFile(inputPath, JSON.stringify(input, null, 2), 'utf-8');
    }
  }

  /**
   * Run OPA evaluation
   */
  private async runOPA(tempDir: string): Promise<unknown> {
    const inputPath = join(tempDir, 'input.json');
    const cmd = `"${this.binaryPath}" eval --data "${tempDir}" --input "${inputPath}" --format json "${this.query}"`;

    const { stdout } = await execAsync(cmd, {
      timeout: this.timeout,
      maxBuffer: 10 * 1024 * 1024, // 10MB buffer for large outputs
    });

    return JSON.parse(stdout);
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
    const content = [
      violation.policy || '',
      violation.rule || '',
      violation.path || '',
      violation.message || '',
    ].join('|');

    return createHash('sha256').update(content).digest('hex').substring(0, 16);
  }

  /**
   * Infer category from policy name
   */
  private inferCategory(policyName: string): ViolationCategory {
    const name = policyName.toLowerCase();

    if (name.includes('security') || name.includes('secret') || name.includes('auth')) {
      return 'security';
    }
    if (name.includes('architecture') || name.includes('layer') || name.includes('boundary')) {
      return 'architecture';
    }
    if (name.includes('coverage') || name.includes('test')) {
      return 'coverage';
    }
    if (name.includes('scope') || name.includes('change') || name.includes('size')) {
      return 'scope';
    }
    if (name.includes('lint') || name.includes('quality') || name.includes('style')) {
      return 'quality';
    }
    if (name.includes('compliance') || name.includes('license') || name.includes('audit')) {
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
      if (existsSync(tempDir)) {
        const { rm } = await import('fs/promises');
        await rm(tempDir, { recursive: true, force: true });
      }
    } catch {
      // Ignore cleanup errors
    }
  }
}
