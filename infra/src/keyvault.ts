import { SecretClient } from '@azure/keyvault-secrets';
import { DefaultAzureCredential } from '@azure/identity';
import * as pulumi from '@pulumi/pulumi';

const config = new pulumi.Config('keyvault');
const vaultName = config.require('vaultName');
const vaultUrl = `https://${vaultName}.vault.azure.net`;

let client: SecretClient | undefined;

function getClient(): SecretClient {
  if (!client) {
    try {
      client = new SecretClient(vaultUrl, new DefaultAzureCredential());
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      throw new Error(`Failed to initialise Azure Key Vault client: ${message}`, { cause: err });
    }
  }
  return client;
}

export function getSecret(secretName: string): pulumi.Output<string> {
  return pulumi.secret(
    pulumi.output(
      (async () => {
        try {
          const s = await getClient().getSecret(secretName);
          if (!s.value) {
            throw new Error(`Secret '${secretName}' has no value in vault '${vaultName}'`);
          }
          return s.value;
        } catch (e: unknown) {
          if (
            e &&
            typeof e === 'object' &&
            'statusCode' in e &&
            (e as { statusCode: number }).statusCode === 404
          ) {
            if (pulumi.runtime.isDryRun()) {
              pulumi.log.warn(
                `Secret '${secretName}' not found in Key Vault '${vaultName}' — using placeholder for preview`
              );
              return `<preview:${secretName}>`;
            }
            throw new Error(`Secret '${secretName}' was not found in Key Vault '${vaultName}'.`, {
              cause: e,
            });
          }
          throw e;
        }
      })()
    )
  );
}
