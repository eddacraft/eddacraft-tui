/**
 * Policy Command - Manage OPA/Rego policies for Anvil
 */

import { Command } from 'commander';
import chalk from 'chalk';
import ora from 'ora';
import { existsSync, mkdirSync, readdirSync, copyFileSync, writeFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { getWorkspaceRoot } from '../utils/file-io.js';
import { success, error, info, warning } from '../utils/output.js';
import { PolicyLoader, OPAExecutor, getOPABinaryManager } from '@anvil/core';

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

export function createPolicyCommand(): Command {
  const command = new Command('policy');

  command.description('Manage OPA/Rego policies');

  // List subcommand
  command
    .command('list')
    .description('List active policies')
    .option('-d, --dir <directory>', 'Policy directory', DEFAULT_POLICY_DIR)
    .action(async (options: { dir: string }) => {
      try {
        const workspaceRoot = getWorkspaceRoot();
        const policyDir = options.dir;

        const loader = new PolicyLoader();
        const result = await loader.loadPolicies(workspaceRoot, { policyDir });

        if (result.policies.length === 0) {
          info(`No policies found in ${result.directory}`);
          console.log(chalk.dim('\nRun `anvil policy init` to create example policies'));
          return;
        }

        console.log(chalk.bold('\nActive Policies:\n'));

        // Create table
        const table = result.policies.map((p) => ({
          name: p.name,
          package: p.package,
          tests: p.hasTests ? chalk.green('✓') : chalk.dim('-'),
        }));

        // Print table header
        console.log(
          chalk.dim('  ') +
            chalk.bold('Name'.padEnd(25)) +
            chalk.bold('Package'.padEnd(35)) +
            chalk.bold('Tests')
        );
        console.log(chalk.dim('  ' + '─'.repeat(70)));

        // Print rows
        for (const row of table) {
          console.log(
            '  ' + chalk.cyan(row.name.padEnd(25)) + chalk.dim(row.package.padEnd(35)) + row.tests
          );
        }

        console.log('');
        success(`Found ${result.policies.length} policies`);

        if (result.errors.length > 0) {
          console.log('');
          warning(`${result.errors.length} policies failed to load:`);
          for (const err of result.errors) {
            console.log(chalk.red(`  • ${err.path}: ${err.error}`));
          }
        }
      } catch (err) {
        error(`Failed to list policies: ${err instanceof Error ? err.message : 'Unknown error'}`);
        process.exit(1);
      }
    });

  // Validate subcommand
  command
    .command('validate <file>')
    .description('Validate Rego syntax for a policy file')
    .action(async (file: string) => {
      const spinner = ora('Validating policy syntax...').start();

      try {
        const binaryManager = getOPABinaryManager();
        const binaryPath = await binaryManager.ensureBinary();

        const { readFile } = await import('fs/promises');
        const content = await readFile(file, 'utf-8');

        const executor = new OPAExecutor(binaryPath);
        const result = await executor.validateSyntax(content);

        if (result.valid) {
          spinner.succeed(chalk.green('Policy syntax is valid'));
        } else {
          spinner.fail(chalk.red('Policy syntax is invalid'));
          for (const err of result.errors) {
            console.log(chalk.red(`  • ${err}`));
          }
          process.exit(1);
        }
      } catch (err) {
        spinner.fail('Validation failed');
        error(err instanceof Error ? err.message : 'Unknown error');
        process.exit(1);
      }
    });

  // Test subcommand
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
          console.log(chalk.dim('\nRun `anvil policy init` to create example policies'));
          return;
        }

        // Filter by policy name if specified
        let policies = discoveryResult.policies;
        if (policy) {
          policies = policies.filter((p) => p.name === policy || p.name.includes(policy));
          if (policies.length === 0) {
            spinner.fail(`Policy '${policy}' not found`);
            process.exit(1);
          }
        }

        // Find test files
        const testFiles = loader.findTestFiles(discoveryResult.directory);
        if (testFiles.length === 0) {
          spinner.warn('No test files found');
          console.log(chalk.dim('\nCreate *_test.rego files to add tests'));
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
          console.log('');
          for (const detail of result.details) {
            const icon = detail.passed ? chalk.green('✓') : chalk.red('✗');
            console.log(`  ${icon} ${detail.name}`);
            if (detail.message) {
              console.log(chalk.dim(`      ${detail.message}`));
            }
          }
        }

        if (result.errors.length > 0) {
          console.log('');
          warning('Errors occurred:');
          for (const err of result.errors) {
            console.log(chalk.red(`  • ${err}`));
          }
        }

        if (!allPassed) {
          process.exit(1);
        }
      } catch (err) {
        spinner.fail('Test run failed');
        error(err instanceof Error ? err.message : 'Unknown error');
        process.exit(1);
      }
    });

  // Init subcommand
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
            console.log(chalk.dim('\nUse --force to overwrite existing policies'));
            process.exit(1);
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

        console.log('\n' + chalk.bold('Created policies:'));
        console.log(
          chalk.cyan('  • coverage_min.rego') + chalk.dim(' - Enforce minimum test coverage')
        );
        console.log(chalk.cyan('  • change_scope.rego') + chalk.dim(' - Limit files per change'));
        console.log(
          chalk.cyan('  • security_baseline.rego') + chalk.dim(' - Security review requirements')
        );

        console.log('\n' + chalk.bold('Next steps:'));
        console.log(chalk.dim('  1. Review and customise policies in ') + chalk.cyan(options.dir));
        console.log(chalk.dim('  2. List policies: ') + chalk.cyan('anvil policy list'));
        console.log(chalk.dim('  3. Run policy tests: ') + chalk.cyan('anvil policy test'));
        console.log(chalk.dim('  4. Enable policy check in .anvilrc'));

        // Show .anvilrc snippet
        console.log('\n' + chalk.bold('Add to .anvilrc:'));
        console.log(
          chalk.dim(`  {
    "name": "policy",
    "enabled": true,
    "config": {
      "policy_dir": "${options.dir}",
      "severity_threshold": "error"
    }
  }`)
        );

        console.log('');
        success('Policy directory initialised!');
      } catch (err) {
        error(
          `Failed to initialise policies: ${err instanceof Error ? err.message : 'Unknown error'}`
        );
        process.exit(1);
      }
    });

  return command;
}
