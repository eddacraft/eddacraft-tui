import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import * as pulumi from '@pulumi/pulumi';
import { outputValue } from './setup.js';

// CIB-119: on an untrusted stack (anything that is not `prod` — the CI PR
// preview runs the untrusted `ci-preview` stack; this suite uses `dev`, another
// untrusted name), the Pulumi program must not contact Azure Key Vault and must
// not define any production resource. Secret reads resolve to an explicit,
// traceable marker instead of a live value or a silent placeholder.

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

    const vercelResources = resources.filter((r) => r.type.startsWith('vercel:'));
    expect(vercelResources).toHaveLength(0);
  });

  it('does not read or define production DNS resources', async () => {
    const mod = await import('../../src/dns/eddacraft-ai.js');
    await new Promise((resolve) => setTimeout(resolve, 300));

    expect(mod.eddacraftAi).toBeUndefined();

    const dnsZones = resources.filter((r) => r.type === 'anvil:dns:Zone');
    const recordSets = resources.filter((r) => r.type === 'azure-native:dns:RecordSet');
    expect(dnsZones).toHaveLength(0);
    expect(recordSets).toHaveLength(0);
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
    // Scoped to the modules imported above (vercel/signing/admin-keys/dns/
    // keyvault). CIB-136 closed the DNS gap that CIB-135 had tracked, so
    // every module infra/index.ts pulls in is now gated at source.
    // Component-resource shells (anvil:*) would be acceptable, but nothing
    // above registers even those — the definitions are gated at source.
    expect(resources).toHaveLength(0);
    expect(secretClientCtor).not.toHaveBeenCalled();
  });
});
