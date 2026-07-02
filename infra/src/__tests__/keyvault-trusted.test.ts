import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import * as pulumi from '@pulumi/pulumi';
import { outputValue } from './setup.js';

// CIB-119: on the trusted `prod` stack, Key Vault reads are live and fail
// closed. A missing or empty secret is a misconfiguration and must abort the
// run — there is no placeholder fallback, in preview or apply.

const secretClientGetSecret = vi.fn();

vi.mock('@azure/keyvault-secrets', () => ({
  SecretClient: class {
    getSecret = secretClientGetSecret;
  },
}));

vi.mock('@azure/identity', () => ({
  DefaultAzureCredential: class {},
}));

describe('trusted-stack Key Vault reads (CIB-119)', () => {
  beforeAll(() => {
    pulumi.runtime.setAllConfig({ 'keyvault:vaultName': 'kv-test' });
    pulumi.runtime.setMocks(
      {
        newResource(args: pulumi.runtime.MockResourceArgs) {
          return { id: `${args.name}-mock-id`, state: args.inputs };
        },
        call(args: pulumi.runtime.MockCallArgs) {
          return args.inputs;
        },
      },
      'test-project',
      'prod',
      // dry-run TRUE: the placeholder fallback used to hide missing secrets
      // exactly here, during previews.
      true
    );
  });

  beforeEach(() => {
    secretClientGetSecret.mockReset();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('resolves to the live secret value', async () => {
    secretClientGetSecret.mockResolvedValueOnce({ value: 's3cret-value' });
    const { getSecret } = await import('../../src/keyvault.js');

    const value = await outputValue(getSecret('token-pepper'));

    expect(value).toBe('s3cret-value');
    expect(secretClientGetSecret).toHaveBeenCalledWith('token-pepper');
  });

  it('fails closed on a missing secret — no preview placeholder', async () => {
    secretClientGetSecret.mockRejectedValueOnce({ statusCode: 404 });
    const { readSecretValue } = await import('../../src/keyvault.js');

    await expect(readSecretValue('missing-secret')).rejects.toThrow(
      /'missing-secret' was not found/
    );
  });

  it('fails closed on a secret with no value', async () => {
    secretClientGetSecret.mockResolvedValueOnce({ value: undefined });
    const { readSecretValue } = await import('../../src/keyvault.js');

    await expect(readSecretValue('empty-secret')).rejects.toThrow(/'empty-secret' has no value/);
  });

  it('propagates non-404 Key Vault errors unchanged', async () => {
    secretClientGetSecret.mockRejectedValueOnce(new Error('403 forbidden'));
    const { readSecretValue } = await import('../../src/keyvault.js');

    await expect(readSecretValue('any-secret')).rejects.toThrow('403 forbidden');
  });
});
