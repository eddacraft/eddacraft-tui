export type ReleaseStepId =
  | 'preflight'
  | 'version'
  | 'changelog'
  | 'commit-tag-push'
  | 'monitor'
  | 'verify';

export type StepStatus = 'pending' | 'running' | 'passed' | 'failed' | 'skipped';

export interface ReleaseStep {
  id: ReleaseStepId;
  label: string;
  status: StepStatus;
  startedAt?: string;
  completedAt?: string;
  error?: string;
}

export interface VersionFile {
  /** Absolute or workspace-relative path */
  path: string;
  /** Regex pattern to match the version string (must have a capture group) */
  pattern: RegExp;
  /** Replacement template — use $1 for captured prefix, ${version} is interpolated */
  replacement: string;
}

export interface ReleaseProfile {
  name: string;
  /** Tag format template — ${version} is interpolated */
  tagFormat: string;
  steps: ReleaseStepId[];
  versionFiles: VersionFile[];
  /** Semver prerelease identifier (e.g. 'beta') */
  prerelease?: string;
  /** npm dist-tag (e.g. 'beta', 'latest') */
  npmTag?: string;
}

export interface ReleaseState {
  version: string;
  previousVersion: string;
  profile: string;
  steps: ReleaseStep[];
  startedAt: string;
  updatedAt: string;
  tagName: string;
  workflowRunId?: number;
  /** Files modified during version bump (for git staging) */
  modifiedFiles: string[];
}

export interface ReleaseConfig {
  execute: boolean;
  verbose: boolean;
  profile: string;
  skipPreflight: boolean;
  targetVersion?: string;
  resume: boolean;
}

export interface PreflightCheck {
  name: string;
  label: string;
  command: string;
  args: string[];
  /** Working directory relative to workspace root */
  cwd?: string;
}

export interface PreflightCheckResult {
  name: string;
  passed: boolean;
  output: string;
  durationMs: number;
}

export interface PreflightResult {
  checks: PreflightCheckResult[];
  allPassed: boolean;
  totalDurationMs: number;
}

// ── Profiles ────────────────────────────────────────────────────────────────

const BETA_VERSION_FILES: VersionFile[] = [
  {
    path: 'apps/anvil-cli/package.json',
    pattern: /("version":\s*")([^"]+)(")/,
    replacement: '$1${version}$3',
  },
  {
    path: 'docs/public/beta/quickstart.md',
    pattern: /(pre-release software\*\*\s*\()([^)]+)(\))/,
    replacement: '$1${version}$3',
  },
  {
    path: 'apps/docs-site/src/data/changelog.json',
    pattern: /("version":\s*")([^"]+)(")/,
    replacement: '$1${version}$3',
  },
];

export const PROFILES: Record<string, ReleaseProfile> = {
  beta: {
    name: 'beta',
    tagFormat: 'v${version}',
    steps: ['preflight', 'version', 'changelog', 'commit-tag-push', 'monitor', 'verify'],
    versionFiles: BETA_VERSION_FILES,
    prerelease: 'beta',
    npmTag: 'beta',
  },
};

export const PREFLIGHT_CHECKS: PreflightCheck[] = [
  {
    name: 'install',
    label: 'pnpm install --frozen-lockfile',
    command: 'pnpm',
    args: ['install', '--frozen-lockfile'],
  },
  { name: 'lint', label: 'pnpm run lint:check', command: 'pnpm', args: ['run', 'lint:check'] },
  { name: 'typecheck', label: 'pnpm run typecheck', command: 'pnpm', args: ['run', 'typecheck'] },
  {
    name: 'test',
    label: 'pnpm run test -- --run',
    command: 'pnpm',
    args: ['run', 'test', '--', '--run'],
  },
  { name: 'build', label: 'pnpm build', command: 'pnpm', args: ['build'] },
  {
    name: 'dry-run',
    label: 'publish --dry-run',
    command: 'pnpm',
    args: [
      '-F',
      '@eddacraft/anvil-cli',
      'publish',
      '--dry-run',
      '--access',
      'public',
      '--no-git-checks',
    ],
  },
];

export const INTERNAL_PACKAGES = [
  '@eddacraft/anvil-core',
  '@eddacraft/anvil-aps',
  '@eddacraft/anvil-policy',
  '@eddacraft/anvil-runtime',
  '@eddacraft/anvil-adapters',
  '@eddacraft/anvil-kindling-integration',
];

export function createInitialSteps(profile: ReleaseProfile): ReleaseStep[] {
  const labels: Record<ReleaseStepId, string> = {
    preflight: 'Preflight checks',
    version: 'Version bump',
    changelog: 'Changelog',
    'commit-tag-push': 'Commit + tag + push',
    monitor: 'Monitor workflow',
    verify: 'Post-release verification',
  };

  return profile.steps.map((id) => ({
    id,
    label: labels[id],
    status: 'pending' as StepStatus,
  }));
}

export function formatTag(profile: ReleaseProfile, version: string): string {
  return profile.tagFormat.replace('${version}', version);
}
