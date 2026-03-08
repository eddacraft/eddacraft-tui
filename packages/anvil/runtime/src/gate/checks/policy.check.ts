/**
 * Policy Check - Evaluate OPA/Rego policies against plans
 */

import { BaseCheck } from '../check.interface.js';
import { CheckContext, GateResult } from '../../types/gate.types.js';
import {
  getOPABinaryManager,
  PolicyLoader,
  OPAExecutor,
  type LoadedPolicy,
  type OPAInput,
  type PolicyViolation,
} from '../policy/index.js';
import {
  parseSeverity,
  createDebugger,
  gitExecSync,
  type ArchitectureContext,
} from '@eddacraft/anvil-core';

const log = createDebugger('check');

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
  /** Require policy tests to pass before evaluating policies */
  require_policy_tests?: boolean;
  /** Include git context in OPA input for repository-aware policies */
  include_git_context?: boolean;
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
    log(`policy check starting, workspace=${context.workspace_root}`);
    const config = this.parseConfig(context.check_config);

    // Policy check requires a plan
    if (!context.plan) {
      log('policy check: no plan provided, skipping');
      return this.createSuccess('Policy check skipped (no plan provided)', 100, {
        skipped: true,
        reason: 'Policy check requires a plan',
      });
    }

    try {
      // Step 1: Ensure OPA binary is available
      const binaryManager = getOPABinaryManager();
      let binaryPath: string;

      try {
        binaryPath = await binaryManager.ensureBinary();
        log(`policy check: OPA binary available at ${binaryPath}`);
      } catch (error) {
        log(
          `policy check: OPA binary not available: ${error instanceof Error ? error.message : 'unknown'}`
        );
        return this.createFailure(
          'OPA binary not available',
          error instanceof Error ? error.message : 'Failed to download OPA'
        );
      }

      // Step 2: Load policies
      const policyDir = config.policy_dir || DEFAULT_POLICY_DIR;
      log(`policy check: loading policies from ${policyDir}`);
      const discoveryResult = await this.policyLoader.loadPolicies(context.workspace_root, {
        policyDir,
        enabledPolicies: config.enabled_policies,
        disabledPolicies: config.disabled_policies,
      });

      // Check for policy loading errors
      if (discoveryResult.errors.length > 0) {
        const errorMessages = discoveryResult.errors.map((e) => `${e.path}: ${e.error}`).join('; ');
        log(`policy check: policy loading errors: ${errorMessages}`);
        return this.createFailure(`Failed to load some policies: ${errorMessages}`, undefined, {
          loadErrors: discoveryResult.errors,
        });
      }

      // No policies found
      if (discoveryResult.policies.length === 0) {
        log(`policy check: no policies configured in ${policyDir}`);
        return this.createSuccess('No policies configured', 100, {
          policyDir: discoveryResult.directory,
          policyCount: 0,
        });
      }

      log(`policy check: loaded ${discoveryResult.policies.length} policies`);

      // Step 3: Run policy tests if required
      if (config.require_policy_tests) {
        log('policy check: running policy tests');
        const testResult = await this.runPolicyTests(
          binaryPath,
          discoveryResult.policies,
          policyDir,
          config.timeout
        );

        if (!testResult.passed) {
          log(`policy check: policy tests failed (${testResult.failed}/${testResult.total})`);
          return this.createFailure(
            `${testResult.failed} of ${testResult.total} policy tests failed`,
            testResult.details.join('; '),
            {
              policyCount: discoveryResult.policies.length,
              testResults: testResult,
            }
          );
        }
      }

      // Step 4: Prepare OPA input
      const input = this.buildOPAInput(context, config.include_git_context !== false);

      // Step 4: Execute OPA
      const executor = new OPAExecutor(binaryPath, {
        timeout: config.timeout,
        query: config.query,
        includeRawOutput: false,
      });

      const result = await executor.evaluate(discoveryResult.policies, input);

      if (!result.success) {
        log(`policy check: evaluation failed: ${result.error}`);
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

      log('policy check result', {
        passed,
        score,
        violations: result.violations.length,
        policies: discoveryResult.policies.length,
        executionTimeMs: result.metadata.execution_time_ms,
      });

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
      log(`policy check error: ${error instanceof Error ? error.message : 'Unknown error'}`);
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
      severity_threshold: parseSeverity(checkConfig.severity_threshold, undefined),
      enabled_policies: Array.isArray(checkConfig.enabled_policies)
        ? checkConfig.enabled_policies.filter((p): p is string => typeof p === 'string')
        : undefined,
      disabled_policies: Array.isArray(checkConfig.disabled_policies)
        ? checkConfig.disabled_policies.filter((p): p is string => typeof p === 'string')
        : undefined,
      query: typeof checkConfig.query === 'string' ? checkConfig.query : undefined,
      timeout: typeof checkConfig.timeout === 'number' ? checkConfig.timeout : undefined,
      require_policy_tests:
        typeof checkConfig.require_policy_tests === 'boolean'
          ? checkConfig.require_policy_tests
          : undefined,
      include_git_context:
        typeof checkConfig.include_git_context === 'boolean'
          ? checkConfig.include_git_context
          : true, // Default to true
    };
  }

  /**
   * Build OPA input from check context
   */
  private buildOPAInput(context: CheckContext, includeGitContext: boolean): OPAInput {
    const plan = context.plan!; // Safe: checked in run()

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

    // Build context with optional git info
    const opaContext: OPAInput['context'] = {
      workspace_root: context.workspace_root,
      timestamp: Date.now(),
    };

    // Add git context if enabled
    if (includeGitContext) {
      const gitContext = this.getGitContext(context.workspace_root);
      if (gitContext) {
        opaContext.git = gitContext;
      }

      // Add CI context from environment
      const ciContext = this.getCIContext();
      if (ciContext) {
        opaContext.ci = ciContext;
      }
    }

    const architecture = this.buildArchitectureInput(context);

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
      context: opaContext,
      architecture,
      config: context.check_config,
    };
  }

  /**
   * Bridges ArchitectureCheck output to PolicyCheck OPA input.
   * Enables Rego policies to query architecture context via input.architecture
   */
  private buildArchitectureInput(context: CheckContext): OPAInput['architecture'] | undefined {
    // Cast to full ArchitectureContext - CheckContext.architectureContext is typed as base for portability
    const archContext = context.architectureContext as ArchitectureContext | undefined;
    if (!archContext) {
      return undefined;
    }

    const layers: Record<string, string[]> = {};
    for (const [layerName, layerStats] of Object.entries(archContext.layers)) {
      layers[layerName] = layerStats.patterns;
    }

    const boundaries: Array<{ from: string; to: string }> = [];
    for (const [layerName, layerStats] of Object.entries(archContext.layers)) {
      for (const depLayer of layerStats.depends_on) {
        boundaries.push({ from: layerName, to: depLayer });
      }
    }

    return {
      layers,
      boundaries,
      dependencies: archContext.dependencies,
      summary: {
        total_modules: archContext.summary.total_modules,
        total_violations: archContext.summary.total_violations,
        new_violations: archContext.summary.new_violations,
        circular_count: archContext.summary.circular_count,
        orphan_count: archContext.summary.orphan_count,
        layer_violation_count: archContext.summary.layer_violation_count,
        error_count: archContext.summary.error_count,
        warn_count: archContext.summary.warn_count,
        baseline_loaded: archContext.summary.baseline_loaded,
      },
      violations: archContext.violations.map((v) => ({
        from: v.from,
        to: v.to,
        rule: v.rule,
        severity: v.severity,
        is_circular: v.is_circular,
        is_new: v.is_new,
        from_layer: v.from_layer,
        to_layer: v.to_layer,
      })),
    };
  }

  /**
   * Get git context for repository-aware policies
   */
  private getGitContext(workspaceRoot: string): OPAInput['context']['git'] | undefined {
    try {
      const execGit = (args: string[]): string | undefined => {
        try {
          return gitExecSync(args, { cwd: workspaceRoot });
        } catch {
          return undefined;
        }
      };

      const branch = execGit(['rev-parse', '--abbrev-ref', 'HEAD']);
      if (!branch) return undefined; // Not a git repository

      const commitSha = execGit(['rev-parse', 'HEAD']);
      const author = execGit(['log', '-1', '--format=%an']);
      const authorEmail = execGit(['log', '-1', '--format=%ae']);

      // Try to get base branch (for PRs)
      let baseBranch: string | undefined;

      // First try origin/HEAD which points to the default branch
      const originHeadRef = execGit(['symbolic-ref', 'refs/remotes/origin/HEAD']);
      if (originHeadRef) {
        const match = originHeadRef.match(/^refs\/remotes\/origin\/(.+)$/);
        if (match?.[1]) {
          baseBranch = match[1];
        }
      }

      // Fallback: probe common default branch names
      if (!baseBranch) {
        const defaultBranches = ['main', 'master', 'develop'];
        for (const defaultBranch of defaultBranches) {
          const exists = execGit(['rev-parse', '--verify', defaultBranch]);
          if (exists) {
            baseBranch = defaultBranch;
            break;
          }
        }
      }

      return {
        branch,
        base_branch: baseBranch,
        commit_sha: commitSha,
        author,
        author_email: authorEmail,
      };
    } catch {
      return undefined;
    }
  }

  /**
   * Get CI context from environment variables
   */
  private getCIContext(): OPAInput['context']['ci'] | undefined {
    // GitHub Actions
    if (process.env.GITHUB_ACTIONS === 'true') {
      return {
        provider: 'github',
        build_id: process.env.GITHUB_RUN_ID,
        pr_number: process.env.GITHUB_PR_NUMBER || this.extractPRNumber(process.env.GITHUB_REF),
        pr_author: process.env.GITHUB_ACTOR,
      };
    }

    // GitLab CI
    if (process.env.GITLAB_CI === 'true') {
      return {
        provider: 'gitlab',
        build_id: process.env.CI_JOB_ID,
        pr_number: process.env.CI_MERGE_REQUEST_IID,
        pr_author: process.env.GITLAB_USER_LOGIN,
      };
    }

    // Jenkins
    if (process.env.JENKINS_URL) {
      return {
        provider: 'jenkins',
        build_id: process.env.BUILD_ID,
        pr_number: process.env.CHANGE_ID,
        pr_author: process.env.CHANGE_AUTHOR,
      };
    }

    // Azure DevOps
    if (process.env.TF_BUILD === 'True') {
      return {
        provider: 'azure',
        build_id: process.env.BUILD_BUILDID,
        pr_number: process.env.SYSTEM_PULLREQUEST_PULLREQUESTID,
        pr_author: process.env.BUILD_REQUESTEDFOR,
      };
    }

    // Local development
    return {
      provider: 'local',
    };
  }

  /**
   * Extract PR number from GitHub ref (e.g., refs/pull/123/merge)
   */
  private extractPRNumber(ref: string | undefined): string | undefined {
    if (!ref) return undefined;
    const match = ref.match(/refs\/pull\/(\d+)/);
    return match?.[1];
  }

  /**
   * Run policy tests and return results
   */
  private async runPolicyTests(
    binaryPath: string,
    policies: LoadedPolicy[],
    policyDir: string,
    timeout?: number
  ): Promise<{ passed: boolean; failed: number; total: number; details: string[] }> {
    const testFiles = policies
      .filter((p) => p.hasTests)
      .map((p) => p.testPath)
      .filter(Boolean);

    if (testFiles.length === 0) {
      return { passed: true, failed: 0, total: 0, details: [] };
    }

    const executor = new OPAExecutor(binaryPath, { timeout });
    const result = await executor.runTests(policies, testFiles as string[]);

    // Check for execution errors in addition to test failures
    const hasErrors = result.errors.length > 0;
    const errorDetails = hasErrors ? result.errors.map((e) => `Execution error: ${e}`) : [];

    return {
      passed: result.failed === 0 && !hasErrors,
      failed: result.failed,
      total: result.passed + result.failed,
      details: [
        ...result.details
          .filter((d) => !d.passed)
          .map((d) => `${d.name}: ${d.message || 'failed'}`),
        ...errorDetails,
      ],
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
