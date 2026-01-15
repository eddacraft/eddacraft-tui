import { exec } from 'child_process';
import { promisify } from 'util';
import { existsSync, readdirSync, statSync } from 'fs';
import { join } from 'path';

const execAsync = promisify(exec);

/**
 * Configuration for sample analysis
 */
export interface SampleAnalysisConfig {
  /** Maximum number of files to analyze */
  maxFiles: number;
  /** Number of days to look back in git history */
  daysBack: number;
  /** File patterns to include */
  includePatterns: string[];
  /** File patterns to exclude */
  excludePatterns: string[];
}

/**
 * File selection strategy
 */
export type SelectionStrategy = 'git-recent' | 'filesystem' | 'hybrid';

/**
 * Result from file selection
 */
export interface SampleSelection {
  /** Selected files to analyze */
  files: string[];
  /** Total files found before limiting */
  totalFound: number;
  /** Strategy used for selection */
  strategy: SelectionStrategy;
  /** Whether git was available */
  gitAvailable: boolean;
}

/**
 * Service for selecting representative files for initial analysis
 */
export class SampleAnalyzer {
  private readonly defaultConfig: SampleAnalysisConfig = {
    maxFiles: 50,
    daysBack: 30,
    includePatterns: ['.ts', '.tsx', '.js', '.jsx', '.vue', '.svelte'],
    excludePatterns: [
      'node_modules',
      '.git',
      'dist',
      'build',
      '.next',
      'coverage',
      '__tests__',
      '__mocks__',
      '.test.',
      '.spec.',
    ],
  };

  constructor(private readonly projectRoot: string) {}

  /**
   * Select representative files for initial analysis
   */
  public async selectFiles(config: Partial<SampleAnalysisConfig> = {}): Promise<SampleSelection> {
    const fullConfig = { ...this.defaultConfig, ...config };

    // Try git-based selection first
    const gitAvailable = await this.isGitAvailable();

    if (gitAvailable) {
      try {
        const gitFiles = await this.getRecentlyChangedFiles(fullConfig);
        if (gitFiles.length > 0) {
          const limited = this.limitFiles(gitFiles, fullConfig.maxFiles);
          return {
            files: limited,
            totalFound: gitFiles.length,
            strategy: 'git-recent',
            gitAvailable: true,
          };
        }
      } catch {
        // Fall through to filesystem search
        console.warn('Git-based file selection failed, falling back to filesystem');
      }
    }

    // Fallback to filesystem search
    const fsFiles = await this.findSourceFiles(fullConfig);
    const limited = this.limitFiles(fsFiles, fullConfig.maxFiles);

    return {
      files: limited,
      totalFound: fsFiles.length,
      strategy: 'filesystem',
      gitAvailable,
    };
  }

  /**
   * Check if git is available
   */
  private async isGitAvailable(): Promise<boolean> {
    try {
      const gitDir = join(this.projectRoot, '.git');
      if (!existsSync(gitDir)) {
        return false;
      }

      await execAsync('git --version', { cwd: this.projectRoot });
      return true;
    } catch {
      return false;
    }
  }

  /**
   * Get recently changed files from git
   */
  private async getRecentlyChangedFiles(config: SampleAnalysisConfig): Promise<string[]> {
    try {
      // Get files changed in the last N days
      const since = `${config.daysBack}.days.ago`;
      const { stdout } = await execAsync(
        `git log --since="${since}" --name-only --pretty=format: --diff-filter=AM`,
        {
          cwd: this.projectRoot,
          maxBuffer: 10 * 1024 * 1024,
        }
      );

      // Parse output, remove empty lines, deduplicate
      const files = stdout
        .split('\n')
        .map((f) => f.trim())
        .filter((f) => f.length > 0)
        .filter((f) => this.shouldIncludeFile(f, config));

      // Remove duplicates while preserving order (most recent first)
      const uniqueFiles = Array.from(new Set(files));

      // Filter to only existing files
      const existingFiles = uniqueFiles.filter((f) => existsSync(join(this.projectRoot, f)));

      return existingFiles;
    } catch (error) {
      throw new Error(`Failed to get git history: ${error}`);
    }
  }

  /**
   * Find source files using filesystem search
   */
  private async findSourceFiles(config: SampleAnalysisConfig): Promise<string[]> {
    const srcDirs = ['src', 'lib', 'app', 'pages', 'components', 'packages', 'apps'];
    const files: string[] = [];

    for (const dir of srcDirs) {
      const dirPath = join(this.projectRoot, dir);
      if (existsSync(dirPath)) {
        this.findFilesRecursive(dirPath, files, config, 10);
      }
    }

    // Sort by modification time (most recent first)
    const filesWithMtime = files.map((file) => {
      try {
        const fullPath = join(this.projectRoot, file);
        const stats = statSync(fullPath);
        return { file, mtime: stats.mtime.getTime() };
      } catch {
        return { file, mtime: 0 };
      }
    });

    filesWithMtime.sort((a, b) => b.mtime - a.mtime);

    return filesWithMtime.map((f) => f.file);
  }

  /**
   * Recursively find files in a directory
   */
  private findFilesRecursive(
    dirPath: string,
    files: string[],
    config: SampleAnalysisConfig,
    maxDepth: number
  ): void {
    if (maxDepth <= 0) return;

    try {
      const entries = readdirSync(dirPath);

      for (const entry of entries) {
        // Skip excluded patterns
        if (config.excludePatterns.some((pattern) => entry.includes(pattern))) {
          continue;
        }

        const fullPath = join(dirPath, entry);
        const relativePath = fullPath.substring(this.projectRoot.length + 1);

        try {
          const stat = statSync(fullPath);

          if (stat.isDirectory()) {
            this.findFilesRecursive(fullPath, files, config, maxDepth - 1);
          } else if (stat.isFile()) {
            if (this.shouldIncludeFile(relativePath, config)) {
              files.push(relativePath);
            }
          }
        } catch {
          // Skip files we can't stat
        }
      }
    } catch {
      // Skip directories we can't read
    }
  }

  /**
   * Check if a file should be included based on patterns
   */
  private shouldIncludeFile(file: string, config: SampleAnalysisConfig): boolean {
    // Check exclude patterns first
    if (config.excludePatterns.some((pattern) => file.includes(pattern))) {
      return false;
    }

    // Check include patterns
    return config.includePatterns.some((pattern) => file.endsWith(pattern));
  }

  /**
   * Limit files to maximum count, prioritizing diversity
   */
  private limitFiles(files: string[], maxFiles: number): string[] {
    if (files.length <= maxFiles) {
      return files;
    }

    // Take files evenly distributed across the list to ensure diversity
    // This helps when we have many files from the same directory
    const step = files.length / maxFiles;
    const selected: string[] = [];

    for (let i = 0; i < maxFiles; i++) {
      const index = Math.floor(i * step);
      selected.push(files[index]);
    }

    return selected;
  }

  /**
   * Get summary statistics about the selection
   */
  public async getSelectionStats(config: Partial<SampleAnalysisConfig> = {}): Promise<{
    totalSourceFiles: number;
    recentFiles: number;
    selectedFiles: number;
  }> {
    const fullConfig = { ...this.defaultConfig, ...config };

    // Count total source files
    const allFiles = await this.findSourceFiles(fullConfig);
    const totalSourceFiles = allFiles.length;

    // Count recent files if git is available
    let recentFiles = 0;
    const gitAvailable = await this.isGitAvailable();
    if (gitAvailable) {
      try {
        const gitFiles = await this.getRecentlyChangedFiles(fullConfig);
        recentFiles = gitFiles.length;
      } catch {
        // Ignore errors
      }
    }

    // Count selected files
    const selection = await this.selectFiles(config);
    const selectedFiles = selection.files.length;

    return {
      totalSourceFiles,
      recentFiles,
      selectedFiles,
    };
  }
}
