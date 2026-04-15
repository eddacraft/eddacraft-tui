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

const config = new pulumi.Config('signing');
const location = config.get('location') ?? 'eastus';

// Phase 1: resource group + signing account (always created)

export const signingResourceGroup = new azure.resources.ResourceGroup('rg-prd-signing', {
  resourceGroupName: 'rg-prd-signing',
  location,
});

export const signingAccount = new azure.codesigning.CodeSigningAccount(
  'eddacraft-signing',
  {
    accountName: 'eddacraft-signing',
    resourceGroupName: signingResourceGroup.name,
    location,
    sku: { name: 'Basic' },
  },
  { parent: signingResourceGroup }
);

// Phase 4: certificate profile (gated on identity validation ID)
//
// Set after portal identity validation completes:
//   pulumi config set signing:identityValidationId <GUID>

const identityValidationId = config.get('identityValidationId');

export const certificateProfile = identityValidationId
  ? new azure.codesigning.CertificateProfile(
      'eddacraft-anvil',
      {
        profileName: 'eddacraft-anvil',
        accountName: signingAccount.name,
        resourceGroupName: signingResourceGroup.name,
        profileType: 'PublicTrust',
        identityValidationId,
      },
      { parent: signingAccount }
    )
  : undefined;
