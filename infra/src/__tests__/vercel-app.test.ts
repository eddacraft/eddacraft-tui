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

      const app = new VercelApp('skip-preview', {
        name: 'skip-preview',
        framework: 'nextjs',
        rootDirectory: 'apps/web',
        gitRepo: 'org/repo',
        domains: ['example.com'],
        skipPreviewDeploys: true,
      });
      expect(app).toBeDefined();

      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    it('passes --skip-preview flag to the ignore script', () => {
      const cmd = getProjectIgnoreCommand('skip-preview');
      expect(cmd).toContain('vercel-ignore-build.sh --skip-preview apps/web');
    });

    it('disables Vercel preview deployments', () => {
      const project = resources.find(
        (r) => r.type === 'vercel:index/project:Project' && r.name === 'skip-preview'
      );
      expect(project?.inputs.previewDeploymentsDisabled).toBe(true);
    });

    it('sets the Vercel production branch to main', () => {
      const project = resources.find(
        (r) => r.type === 'vercel:index/project:Project' && r.name === 'skip-preview'
      );
      expect(project?.inputs.gitRepository).toMatchObject({ productionBranch: 'main' });
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

      const app = new VercelApp('no-skip', {
        name: 'no-skip',
        framework: 'nextjs',
        rootDirectory: 'apps/web',
        gitRepo: 'org/repo',
        domains: ['example.com'],
      });
      expect(app).toBeDefined();

      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    it('uses only the file-check command (no branch gate)', () => {
      const cmd = getProjectIgnoreCommand('no-skip');
      expect(cmd).not.toContain('VERCEL_GIT_COMMIT_REF');
      expect(cmd).toContain('vercel-ignore-build.sh apps/web');
    });

    it('leaves Vercel preview deployments enabled', () => {
      const project = resources.find(
        (r) => r.type === 'vercel:index/project:Project' && r.name === 'no-skip'
      );
      expect(project?.inputs.previewDeploymentsDisabled).toBeUndefined();
    });

    it('does not manage the Vercel production branch by default', () => {
      const project = resources.find(
        (r) => r.type === 'vercel:index/project:Project' && r.name === 'no-skip'
      );
      expect(project?.inputs.gitRepository).not.toHaveProperty('productionBranch');
    });
  });

  describe('custom productionBranch without skipPreviewDeploys', () => {
    beforeAll(async () => {
      resources.length = 0;
      const { VercelApp } = await import('../../src/components/vercel-app.js');

      const app = new VercelApp('custom-branch-no-skip', {
        name: 'custom-branch-no-skip',
        framework: 'nextjs',
        rootDirectory: 'apps/web',
        gitRepo: 'org/repo',
        domains: ['example.com'],
        productionBranch: 'release/stable',
      });
      expect(app).toBeDefined();

      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    it('sets the explicitly configured Vercel production branch', () => {
      const project = resources.find(
        (r) => r.type === 'vercel:index/project:Project' && r.name === 'custom-branch-no-skip'
      );
      expect(project?.inputs.gitRepository).toMatchObject({ productionBranch: 'release/stable' });
    });
  });

  describe('custom productionBranch', () => {
    beforeAll(async () => {
      resources.length = 0;
      const { VercelApp } = await import('../../src/components/vercel-app.js');

      const app = new VercelApp('custom-branch', {
        name: 'custom-branch',
        framework: 'nextjs',
        rootDirectory: 'apps/web',
        gitRepo: 'org/repo',
        domains: ['example.com'],
        skipPreviewDeploys: true,
        productionBranch: 'release/stable',
      });
      expect(app).toBeDefined();

      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    it('passes --prod-branch flag with the custom branch', () => {
      const cmd = getProjectIgnoreCommand('custom-branch');
      expect(cmd).toContain('--skip-preview --prod-branch release/stable');
    });

    it('sets the Vercel production branch to the custom branch', () => {
      const project = resources.find(
        (r) => r.type === 'vercel:index/project:Project' && r.name === 'custom-branch'
      );
      expect(project?.inputs.gitRepository).toMatchObject({ productionBranch: 'release/stable' });
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

  describe('domainImports', () => {
    beforeAll(async () => {
      resources.length = 0;
      const { VercelApp } = await import('../../src/components/vercel-app.js');

      const app = new VercelApp('with-import', {
        name: 'with-import',
        framework: 'nextjs',
        rootDirectory: 'apps/web',
        gitRepo: 'org/repo',
        domains: ['example.com', 'www.example.com'],
        domainImports: {
          'www.example.com': 'prj_test123abc/www.example.com',
        },
      });
      expect(app).toBeDefined();

      await new Promise((resolve) => setTimeout(resolve, 100));
    });

    function getDomain(name: string) {
      return resources.find(
        (r) => r.type === 'vercel:index/projectDomain:ProjectDomain' && r.name === name
      );
    }

    it('passes the import id to ProjectDomain for a domain in domainImports', () => {
      // The Pulumi mock framework sets `args.id` to the import ID when the
      // `import` resource option is in effect, so observing it verifies the
      // option flowed through without instrumenting runtime internals.
      expect(getDomain('with-import-www-example-com')?.id).toBe('prj_test123abc/www.example.com');
    });

    it('does not set an import id for domains absent from domainImports', () => {
      expect(getDomain('with-import-example-com')?.id).toBeFalsy();
    });

    it('rejects keys that are not in the domains list', async () => {
      const { VercelApp } = await import('../../src/components/vercel-app.js');

      expect(
        () =>
          new VercelApp('bad-key', {
            name: 'bad-key',
            framework: 'nextjs',
            rootDirectory: 'apps/web',
            gitRepo: 'org/repo',
            domains: ['example.com'],
            domainImports: { 'other.com': 'prj_test/other.com' },
          })
      ).toThrow(/must appear in the domains list/);
    });

    it('rejects malformed import IDs', async () => {
      const { VercelApp } = await import('../../src/components/vercel-app.js');

      expect(
        () =>
          new VercelApp('bad-id', {
            name: 'bad-id',
            framework: 'nextjs',
            rootDirectory: 'apps/web',
            gitRepo: 'org/repo',
            domains: ['example.com'],
            domainImports: { 'example.com': 'just-the-domain.com' },
          })
      ).toThrow(/<projectId>\/<domain>/);
    });

    it('accepts the team-prefixed import ID form', async () => {
      const { VercelApp } = await import('../../src/components/vercel-app.js');

      expect(
        () =>
          new VercelApp('team-prefixed', {
            name: 'team-prefixed',
            framework: 'nextjs',
            rootDirectory: 'apps/web',
            gitRepo: 'org/repo',
            domains: ['example.com'],
            domainImports: { 'example.com': 'team_abc123/prj_def456/example.com' },
          })
      ).not.toThrow();
    });
  });
});
