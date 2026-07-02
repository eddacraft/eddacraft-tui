import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import * as pulumi from '@pulumi/pulumi';

// CIB-119: the trusted `prod` stack still defines every production resource.
// This is the counterpart to untrusted-stack.test.ts — gating must not
// accidentally turn the production stack into a no-op.

vi.mock('../../src/keyvault.js', () => ({
  getSecret: (name: string) => pulumi.secret(pulumi.output(`mock-${name}`)),
}));

describe('trusted stack provisioning (CIB-119)', () => {
  const resources: pulumi.runtime.MockResourceArgs[] = [];

  afterEach(() => {
    vi.restoreAllMocks();
  });

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
      'prod',
      false
    );

    await import('../../src/signing.js');
    await import('../../src/admin-keys.js');

    // Allow Pulumi to process async resource registrations
    await new Promise((resolve) => setTimeout(resolve, 500));
  });

  it('creates the signing resource group and account', () => {
    const groups = resources.filter((r) => r.type === 'azure-native:resources:ResourceGroup');
    expect(groups.map((g) => g.name)).toContain('rg-prd-signing');

    const accounts = resources.filter(
      (r) => r.type === 'azure-native:codesigning:CodeSigningAccount'
    );
    expect(accounts).toHaveLength(1);
    expect(accounts[0].name).toBe('eddacraft-signing');
  });

  it('skips the certificate profile until identity validation is configured', async () => {
    const mod = await import('../../src/signing.js');
    expect(mod.certificateProfile).toBeUndefined();

    const profiles = resources.filter(
      (r) => r.type === 'azure-native:codesigning:CertificateProfile'
    );
    expect(profiles).toHaveLength(0);
  });

  it('provisions one admin key per seed entry', async () => {
    const mod = await import('../../src/admin-keys.js');
    expect(mod.adminKeys).toHaveLength(1);

    const bearers = resources.filter((r) => r.type === 'random:index/randomBytes:RandomBytes');
    expect(bearers).toHaveLength(1);

    const rows = resources.filter((r) => r.type === 'command:local:Command');
    expect(rows).toHaveLength(1);
  });
});
