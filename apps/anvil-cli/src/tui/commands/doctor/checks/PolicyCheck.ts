/**
 * Policy-related diagnostic checks for `anvil doctor`
 *
 * Checks for:
 * - Policy config (.anvil/config.yml) exists
 * - Policy directory has .rego files
 * - Policies have reasons/owners documented
 * - Policy tests exist
 * - Org source version is pinned
 */

import * as fs from 'node:fs';
import * as path from 'node:path';

import type { DiagnosticCheck, DiagnosticContext, DiagnosticResult, FixResult } from '../types.js';

export class PolicyConfigCheck implements DiagnosticCheck {
  readonly id = 'policy-config';
  readonly name = 'Policy Configuration';
  readonly description = 'Verifies .anvil/config.yml exists with policy definitions';

  async run(context: DiagnosticContext): Promise<DiagnosticResult> {
    const configPath = path.join(context.projectRoot, '.anvil', 'config.yml');

    if (fs.existsSync(configPath)) {
      return {
        checkId: this.id,
        name: this.name,
        status: 'pass',
        message: '.anvil/config.yml found',
        fixable: false,
      };
    }

    return {
      checkId: this.id,
      name: this.name,
      status: 'warn',
      message: 'No .anvil/config.yml — policy metadata (reasons, owners) not configured',
      fixable: false,
      suggestion: 'Run: anvil init --org <name> or create .anvil/config.yml manually',
    };
  }
}

export class PolicyDirectoryCheck implements DiagnosticCheck {
  readonly id = 'policy-directory';
  readonly name = 'Policy Files';
  readonly description = 'Checks for .rego policy files in .anvil/policies/';

  async run(context: DiagnosticContext): Promise<DiagnosticResult> {
    const policyDir = path.join(context.projectRoot, '.anvil', 'policies');

    if (!fs.existsSync(policyDir)) {
      return {
        checkId: this.id,
        name: this.name,
        status: 'warn',
        message: 'No .anvil/policies/ directory',
        fixable: true,
        suggestion: 'Run: anvil policy init',
      };
    }

    const regoFiles = fs
      .readdirSync(policyDir)
      .filter((f) => f.endsWith('.rego') && !f.endsWith('_test.rego'));
    const testFiles = fs.readdirSync(policyDir).filter((f) => f.endsWith('_test.rego'));

    if (regoFiles.length === 0) {
      return {
        checkId: this.id,
        name: this.name,
        status: 'warn',
        message: 'Policy directory exists but contains no .rego files',
        fixable: true,
        suggestion: 'Run: anvil policy init',
      };
    }

    const untestedPolicies = regoFiles.filter((f) => {
      const testName = f.replace('.rego', '_test.rego');
      return !testFiles.includes(testName);
    });

    if (untestedPolicies.length > 0) {
      return {
        checkId: this.id,
        name: this.name,
        status: 'warn',
        message: `${regoFiles.length} policies found, ${untestedPolicies.length} missing tests`,
        fixable: false,
        details: `Untested: ${untestedPolicies.join(', ')}`,
        suggestion: 'Create _test.rego files for untested policies',
      };
    }

    return {
      checkId: this.id,
      name: this.name,
      status: 'pass',
      message: `${regoFiles.length} policies, all with tests`,
      fixable: false,
    };
  }

  async fix(_context: DiagnosticContext): Promise<FixResult> {
    // We could run `anvil policy init` but that's better left to the user
    return {
      success: false,
      message: 'Run `anvil policy init` to create starter policies',
    };
  }
}

export class PolicyDocumentationCheck implements DiagnosticCheck {
  readonly id = 'policy-docs';
  readonly name = 'Policy Documentation';
  readonly description = 'Checks that policies have reasons and owners documented';

  async run(context: DiagnosticContext): Promise<DiagnosticResult> {
    const configPath = path.join(context.projectRoot, '.anvil', 'config.yml');

    if (!fs.existsSync(configPath)) {
      return {
        checkId: this.id,
        name: this.name,
        status: 'skip',
        message: 'Skipped (no .anvil/config.yml)',
        fixable: false,
      };
    }

    try {
      const content = fs.readFileSync(configPath, 'utf-8');

      // Simple YAML checks without parsing (avoids dependency)
      const hasTeamPolicies = content.includes('team:');
      const hasReasons = content.includes('reason:');
      const hasOwners = content.includes('owner:');

      if (!hasTeamPolicies) {
        return {
          checkId: this.id,
          name: this.name,
          status: 'warn',
          message: 'No team policies defined in config.yml',
          fixable: false,
          suggestion: 'Add team policies with reasons and owners to .anvil/config.yml',
        };
      }

      if (!hasReasons || !hasOwners) {
        const missing: string[] = [];
        if (!hasReasons) missing.push('reasons');
        if (!hasOwners) missing.push('owners');

        return {
          checkId: this.id,
          name: this.name,
          status: 'warn',
          message: `Team policies missing: ${missing.join(', ')}`,
          fixable: false,
          suggestion: 'Add "reason" and "owner" fields to policy entries in .anvil/config.yml',
        };
      }

      return {
        checkId: this.id,
        name: this.name,
        status: 'pass',
        message: 'Policies have reasons and owners documented',
        fixable: false,
      };
    } catch {
      return {
        checkId: this.id,
        name: this.name,
        status: 'warn',
        message: 'Could not parse .anvil/config.yml',
        fixable: false,
      };
    }
  }
}

export class PolicyOrgVersionCheck implements DiagnosticCheck {
  readonly id = 'policy-org-version';
  readonly name = 'Org Policy Version';
  readonly description = 'Checks that org policy source has a pinned version ref';

  async run(context: DiagnosticContext): Promise<DiagnosticResult> {
    const configPath = path.join(context.projectRoot, '.anvil', 'config.yml');

    if (!fs.existsSync(configPath)) {
      return {
        checkId: this.id,
        name: this.name,
        status: 'skip',
        message: 'Skipped (no .anvil/config.yml)',
        fixable: false,
      };
    }

    try {
      const content = fs.readFileSync(configPath, 'utf-8');

      if (!content.includes('org:')) {
        return {
          checkId: this.id,
          name: this.name,
          status: 'skip',
          message: 'No org source configured',
          fixable: false,
        };
      }

      if (!content.includes('ref:')) {
        return {
          checkId: this.id,
          name: this.name,
          status: 'warn',
          message: 'Org source has no pinned version ref',
          fixable: false,
          suggestion: 'Add a "ref" field (e.g., "v1.0.0") to pin the org policy version',
        };
      }

      return {
        checkId: this.id,
        name: this.name,
        status: 'pass',
        message: 'Org source version pinned',
        fixable: false,
      };
    } catch {
      return {
        checkId: this.id,
        name: this.name,
        status: 'warn',
        message: 'Could not check org policy version',
        fixable: false,
      };
    }
  }
}
