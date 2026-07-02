// Authenticode signing infrastructure for Windows binaries.
//
// Deployment phases:
//   1. `pulumi up` — creates resource group + Trusted Signing account.
//   2. Azure Portal → Trusted Signing → Identity validation (1-3 business days).
//   3. Set config: `pulumi config set signing:identityValidationId <ID_FROM_PORTAL>`
//   4. `pulumi up` — creates certificate profile (gated on config).
//   5. Create service principal + store credentials in Key Vault (az CLI below).
//
// ──────────────────────────────────────────────────────────────────────
// Phase 5 commands (run after pulumi up creates the cert profile):
//
//   # Create service principal with signing role
//   az ad sp create-for-rbac \
//     --name "eddacraft-anvil-signing-ci" \
//     --role "Trusted Signing Certificate Profile Signer" \
//     --scopes "/subscriptions/$(az account show --query id -o tsv)/resourceGroups/rg-prd-signing/providers/Microsoft.CodeSigning/codeSigningAccounts/eddacraft-signing/certificateProfiles/eddacraft-anvil"
//
//   # Store SP credentials in Key Vault
//   az keyvault secret set --vault-name kv-iac-anvil --name signing-tenant-id     --value "<tenant-id>"
//   az keyvault secret set --vault-name kv-iac-anvil --name signing-client-id     --value "<client-id>"
//   az keyvault secret set --vault-name kv-iac-anvil --name signing-client-secret --value "<client-secret>"
//   az keyvault secret set --vault-name kv-iac-anvil --name signing-account-name  --value "eddacraft-signing"
//   az keyvault secret set --vault-name kv-iac-anvil --name signing-profile-name  --value "eddacraft-anvil"
// ──────────────────────────────────────────────────────────────────────

import * as azure from '@pulumi/azure-native';
import * as pulumi from '@pulumi/pulumi';
import { isTrustedStack, warnUntrustedSkip } from './stack-trust.js';

// CIB-119: the signing account and its resource group carry fixed production
// physical names (`rg-prd-signing`, `eddacraft-signing`). Only the trusted
// prod stack may define them; untrusted stacks export undefined.
function defineSigning() {
  const config = new pulumi.Config('signing');
  const location = config.get('location') ?? 'eastus';

  // Phase 1: resource group + signing account (always created on prod)

  const resourceGroup = new azure.resources.ResourceGroup('rg-prd-signing', {
    resourceGroupName: 'rg-prd-signing',
    location,
  });

  const account = new azure.codesigning.CodeSigningAccount(
    'eddacraft-signing',
    {
      accountName: 'eddacraft-signing',
      resourceGroupName: resourceGroup.name,
      location,
      sku: { name: 'Basic' },
    },
    { parent: resourceGroup }
  );

  // Phase 4: certificate profile (gated on identity validation ID)
  //
  // Set after portal identity validation completes:
  //   pulumi config set signing:identityValidationId <GUID>

  const identityValidationId = config.get('identityValidationId');

  const profile = identityValidationId
    ? new azure.codesigning.CertificateProfile(
        'eddacraft-anvil',
        {
          profileName: 'eddacraft-anvil',
          accountName: account.name,
          resourceGroupName: resourceGroup.name,
          profileType: 'PublicTrust',
          identityValidationId,
        },
        { parent: account }
      )
    : undefined;

  return { resourceGroup, account, profile };
}

const signing = isTrustedStack() ? defineSigning() : undefined;
if (!signing) {
  warnUntrustedSkip('production Authenticode signing resources');
}

export const signingResourceGroup = signing?.resourceGroup;
export const signingAccount = signing?.account;
export const certificateProfile = signing?.profile;
