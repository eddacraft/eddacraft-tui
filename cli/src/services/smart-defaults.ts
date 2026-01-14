import { GateConfig } from '@anvil/core';
import {
  ProjectContext,
  FrameworkType,
  MonorepoType,
  ProjectSize,
  TypeScriptStrictness,
} from './project-detector.js';

/**
 * Default configuration profiles
 */
interface ConfigProfile {
  /** Coverage thresholds */
  coverageThresholds: {
    lines: number;
    functions: number;
    branches: number;
    statements: number;
  };
  /** ESLint minimum score */
  eslintMinScore: number;
  /** Overall score threshold */
  overallScore: number;
  /** Whether to enable coverage check */
  enableCoverage: boolean;
  /** Whether to enable ESLint check */
  enableEslint: boolean;
  /** File patterns to include */
  includePatterns?: string[];
  /** File patterns to exclude/allowlist */
  allowlistPatterns?: string[];
}

/**
 * Generates smart default .anvilrc configuration based on project characteristics
 */
export class SmartDefaultsGenerator {
  /**
   * Generate optimal gate configuration based on project context
   */
  public generate(context: ProjectContext): GateConfig {
    const profile = this.selectProfile(context);
    const checks = this.buildChecks(context, profile);
    const thresholds = this.buildThresholds(profile);

    return {
      version: 1,
      checks,
      thresholds,
    };
  }

  /**
   * Select appropriate configuration profile based on project characteristics
   */
  private selectProfile(context: ProjectContext): ConfigProfile {
    // Start with base profile
    const baseProfile: ConfigProfile = {
      coverageThresholds: {
        lines: 80,
        functions: 80,
        branches: 80,
        statements: 80,
      },
      eslintMinScore: 80,
      overallScore: 80,
      enableCoverage: context.hasTests,
      enableEslint: context.hasEslint,
    };

    // Adjust based on project size - larger projects get more lenient during adoption
    const sizeAdjustment = this.getSizeAdjustment(context.size);
    baseProfile.coverageThresholds = this.adjustThresholds(
      baseProfile.coverageThresholds,
      sizeAdjustment
    );
    baseProfile.eslintMinScore = Math.max(60, baseProfile.eslintMinScore + sizeAdjustment);
    baseProfile.overallScore = Math.max(70, baseProfile.overallScore + sizeAdjustment);

    // Adjust based on TypeScript strictness
    const strictnessAdjustment = this.getStrictnessAdjustment(context.tsStrictness);
    baseProfile.coverageThresholds = this.adjustThresholds(
      baseProfile.coverageThresholds,
      strictnessAdjustment
    );
    baseProfile.eslintMinScore = Math.min(
      95,
      Math.max(60, baseProfile.eslintMinScore + strictnessAdjustment)
    );

    // Framework-specific adjustments
    baseProfile.allowlistPatterns = this.getFrameworkAllowlist(context.framework);
    baseProfile.includePatterns = this.getFrameworkIncludes(context.framework, context.monorepo);

    return baseProfile;
  }

  /**
   * Get size-based adjustment factor
   */
  private getSizeAdjustment(size: ProjectSize): number {
    switch (size) {
      case 'small':
        return 5; // Slightly higher standards for small projects
      case 'medium':
        return 0; // Baseline
      case 'large':
        return -5; // More lenient for large projects
      case 'xlarge':
        return -10; // Most lenient for very large projects
    }
  }

  /**
   * Get strictness-based adjustment factor
   */
  private getStrictnessAdjustment(strictness: TypeScriptStrictness): number {
    switch (strictness) {
      case 'strict':
        return 5; // Higher standards if already strict
      case 'moderate':
        return 0; // Baseline
      case 'loose':
        return -5; // More lenient if loose
      case 'none':
        return -10; // Most lenient without TS
    }
  }

  /**
   * Adjust thresholds by a percentage
   */
  private adjustThresholds(
    thresholds: ConfigProfile['coverageThresholds'],
    adjustment: number
  ): ConfigProfile['coverageThresholds'] {
    return {
      lines: Math.max(60, Math.min(95, thresholds.lines + adjustment)),
      functions: Math.max(60, Math.min(95, thresholds.functions + adjustment)),
      branches: Math.max(60, Math.min(95, thresholds.branches + adjustment)),
      statements: Math.max(60, Math.min(95, thresholds.statements + adjustment)),
    };
  }

  /**
   * Get framework-specific allowlist patterns
   */
  private getFrameworkAllowlist(framework: FrameworkType): string[] {
    const common = [
      '**/*.d.ts', // Type definitions
      '**/*.test.ts',
      '**/*.test.tsx',
      '**/*.spec.ts',
      '**/*.spec.tsx',
      '**/__tests__/**',
      '**/__mocks__/**',
      '**/test/**',
      '**/tests/**',
      '**/fixtures/**',
    ];

    const frameworkSpecific: Record<FrameworkType, string[]> = {
      nextjs: [
        ...common,
        'next.config.js',
        'next.config.mjs',
        'next.config.ts',
        '**/app/layout.tsx', // Next.js 13+ app router metadata
        '**/pages/_app.tsx',
        '**/pages/_document.tsx',
        '**/.next/**',
      ],
      react: [
        ...common,
        'vite.config.ts',
        'vite.config.js',
        'webpack.config.js',
        '**/setupTests.ts',
      ],
      vue: [...common, 'vite.config.ts', 'vue.config.js'],
      angular: [...common, 'angular.json', '**/environment.ts', '**/environment.*.ts'],
      svelte: [...common, 'svelte.config.js', 'vite.config.ts'],
      express: [...common, '**/routes/**/*.ts', '**/middleware/**/*.ts'],
      nestjs: [...common, '**/main.ts', '**/*.module.ts'],
      nx: [...common, '**/project.json', '**/workspace.json', 'nx.json'],
      node: common,
      unknown: common,
    };

    return frameworkSpecific[framework] || common;
  }

  /**
   * Get framework-specific include patterns
   */
  private getFrameworkIncludes(framework: FrameworkType, monorepo: MonorepoType): string[] {
    // Base patterns
    const base = ['src/**/*.ts', 'src/**/*.tsx', 'src/**/*.js', 'src/**/*.jsx'];

    // Monorepo adjustments
    if (monorepo !== 'none') {
      return [
        'packages/*/src/**/*.ts',
        'packages/*/src/**/*.tsx',
        'packages/*/src/**/*.js',
        'packages/*/src/**/*.jsx',
        'apps/*/src/**/*.ts',
        'apps/*/src/**/*.tsx',
        'apps/*/src/**/*.js',
        'apps/*/src/**/*.jsx',
        'libs/*/src/**/*.ts',
        'libs/*/src/**/*.tsx',
        ...base,
      ];
    }

    // Framework-specific additions
    const frameworkAdditions: Partial<Record<FrameworkType, string[]>> = {
      nextjs: ['pages/**/*.tsx', 'pages/**/*.ts', 'app/**/*.tsx', 'app/**/*.ts'],
      react: ['components/**/*.tsx', 'components/**/*.ts'],
      vue: ['components/**/*.vue'],
      angular: ['app/**/*.ts'],
    };

    return [...base, ...(frameworkAdditions[framework] || [])];
  }

  /**
   * Build checks array based on profile and context
   */
  private buildChecks(context: ProjectContext, profile: ConfigProfile): GateConfig['checks'] {
    const checks: GateConfig['checks'] = [];

    // ESLint check
    if (profile.enableEslint && context.hasEslint) {
      checks.push({
        name: 'eslint',
        description: 'Code quality checks',
        enabled: true,
        config: {
          min_score: profile.eslintMinScore,
        },
      });
    }

    // Coverage check
    if (profile.enableCoverage && context.hasTests) {
      checks.push({
        name: 'coverage',
        description: 'Test coverage validation',
        enabled: true,
        config: {
          min_score: 80,
          thresholds: profile.coverageThresholds,
        },
      });
    }

    // Secret scanning - always enabled
    checks.push({
      name: 'secret',
      description: 'Secret scanning',
      enabled: true,
      config: {},
    });

    // Dependency scanning - always enabled
    checks.push({
      name: 'dependency',
      description: 'Dependency vulnerability scanning',
      enabled: true,
      config: {},
    });

    // Anti-pattern check - always enabled
    checks.push({
      name: 'antipattern',
      description: 'AI anti-pattern detection',
      enabled: true,
      config: {
        patterns: ['AP-001', 'AP-003', 'AP-004', 'AP-006'],
        allowlist: profile.allowlistPatterns || [],
      },
    });

    // Architecture check - enabled for medium+ projects with TypeScript
    if (context.size !== 'small' && context.tsStrictness !== 'none') {
      checks.push({
        name: 'architecture',
        description: 'Architecture boundary detection',
        enabled: true,
        config: {
          baseline: '.anvil/baseline.json',
        },
      });
    }

    return checks;
  }

  /**
   * Build thresholds object
   */
  private buildThresholds(profile: ConfigProfile): GateConfig['thresholds'] {
    return {
      overall_score: profile.overallScore,
    };
  }

  /**
   * Get a human-readable explanation of the generated config
   */
  public explainConfig(context: ProjectContext, config: GateConfig): string {
    const lines: string[] = [];

    lines.push('Smart defaults generated based on your project:');
    lines.push('');

    // Project characteristics
    lines.push(`Framework: ${context.framework}`);
    lines.push(`Project size: ${context.size} (${context.fileCount} source files)`);
    lines.push(`TypeScript strictness: ${context.tsStrictness}`);
    if (context.monorepo !== 'none') {
      lines.push(`Monorepo: ${context.monorepo}`);
    }
    lines.push('');

    // Enabled checks
    lines.push('Enabled checks:');
    config.checks
      .filter((c) => c.enabled)
      .forEach((check) => {
        lines.push(`  • ${check.name}: ${check.description}`);
      });
    lines.push('');

    // Thresholds
    const coverageCheck = config.checks.find((c) => c.name === 'coverage');
    if (coverageCheck && coverageCheck.config?.thresholds) {
      const t = coverageCheck.config.thresholds as { lines?: number; functions?: number };
      lines.push(`Coverage thresholds: ${t.lines ?? 0}% lines, ${t.functions ?? 0}% functions`);
    }

    const eslintCheck = config.checks.find((c) => c.name === 'eslint');
    if (eslintCheck && eslintCheck.config?.min_score) {
      lines.push(`ESLint minimum score: ${eslintCheck.config.min_score}%`);
    }

    lines.push(`Overall score threshold: ${config.thresholds.overall_score}%`);

    return lines.join('\n');
  }
}
