/**
 * Policy Check - Evaluate OPA/Rego policies against plans
 */

import { BaseCheck } from '../check.interface.js';
import { CheckContext, GateResult } from '../../types/gate.types.js';
import { getOPABinaryManager } from '../policy/opa-binary-manager.js';
import { PolicyLoader, type LoadedPolicy } from '../policy/policy-loader.js';
import { OPAExecutor, type OPAInput, type PolicyViolation } from '../policy/opa-executor.js';

/**
 * Configuration for policy check
 */
export interface PolicyCheckConfig {
  /** Policy directory relative to workspace root */
  policy_dir?: string;
  /** Minimum severity to fail the check */
  severity_threshold?: 'error' | 'warning' | 'info';
  /** Policies to enable (if empty, all are enabled) */
  enabled_policies?: string[];
  /** Policies to disable */
  disabled_policies?: string[];
  /** Custom query (default: data.anvil.policies) */
  query?: string;
  /** Timeout in milliseconds */
  timeout?: number;
}

/**
 * Default policy directory
 */
const DEFAULT_POLICY_DIR = '.anvil/policies';

/**
 * Score penalties per severity
 */
const SEVERITY_PENALTIES = {
  error: 20,
  warning: 5,
  info: 1,
};

/**
 * Policy check that evaluates OPA/Rego policies against plans
 */
export class PolicyCheck extends BaseCheck {
  name = 'policy';
  description = 'Evaluate OPA/Rego policies against plans';

  private policyLoader: PolicyLoader;

  constructor() {
    super();
    this.policyLoader = new PolicyLoader();
  }

  async run(context: CheckContext): Promise<GateResult> {
    const config = this.parseConfig(context.check_config);

    try {
      // Step 1: Ensure OPA binary is available
      const binaryManager = getOPABinaryManager();
      let binaryPath: string;

      try {
        binaryPath = await binaryManager.ensureBinary();
      } catch (error) {
        return this.createFailure(
          'OPA binary not available',
          error instanceof Error ? error.message : 'Failed to download OPA'
        );
      }

      // Step 2: Load policies
      const policyDir = config.policy_dir || DEFAULT_POLICY_DIR;
      const discoveryResult = await this.policyLoader.loadPolicies(context.workspace_root, {
        policyDir,
        enabledPolicies: config.enabled_policies,
        disabledPolicies: config.disabled_policies,
      });

      // Check for policy loading errors
      if (discoveryResult.errors.length > 0) {
        const errorMessages = discoveryResult.errors.map((e) => `${e.path}: ${e.error}`).join('; ');
        return this.createFailure(`Failed to load some policies: ${errorMessages}`, undefined, {
          loadErrors: discoveryResult.errors,
        });
      }

      // No policies found
      if (discoveryResult.policies.length === 0) {
        return this.createSuccess('No policies configured', 100, {
          policyDir: discoveryResult.directory,
          policyCount: 0,
        });
      }

      // Step 3: Prepare OPA input
      const input = this.buildOPAInput(context);

      // Step 4: Execute OPA
      const executor = new OPAExecutor(binaryPath, {
        timeout: config.timeout,
        query: config.query,
        includeRawOutput: false,
      });

      const result = await executor.evaluate(discoveryResult.policies, input);

      if (!result.success) {
        return this.createFailure('Policy evaluation failed', result.error, {
          policyCount: discoveryResult.policies.length,
          executionTimeMs: result.metadata.execution_time_ms,
        });
      }

      // Step 5: Calculate score and determine pass/fail
      const { score, passed, violationsByPolicy } = this.calculateScore(
        result.violations,
        config.severity_threshold || 'error'
      );

      const message = this.buildMessage(result.violations, discoveryResult.policies, passed);

      return this.createResult(passed, message, score, {
        policyCount: discoveryResult.policies.length,
        violationCount: result.violations.length,
        violations: result.violations,
        violationsByPolicy,
        executionTimeMs: result.metadata.execution_time_ms,
        policies: discoveryResult.policies.map((p) => ({
          name: p.name,
          package: p.package,
          hasTests: p.hasTests,
        })),
      });
    } catch (error) {
      return this.createFailure(
        'Policy check failed unexpectedly',
        error instanceof Error ? error.message : 'Unknown error'
      );
    }
  }

  /**
   * Parse and validate check configuration
   */
  private parseConfig(checkConfig: Record<string, unknown>): PolicyCheckConfig {
    return {
      policy_dir: typeof checkConfig.policy_dir === 'string' ? checkConfig.policy_dir : undefined,
      severity_threshold: this.parseSeverity(checkConfig.severity_threshold),
      enabled_policies: Array.isArray(checkConfig.enabled_policies)
        ? checkConfig.enabled_policies.filter((p): p is string => typeof p === 'string')
        : undefined,
      disabled_policies: Array.isArray(checkConfig.disabled_policies)
        ? checkConfig.disabled_policies.filter((p): p is string => typeof p === 'string')
        : undefined,
      query: typeof checkConfig.query === 'string' ? checkConfig.query : undefined,
      timeout: typeof checkConfig.timeout === 'number' ? checkConfig.timeout : undefined,
    };
  }

  /**
   * Parse severity threshold
   */
  private parseSeverity(value: unknown): 'error' | 'warning' | 'info' | undefined {
    if (typeof value !== 'string') return undefined;
    const lower = value.toLowerCase();
    if (lower === 'error' || lower === 'warning' || lower === 'info') {
      return lower;
    }
    return undefined;
  }

  /**
   * Build OPA input from check context
   */
  private buildOPAInput(context: CheckContext): OPAInput {
    const plan = context.plan;

    // Calculate affected directories
    const affectedDirectories = new Set<string>();
    for (const change of plan.proposed_changes) {
      if (change.path) {
        const parts = change.path.split('/');
        if (parts.length > 1) {
          affectedDirectories.add(parts.slice(0, -1).join('/'));
        }
      }
    }

    return {
      plan: {
        id: plan.id,
        hash: plan.hash,
        intent: plan.intent,
        schema_version: plan.schema_version,
        proposed_changes: plan.proposed_changes.map((change) => ({
          type: change.type,
          path: change.path,
          description: change.description,
          metadata: change.metadata,
          extension: change.path?.split('.').pop(),
          directory: change.path?.split('/').slice(0, -1).join('/'),
        })),
        provenance: plan.provenance,
        validations: plan.validations,
        tags: plan.tags,
        change_count: plan.proposed_changes.length,
        affected_directories: Array.from(affectedDirectories),
      },
      context: {
        workspace_root: context.workspace_root,
        timestamp: Date.now(),
      },
      config: context.check_config,
    };
  }

  /**
   * Calculate score based on violations
   */
  private calculateScore(
    violations: PolicyViolation[],
    severityThreshold: 'error' | 'warning' | 'info'
  ): {
    score: number;
    passed: boolean;
    violationsByPolicy: Record<string, PolicyViolation[]>;
  } {
    // Group violations by policy
    const violationsByPolicy: Record<string, PolicyViolation[]> = {};
    for (const v of violations) {
      const policy = v.policy || 'unknown';
      if (!violationsByPolicy[policy]) {
        violationsByPolicy[policy] = [];
      }
      violationsByPolicy[policy].push(v);
    }

    // Calculate total penalty
    let totalPenalty = 0;
    let hasBlockingViolation = false;

    for (const v of violations) {
      const penalty = SEVERITY_PENALTIES[v.severity] || 0;
      totalPenalty += penalty;

      // Check if this violation should block
      if (this.isBlockingSeverity(v.severity, severityThreshold)) {
        hasBlockingViolation = true;
      }
    }

    const score = Math.max(0, 100 - totalPenalty);
    const passed = !hasBlockingViolation;

    return { score, passed, violationsByPolicy };
  }

  /**
   * Check if a severity level should block the check
   */
  private isBlockingSeverity(
    severity: 'error' | 'warning' | 'info',
    threshold: 'error' | 'warning' | 'info'
  ): boolean {
    const levels = { error: 3, warning: 2, info: 1 };
    return levels[severity] >= levels[threshold];
  }

  /**
   * Build human-readable message
   */
  private buildMessage(
    violations: PolicyViolation[],
    policies: LoadedPolicy[],
    passed: boolean
  ): string {
    if (violations.length === 0) {
      return `All ${policies.length} policies passed`;
    }

    const errorCount = violations.filter((v) => v.severity === 'error').length;
    const warningCount = violations.filter((v) => v.severity === 'warning').length;
    const infoCount = violations.filter((v) => v.severity === 'info').length;

    const parts: string[] = [];
    if (errorCount > 0) parts.push(`${errorCount} error${errorCount > 1 ? 's' : ''}`);
    if (warningCount > 0) parts.push(`${warningCount} warning${warningCount > 1 ? 's' : ''}`);
    if (infoCount > 0) parts.push(`${infoCount} info`);

    const status = passed ? 'passed with issues' : 'failed';
    return `Policy check ${status}: ${parts.join(', ')}`;
  }
}
