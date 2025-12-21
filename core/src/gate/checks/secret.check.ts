import { BaseCheck } from '../check.interface.js';
import { CheckContext, GateResult } from '../../types/gate.types.js';
import { readFileSync, existsSync } from 'fs';
import { join } from 'path';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);

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

export class SecretCheck extends BaseCheck {
  name = 'secret';
  description = 'Scan for potential secrets and sensitive data using patterns and entropy analysis';

  private readonly secretPatterns = [
    // API Keys
    { name: 'API Key', pattern: /(?:api[_-]?key|apikey)\s*[:=]\s*['"]?[a-zA-Z0-9_-]{16,}['"]?/i },
    // JWT Tokens
    { name: 'JWT Token', pattern: /eyJ[a-zA-Z0-9_-]*\.eyJ[a-zA-Z0-9_-]*\.[a-zA-Z0-9_-]*/ },
    // AWS Keys
    { name: 'AWS Key', pattern: /AKIA[0-9A-Z]{16}/ },
    {
      name: 'AWS Secret Key',
      pattern: /(?:aws_secret|aws_secret_access_key)\s*[:=]\s*['"]?[A-Za-z0-9/+=]{40}['"]?/i,
    },
    // Private Keys
    { name: 'Private Key', pattern: /-----BEGIN\s+(?:RSA\s+)?PRIVATE\s+KEY-----/ },
    { name: 'PGP Private Key', pattern: /-----BEGIN PGP PRIVATE KEY BLOCK-----/ },
    // Database URLs
    { name: 'Database URL', pattern: /(?:postgres|mysql|mongodb|redis):\/\/[^:\s]+:[^@\s]+@/ },
    // Generic secrets
    {
      name: 'Generic Secret',
      pattern: /(?:secret|password|passwd|pwd)\s*[:=]\s*['"]?[^\s'"]{8,}['"]?/i,
    },
    // Credit Cards (basic pattern)
    { name: 'Credit Card', pattern: /\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b/ },
    // GitHub tokens
    { name: 'GitHub Token', pattern: /gh[pousr]_[A-Za-z0-9_]{36,}/ },
    // Slack tokens
    { name: 'Slack Token', pattern: /xox[baprs]-[0-9]{10,13}-[0-9]{10,13}-[a-zA-Z0-9]{24}/ },
    // Stripe keys
    { name: 'Stripe Key', pattern: /sk_live_[0-9a-zA-Z]{24}/ },
    { name: 'Stripe Test Key', pattern: /sk_test_[0-9a-zA-Z]{24}/ },
    // Google API keys
    { name: 'Google API Key', pattern: /AIza[0-9A-Za-z_-]{35}/ },
    // Heroku API key
    {
      name: 'Heroku API Key',
      pattern:
        /[h|H]eroku[a-zA-Z0-9_-]*[:=]\s*['"]?[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}['"]?/,
    },
    // SendGrid API key
    { name: 'SendGrid API Key', pattern: /SG\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9_-]{43}/ },
    // Twilio API key
    { name: 'Twilio API Key', pattern: /SK[a-f0-9]{32}/ },
    // npm token
    { name: 'NPM Token', pattern: /npm_[A-Za-z0-9]{36}/ },
  ];

  /** Default patterns to allowlist (common false positives) */
  private readonly defaultAllowlist = [
    /^[a-f0-9]{32}$/, // MD5 hashes
    /^[a-f0-9]{40}$/, // SHA1 hashes
    /^[a-f0-9]{64}$/, // SHA256 hashes
    /^0x[a-f0-9]+$/i, // Hex literals
    /^data:image\/[a-z]+;base64,/, // Base64 images
    /placeholder/i,
    /example/i,
    /test/i,
    /dummy/i,
    /sample/i,
    /lorem ipsum/i,
  ];

  /** File extensions to skip by default */
  private readonly defaultSkipExtensions = [
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
        const historyFindings = await this.scanGitHistory(context.workspace_root, config);
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
    const skipExtensions = [...this.defaultSkipExtensions, ...(config.skip_extensions || [])];
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
      for (const pattern of this.secretPatterns) {
        const matches = line.match(pattern.pattern);
        if (matches && !this.isAllowlisted(matches[0], config)) {
          findings.push({
            file: file.replace(workspaceRoot, ''),
            line: lineNumber,
            type: pattern.name,
            match: this.redactSecret(matches[0]),
            context: this.redactLine(line.trim()),
            source: 'pattern',
          });
        }
      }

      // Entropy-based detection (if enabled)
      if (config.enable_entropy) {
        const entropyFindings = this.detectHighEntropyStrings(
          line,
          lineNumber,
          file.replace(workspaceRoot, ''),
          config
        );
        findings.push(...entropyFindings);
      }
    }

    return findings;
  }

  /**
   * Calculate Shannon entropy of a string
   * Higher entropy = more randomness = likely a secret
   */
  calculateEntropy(str: string): number {
    if (!str || str.length === 0) return 0;

    const charFrequency: Record<string, number> = {};
    for (const char of str) {
      charFrequency[char] = (charFrequency[char] || 0) + 1;
    }

    let entropy = 0;
    const len = str.length;
    for (const char in charFrequency) {
      const frequency = charFrequency[char] / len;
      entropy -= frequency * Math.log2(frequency);
    }

    return entropy;
  }

  /**
   * Detect high-entropy strings that might be secrets
   */
  private detectHighEntropyStrings(
    line: string,
    lineNumber: number,
    file: string,
    config: SecretCheckConfig
  ): SecretFinding[] {
    const findings: SecretFinding[] = [];
    const threshold = config.entropy_threshold ?? 4.5;
    const minLength = config.min_entropy_length ?? 16;

    // Extract potential secret strings (quoted strings, assignments)
    const stringPatterns = [
      // Quoted strings
      /['"]([^'"]{16,})['"]/,
      // Variable assignments with alphanumeric values
      /[:=]\s*['"]?([a-zA-Z0-9_/+=-]{16,})['"]?/,
    ];

    for (const pattern of stringPatterns) {
      const matches = line.match(pattern);
      if (matches && matches[1]) {
        const candidate = matches[1];

        // Skip if too short or already detected by pattern
        if (candidate.length < minLength) continue;
        if (this.isAllowlisted(candidate, config)) continue;

        // Skip if it looks like code or common patterns
        if (this.looksLikeCode(candidate)) continue;

        const entropy = this.calculateEntropy(candidate);
        if (entropy >= threshold) {
          // Avoid duplicate if already detected by pattern matching
          const alreadyDetected = this.secretPatterns.some((p) => p.pattern.test(line));
          if (!alreadyDetected) {
            findings.push({
              file,
              line: lineNumber,
              type: 'High Entropy String',
              match: this.redactSecret(candidate),
              context: this.redactLine(line.trim()),
              entropy: Math.round(entropy * 100) / 100,
              source: 'entropy',
            });
          }
        }
      }
    }

    return findings;
  }

  /**
   * Scan git history for secrets in recent commits
   */
  private async scanGitHistory(
    workspaceRoot: string,
    config: SecretCheckConfig
  ): Promise<SecretFinding[]> {
    const findings: SecretFinding[] = [];
    const depth = config.git_history_depth ?? 10;

    try {
      // Check if we're in a git repository
      const isGitRepo = existsSync(join(workspaceRoot, '.git'));
      if (!isGitRepo) {
        return findings;
      }

      // Get recent commit diffs
      const { stdout } = await execAsync(
        `git log -p -${depth} --all --diff-filter=A -- '*.ts' '*.js' '*.json' '*.env*' '*.yaml' '*.yml'`,
        {
          cwd: workspaceRoot,
          maxBuffer: 10 * 1024 * 1024,
        }
      );

      // Parse git diff output
      const commitBlocks = stdout.split(/^commit /m).slice(1);

      for (const block of commitBlocks) {
        const commitMatch = block.match(/^([a-f0-9]+)/);
        const commitHash = commitMatch ? commitMatch[1].substring(0, 8) : 'unknown';

        // Find added lines (starting with +)
        const addedLines = block
          .split('\n')
          .filter((line) => line.startsWith('+') && !line.startsWith('+++'));

        for (const addedLine of addedLines) {
          const lineContent = addedLine.substring(1); // Remove the + prefix

          // Check for pattern matches
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
      // Git command failed, likely not a git repo or git not available
      // Silently skip git history scanning
    }

    return findings;
  }

  /**
   * Check if a string matches the allowlist
   */
  private isAllowlisted(str: string, config: SecretCheckConfig): boolean {
    // Check default allowlist
    for (const pattern of this.defaultAllowlist) {
      if (pattern.test(str)) return true;
    }

    // Check custom allowlist
    for (const pattern of config.allowlist || []) {
      if (new RegExp(pattern, 'i').test(str)) return true;
    }

    return false;
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
   * Redact secret value for safe display
   */
  private redactSecret(secret: string): string {
    if (secret.length <= 8) return '***';
    const prefix = secret.substring(0, 4);
    const suffix = secret.substring(secret.length - 4);
    return `${prefix}...${suffix}`;
  }

  /**
   * Redact potential secrets in a line for safe display
   */
  private redactLine(line: string): string {
    let redacted = line;

    // Redact quoted strings that look like secrets
    redacted = redacted.replace(/(['"])[a-zA-Z0-9_/+=-]{16,}\1/g, '$1[REDACTED]$1');

    return redacted;
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
    // Use targetFiles if provided (planless mode)
    if (context.targetFiles && context.targetFiles.length > 0) {
      return context.targetFiles;
    }

    // Otherwise use files from plan
    const files: string[] = [];

    if (context.plan) {
      for (const change of context.plan.proposed_changes) {
        // Check for file-related change types
        const isFileChange =
          change.type === 'file_create' ||
          change.type === 'file_update' ||
          change.type === 'file_delete';

        if (isFileChange) {
          const fullPath = join(context.workspace_root, change.path);
          if (existsSync(fullPath)) {
            files.push(fullPath);
          }
        }
      }
    }

    return files;
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
      } catch {
        // Ignore glob errors
      }
    }

    // Filter out skipped extensions
    return files.filter((file) => !this.shouldSkipFile(file, config));
  }
}
