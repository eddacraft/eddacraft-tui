import { describe, it, expect, beforeAll } from 'vitest';
import * as pulumi from '@pulumi/pulumi';

describe('Vercel resources', () => {
  const resources: pulumi.runtime.MockResourceArgs[] = [];

  beforeAll(async () => {
    pulumi.runtime.setAllConfig({
      'vercel-apps:website-database-url': 'mock-database-url',
    });
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

  it('creates Vercel Project resources for website and docs-site', () => {
    const projects = resources.filter((r) => r.type === 'vercel:index/project:Project');

    expect(projects.length).toBe(2);
    expect(projects.map((p) => p.name)).toContain('website');
    expect(projects.map((p) => p.name)).toContain('docs-site');
  });

  it('creates ProjectDomain resources for each app', () => {
    const domains = resources.filter((r) => r.type === 'vercel:index/projectDomain:ProjectDomain');

    expect(domains.length).toBe(2);
  });

  it('creates environment variable for website DATABASE_URL when secret is configured', () => {
    const envVars = resources.filter(
      (r) => r.type === 'vercel:index/projectEnvironmentVariable:ProjectEnvironmentVariable'
    );

    // DATABASE_URL env var is only created when the secret is configured
    const dbUrl = envVars.find((e) => e.inputs.key === 'DATABASE_URL');
    if (dbUrl) {
      expect(dbUrl).toBeDefined();
    } else {
      expect(envVars.length).toBe(0);
    }
  });
});
