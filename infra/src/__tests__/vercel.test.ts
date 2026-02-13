import { describe, it, expect, beforeAll } from 'vitest';
import * as pulumi from '@pulumi/pulumi';

describe('Vercel resources', () => {
  const resources: pulumi.runtime.MockResourceArgs[] = [];

  beforeAll(async () => {
    pulumi.runtime.setAllConfig({
      'vercel-apps:website-database-url': 'mock-database-url',
      'vercel-apps:unosend-api-key': 'mock-unosend-key',
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

  it('creates environment variables for website when secrets are configured', () => {
    const envVars = resources.filter(
      (r) => r.type === 'vercel:index/projectEnvironmentVariable:ProjectEnvironmentVariable'
    );

    const dbUrl = envVars.find((e) => e.inputs.key === 'DATABASE_URL');
    expect(dbUrl).toBeDefined();

    const unosend = envVars.find((e) => e.inputs.key === 'UNOSEND_API_KEY');
    expect(unosend).toBeDefined();
  });
});
