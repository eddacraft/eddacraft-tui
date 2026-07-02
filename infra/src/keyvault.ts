import { SecretClient } from '@azure/keyvault-secrets';
import { DefaultAzureCredential } from '@azure/identity';
import * as pulumi from '@pulumi/pulumi';
import { isTrustedStack, untrustedSecretMarker } from './stack-trust.js';

let vaultName: string | undefined;
let client: SecretClient | undefined;

// Resolved lazily so untrusted stacks never need Key Vault configuration —
// they never construct a client at all.
function getVaultName(): string {
  if (!vaultName) {
    vaultName = new pulumi.Config('keyvault').require('vaultName');
  }
  return vaultName;
}

function getClient(): SecretClient {
  if (!client) {
    try {
      client = new SecretClient(
        `https://${getVaultName()}.vault.azure.net`,
        new DefaultAzureCredential()
      );
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      throw new Error(`Failed to initialise Azure Key Vault client: ${message}`, { cause: err });
    }
  }
  return client;
}

// Live Key Vault read for the trusted stack. Fails closed: a missing or
// empty secret aborts the run — including previews — instead of masking the
// misconfiguration behind a placeholder value. Exported for tests.
export async function readSecretValue(secretName: string): Promise<string> {
  try {
    const s = await getClient().getSecret(secretName);
    if (!s.value) {
      throw new Error(`Secret '${secretName}' has no value in vault '${getVaultName()}'`);
    }
    return s.value;
  } catch (e: unknown) {
    if (
      e &&
      typeof e === 'object' &&
      'statusCode' in e &&
      (e as { statusCode: number }).statusCode === 404
    ) {
      throw new Error(
        `Secret '${secretName}' was not found in Key Vault '${getVaultName()}'. ` +
          `Refusing to continue: a missing production secret is a misconfiguration ` +
          `and there is no placeholder fallback (CIB-119).`,
        { cause: e }
      );
    }
    throw e;
  }
}

// CIB-119: secret reads are gated by stack trust. Untrusted stacks (for
// example the `dev` stack used for PR previews) never contact Key Vault;
// they resolve to an explicit marker so nothing downstream can mistake the
// value for a live credential.
export function getSecret(secretName: string): pulumi.Output<string> {
  if (!isTrustedStack()) {
    pulumi.log.warn(
      `Skipping Key Vault read for secret '${secretName}': stack '${pulumi.getStack()}' ` +
        `is not authorised to read production secrets (CIB-119).`
    );
    return pulumi.secret(pulumi.output(untrustedSecretMarker(secretName)));
  }
  return pulumi.secret(pulumi.output(readSecretValue(secretName)));
}
