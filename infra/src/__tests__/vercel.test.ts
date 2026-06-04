import { afterEach, describe, it, expect, beforeAll, vi } from 'vitest';
import * as pulumi from '@pulumi/pulumi';

vi.mock('../../src/keyvault.js', () => ({
  getSecret: (name: string) => {
    const secrets: Record<string, string> = {
      'anvil-api-database-url': 'mock-database-url',
      'resend-api-key': 'mock-resend-key',
      'anvil-admin-key': 'mock-admin-key',
      'admin-key-pepper': 'mock-admin-pepper',
      'waitlist-resend-admin-token': 'mock-waitlist-token',
      'resend-waitlist-audience-id': 'mock-waitlist-audience-id',
      'resend-beta-audience-id': 'mock-beta-audience-id',
      'cron-secret': 'mock-cron-secret',
      'token-pepper': 'mock-token-pepper',
      'github-oauth-client-id': 'mock-github-client-id',
      'github-oauth-client-secret': 'mock-github-client-secret',
      'github-cli-client-id': 'mock-github-cli-client-id',
      'github-cli-client-secret': 'mock-github-cli-client-secret',
      'license-public-key': 'mock-license-public-key',
      'license-signing-key': 'mock-license-signing-key',
      'docs-state-secret': 'mock-docs-state-secret',
      'docs-upstream-secret': 'mock-upstream-secret',
    };
    const value = secrets[name];
    if (value === undefined) {
      throw new Error(`Unknown secret requested in test: ${name}`);
    }
    return pulumi.output(value);
  },
}));

describe('Vercel resources', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

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

    await import('../../src/vercel.js');

    // Allow Pulumi to process async resource registrations
    await new Promise((resolve) => setTimeout(resolve, 500));
  });

  it('creates Vercel Project resources for all apps', () => {
    const projects = resources.filter((r) => r.type === 'vercel:index/project:Project');

    expect(projects.length).toBe(6);
    expect(projects.map((p) => p.name)).toContain('website');
    expect(projects.map((p) => p.name)).toContain('docs-site');
    expect(projects.map((p) => p.name)).toContain('anvil-api');
    expect(projects.map((p) => p.name)).toContain('anvil-docs-private');
    expect(projects.map((p) => p.name)).toContain('docs-public');
    expect(projects.map((p) => p.name)).toContain('docs-shell');
  });

  it('creates ProjectDomain resources only for apps with domains', () => {
    const domains = resources.filter((r) => r.type === 'vercel:index/projectDomain:ProjectDomain');

    expect(domains.length).toBe(4);
  });

  it('assigns both apex and www domains to the website', () => {
    const domains = resources.filter((r) => r.type === 'vercel:index/projectDomain:ProjectDomain');

    const apex = domains.find((d) => d.inputs.domain === 'eddacraft.ai');
    const www = domains.find((d) => d.inputs.domain === 'www.eddacraft.ai');

    expect(apex).toBeDefined();
    expect(apex!.name).toBe('website-eddacraft-ai');
    expect(www).toBeDefined();
    expect(www!.name).toBe('website-www-eddacraft-ai');
  });

  it('limits managed Vercel Git deployments to the main branch', () => {
    const projects = resources.filter((r) => r.type === 'vercel:index/project:Project');

    expect(projects).toHaveLength(6);
    for (const project of projects) {
      expect(project.inputs.previewDeploymentsDisabled, project.name).toBe(true);
      expect(project.inputs.gitRepository, project.name).toMatchObject({
        productionBranch: 'main',
      });
    }
  });

  it('assigns docs.eddacraft.ai to docs-shell, not docs-site', () => {
    const domains = resources.filter((r) => r.type === 'vercel:index/projectDomain:ProjectDomain');

    const docsDomain = domains.find((d) => d.inputs.domain === 'docs.eddacraft.ai');
    expect(docsDomain).toBeDefined();
    expect(docsDomain!.name).toBe('docs-shell-docs-eddacraft-ai');
  });

  it('creates environment variables for anvil-api when secrets are configured', () => {
    const envVars = resources.filter(
      (r) => r.type === 'vercel:index/projectEnvironmentVariable:ProjectEnvironmentVariable'
    );

    const dbUrl = envVars.find((e) => e.inputs.key === 'DATABASE_URL');
    expect(dbUrl).toBeDefined();

    const resend = envVars.find((e) => e.inputs.key === 'RESEND_API_KEY');
    expect(resend).toBeDefined();
  });

  it('wires the dedicated Anvil CLI GitHub OAuth credentials into anvil-api (GHCLIAUTH-002)', () => {
    const envVars = resources.filter(
      (r) => r.type === 'vercel:index/projectEnvironmentVariable:ProjectEnvironmentVariable'
    );
    const cliId = envVars.find((e) => e.inputs.key === 'GITHUB_CLI_CLIENT_ID');
    expect(cliId).toBeDefined();
    const cliSecret = envVars.find((e) => e.inputs.key === 'GITHUB_CLI_CLIENT_SECRET');
    expect(cliSecret).toBeDefined();
  });

  it('wires ADMIN_KEY_PEPPER + ADMIN_PER_OPERATOR_KEYS into anvil-api', () => {
    const envVars = resources.filter(
      (r) => r.type === 'vercel:index/projectEnvironmentVariable:ProjectEnvironmentVariable'
    );
    const pepper = envVars.find((e) => e.inputs.key === 'ADMIN_KEY_PEPPER');
    expect(pepper).toBeDefined();

    const flag = envVars.find((e) => e.inputs.key === 'ADMIN_PER_OPERATOR_KEYS');
    expect(flag).toBeDefined();
  });

  // Guards the NEXT_PUBLIC_ auto-public rule in components/vercel-app.ts.
  // Marking NEXT_PUBLIC_* vars sensitive silently breaks Next.js builds
  // because Vercel does not expose sensitive vars to the build environment.
  it('marks NEXT_PUBLIC_* env vars as non-sensitive', () => {
    const envVars = resources.filter(
      (r) => r.type === 'vercel:index/projectEnvironmentVariable:ProjectEnvironmentVariable'
    );
    const apiUrl = envVars.find((e) => e.inputs.key === 'NEXT_PUBLIC_API_URL');
    expect(apiUrl).toBeDefined();
    expect(apiUrl!.inputs.sensitive).toBe(false);
  });

  it('keeps secret env vars sensitive', () => {
    const envVars = resources.filter(
      (r) => r.type === 'vercel:index/projectEnvironmentVariable:ProjectEnvironmentVariable'
    );
    for (const key of [
      'DATABASE_URL',
      'RESEND_API_KEY',
      'ADMIN_KEY',
      'ADMIN_KEY_PEPPER',
      'TOKEN_PEPPER',
      'LICENSE_SIGNING_KEY',
      'GITHUB_CLIENT_SECRET',
      'GITHUB_CLI_CLIENT_SECRET',
    ]) {
      const v = envVars.find((e) => e.inputs.key === key);
      expect(v, `expected ${key} to be configured`).toBeDefined();
      expect(v!.inputs.sensitive, `expected ${key} to be sensitive`).toBe(true);
    }
  });

  it('allows both live website origins in anvil-api CORS', async () => {
    const envVars = resources.filter(
      (r) => r.type === 'vercel:index/projectEnvironmentVariable:ProjectEnvironmentVariable'
    );
    const cors = envVars.find((e) => e.inputs.key === 'ANVIL_CORS_ORIGINS');
    const { API_CORS_ORIGINS } = await import('../../src/vercel.js');

    expect(cors).toBeDefined();
    expect(API_CORS_ORIGINS).toContain('https://eddacraft.ai');
    expect(API_CORS_ORIGINS).toContain('https://www.eddacraft.ai');
  });
});
