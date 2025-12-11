/**
 * Policy Loader - Discover and load Rego policy files
 */

import { existsSync, readdirSync, statSync } from 'fs';
import { readFile } from 'fs/promises';
import { join, basename, extname } from 'path';

/**
 * Default policy directory relative to workspace root
 */
const DEFAULT_POLICY_DIR = '.anvil/policies';

/**
 * Represents a loaded policy file
 */
export interface LoadedPolicy {
  /** Policy name (filename without extension) */
  name: string;
  /** Full path to the .rego file */
  path: string;
  /** Policy source code */
  content: string;
  /** Rego package name extracted from source */
  package: string;
  /** Whether a corresponding _test.rego file exists */
  hasTests: boolean;
}

/**
 * Result of policy discovery
 */
export interface PolicyDiscoveryResult {
  /** Successfully loaded policies */
  policies: LoadedPolicy[];
  /** Files that failed to load with error messages */
  errors: Array<{ path: string; error: string }>;
  /** Policy directory path */
  directory: string;
}

/**
 * Configuration for policy loading
 */
export interface PolicyLoaderConfig {
  /** Custom policy directory (relative to workspace root) */
  policyDir?: string;
  /** Only load specific policies by name */
  enabledPolicies?: string[];
  /** Exclude specific policies by name */
  disabledPolicies?: string[];
}

/**
 * Discovers and loads Rego policy files
 */
export class PolicyLoader {
  /**
   * Load all policies from the policy directory
   */
  async loadPolicies(
    workspaceRoot: string,
    config: PolicyLoaderConfig = {}
  ): Promise<PolicyDiscoveryResult> {
    const policyDir = config.policyDir || DEFAULT_POLICY_DIR;
    const fullPolicyDir = join(workspaceRoot, policyDir);

    const result: PolicyDiscoveryResult = {
      policies: [],
      errors: [],
      directory: fullPolicyDir,
    };

    // Check if policy directory exists
    if (!existsSync(fullPolicyDir)) {
      return result;
    }

    // Find all .rego files (excluding _test.rego)
    const regoFiles = this.findRegoFiles(fullPolicyDir);

    // Load each policy
    for (const filePath of regoFiles) {
      try {
        const policy = await this.loadPolicy(filePath);

        // Apply filters
        if (this.shouldIncludePolicy(policy.name, config)) {
          result.policies.push(policy);
        }
      } catch (error) {
        result.errors.push({
          path: filePath,
          error: error instanceof Error ? error.message : 'Unknown error',
        });
      }
    }

    return result;
  }

  /**
   * Load a single policy file
   */
  async loadPolicy(filePath: string): Promise<LoadedPolicy> {
    const content = await readFile(filePath, 'utf-8');
    const name = this.extractPolicyName(filePath);
    const packageName = this.extractPackageName(content);
    const hasTests = this.hasTestFile(filePath);

    return {
      name,
      path: filePath,
      content,
      package: packageName,
      hasTests,
    };
  }

  /**
   * Find all .rego files in a directory (excluding test files)
   */
  private findRegoFiles(directory: string): string[] {
    const files: string[] = [];

    const entries = readdirSync(directory);
    for (const entry of entries) {
      const fullPath = join(directory, entry);
      const stat = statSync(fullPath);

      if (stat.isDirectory()) {
        // Recursively search subdirectories
        files.push(...this.findRegoFiles(fullPath));
      } else if (stat.isFile() && this.isRegoFile(entry) && !this.isTestFile(entry)) {
        files.push(fullPath);
      }
    }

    return files;
  }

  /**
   * Check if a filename is a .rego file
   */
  private isRegoFile(filename: string): boolean {
    return extname(filename).toLowerCase() === '.rego';
  }

  /**
   * Check if a filename is a test file
   */
  private isTestFile(filename: string): boolean {
    return filename.endsWith('_test.rego');
  }

  /**
   * Extract policy name from file path
   */
  private extractPolicyName(filePath: string): string {
    const filename = basename(filePath);
    return filename.replace(/\.rego$/, '');
  }

  /**
   * Extract package name from Rego source
   */
  private extractPackageName(content: string): string {
    // Match package declaration: package anvil.policies.coverage_min
    const match = content.match(/^\s*package\s+([\w.]+)/m);
    return match ? match[1] : 'unknown';
  }

  /**
   * Check if a corresponding test file exists
   */
  private hasTestFile(policyPath: string): boolean {
    const testPath = policyPath.replace(/\.rego$/, '_test.rego');
    return existsSync(testPath);
  }

  /**
   * Determine if a policy should be included based on filters
   */
  private shouldIncludePolicy(policyName: string, config: PolicyLoaderConfig): boolean {
    // Check disabled list first
    if (config.disabledPolicies?.includes(policyName)) {
      return false;
    }

    // If enabled list is specified, policy must be in it
    if (config.enabledPolicies && config.enabledPolicies.length > 0) {
      return config.enabledPolicies.includes(policyName);
    }

    return true;
  }

  /**
   * Get all test files for policies in a directory
   */
  findTestFiles(directory: string): string[] {
    const files: string[] = [];

    if (!existsSync(directory)) {
      return files;
    }

    const entries = readdirSync(directory);
    for (const entry of entries) {
      const fullPath = join(directory, entry);
      const stat = statSync(fullPath);

      if (stat.isDirectory()) {
        files.push(...this.findTestFiles(fullPath));
      } else if (stat.isFile() && this.isTestFile(entry)) {
        files.push(fullPath);
      }
    }

    return files;
  }
}
