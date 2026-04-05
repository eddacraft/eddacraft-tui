import { afterEach, describe, it, expect, beforeAll, vi } from 'vitest';
import * as pulumi from '@pulumi/pulumi';

vi.mock('../../src/keyvault.js', () => ({
  getSecret: (name: string) => {
    const secrets: Record<string, string> = {
      'website-database-url': 'mock-database-url',
      'resend-api-key': 'mock-resend-key',
      'anvil-admin-key': 'mock-admin-key',
      'waitlist-resend-admin-token': 'mock-waitlist-token',
      'resend-waitlist-audience-id': 'mock-waitlist-audience-id',
      'resend-beta-audience-id': 'mock-beta-audience-id',
      'cron-secret': 'mock-cron-secret',
      'github-oauth-client-id': 'mock-github-client-id',
      'github-oauth-client-secret': 'mock-github-client-secret',
      'license-public-key': 'mock-license-public-key',
      'docs-state-secret': 'mock-docs-state-secret',
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

    expect(projects.length).toBe(3);
    expect(projects.map((p) => p.name)).toContain('website');
    expect(projects.map((p) => p.name)).toContain('docs-site');
    expect(projects.map((p) => p.name)).toContain('anvil-api');
  });

  it('creates ProjectDomain resources for each app', () => {
    const domains = resources.filter((r) => r.type === 'vercel:index/projectDomain:ProjectDomain');

    expect(domains.length).toBe(3);
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
});
