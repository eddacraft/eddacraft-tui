/**
 * Policy Command - Manage OPA/Rego policies for Anvil
 *
 * Supports org/team/local policy layering with rich metadata,
 * graduated enforcement, and human-readable explanations.
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { createDebugger, validatePathWithinRoot } from '@eddacraft/anvil-core';
import { CliError, CliExit } from '../utils/cli-error.js';
import {
  existsSync,
  mkdirSync,
  readdirSync,
  copyFileSync,
  writeFileSync,
  readFileSync,
} from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { getWorkspaceRoot } from '../utils/file-io.js';
import { success, error, info, warning, print, blank, data } from '../utils/output.js';
import { coerceNonNegativeInt } from '../utils/option-coerce.js';
import {
  PolicyLoader,
  OPAExecutor,
  getOPABinaryManager,
  GateConfigManager,
  BundleManager,
  getBundleManager,
  type BundleAuthConfig,
  type BundleConfig,
  type PolicyBundleConfig,
  type PolicyVerificationConfig,
} from '@eddacraft/anvil-runtime';
import {
  PolicyConfigManager,
  type ResolvedPolicy,
  type EnforcementLevel,
} from '../services/policy-config.js';

const log = createDebugger('cli');

/**
 * Default policy directory relative to workspace root
 */
const DEFAULT_POLICY_DIR = '.anvil/policies';

/**
 * Get the path to bundled example policies
 */
function getExamplePoliciesPath(): string {
  // When running from built CLI, policies are in core package
  const currentDir = dirname(fileURLToPath(import.meta.url));
  // Navigate from cli/dist/commands to core/src/gate/__fixtures__/policies
  const possiblePaths = [
    join(currentDir, '../../../core/src/gate/__fixtures__/policies'),
    join(currentDir, '../../core/src/gate/__fixtures__/policies'),
    join(currentDir, '../../../../core/src/gate/__fixtures__/policies'),
  ];

  for (const path of possiblePaths) {
    if (existsSync(path)) {
      return path;
    }
  }

  // Fallback: policies are embedded in the command
  return '';
}

/**
 * Example policy templates (used when fixtures not found)
 */
const EXAMPLE_POLICIES = {
  'coverage_min.rego': `# Coverage Minimum Policy
# Enforces minimum test coverage thresholds

package anvil.policies.coverage_min

import future.keywords.if
import future.keywords.in

# Default minimum coverage (configurable via input.config)
default min_coverage := 80

min_coverage := input.config.min_coverage if {
  input.config.min_coverage
}

# Violation when coverage is below threshold
violation[msg] {
  coverage := input.context.coverage.lines
  coverage < min_coverage
  msg := sprintf("Test coverage %v%% is below minimum %v%%", [coverage, min_coverage])
}

# Info when coverage is good but could be improved
info[msg] {
  coverage := input.context.coverage.lines
  coverage >= min_coverage
  coverage < 90
  msg := sprintf("Coverage is %v%% - consider improving to 90%%+", [coverage])
}
`,

  'coverage_min_test.rego': `# Tests for coverage_min policy

package anvil.policies.coverage_min_test

import future.keywords.if
import data.anvil.policies.coverage_min

# Test that low coverage triggers violation
test_low_coverage_fails if {
  count(coverage_min.violation) > 0 with input as {
    "context": {"coverage": {"lines": 50}},
    "config": {"min_coverage": 80}
  }
}

# Test that sufficient coverage passes
test_sufficient_coverage_passes if {
  count(coverage_min.violation) == 0 with input as {
    "context": {"coverage": {"lines": 85}},
    "config": {"min_coverage": 80}
  }
}

# Test custom threshold
test_custom_threshold if {
  count(coverage_min.violation) > 0 with input as {
    "context": {"coverage": {"lines": 85}},
    "config": {"min_coverage": 90}
  }
}
`,

  'change_scope.rego': `# Change Scope Policy
# Limits the scope of changes per plan

package anvil.policies.change_scope

import future.keywords.if
import future.keywords.in

# Default limits (configurable via input.config)
default max_files := 20
default max_directories := 5

max_files := input.config.max_files if {
  input.config.max_files
}

max_directories := input.config.max_directories if {
  input.config.max_directories
}

# Violation when too many files changed
violation[msg] {
  file_count := count(input.plan.proposed_changes)
  file_count > max_files
  msg := sprintf("Plan touches %v files, maximum is %v", [file_count, max_files])
}

# Violation when too many directories affected
violation[msg] {
  directories := {dir | change := input.plan.proposed_changes[_]; dir := change.directory; dir != ""}
  dir_count := count(directories)
  dir_count > max_directories
  msg := sprintf("Plan touches %v directories, maximum is %v", [dir_count, max_directories])
}

# Warning for large but acceptable changes
warning[msg] {
  file_count := count(input.plan.proposed_changes)
  file_count > 10
  file_count <= max_files
  msg := sprintf("Plan touches %v files - consider splitting into smaller changes", [file_count])
}
`,

  'change_scope_test.rego': `# Tests for change_scope policy

package anvil.policies.change_scope_test

import future.keywords.if
import data.anvil.policies.change_scope

# Test that too many files triggers violation
test_too_many_files if {
  count(change_scope.violation) > 0 with input as {
    "plan": {
      "proposed_changes": [
        {"type": "file_create", "path": "f1.ts", "directory": "src"},
        {"type": "file_create", "path": "f2.ts", "directory": "src"},
        {"type": "file_create", "path": "f3.ts", "directory": "src"}
      ]
    },
    "config": {"max_files": 2}
  }
}

# Test that acceptable file count passes
test_acceptable_files if {
  count(change_scope.violation) == 0 with input as {
    "plan": {
      "proposed_changes": [
        {"type": "file_create", "path": "f1.ts", "directory": "src"}
      ]
    },
    "config": {"max_files": 20}
  }
}
`,

  'security_baseline.rego': `# Security Baseline Policy
# Requires security review for sensitive changes

package anvil.policies.security_baseline

import future.keywords.if
import future.keywords.in

# Sensitive path patterns (configurable via input.config)
default sensitive_patterns := [
  "**/auth/**",
  "**/security/**",
  "**/*credential*",
  "**/*secret*",
  "**/*.env*",
  "**/config/keys/**"
]

sensitive_patterns := input.config.sensitive_patterns if {
  input.config.sensitive_patterns
}

# Check if a path matches any sensitive pattern
is_sensitive(path) if {
  pattern := sensitive_patterns[_]
  glob.match(pattern, ["/"], path)
}

# Violation when changing sensitive files without security-review tag
violation[msg] {
  change := input.plan.proposed_changes[_]
  is_sensitive(change.path)
  not has_security_review
  msg := sprintf("Changes to '%s' require security-review tag", [change.path])
}

# Check for security-review tag
has_security_review if {
  "security-review" in input.plan.tags
}

has_security_review if {
  "security-reviewed" in input.plan.tags
}
`,

  'security_baseline_test.rego': `# Tests for security_baseline policy

package anvil.policies.security_baseline_test

import future.keywords.if
import data.anvil.policies.security_baseline

# Test that sensitive file without review triggers violation
test_sensitive_without_review if {
  count(security_baseline.violation) > 0 with input as {
    "plan": {
      "proposed_changes": [
        {"type": "file_update", "path": "src/auth/login.ts"}
      ],
      "tags": []
    },
    "config": {"sensitive_patterns": ["**/auth/**"]}
  }
}

# Test that sensitive file with review passes
test_sensitive_with_review if {
  count(security_baseline.violation) == 0 with input as {
    "plan": {
      "proposed_changes": [
        {"type": "file_update", "path": "src/auth/login.ts"}
      ],
      "tags": ["security-review"]
    },
    "config": {"sensitive_patterns": ["**/auth/**"]}
  }
}

# Test that non-sensitive file passes
test_nonsensitive_passes if {
  count(security_baseline.violation) == 0 with input as {
    "plan": {
      "proposed_changes": [
        {"type": "file_update", "path": "src/utils/helpers.ts"}
      ],
      "tags": []
    },
    "config": {"sensitive_patterns": ["**/auth/**"]}
  }
}
`,
};

// ---------------------------------------------------------------------------
// Enforcement level formatting
// ---------------------------------------------------------------------------

function formatEnforcement(level: EnforcementLevel): string {
  switch (level) {
    case 'block':
      return chalk.red('block');
    case 'warn':
      return chalk.yellow('warn');
    case 'info':
      return chalk.blue('info');
    case 'off':
      return chalk.dim('off');
  }
}

function formatSource(source: ResolvedPolicy['source']): string {
  switch (source) {
    case 'org':
      return chalk.magenta('org');
    case 'team':
      return chalk.cyan('team');
    case 'local':
      return chalk.green('local');
    case 'starter':
      return chalk.dim('starter');
    case 'bundle':
      return chalk.blue('bundle');
  }
}

// ---------------------------------------------------------------------------
// Main command
// ---------------------------------------------------------------------------

export function createPolicyCommand(): Command {
  const command = new Command('policy');

  command.description('Manage OPA/Rego policies');

  // -----------------------------------------------------------------------
  // list — enhanced with source, enforcement, owner, reason
  // -----------------------------------------------------------------------
  command
    .command('list')
    .description('List active policies with source, enforcement level, and ownership')
    .option('-d, --dir <directory>', 'Policy directory', DEFAULT_POLICY_DIR)
    .option('-a, --all', 'Include disabled and pending policies')
    .option('--json', 'Output as JSON')
    .action(async (options: { dir: string; all?: boolean; json?: boolean }) => {
      log(`policy list: dir=${options.dir} all=${options.all}`);
      try {
        const workspaceRoot = getWorkspaceRoot();
        const policyDir = options.dir;

        // Load from YAML config (layered policies)
        const configMgr = new PolicyConfigManager(workspaceRoot);
        const resolved = configMgr.resolvePolicies();

        // Also load rego-level policies for package info
        const loader = new PolicyLoader();
        const regoResult = await loader.loadPolicies(workspaceRoot, { policyDir });
        const regoByName = new Map(regoResult.policies.map((p) => [p.name, p]));

        // Merge: resolved config policies + any rego-only policies not in config
        const allPolicies = [...resolved];
        for (const rego of regoResult.policies) {
          if (!allPolicies.some((p) => p.name === rego.name)) {
            allPolicies.push({
              name: rego.name,
              source: 'starter',
              enforcement: 'block',
              active: true,
              hasRegoFile: true,
              regoPath: rego.path,
            });
          }
        }

        const displayPolicies = options.all ? allPolicies : allPolicies.filter((p) => p.active);

        if (displayPolicies.length === 0) {
          info('No policies found');
          print(chalk.dim('\nRun `anvil policy init` to create example policies'));
          return;
        }

        if (options.json) {
          data(JSON.stringify(displayPolicies, null, 2));
          return;
        }

        print(chalk.bold('\nPolicies:\n'));

        // Print table header
        print(
          chalk.dim('  ') +
            chalk.bold('Name'.padEnd(22)) +
            chalk.bold('Source'.padEnd(10)) +
            chalk.bold('Enforce'.padEnd(10)) +
            chalk.bold('Owner'.padEnd(18)) +
            chalk.bold('Reason')
        );
        print(chalk.dim('  ' + '─'.repeat(90)));

        for (const p of displayPolicies) {
          const rego = regoByName.get(p.name);
          const tests = rego?.hasTests ? chalk.green(' ✓') : '';

          // Pad manually for ANSI-colored strings
          print(
            '  ' +
              p.name.padEnd(22).replace(p.name, p.active ? chalk.cyan(p.name) : chalk.dim(p.name)) +
              (p.source as string).padEnd(10).replace(p.source, formatSource(p.source)) +
              (p.enforcement as string)
                .padEnd(10)
                .replace(p.enforcement, formatEnforcement(p.enforcement)) +
              chalk.dim((p.owner ?? '-').padEnd(18)) +
              chalk.dim(truncate(p.reason ?? '', 40)) +
              tests
          );

          // Show effective date for pending policies
          if (p.effective && !p.active) {
            print(chalk.dim(`                      effective: ${p.effective}`));
          }
        }

        blank();

        const activeCount = allPolicies.filter((p) => p.active).length;
        const totalCount = allPolicies.length;
        if (options.all) {
          success(
            `${activeCount} active, ${totalCount - activeCount} inactive (${totalCount} total)`
          );
        } else {
          success(`${activeCount} active policies`);
          if (totalCount > activeCount) {
            print(chalk.dim(`  ${totalCount - activeCount} more hidden. Use --all to show.`));
          }
        }

        // Show org source if configured
        const config = configMgr.load();
        if (config.policies?.org) {
          blank();
          info(
            `Org source: ${chalk.cyan(config.policies.org.source)}${config.policies.org.ref ? ` @ ${config.policies.org.ref}` : ''}`
          );
        }

        if (regoResult.errors.length > 0) {
          blank();
          warning(`${regoResult.errors.length} policies failed to load:`);
          for (const err of regoResult.errors) {
            print(chalk.red(`  • ${err.path}: ${err.error}`));
          }
        }
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(`Failed to list policies: ${err instanceof Error ? err.message : 'Unknown error'}`);
        throw new CliError('Failed to list policies');
      }
    });

  // -----------------------------------------------------------------------
  // explain <name> — human-readable rationale and ownership
  // -----------------------------------------------------------------------
  command
    .command('explain <name>')
    .description('Show detailed explanation for a policy')
    .action(async (name: string) => {
      log(`policy explain: name=${name}`);
      try {
        const workspaceRoot = getWorkspaceRoot();
        const configMgr = new PolicyConfigManager(workspaceRoot);
        const resolved = configMgr.resolvePolicies();
        const policy = resolved.find((p) => p.name === name);

        if (!policy) {
          error(`Policy '${name}' not found`);
          print(chalk.dim('\nRun `anvil policy list --all` to see available policies'));
          throw new CliError(`Policy '${name}' not found`);
        }

        blank();
        print(chalk.bold(`Policy: ${policy.name}`));
        print(chalk.dim('─'.repeat(50)));
        blank();

        print(`  ${chalk.bold('Source:')}        ${formatSource(policy.source)}`);
        print(`  ${chalk.bold('Enforcement:')}   ${formatEnforcement(policy.enforcement)}`);
        print(
          `  ${chalk.bold('Status:')}        ${policy.active ? chalk.green('active') : chalk.yellow('inactive')}`
        );

        if (policy.owner) {
          print(`  ${chalk.bold('Owner:')}         ${policy.owner}`);
        }

        if (policy.effective) {
          const effectiveDate = new Date(policy.effective);
          const isEffective = effectiveDate <= new Date();
          print(
            `  ${chalk.bold('Effective:')}     ${policy.effective} ${isEffective ? chalk.green('(in effect)') : chalk.yellow('(pending)')}`
          );
        }

        if (policy.tags && policy.tags.length > 0) {
          print(`  ${chalk.bold('Tags:')}          ${policy.tags.join(', ')}`);
        }

        if (policy.reason) {
          blank();
          print(chalk.bold('  Why this policy exists:'));
          print(`  ${policy.reason}`);
        }

        if (policy.hasRegoFile && policy.regoPath) {
          blank();
          print(chalk.bold('  Rego file:'));
          print(chalk.dim(`  ${policy.regoPath}`));

          // Show first few comment lines from the rego file as documentation
          try {
            const content = readFileSync(policy.regoPath, 'utf-8');
            const commentLines = content
              .split('\n')
              .filter((line) => line.startsWith('#'))
              .slice(0, 5)
              .map((line) => line.replace(/^#\s?/, ''));

            if (commentLines.length > 0) {
              blank();
              print(chalk.bold('  Description (from source):'));
              for (const line of commentLines) {
                print(chalk.dim(`  ${line}`));
              }
            }
          } catch {
            // Ignore read errors
          }
        }

        blank();
        print(chalk.dim('  Commands:'));
        print(chalk.dim(`    anvil policy disable ${name}    # turn it off`));
        print(chalk.dim(`    anvil gate --skip ${name}       # skip just this once`));
        blank();
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(`Failed to explain policy: ${err instanceof Error ? err.message : 'Unknown error'}`);
        throw new CliError('Failed to explain policy');
      }
    });

  // -----------------------------------------------------------------------
  // why <violation> — business reason for a violation
  // -----------------------------------------------------------------------
  command
    .command('why <violation>')
    .description('Explain the business reason behind a policy violation')
    .action(async (violation: string) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const configMgr = new PolicyConfigManager(workspaceRoot);
        const resolved = configMgr.resolvePolicies();

        // Try to match the violation to a policy name (fuzzy)
        const match = resolved.find(
          (p) => p.name === violation || p.name.includes(violation) || violation.includes(p.name)
        );

        if (!match) {
          // If no exact match, try broader search
          const partial = resolved.filter(
            (p) =>
              violation.toLowerCase().includes(p.name.toLowerCase().replace(/_/g, '-')) ||
              violation.toLowerCase().includes(p.name.toLowerCase().replace(/-/g, '_'))
          );

          if (partial.length === 0) {
            error(`Could not match '${violation}' to any known policy`);
            print(chalk.dim('\nAvailable policies:'));
            for (const p of resolved) {
              print(chalk.dim(`  • ${p.name}`));
            }
            throw new CliError(`Could not match '${violation}' to any known policy`);
          }

          // Show all matches
          for (const p of partial) {
            printWhyBlock(p);
          }
          return;
        }

        printWhyBlock(match);
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(
          `Failed to explain violation: ${err instanceof Error ? err.message : 'Unknown error'}`
        );
        throw new CliError('Failed to explain policy violation');
      }
    });

  // -----------------------------------------------------------------------
  // diff — what changed since last sync
  // -----------------------------------------------------------------------
  command
    .command('diff')
    .description('Show policy changes since last sync or commit')
    .option('-d, --dir <directory>', 'Policy directory', DEFAULT_POLICY_DIR)
    .action(async (options: { dir: string }) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const { execFileSync } = await import('node:child_process');

        // Check git status of policy files
        const policyDir = options.dir;
        const configPath = join('.anvil', 'config.yml');

        print(chalk.bold('\nPolicy Changes:\n'));

        let hasChanges = false;

        // Check for config.yml changes
        try {
          const configDiff = execFileSync(
            'git',
            ['diff', '--name-status', 'HEAD', '--', configPath],
            { cwd: workspaceRoot, encoding: 'utf-8', timeout: 30_000 }
          ).trim();

          if (configDiff) {
            hasChanges = true;
            print(chalk.bold('  Config changes:'));
            for (const line of configDiff.split('\n')) {
              const [status, ...pathParts] = line.split('\t');
              const filePath = pathParts.join('\t');
              const statusLabel =
                status === 'M'
                  ? chalk.yellow('modified')
                  : status === 'A'
                    ? chalk.green('added')
                    : status === 'D'
                      ? chalk.red('deleted')
                      : chalk.dim(status ?? '');
              print(`    ${statusLabel} ${filePath}`);
            }
            blank();
          }
        } catch {
          // Not a git repo or no changes
        }

        // Check for policy file changes
        try {
          const policyDiff = execFileSync(
            'git',
            ['diff', '--name-status', 'HEAD', '--', policyDir],
            { cwd: workspaceRoot, encoding: 'utf-8', timeout: 30_000 }
          ).trim();

          if (policyDiff) {
            hasChanges = true;
            print(chalk.bold('  Policy file changes:'));
            for (const line of policyDiff.split('\n')) {
              const [status, ...pathParts] = line.split('\t');
              const filePath = pathParts.join('\t');
              const statusLabel =
                status === 'M'
                  ? chalk.yellow('modified')
                  : status === 'A'
                    ? chalk.green('added')
                    : status === 'D'
                      ? chalk.red('deleted')
                      : chalk.dim(status ?? '');
              print(`    ${statusLabel} ${filePath}`);
            }
            blank();
          }
        } catch {
          // Not a git repo or no changes
        }

        // Check for untracked policy files
        try {
          const untracked = execFileSync(
            'git',
            ['ls-files', '--others', '--exclude-standard', '--', policyDir, configPath],
            { cwd: workspaceRoot, encoding: 'utf-8', timeout: 30_000 }
          ).trim();

          if (untracked) {
            hasChanges = true;
            print(chalk.bold('  New (untracked):'));
            for (const file of untracked.split('\n')) {
              print(`    ${chalk.green('new')} ${file}`);
            }
            blank();
          }
        } catch {
          // Ignore
        }

        if (!hasChanges) {
          info('No policy changes detected');
        }
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(`Failed to diff policies: ${err instanceof Error ? err.message : 'Unknown error'}`);
        throw new CliError('Failed to diff policies');
      }
    });

  // -----------------------------------------------------------------------
  // disable <name>
  // -----------------------------------------------------------------------
  command
    .command('disable <name>')
    .description('Disable a policy (adds local override)')
    .action(async (name: string) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const configMgr = new PolicyConfigManager(workspaceRoot);

        // Verify policy exists
        const resolved = configMgr.resolvePolicies();
        const policy = resolved.find((p) => p.name === name);

        if (!policy) {
          error(`Policy '${name}' not found`);
          throw new CliError(`Policy '${name}' not found for disable`);
        }

        if (!policy.active) {
          info(`Policy '${name}' is already inactive`);
          return;
        }

        configMgr.disablePolicy(name);
        success(`Disabled policy '${name}'`);
        print(chalk.dim(`  To re-enable: anvil policy enable ${name}`));
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(`Failed to disable policy: ${err instanceof Error ? err.message : 'Unknown error'}`);
        throw new CliError('Failed to disable policy');
      }
    });

  // -----------------------------------------------------------------------
  // enable <name>
  // -----------------------------------------------------------------------
  command
    .command('enable <name>')
    .description('Re-enable a disabled policy')
    .option('-e, --enforcement <level>', 'Enforcement level (block, warn, info)', 'block')
    .action(async (name: string, options: { enforcement: string }) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const configMgr = new PolicyConfigManager(workspaceRoot);

        const validEnforcementLevels: readonly string[] = ['block', 'warn', 'info', 'off'];
        if (!validEnforcementLevels.includes(options.enforcement)) {
          error(
            `Invalid enforcement level '${options.enforcement}'. Must be one of: ${validEnforcementLevels.join(', ')}`
          );
          throw new CliError(`Invalid enforcement level: ${options.enforcement}`);
        }
        const enforcement = options.enforcement as EnforcementLevel;

        configMgr.enablePolicy(name, enforcement);
        success(`Enabled policy '${name}' with enforcement: ${formatEnforcement(enforcement)}`);
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(`Failed to enable policy: ${err instanceof Error ? err.message : 'Unknown error'}`);
        throw new CliError('Failed to enable policy');
      }
    });

  // -----------------------------------------------------------------------
  // doc — generate POLICIES.md
  // -----------------------------------------------------------------------
  command
    .command('doc')
    .description('Generate .anvil/POLICIES.md from current policy configuration')
    .option('-o, --output <path>', 'Output file path', '.anvil/POLICIES.md')
    .action(async (options: { output: string }) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const configMgr = new PolicyConfigManager(workspaceRoot);
        const markdown = configMgr.generatePoliciesDoc();

        const outputPath = validatePathWithinRoot(options.output, workspaceRoot);
        const outputDir = dirname(outputPath);
        if (!existsSync(outputDir)) {
          mkdirSync(outputDir, { recursive: true });
        }

        writeFileSync(outputPath, markdown, 'utf-8');
        success(`Generated ${options.output}`);
        print(chalk.dim('  Commit this file so the team can read it in any editor.'));
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(`Failed to generate docs: ${err instanceof Error ? err.message : 'Unknown error'}`);
        throw new CliError('Failed to generate policy docs');
      }
    });

  // -----------------------------------------------------------------------
  // scaffold --org <name> — extract local policies to org repo
  // -----------------------------------------------------------------------
  command
    .command('scaffold')
    .description('Scaffold policy structure for org-wide sharing')
    .requiredOption('--org <name>', 'Organisation name')
    .option('--out <dir>', 'Output directory for org policy repo', './anvil-policies')
    .action(async (options: { org: string; out: string }) => {
      try {
        const workspaceRoot = getWorkspaceRoot();

        // Validate --out does not escape workspace
        let outDir: string;
        try {
          outDir = validatePathWithinRoot(options.out, workspaceRoot);
        } catch {
          error(`--out path escapes workspace: ${options.out}`);
          throw new CliError('Scaffold output path escapes workspace');
        }

        const configMgr = new PolicyConfigManager(workspaceRoot);
        const spinner = ora(`Scaffolding org policies for ${options.org}...`).start();

        // Create org directory structure
        mkdirSync(join(outDir, '.anvil', 'policies'), { recursive: true });

        // Copy existing local policies
        const localPolicyDir = join(workspaceRoot, DEFAULT_POLICY_DIR);
        let copiedCount = 0;
        if (existsSync(localPolicyDir)) {
          const files = readdirSync(localPolicyDir).filter((f) => f.endsWith('.rego'));
          for (const file of files) {
            copyFileSync(join(localPolicyDir, file), join(outDir, '.anvil', 'policies', file));
            copiedCount++;
          }
        }

        // Generate org config.yml
        const orgConfigYaml = configMgr.generateOrgScaffold(options.org);
        writeFileSync(join(outDir, '.anvil', 'config.yml'), orgConfigYaml, 'utf-8');

        // Generate a README
        const readme = [
          `# ${options.org} Anvil Policies`,
          '',
          'Shared policy repository for org-wide Anvil policies.',
          '',
          '## Usage',
          '',
          "In your project's `.anvil/config.yml`:",
          '',
          '```yaml',
          'policies:',
          '  org:',
          `    source: "git@github.com:${options.org}/anvil-policies.git"`,
          '    ref: "v1.0.0"',
          '```',
          '',
          'Then run:',
          '',
          '```sh',
          `anvil init --org ${options.org}`,
          '```',
          '',
          '## Policies',
          '',
          'Run `anvil policy list` to see all active policies.',
          'Run `anvil policy doc` to regenerate POLICIES.md.',
          '',
        ].join('\n');
        writeFileSync(join(outDir, 'README.md'), readme, 'utf-8');

        spinner.succeed(`Scaffolded org policy repo at ${options.out}`);

        blank();
        print(chalk.bold('Created:'));
        print(chalk.dim(`  ${options.out}/.anvil/config.yml`));
        print(chalk.dim(`  ${options.out}/.anvil/policies/ (${copiedCount} policies)`));
        print(chalk.dim(`  ${options.out}/README.md`));

        blank();
        print(chalk.bold('Next steps:'));
        print(chalk.dim(`  1. cd ${options.out}`));
        print(chalk.dim('  2. git init && git add . && git commit -m "Initial policy repo"'));
        print(chalk.dim(`  3. Push to git@github.com:${options.org}/anvil-policies.git`));
        print(chalk.dim('  4. In your project, add the org source to .anvil/config.yml'));
        blank();
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(`Failed to scaffold: ${err instanceof Error ? err.message : 'Unknown error'}`);
        throw new CliError('Failed to scaffold org policies');
      }
    });

  // -----------------------------------------------------------------------
  // validate <file> (existing)
  // -----------------------------------------------------------------------
  command
    .command('validate <file>')
    .description('Validate Rego syntax for a policy file')
    .action(async (file: string) => {
      const spinner = ora('Validating policy syntax...').start();

      try {
        const { resolve } = await import('node:path');
        const workspaceRoot = getWorkspaceRoot();
        const absolutePath = resolve(file);
        const validatedPath = validatePathWithinRoot(absolutePath, workspaceRoot);

        const binaryManager = getOPABinaryManager();
        const binaryPath = await binaryManager.ensureBinary();

        const { readFile } = await import('node:fs/promises');
        const content = await readFile(validatedPath, 'utf-8');

        const executor = new OPAExecutor(binaryPath);
        const result = await executor.validateSyntax(content);

        if (result.valid) {
          spinner.succeed(chalk.green('Policy syntax is valid'));
        } else {
          spinner.fail(chalk.red('Policy syntax is invalid'));
          for (const err of result.errors) {
            print(chalk.red(`  • ${err}`));
          }
          throw new CliError('Policy syntax is invalid');
        }
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        spinner.fail('Validation failed');
        error(err instanceof Error ? err.message : 'Unknown error');
        throw new CliError('Policy syntax validation failed');
      }
    });

  // -----------------------------------------------------------------------
  // test [policy] (existing)
  // -----------------------------------------------------------------------
  command
    .command('test [policy]')
    .description('Run policy unit tests')
    .option('-d, --dir <directory>', 'Policy directory', DEFAULT_POLICY_DIR)
    .option('-v, --verbose', 'Show detailed test output')
    .action(async (policy: string | undefined, options: { dir: string; verbose?: boolean }) => {
      const spinner = ora('Running policy tests...').start();

      try {
        const workspaceRoot = getWorkspaceRoot();
        const policyDir = options.dir;

        // Ensure OPA binary
        const binaryManager = getOPABinaryManager();
        const binaryPath = await binaryManager.ensureBinary();

        // Load policies
        const loader = new PolicyLoader();
        const discoveryResult = await loader.loadPolicies(workspaceRoot, { policyDir });

        if (discoveryResult.policies.length === 0) {
          spinner.warn('No policies found');
          print(chalk.dim('\nRun `anvil policy init` to create example policies'));
          return;
        }

        // Filter by policy name if specified
        let policies = discoveryResult.policies;
        if (policy) {
          policies = policies.filter((p) => p.name === policy || p.name.includes(policy));
          if (policies.length === 0) {
            spinner.fail(`Policy '${policy}' not found`);
            throw new CliError(`Policy '${policy}' not found for testing`);
          }
        }

        // Find test files
        const testFiles = loader.findTestFiles(discoveryResult.directory);
        if (testFiles.length === 0) {
          spinner.warn('No test files found');
          print(chalk.dim('\nCreate *_test.rego files to add tests'));
          return;
        }

        // Run tests
        const executor = new OPAExecutor(binaryPath);
        const result = await executor.runTests(policies, testFiles);

        if (result.passed === 0 && result.failed === 0) {
          spinner.warn('No tests were executed');
          return;
        }

        const allPassed = result.failed === 0 && result.errors.length === 0;

        if (allPassed) {
          spinner.succeed(chalk.green(`All ${result.passed} tests passed`));
        } else {
          spinner.fail(chalk.red(`${result.failed} tests failed, ${result.passed} passed`));
        }

        // Show details if verbose or if there are failures
        if (options.verbose || !allPassed) {
          blank();
          for (const detail of result.details) {
            const icon = detail.passed ? chalk.green('✓') : chalk.red('✗');
            print(`  ${icon} ${detail.name}`);
            if (detail.message) {
              print(chalk.dim(`      ${detail.message}`));
            }
          }
        }

        if (result.errors.length > 0) {
          blank();
          warning('Errors occurred:');
          for (const err of result.errors) {
            print(chalk.red(`  • ${err}`));
          }
        }

        if (!allPassed) {
          throw new CliError('Policy tests failed');
        }
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        spinner.fail('Test run failed');
        error(err instanceof Error ? err.message : 'Unknown error');
        throw new CliError('Policy test run failed');
      }
    });

  // -----------------------------------------------------------------------
  // init (existing)
  // -----------------------------------------------------------------------
  command
    .command('init')
    .description('Initialise policy directory with example policies')
    .option('-d, --dir <directory>', 'Policy directory', DEFAULT_POLICY_DIR)
    .option('--force', 'Overwrite existing policies')
    .action(async (options: { dir: string; force?: boolean }) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const policyDir = join(workspaceRoot, options.dir);

        // Check if directory exists
        if (existsSync(policyDir) && !options.force) {
          const files = readdirSync(policyDir).filter((f) => f.endsWith('.rego'));
          if (files.length > 0) {
            error(`Policy directory already contains ${files.length} policies`);
            print(chalk.dim('\nUse --force to overwrite existing policies'));
            throw new CliError('Policy directory already contains policies');
          }
        }

        const spinner = ora('Creating policy directory...').start();

        // Create directory
        if (!existsSync(policyDir)) {
          mkdirSync(policyDir, { recursive: true });
        }

        // Try to copy from fixtures, otherwise use embedded templates
        const fixturesPath = getExamplePoliciesPath();
        let copiedCount = 0;

        if (fixturesPath && existsSync(fixturesPath)) {
          // Copy from fixtures
          const fixtures = readdirSync(fixturesPath).filter((f) => f.endsWith('.rego'));
          for (const file of fixtures) {
            const src = join(fixturesPath, file);
            const dest = join(policyDir, file);
            copyFileSync(src, dest);
            copiedCount++;
          }
        } else {
          // Use embedded templates
          for (const [filename, content] of Object.entries(EXAMPLE_POLICIES)) {
            const dest = join(policyDir, filename);
            writeFileSync(dest, content, 'utf-8');
            copiedCount++;
          }
        }

        spinner.succeed(chalk.green(`Created ${copiedCount} example policies`));

        print('\n' + chalk.bold('Created policies:'));
        print(chalk.cyan('  • coverage_min.rego') + chalk.dim(' - Enforce minimum test coverage'));
        print(chalk.cyan('  • change_scope.rego') + chalk.dim(' - Limit files per change'));
        print(
          chalk.cyan('  • security_baseline.rego') + chalk.dim(' - Security review requirements')
        );

        print('\n' + chalk.bold('Next steps:'));
        print(chalk.dim('  1. Review and customise policies in ') + chalk.cyan(options.dir));
        print(chalk.dim('  2. List policies: ') + chalk.cyan('anvil policy list'));
        print(chalk.dim('  3. Run policy tests: ') + chalk.cyan('anvil policy test'));
        print(chalk.dim('  4. Enable policy check in .anvilrc'));

        // Show .anvilrc snippet
        print('\n' + chalk.bold('Add to .anvilrc:'));
        print(
          chalk.dim(`  {
            "name": "policy",
            "enabled": true,
            "config": {
              "policy_dir": "${options.dir}",
              "severity_threshold": "error"
            }
          }`)
        );

        blank();
        success('Policy directory initialised!');
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        error(
          `Failed to initialise policies: ${err instanceof Error ? err.message : 'Unknown error'}`
        );
        throw new CliError('Failed to initialise policies');
      }
    });

  // -----------------------------------------------------------------------
  // bundle (existing group)
  // -----------------------------------------------------------------------
  command
    .command('bundle')
    .description('Manage remote policy bundles')
    .addCommand(createBundleListCommand())
    .addCommand(createBundleAddCommand())
    .addCommand(createBundleRemoveCommand())
    .addCommand(createBundleSyncCommand());

  return command;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function truncate(str: string, maxLen: number): string {
  if (str.length <= maxLen) return str;
  return str.slice(0, maxLen - 1) + '…';
}

function printWhyBlock(policy: ResolvedPolicy): void {
  blank();
  print(`  ${chalk.red('✗')} ${chalk.bold(policy.name)}: ${formatEnforcement(policy.enforcement)}`);
  blank();

  if (policy.reason) {
    print(`  ${chalk.bold('Why:')} ${policy.reason}`);
  } else {
    print(chalk.dim(`  No business reason documented for this policy.`));
    print(chalk.dim(`  Add a "reason" field in .anvil/config.yml to document it.`));
  }

  if (policy.owner) {
    print(`  ${chalk.bold('Owner:')} ${policy.owner}`);
  }

  print(`  ${chalk.bold('Source:')} ${formatSource(policy.source)} policy`);

  blank();
  print(chalk.dim(`  anvil policy explain ${policy.name}    # full details`));
  print(chalk.dim(`  anvil policy disable ${policy.name}    # turn it off`));
  print(chalk.dim(`  anvil gate --skip ${policy.name}       # skip just this once`));
  blank();
}

/**
 * Format a timestamp as relative time (e.g., "2 hours ago")
 */
function formatRelativeTime(timestamp: number): string {
  const diff = Date.now() - timestamp;
  const seconds = Math.floor(diff / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) return `${days}d ago`;
  if (hours > 0) return `${hours}h ago`;
  if (minutes > 0) return `${minutes}m ago`;
  return 'just now';
}

/**
 * Format bytes as human-readable size
 */
function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)}KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`;
}

/**
 * Get bundle status string
 */
function getBundleStatus(entry: { expires_at: number; signature_verified: boolean } | null): {
  text: string;
  color: typeof chalk.green;
} {
  if (!entry) {
    return { text: 'not synced', color: chalk.yellow };
  }

  const isExpired = Date.now() > entry.expires_at;

  if (isExpired) {
    return { text: 'expired', color: chalk.yellow };
  }

  if (entry.signature_verified) {
    return { text: 'verified', color: chalk.green };
  }

  return { text: 'synced', color: chalk.cyan };
}

function resolveBundleName(bundle: PolicyBundleConfig): string {
  return bundle.name ?? deriveBundleName(bundle.url);
}

function toBundleAuthConfig(
  auth: PolicyBundleConfig['auth'] | undefined
): BundleAuthConfig | undefined {
  if (!auth) {
    return undefined;
  }

  if (auth.type === 'basic') {
    const basicAuth: BundleAuthConfig = {
      type: 'basic',
    };

    if (auth.username) {
      basicAuth.username = auth.username;
    }

    if (auth.password) {
      basicAuth.password_env = auth.password;
    }

    return basicAuth;
  }

  if (auth.type === 'bearer') {
    const bearerAuth: BundleAuthConfig = {
      type: 'bearer',
    };

    if (auth.token) {
      bearerAuth.token_env = auth.token;
    }

    return bearerAuth;
  }

  return undefined;
}

function toBundleConfig(
  bundle: PolicyBundleConfig,
  verification: PolicyVerificationConfig | undefined
): BundleConfig {
  const name = resolveBundleName(bundle);
  const config: BundleConfig = {
    name,
    url: bundle.url,
  };

  if (bundle.polling_interval !== undefined) {
    config.refresh_interval_ms = bundle.polling_interval;
  }

  const auth = toBundleAuthConfig(bundle.auth);
  if (auth) {
    config.auth = auth;
  }

  const signatureKey = verification?.keys?.[name];
  if (signatureKey) {
    config.signature_key = signatureKey;
  }

  return config;
}

/**
 * Create bundle list subcommand
 */
function createBundleListCommand(): Command {
  return new Command('list').description('List configured policy bundles').action(async () => {
    try {
      const workspaceRoot = getWorkspaceRoot();
      const configManager = new GateConfigManager(workspaceRoot);
      const config = configManager.loadConfig();

      const bundles = config.policy?.bundles || [];

      if (bundles.length === 0) {
        info('No policy bundles configured');
        print(chalk.dim('\nRun `anvil policy bundle add <url>` to add a bundle'));
        return;
      }

      // Initialize bundle manager to get cache status
      const bundleManager = getBundleManager();

      print(chalk.bold('\nConfigured Policy Bundles:\n'));

      // Print table header
      print(
        chalk.dim('  ') +
          chalk.bold('Name'.padEnd(20)) +
          chalk.bold('URL'.padEnd(40)) +
          chalk.bold('Last Sync'.padEnd(15)) +
          chalk.bold('Status')
      );
      print(chalk.dim('  ' + '-'.repeat(85)));

      // Print each bundle
      for (const bundle of bundles) {
        const bundleName = resolveBundleName(bundle);
        const entry = await bundleManager.getBundleEntry(bundleName);
        const lastSync = entry ? formatRelativeTime(entry.downloaded_at) : '-';
        const status = getBundleStatus(entry);
        const enabledIndicator = bundle.enabled === false ? chalk.dim('[disabled] ') : '';

        // Truncate URL if too long
        const maxUrlLen = 38;
        const displayUrl =
          bundle.url.length > maxUrlLen ? bundle.url.slice(0, maxUrlLen - 2) + '..' : bundle.url;

        print(
          '  ' +
            enabledIndicator +
            chalk.cyan(bundleName.padEnd(20 - enabledIndicator.length)) +
            chalk.dim(displayUrl.padEnd(40)) +
            chalk.dim(lastSync.padEnd(15)) +
            status.color(status.text)
        );

        if (entry) {
          print(chalk.dim(`      Size: ${formatSize(entry.size_bytes)}`));
        }
      }

      blank();
      success(`${bundles.length} bundle(s) configured`);
    } catch (err) {
      if (err instanceof CliError || err instanceof CliExit) throw err;
      error(`Failed to list bundles: ${err instanceof Error ? err.message : 'Unknown error'}`);
      throw new CliError('Failed to list bundles');
    }
  });
}

/**
 * Create bundle add subcommand
 */
function createBundleAddCommand(): Command {
  return new Command('add')
    .description('Add a remote policy bundle')
    .argument('<url>', 'URL of the bundle to add')
    .option('-n, --name <name>', 'Name for the bundle (defaults to URL basename)')
    .option('-r, --refresh <ms>', 'Refresh interval in milliseconds', '300000')
    .option('-k, --key <path>', 'Path to public key for signature verification')
    .option('--auth-user <username>', 'Username for basic authentication')
    .option('--auth-pass-env <envvar>', 'Environment variable containing password for basic auth')
    .option('--auth-token-env <envvar>', 'Environment variable containing bearer token')
    .option('--no-sync', 'Do not download the bundle immediately')
    .action(
      async (
        url: string,
        options: {
          name?: string;
          refresh?: string;
          key?: string;
          authUser?: string;
          authPassEnv?: string;
          authTokenEnv?: string;
          sync?: boolean;
        }
      ) => {
        const spinner = ora('Adding bundle configuration...').start();

        try {
          const workspaceRoot = getWorkspaceRoot();
          const configManager = new GateConfigManager(workspaceRoot);
          const config = configManager.loadConfig();

          // Initialize policy config if needed
          if (!config.policy) {
            config.policy = {};
          }
          if (!config.policy.bundles) {
            config.policy.bundles = [];
          }

          // Derive bundle name from URL if not provided
          const bundleName = options.name || deriveBundleName(url);

          // Check if bundle with this name already exists
          const existingIndex = config.policy.bundles.findIndex((b) => b.name === bundleName);
          if (existingIndex >= 0) {
            spinner.fail(`Bundle '${bundleName}' already exists`);
            print(chalk.dim('\nUse a different --name or remove the existing bundle first'));
            throw new CliError(`Bundle '${bundleName}' already exists`);
          }

          // Build bundle config
          const bundleConfig: PolicyBundleConfig = {
            name: bundleName,
            url,
            polling_interval: (() => {
              return coerceNonNegativeInt(options.refresh || '300000', '--refresh');
            })(),
            enabled: true,
          };

          // Add signature key if provided
          if (options.key) {
            if (!existsSync(options.key)) {
              spinner.fail(`Key file not found: ${options.key}`);
              throw new CliError(`Bundle signature key file not found: ${options.key}`);
            }
            const signatureKey = readFileSync(options.key, 'utf-8').trim();

            if (!config.policy.verification) {
              config.policy.verification = {};
            }
            if (!config.policy.verification.keys) {
              config.policy.verification.keys = {};
            }
            config.policy.verification.keys[bundleName] = signatureKey;
            config.policy.verification.require_signatures = true;
          }

          // Add auth config if provided
          if (options.authUser || options.authPassEnv || options.authTokenEnv) {
            const auth: PolicyBundleConfig['auth'] = {
              type: options.authTokenEnv ? 'bearer' : 'basic',
            };

            if (options.authUser) {
              auth.username = options.authUser;
            }
            if (options.authPassEnv) {
              auth.password = options.authPassEnv;
            }
            if (options.authTokenEnv) {
              auth.token = options.authTokenEnv;
            }

            bundleConfig.auth = auth;
          }

          // Add to config
          config.policy.bundles.push(bundleConfig);
          configManager.saveConfig(config);

          spinner.succeed(`Added bundle '${bundleName}'`);

          // Optionally sync immediately
          if (options.sync !== false) {
            const syncSpinner = ora('Downloading bundle...').start();

            try {
              const bundleManagerConfig = toBundleConfig(bundleConfig, config.policy?.verification);
              const bundleManager = new BundleManager({
                bundles: [bundleManagerConfig],
              });

              const result = await bundleManager.downloadBundle(bundleManagerConfig.name);

              if (result.success) {
                syncSpinner.succeed(`Bundle downloaded to ${result.path}`);
              } else {
                syncSpinner.warn(`Download failed: ${result.error}`);
                print(chalk.dim('\nRun `anvil policy bundle sync` to retry'));
              }
            } catch (syncErr) {
              syncSpinner.warn(
                `Download failed: ${syncErr instanceof Error ? syncErr.message : 'Unknown error'}`
              );
              print(chalk.dim('\nRun `anvil policy bundle sync` to retry'));
            }
          }

          blank();
          success('Bundle configuration saved to .anvilrc');
        } catch (err) {
          if (err instanceof CliError || err instanceof CliExit) throw err;
          spinner.fail('Failed to add bundle');
          error(err instanceof Error ? err.message : 'Unknown error');
          throw new CliError('Failed to add bundle');
        }
      }
    );
}

/**
 * Derive a bundle name from its URL
 */
function deriveBundleName(url: string): string {
  try {
    const parsed = new URL(url);
    const pathParts = parsed.pathname.split('/').filter(Boolean);
    const lastPart = pathParts[pathParts.length - 1] || '';

    // Remove common bundle extensions
    let name = lastPart
      .replace(/\.tar\.gz$/, '')
      .replace(/\.tgz$/, '')
      .replace(/\.bundle$/, '')
      .replace(/\.opa$/, '');

    // Fallback to hostname if path is empty
    if (!name) {
      name = parsed.hostname.replace(/\./g, '-');
    }

    return name;
  } catch {
    // If URL parsing fails, use a hash
    return `bundle-${Date.now().toString(36)}`;
  }
}

/**
 * Create bundle remove subcommand
 */
function createBundleRemoveCommand(): Command {
  return new Command('remove')
    .description('Remove a policy bundle')
    .argument('<name>', 'Name of the bundle to remove')
    .option('--keep-cache', 'Keep cached bundle files')
    .action(async (name: string, options: { keepCache?: boolean }) => {
      const spinner = ora(`Removing bundle '${name}'...`).start();

      try {
        const workspaceRoot = getWorkspaceRoot();
        const configManager = new GateConfigManager(workspaceRoot);
        const config = configManager.loadConfig();

        const bundles = config.policy?.bundles || [];
        const bundleIndex = bundles.findIndex((b) => resolveBundleName(b) === name);

        if (bundleIndex < 0) {
          spinner.fail(`Bundle '${name}' not found`);
          print(chalk.dim('\nUse `anvil policy bundle list` to see available bundles'));
          throw new CliError(`Bundle '${name}' not found for removal`);
        }

        const bundleName = resolveBundleName(bundles[bundleIndex]);

        // Remove from config
        bundles.splice(bundleIndex, 1);
        if (config.policy) {
          config.policy.bundles = bundles;
        }
        configManager.saveConfig(config);

        // Clear cache unless --keep-cache
        if (!options.keepCache) {
          const bundleManager = getBundleManager();
          await bundleManager.invalidateBundle(bundleName);
          spinner.succeed(`Removed bundle '${bundleName}' and cleared cache`);
        } else {
          spinner.succeed(`Removed bundle '${bundleName}' (cache preserved)`);
        }

        success('Bundle configuration updated');
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        spinner.fail('Failed to remove bundle');
        error(err instanceof Error ? err.message : 'Unknown error');
        throw new CliError('Failed to remove bundle');
      }
    });
}

/**
 * Create bundle sync subcommand
 */
function createBundleSyncCommand(): Command {
  return new Command('sync')
    .description('Download or update policy bundles')
    .option('-f, --force', 'Force re-download even if cached')
    .option('-n, --name <name>', 'Sync only a specific bundle')
    .action(async (options: { force?: boolean; name?: string }) => {
      const spinner = ora('Syncing policy bundles...').start();

      try {
        const workspaceRoot = getWorkspaceRoot();
        const configManager = new GateConfigManager(workspaceRoot);
        const config = configManager.loadConfig();

        let bundles = config.policy?.bundles || [];
        const verification = config.policy?.verification;

        if (bundles.length === 0) {
          spinner.warn('No bundles configured');
          print(chalk.dim('\nRun `anvil policy bundle add <url>` to add a bundle'));
          return;
        }

        // Filter to specific bundle if --name provided
        if (options.name) {
          bundles = bundles.filter((b) => resolveBundleName(b) === options.name);
          if (bundles.length === 0) {
            spinner.fail(`Bundle '${options.name}' not found`);
            throw new CliError(`Bundle '${options.name}' not found for sync`);
          }
        }

        // Filter out disabled bundles
        const enabledBundles = bundles.filter((b) => b.enabled !== false);

        if (enabledBundles.length === 0) {
          spinner.warn('All bundles are disabled');
          return;
        }

        const bundleConfigs = enabledBundles.map((bundle) => toBundleConfig(bundle, verification));

        // If force, clear cache first
        if (options.force) {
          const bundleManager = getBundleManager();
          for (const bundle of bundleConfigs) {
            await bundleManager.invalidateBundle(bundle.name);
          }
        }

        // Create bundle manager with configured bundles
        const bundleManager = new BundleManager({
          bundles: bundleConfigs,
        });

        spinner.text = `Syncing ${bundleConfigs.length} bundle(s)...`;

        const results = await bundleManager.syncAll();

        spinner.stop();

        // Display results
        print(chalk.bold('\nBundle Sync Results:\n'));

        let successCount = 0;
        let failCount = 0;

        for (const result of results) {
          if (result.success) {
            successCount++;
            const updateStatus = result.updated ? chalk.green('updated') : chalk.dim('unchanged');
            print(`  ${chalk.green('✓')} ${result.name}: ${updateStatus}`);
            if (result.path) {
              print(chalk.dim(`      Path: ${result.path}`));
            }
          } else {
            failCount++;
            print(`  ${chalk.red('✗')} ${result.name}: ${chalk.red(result.error || 'Failed')}`);
          }
        }

        blank();

        if (failCount === 0) {
          success(`All ${successCount} bundle(s) synced successfully`);
        } else if (successCount > 0) {
          warning(`${successCount} succeeded, ${failCount} failed`);
        } else {
          error(`All ${failCount} bundle(s) failed to sync`);
          throw new CliError('All bundles failed to sync');
        }
      } catch (err) {
        if (err instanceof CliError || err instanceof CliExit) throw err;
        spinner.fail('Bundle sync failed');
        error(err instanceof Error ? err.message : 'Unknown error');
        throw new CliError('Bundle sync failed');
      }
    });
}
