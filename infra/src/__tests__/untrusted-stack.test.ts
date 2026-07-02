import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import * as pulumi from '@pulumi/pulumi';
import { outputValue } from './setup.js';

// CIB-119: on an untrusted stack (anything that is not `prod` — the CI PR
// preview runs the `dev` stack), the Pulumi program must not contact Azure
// Key Vault and must not define any production resource. Secret reads
// resolve to an explicit, traceable marker instead of a live value or a
// silent placeholder.

const secretClientCtor = vi.fn();
const secretClientGetSecret = vi.fn();

vi.mock('@azure/keyvault-secrets', () => ({
  SecretClient: class {
    constructor(...args: unknown[]) {
      secretClientCtor(...args);
    }
    getSecret = secretClientGetSecret;
  },
}));

vi.mock('@azure/identity', () => ({
  DefaultAzureCredential: class {},
}));

describe('untrusted stack gating (CIB-119)', () => {
  const resources: pulumi.runtime.MockResourceArgs[] = [];

  afterEach(() => {
    vi.restoreAllMocks();
  });

  beforeAll(async () => {
    // Stack `dev`, dry-run true — the exact shape of a PR preview.
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
      'dev',
      true
    );
  });

  it('resolves secret reads to an explicit marker without contacting Key Vault', async () => {
    const { getSecret } = await import('../../src/keyvault.js');
    const value = await outputValue(getSecret('anvil-api-database-url'));

    expect(value).toBe('<untrusted-stack-secret:anvil-api-database-url>');
    expect(secretClientCtor).not.toHaveBeenCalled();
    expect(secretClientGetSecret).not.toHaveBeenCalled();
  });

  it('does not define production Vercel resources', async () => {
    const mod = await import('../../src/vercel.js');
    await new Promise((resolve) => setTimeout(resolve, 300));

    expect(mod.website).toBeUndefined();
    expect(mod.api).toBeUndefined();
    expect(mod.anvilDocsPrivate).toBeUndefined();
    expect(mod.docsPublic).toBeUndefined();
    expect(mod.docsShell).toBeUndefined();
    expect(mod.docsSite).toBeUndefined();

    const vercelResources = resources.filter((r) => r.type.startsWith('vercel:'));
    expect(vercelResources).toHaveLength(0);
  });

  it('does not define production signing resources', async () => {
    const mod = await import('../../src/signing.js');
    await new Promise((resolve) => setTimeout(resolve, 300));

    expect(mod.signingResourceGroup).toBeUndefined();
    expect(mod.signingAccount).toBeUndefined();
    expect(mod.certificateProfile).toBeUndefined();

    const azureResources = resources.filter((r) => r.type.startsWith('azure-native:'));
    expect(azureResources).toHaveLength(0);
  });

  it('does not provision admin keys or touch the production database', async () => {
    const mod = await import('../../src/admin-keys.js');
    await new Promise((resolve) => setTimeout(resolve, 300));

    expect(mod.adminKeys).toEqual([]);

    const keyResources = resources.filter(
      (r) => r.type.startsWith('random:') || r.type.startsWith('command:')
    );
    expect(keyResources).toHaveLength(0);
  });

  it('registers no cloud resources from the gated modules on an untrusted stack', () => {
    // Scoped to the modules imported above (vercel/signing/admin-keys/
    // keyvault) — infra/index.ts still imports DNS resources
    // unconditionally; that residual gap is tracked as CIB-135.
    // Component-resource shells (anvil:*) would be acceptable, but nothing
    // above registers even those — the definitions are gated at source.
    expect(resources).toHaveLength(0);
    expect(secretClientCtor).not.toHaveBeenCalled();
  });
});
