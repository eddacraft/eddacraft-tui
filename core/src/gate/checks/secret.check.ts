import { BaseCheck } from '../check.interface.js';
import { CheckContext, GateResult, getFilesFromContext } from '../../types/gate.types.js';
import { readFileSync, existsSync } from 'fs';
import { SECRET_PATTERNS, PatternMatcher } from './secret/secret-patterns.js';
import { EntropyDetector } from './secret/entropy-detector.js';
import { GitScanner } from './secret/git-scanner.js';

export interface SecretFinding {
  file: string;
  line: number;
  type: string;
  match: string;
  context: string;
  entropy?: number;
  source?: 'pattern' | 'entropy' | 'git-history';
}

export interface SecretCheckConfig {
  /** Enable entropy-based detection (default: true) */
  enable_entropy?: boolean;
  /** Minimum entropy threshold for detection (default: 4.5) */
  entropy_threshold?: number;
  /** Minimum string length for entropy analysis (default: 16) */
  min_entropy_length?: number;
  /** Enable git history scanning (default: false) */
  scan_git_history?: boolean;
  /** Number of commits to scan in git history (default: 10) */
  git_history_depth?: number;
  /** Patterns to allowlist (reduce false positives) */
  allowlist?: string[];
  /** File extensions to skip */
  skip_extensions?: string[];
}

/** File extensions to skip by default */
const DEFAULT_SKIP_EXTENSIONS = [
  '.lock',
  '.min.js',
  '.min.css',
  '.map',
  '.svg',
  '.png',
  '.jpg',
  '.jpeg',
  '.gif',
  '.ico',
  '.woff',
  '.woff2',
  '.ttf',
  '.eot',
];

export class SecretCheck extends BaseCheck {
  name = 'secret';
  description = 'Scan for potential secrets and sensitive data using patterns and entropy analysis';

  private matcher = new PatternMatcher();
  private entropyDetector = new EntropyDetector();
  private gitScanner = new GitScanner();

  async run(context: CheckContext): Promise<GateResult> {
    try {
      const config = this.getConfig(context);
      const files = context.fullScan
        ? await this.getFilesForFullScan(context.workspace_root, config)
        : this.getFilesFromPlan(context);
      const findings: SecretFinding[] = [];

      // Scan files for pattern-based secrets
      for (const file of files) {
        if (this.shouldSkipFile(file, config)) {
          continue;
        }

        if (existsSync(file)) {
          const content = readFileSync(file, 'utf-8');
          const fileFindings = this.scanFileContent(file, content, context.workspace_root, config);
          findings.push(...fileFindings);
        }
      }

      // Scan git history if enabled
      if (config.scan_git_history) {
        const historyFindings = await this.gitScanner.scanGitHistory(context.workspace_root, {
          git_history_depth: config.git_history_depth ?? 10,
          allowlist: config.allowlist ?? [],
        });
        findings.push(...historyFindings);
      }

      // Deduplicate findings
      const uniqueFindings = this.deduplicateFindings(findings);

      const passed = uniqueFindings.length === 0;
      const patternCount = uniqueFindings.filter((f) => f.source === 'pattern').length;
      const entropyCount = uniqueFindings.filter((f) => f.source === 'entropy').length;
      const gitHistoryCount = uniqueFindings.filter((f) => f.source === 'git-history').length;

      let message: string;
      if (passed) {
        message = 'No secrets detected';
      } else {
        const parts: string[] = [];
        if (patternCount > 0) parts.push(`${patternCount} pattern match(es)`);
        if (entropyCount > 0) parts.push(`${entropyCount} high-entropy string(s)`);
        if (gitHistoryCount > 0) parts.push(`${gitHistoryCount} in git history`);
        message = `Found ${uniqueFindings.length} potential secret(s): ${parts.join(', ')}`;
      }

      return this.createResult(
        passed,
        message,
        passed ? 100 : Math.max(0, 100 - uniqueFindings.length * 10),
        {
          findings: uniqueFindings,
          summary: {
            total: uniqueFindings.length,
            by_pattern: patternCount,
            by_entropy: entropyCount,
            by_git_history: gitHistoryCount,
          },
        }
      );
    } catch (error) {
      return this.createFailure(
        'Secret scan failed',
        error instanceof Error ? error.message : 'Unknown error'
      );
    }
  }

  /**
   * Get configuration with defaults
   */
  private getConfig(context: CheckContext): SecretCheckConfig {
    const checkConfig = context.check_config as SecretCheckConfig;
    return {
      enable_entropy: checkConfig.enable_entropy ?? true,
      entropy_threshold: checkConfig.entropy_threshold ?? 4.5,
      min_entropy_length: checkConfig.min_entropy_length ?? 16,
      scan_git_history: checkConfig.scan_git_history ?? false,
      git_history_depth: checkConfig.git_history_depth ?? 10,
      allowlist: checkConfig.allowlist ?? [],
      skip_extensions: checkConfig.skip_extensions ?? [],
    };
  }

  /**
   * Check if a file should be skipped based on extension
   */
  private shouldSkipFile(file: string, config: SecretCheckConfig): boolean {
    const skipExtensions = [...DEFAULT_SKIP_EXTENSIONS, ...(config.skip_extensions || [])];
    return skipExtensions.some((ext) => file.endsWith(ext));
  }

  /**
   * Scan file content for secrets using both patterns and entropy
   */
  private scanFileContent(
    file: string,
    content: string,
    workspaceRoot: string,
    config: SecretCheckConfig
  ): SecretFinding[] {
    const findings: SecretFinding[] = [];
    const lines = content.split('\n');

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const lineNumber = i + 1;

      // Pattern-based detection
      for (const pattern of SECRET_PATTERNS) {
        const matches = line.match(pattern.pattern);
        if (matches && !this.matcher.isAllowlisted(matches[0], config.allowlist ?? [])) {
          findings.push({
            file: file.replace(workspaceRoot, ''),
            line: lineNumber,
            type: pattern.name,
            match: this.matcher.redactSecret(matches[0]),
            context: this.matcher.redactLine(line.trim()),
            source: 'pattern',
          });
        }
      }

      // Entropy-based detection (if enabled)
      if (config.enable_entropy) {
        const entropyFindings = this.entropyDetector.detectHighEntropyStrings(
          line,
          lineNumber,
          file.replace(workspaceRoot, ''),
          {
            entropy_threshold: config.entropy_threshold ?? 4.5,
            min_entropy_length: config.min_entropy_length ?? 16,
            allowlist: config.allowlist ?? [],
          }
        );

        // Filter out findings that were already detected by pattern matching
        const newEntropyFindings = entropyFindings.filter(() => {
          const alreadyDetected = SECRET_PATTERNS.some((p) => p.pattern.test(line));
          return !alreadyDetected;
        });

        findings.push(...newEntropyFindings);
      }
    }

    return findings;
  }

  /**

   * Remove duplicate findings
   */
  private deduplicateFindings(findings: SecretFinding[]): SecretFinding[] {
    const seen = new Set<string>();
    return findings.filter((finding) => {
      const key = `${finding.file}:${finding.line}:${finding.type}:${finding.match}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
  }

  private getFilesFromPlan(context: CheckContext): string[] {
    // Use unified helper for both planless and plan-based modes
    // This ensures consistent path normalisation and existence checking
    return getFilesFromContext(context);
  }

  /**
   * Get all files for full codebase scan
   */
  private async getFilesForFullScan(
    workspaceRoot: string,
    config: SecretCheckConfig
  ): Promise<string[]> {
    const { glob } = await import('glob');
    const files: string[] = [];

    // Scan common file types that might contain secrets
    const patterns = [
      '**/*.ts',
      '**/*.tsx',
      '**/*.js',
      '**/*.jsx',
      '**/*.json',
      '**/*.yaml',
      '**/*.yml',
      '**/*.env*',
      '**/*.config.*',
    ];

    for (const pattern of patterns) {
      try {
        const matches = await glob(pattern, {
          cwd: workspaceRoot,
          absolute: true,
          ignore: [
            '**/node_modules/**',
            '**/dist/**',
            '**/build/**',
            '**/.git/**',
            '**/coverage/**',
            '**/*.lock',
            '**/package-lock.json',
          ],
        });
        files.push(...matches);
      } catch (error) {
        // Log glob errors for debugging but continue scanning
        console.debug(`[SecretCheck] Glob error for pattern ${pattern}:`, error);
      }
    }

    // Filter out skipped extensions
    return files.filter((file) => !this.shouldSkipFile(file, config));
  }
}
