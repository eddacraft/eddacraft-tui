import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

export const DEFAULT_POLICY_DIR = '.anvil/policies';

export function getExamplePoliciesPath(): string {
  const currentDir = dirname(fileURLToPath(import.meta.url));
  const possiblePaths = [
    // From bundled dist/index.js: up 3 levels to repo root
    join(currentDir, '../../../packages/anvil/runtime/src/gate/__fixtures__/policies'),
    // From source: up 5 levels to repo root
    join(currentDir, '../../../../../packages/anvil/runtime/src/gate/__fixtures__/policies'),
  ];

  for (const path of possiblePaths) {
    if (existsSync(path)) {
      return path;
    }
  }

  return '';
}

export const EXAMPLE_POLICIES = {
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
