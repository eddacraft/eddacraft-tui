import { describe, it, expect, beforeAll } from 'vitest';
import * as pulumi from '@pulumi/pulumi';

describe('VercelApp component', () => {
  const resources: pulumi.runtime.MockResourceArgs[] = [];

  beforeAll(async () => {
    pulumi.runtime.setMocks(
      {
        newResource(args: pulumi.runtime.MockResourceArgs) {
          resources.push(args);
          return { id: `${args.name}-mock-id`, state: args.inputs };
        },
        call(args: pulumi.runtime.MockCallArgs) {
          return args.inputs;
        },
      },
      'test-project',
      'test-stack',
      false
    );
  });

  function getProjectIgnoreCommand(name: string): string | undefined {
    const project = resources.find(
      (r) => r.type === 'vercel:index/project:Project' && r.name === name
    );
    return project?.inputs.ignoreCommand as string | undefined;
  }

  describe('ignoreCommand with skipPreviewDeploys', () => {
    beforeAll(async () => {
      resources.length = 0;
      const { VercelApp } = await import('../../src/components/vercel-app.js');

      new VercelApp('skip-preview', {
        name: 'skip-preview',
        framework: 'nextjs',
        rootDirectory: 'apps/web',
        gitRepo: 'org/repo',
        domains: ['example.com'],
        skipPreviewDeploys: true,
      });

      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    it('passes --skip-preview flag to the ignore script', () => {
      const cmd = getProjectIgnoreCommand('skip-preview');
      expect(cmd).toContain('vercel-ignore-build.sh --skip-preview apps/web');
    });

    it('does not pass --prod-branch when using default (main)', () => {
      const cmd = getProjectIgnoreCommand('skip-preview');
      expect(cmd).not.toContain('--prod-branch');
    });

    it('starts with cd to repo root', () => {
      const cmd = getProjectIgnoreCommand('skip-preview');
      expect(cmd).toMatch(/^cd \$\(git rev-parse --show-toplevel\) && bash/);
    });
  });

  describe('ignoreCommand without skipPreviewDeploys', () => {
    beforeAll(async () => {
      resources.length = 0;
      const { VercelApp } = await import('../../src/components/vercel-app.js');

      new VercelApp('no-skip', {
        name: 'no-skip',
        framework: 'nextjs',
        rootDirectory: 'apps/web',
        gitRepo: 'org/repo',
        domains: ['example.com'],
      });

      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    it('uses only the file-check command (no branch gate)', () => {
      const cmd = getProjectIgnoreCommand('no-skip');
      expect(cmd).not.toContain('VERCEL_GIT_COMMIT_REF');
      expect(cmd).toContain('vercel-ignore-build.sh apps/web');
    });
  });

  describe('custom productionBranch', () => {
    beforeAll(async () => {
      resources.length = 0;
      const { VercelApp } = await import('../../src/components/vercel-app.js');

      new VercelApp('custom-branch', {
        name: 'custom-branch',
        framework: 'nextjs',
        rootDirectory: 'apps/web',
        gitRepo: 'org/repo',
        domains: ['example.com'],
        skipPreviewDeploys: true,
        productionBranch: 'release/stable',
      });

      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    it('passes --prod-branch flag with the custom branch', () => {
      const cmd = getProjectIgnoreCommand('custom-branch');
      expect(cmd).toContain('--skip-preview --prod-branch release/stable');
    });
  });

  describe('productionBranch validation', () => {
    it('rejects branch names with shell metacharacters', async () => {
      const { VercelApp } = await import('../../src/components/vercel-app.js');

      expect(
        () =>
          new VercelApp('bad-branch', {
            name: 'bad-branch',
            framework: 'nextjs',
            rootDirectory: 'apps/web',
            gitRepo: 'org/repo',
            domains: ['example.com'],
            skipPreviewDeploys: true,
            productionBranch: 'main"; rm -rf /',
          })
      ).toThrow(/Invalid productionBranch/);
    });
  });
});
