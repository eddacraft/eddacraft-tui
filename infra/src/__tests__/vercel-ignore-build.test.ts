import { describe, expect, it } from 'vitest';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, rmSync, mkdirSync, writeFileSync, readFileSync, globSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = fileURLToPath(new URL('../../../', import.meta.url));
const scriptPath = join(repoRoot, 'tools/scripts/vercel-ignore-build.sh');

const gitEnv = {
  ...process.env,
  GIT_AUTHOR_NAME: 'Test User',
  GIT_AUTHOR_EMAIL: 'test@example.com',
  GIT_COMMITTER_NAME: 'Test User',
  GIT_COMMITTER_EMAIL: 'test@example.com',
};

function git(repo: string, args: string[]): string {
  return execFileSync('git', args, {
    cwd: repo,
    env: gitEnv,
    encoding: 'utf8',
  }).trim();
}

function createFixtureRepo(): string {
  const repo = mkdtempSync(join(tmpdir(), 'anvil-vercel-ignore-'));

  mkdirSync(join(repo, 'apps/website'), { recursive: true });
  mkdirSync(join(repo, 'docs/public/anvil'), { recursive: true });
  writeFileSync(join(repo, 'package.json'), '{"private":true}\n');
  writeFileSync(join(repo, 'apps/website/page.tsx'), 'export default function Page() {}\n');
  writeFileSync(join(repo, 'docs/public/anvil/intro.md'), '# intro\n');

  git(repo, ['init', '--initial-branch=main']);
  git(repo, ['add', '.']);
  git(repo, ['commit', '-m', 'initial']);

  return repo;
}

function commitChange(
  repo: string,
  path: string,
  contents: string
): { previous: string; current: string } {
  const previous = git(repo, ['rev-parse', 'HEAD']);
  writeFileSync(join(repo, path), contents);
  git(repo, ['add', path]);
  git(repo, ['commit', '-m', `change ${path}`]);
  return { previous, current: git(repo, ['rev-parse', 'HEAD']) };
}

function runIgnore(
  repo: string,
  args: string[],
  env: Record<string, string | undefined>
): ReturnType<typeof spawnSync> {
  return spawnSync('bash', [scriptPath, ...args], {
    cwd: repo,
    env: {
      ...process.env,
      ...env,
    },
    encoding: 'utf8',
  });
}

// `git init` + a commit + a `bash` subprocess can exceed the 5s vitest default
// on the Windows runner under load; 30s leaves headroom without masking
// regressions.
const FIXTURE_TEST_TIMEOUT_MS = 30_000;

describe('vercel-ignore-build.sh', () => {
  it(
    'skips preview branch deployments before diffing commits',
    () => {
      const repo = createFixtureRepo();

      try {
        const result = runIgnore(repo, ['--skip-preview', 'apps/website'], {
          VERCEL_GIT_COMMIT_REF: 'feature/no-build',
        });

        expect(result.status).toBe(0);
        expect(result.stdout).toContain('Skipping non-production branch');
      } finally {
        rmSync(repo, { recursive: true, force: true });
      }
    },
    FIXTURE_TEST_TIMEOUT_MS
  );

  it(
    'skips Vercel preview deployments when the branch ref is unavailable',
    () => {
      const repo = createFixtureRepo();

      try {
        const result = runIgnore(repo, ['--skip-preview', 'apps/website'], {
          VERCEL_ENV: 'preview',
        });

        expect(result.status).toBe(0);
        expect(result.stdout).toContain('Skipping preview deployment');
      } finally {
        rmSync(repo, { recursive: true, force: true });
      }
    },
    FIXTURE_TEST_TIMEOUT_MS
  );

  it(
    'always skips when --always-skip is set',
    () => {
      const repo = createFixtureRepo();

      try {
        const { previous, current } = commitChange(
          repo,
          'apps/website/page.tsx',
          'export default function ChangedPage() {}\n'
        );

        const result = runIgnore(repo, ['--always-skip'], {
          VERCEL_GIT_PREVIOUS_SHA: previous,
          VERCEL_GIT_COMMIT_SHA: current,
          VERCEL_GIT_COMMIT_REF: 'main',
          VERCEL_ENV: 'production',
        });

        expect(result.status).toBe(0);
        expect(result.stdout).toContain('Always-skip enabled');
      } finally {
        rmSync(repo, { recursive: true, force: true });
      }
    },
    FIXTURE_TEST_TIMEOUT_MS
  );

  it(
    'builds on production branch root dependency metadata changes',
    () => {
      const repo = createFixtureRepo();

      try {
        const { previous, current } = commitChange(
          repo,
          'package.json',
          '{"private":true,"x":1}\n'
        );

        const result = runIgnore(repo, ['apps/website'], {
          VERCEL_GIT_PREVIOUS_SHA: previous,
          VERCEL_GIT_COMMIT_SHA: current,
          VERCEL_GIT_COMMIT_REF: 'main',
          VERCEL_ENV: 'production',
        });

        expect(result.status).toBe(1);
        expect(result.stdout).toContain('Changes detected in extra watched path package.json');
      } finally {
        rmSync(repo, { recursive: true, force: true });
      }
    },
    FIXTURE_TEST_TIMEOUT_MS
  );

  it(
    'builds when the app directory changes',
    () => {
      const repo = createFixtureRepo();

      try {
        const { previous, current } = commitChange(
          repo,
          'apps/website/page.tsx',
          'export default function ChangedPage() {}\n'
        );

        const result = runIgnore(repo, ['apps/website'], {
          VERCEL_GIT_PREVIOUS_SHA: previous,
          VERCEL_GIT_COMMIT_SHA: current,
          VERCEL_GIT_COMMIT_REF: 'main',
        });

        expect(result.status).toBe(1);
        expect(result.stdout).toContain('Changes detected in apps/website');
      } finally {
        rmSync(repo, { recursive: true, force: true });
      }
    },
    FIXTURE_TEST_TIMEOUT_MS
  );

  it(
    'builds when an explicit extra watched path changes',
    () => {
      const repo = createFixtureRepo();

      try {
        const { previous, current } = commitChange(
          repo,
          'docs/public/anvil/intro.md',
          '# changed\n'
        );

        const result = runIgnore(repo, ['apps/website', 'docs/public/anvil'], {
          VERCEL_GIT_PREVIOUS_SHA: previous,
          VERCEL_GIT_COMMIT_SHA: current,
          VERCEL_GIT_COMMIT_REF: 'main',
        });

        expect(result.status).toBe(1);
        expect(result.stdout).toContain('Changes detected in extra watched path docs/public/anvil');
      } finally {
        rmSync(repo, { recursive: true, force: true });
      }
    },
    FIXTURE_TEST_TIMEOUT_MS
  );
});

describe('Vercel project configs', () => {
  it.each([
    ['apps/website/vercel.json', '--skip-preview apps/website'],
    ['apps/docs-shell/vercel.json', '--skip-preview apps/docs-shell'],
    ['apps/docs-public/vercel.json', '--skip-preview apps/docs-public'],
    ['apps/anvil-docs-private/vercel.json', '--skip-preview apps/anvil-docs-private'],
    ['apps/anvil-api/vercel.json', '--skip-preview apps/anvil-api'],
  ])('%s skips preview builds', (configPath, commandFragment) => {
    const config = JSON.parse(readFileSync(join(repoRoot, configPath), 'utf8')) as {
      ignoreCommand?: string;
    };

    expect(config.ignoreCommand).toContain(commandFragment);
  });

  // The `--always-skip` case this suite used to cover was
  // apps/docs-site/vercel.json. That host was retired 2026-07-08 and deleted;
  // no project uses --always-skip now, so there is nothing left to assert.
  // Reinstate a case here if another project is ever retired that way.
  it('no Vercel project is configured with --always-skip', () => {
    const configs = globSync('apps/*/vercel.json', { cwd: repoRoot });
    expect(configs.length).toBeGreaterThan(0);
    for (const configPath of configs) {
      const config = JSON.parse(readFileSync(join(repoRoot, configPath), 'utf8')) as {
        ignoreCommand?: string;
      };
      expect(config.ignoreCommand ?? '').not.toContain('--always-skip');
    }
  });
});
