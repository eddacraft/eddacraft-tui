import { BaseCheck } from '../check.interface.js';
import { CheckContext, GateResult, getFilesFromContext } from '../../types/gate.types.js';
import { readFileSync, existsSync } from 'fs';
import { join } from 'path';
import { spawn } from 'child_process';
import { z } from 'zod';
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

/**
 * Zod schema for SecretCheckConfig with runtime validation
 */
export const SecretCheckConfigSchema = z.object({
  /** Enable entropy-based detection (default: true) */
  enable_entropy: z.boolean().optional(),
  /** Minimum entropy threshold for detection (default: 4.5) */
  entropy_threshold: z.number().optional(),
  /** Minimum string length for entropy analysis (default: 16) */
  min_entropy_length: z.number().optional(),
  /** Enable git history scanning (default: false) */
  scan_git_history: z.boolean().optional(),
  /** Number of commits to scan in git history (default: 10) */
  git_history_depth: z.number().optional(),
  /** Patterns to allowlist (reduce false positives) */
  allowlist: z.array(z.string()).optional(),
  /** File extensions to skip */
  skip_extensions: z.array(z.string()).optional(),
});

export type SecretCheckConfig = z.infer<typeof SecretCheckConfigSchema>;

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
    // Parse and validate check_config using Zod schema
    const parseResult = SecretCheckConfigSchema.safeParse(context.check_config);

    // If parsing fails, use empty config (all defaults will be applied)
    const checkConfig = parseResult.success ? parseResult.data : {};

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
        }
      }
    }

    return findings;
  }

  /**
   * Scan git history for secrets in recent commits
   * Uses spawn with argument arrays to prevent shell injection
   */
  private async scanGitHistory(
    workspaceRoot: string,
    config: SecretCheckConfig
  ): Promise<SecretFinding[]> {
    const findings: SecretFinding[] = [];
    const depth = config.git_history_depth ?? 10;

    try {
      const isGitRepo = existsSync(join(workspaceRoot, '.git'));
      if (!isGitRepo) {
        return findings;
      }

      const stdout = await this.executeGitCommand(
        [
          'log',
          '-p',
          `-${depth}`,
          '--all',
          '--diff-filter=A',
          '--',
          '*.ts',
          '*.js',
          '*.json',
          '*.env*',
          '*.yaml',
          '*.yml',
        ],
        workspaceRoot
      );

      const commitBlocks = stdout.split(/^commit /m).slice(1);

      for (const block of commitBlocks) {
        const commitMatch = block.match(/^([a-f0-9]+)/);
        const commitHash = commitMatch ? commitMatch[1].substring(0, 8) : 'unknown';

        const addedLines = block
          .split('\n')
          .filter((line) => line.startsWith('+') && !line.startsWith('+++'));

        for (const addedLine of addedLines) {
          const lineContent = addedLine.substring(1);

          for (const pattern of this.secretPatterns) {
            const matches = lineContent.match(pattern.pattern);
            if (matches && !this.isAllowlisted(matches[0], config)) {
              findings.push({
                file: `git-history:${commitHash}`,
                line: 0,
                type: `${pattern.name} (in git history)`,
                match: this.redactSecret(matches[0]),
                context: this.redactLine(lineContent.trim()),
                source: 'git-history',
              });
            }
          }
        }
      }
    } catch {
      // Git command failed - not a git repo or git unavailable
    }

    return findings;
  }

  /**
   * Execute git command safely using spawn (prevents shell injection)
   */
  private executeGitCommand(args: string[], cwd: string): Promise<string> {
    return new Promise((resolve, reject) => {
      const child = spawn('git', args, {
        cwd,
        stdio: ['pipe', 'pipe', 'pipe'],
      });

      let stdout = '';
      let stderr = '';

      child.stdout.on('data', (data: Buffer) => {
        stdout += data.toString('utf8');
      });

      child.stderr.on('data', (data: Buffer) => {
        stderr += data.toString('utf8');
      });

      child.on('error', (error: Error) => {
        reject(new Error(`Git command failed: ${error.message}`));
      });

      child.on('close', (code: number | null) => {
        if (code !== 0) {
          reject(new Error(stderr || `Git exited with code ${code}`));
          return;
        }
        resolve(stdout);
      });
    });
  }

  private isAllowlisted(str: string, config: SecretCheckConfig): boolean {
    const MAX_INPUT_LENGTH = 1000;
    const truncatedStr = str.length > MAX_INPUT_LENGTH ? str.substring(0, MAX_INPUT_LENGTH) : str;

    for (const pattern of this.defaultAllowlist) {
      if (pattern.test(truncatedStr)) return true;
    }

    for (const patternStr of config.allowlist || []) {
      if (this.isSafeRegexPattern(patternStr)) {
        try {
          if (new RegExp(patternStr, 'i').test(truncatedStr)) return true;
        } catch {
          continue;
        }
      }
    }

    return false;
  }

  private isSafeRegexPattern(pattern: string): boolean {
    const MAX_PATTERN_LENGTH = 200;
    if (pattern.length > MAX_PATTERN_LENGTH) {
      return false;
    }

    const dangerousPatterns = [
      /\(\?[^)]*\+[^)]*\)\+/,
      /\([^)]+\+\)\+/,
      /\([^)]+\*\)\+/,
      /\([^)]+\+\)\*/,
      /\([^)]+\*\)\*/,
      /\(\.\*\)\{/,
      /\(\.\+\)\{/,
    ];

    return !dangerousPatterns.some((dp) => dp.test(pattern));
  }

  /**
   * Check if a string looks like code (not a secret)
   */
  private looksLikeCode(str: string): boolean {
    // Common code patterns that aren't secrets
    const codePatterns = [
      /^[a-z][a-zA-Z0-9]*\(/, // Function call
      /^[a-z][a-zA-Z0-9]*\.[a-z]/, // Method chain
      /^https?:\/\//, // URLs
      /^[a-z]+:\/\//, // Protocol URLs
      /\.(js|ts|css|html|json|md|txt)$/, // File extensions
      /^[A-Z][A-Z0-9_]+$/, // Constants (all caps)
      /\s+/, // Contains whitespace
      /^[a-z][a-z0-9]*[A-Z]/, // camelCase
      /^[A-Z][a-z]+[A-Z]/, // PascalCase
    ];

    return codePatterns.some((pattern) => pattern.test(str));
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
        // Log glob errors to stderr for debugging but continue scanning
        console.error(`[SecretCheck] Glob error for pattern ${pattern}:`, error);
      }
    }

    // Filter out skipped extensions
    return files.filter((file) => !this.shouldSkipFile(file, config));
  }
}
