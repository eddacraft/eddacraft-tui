#!/usr/bin/env bash
# Bootstrap Azure infrastructure for Pulumi self-managed backend.
# Idempotent — safe to re-run. Requires Azure CLI and ARM_CLIENT_ID env var.
set -euo pipefail

# --- Configuration ---
LOCATION="uksouth"
# Allow override via AZURE_SUBSCRIPTION_ID for portability; default preserves existing behaviour.
SUBSCRIPTION_ID="${AZURE_SUBSCRIPTION_ID:-290aa167-2d41-45aa-9b36-8ef5b9be99e0}"

# Naming convention:
# - Resource group:   rg-iac-state      (rg = resource group, iac = infrastructure as code, state = state resources)
# - Storage account:  stiacstateprod    (st = storage, iac = infrastructure as code, state = state storage, prod = production)
# - Key Vault:        kv-iac-anvil      (kv = Key Vault, iac = infrastructure as code, anvil = Anvil platform)
#
# These names are stable identifiers for existing Azure resources; update with care if conventions change.
RG_NAME="rg-iac-state"
STORAGE_ACCOUNT="stiacstateprod"
CONTAINER_NAME="pulumi-state"
KV_NAME="kv-iac-anvil"
KEY_NAME="pulumi-secrets-key"

SP_CLIENT_ID="${ARM_CLIENT_ID:?ARM_CLIENT_ID must be set}"

echo "=== Bootstrapping Pulumi backend infrastructure ==="
echo "Subscription: $SUBSCRIPTION_ID"
echo "Location:     $LOCATION"
echo ""

az account set --subscription "$SUBSCRIPTION_ID"

# 1. Resource group
echo "--- Creating resource group: $RG_NAME ---"
az group create --name "$RG_NAME" --location "$LOCATION" --output none

# 2. Storage account
echo "--- Creating storage account: $STORAGE_ACCOUNT ---"
az storage account create \
  --name "$STORAGE_ACCOUNT" \
  --resource-group "$RG_NAME" \
  --location "$LOCATION" \
  --sku Standard_LRS \
  --kind StorageV2 \
  --allow-blob-public-access false \
  --min-tls-version TLS1_2 \
  --output none

# 3. Blob container (ignore "already exists" errors)
echo "--- Creating blob container: $CONTAINER_NAME ---"
if ! az storage container create \
  --name "$CONTAINER_NAME" \
  --account-name "$STORAGE_ACCOUNT" \
  --auth-mode login \
  --output none 2>&1; then
  echo "WARNING: blob container creation returned an error (may already exist)" >&2
fi

# 4. Key Vault (RBAC authorization)
echo "--- Creating Key Vault: $KV_NAME ---"
az keyvault create \
  --name "$KV_NAME" \
  --resource-group "$RG_NAME" \
  --location "$LOCATION" \
  --enable-rbac-authorization true \
  --sku standard \
  --output none

# 5. Encryption key for Pulumi secrets provider (ignore "already exists" errors)
echo "--- Creating encryption key: $KEY_NAME ---"
if ! az keyvault key create \
  --vault-name "$KV_NAME" \
  --name "$KEY_NAME" \
  --kty RSA \
  --size 2048 \
  --output none 2>&1; then
  echo "WARNING: encryption key creation returned an error (may already exist)" >&2
fi

# 6. RBAC assignments for service principal
echo "--- Configuring RBAC for service principal ---"
SP_OBJECT_ID=$(az ad sp show --id "$SP_CLIENT_ID" --query id -o tsv)
STORAGE_SCOPE="/subscriptions/$SUBSCRIPTION_ID/resourceGroups/$RG_NAME/providers/Microsoft.Storage/storageAccounts/$STORAGE_ACCOUNT"
KV_SCOPE="/subscriptions/$SUBSCRIPTION_ID/resourceGroups/$RG_NAME/providers/Microsoft.KeyVault/vaults/$KV_NAME"

assign_role() {
  local role="$1" scope="$2"
  if ! az role assignment create \
    --assignee-object-id "$SP_OBJECT_ID" \
    --assignee-principal-type ServicePrincipal \
    --role "$role" \
    --scope "$scope" \
    --output none 2>&1; then
    echo "WARNING: role assignment '$role' returned an error (may already exist)" >&2
  fi
}

# Storage Blob Data Contributor (state read/write)
assign_role "Storage Blob Data Contributor" "$STORAGE_SCOPE"

# Key Vault Crypto User (encrypt/decrypt Pulumi config)
assign_role "Key Vault Crypto User" "$KV_SCOPE"

# Key Vault Secrets User (read application secrets)
assign_role "Key Vault Secrets User" "$KV_SCOPE"

# 7. Output storage account key
echo ""
echo "=== Bootstrap complete ==="
echo ""
echo "Backend URL:       azblob://$CONTAINER_NAME"
echo "Secrets provider:  azurekeyvault://$KV_NAME.vault.azure.net/keys/$KEY_NAME"
echo "Storage account:   $STORAGE_ACCOUNT"
echo "Key Vault:         $KV_NAME"
echo ""
echo "--- Storage Account Key (store as GitHub Secret: AZURE_STORAGE_KEY) ---"
az storage account keys list \
  --account-name "$STORAGE_ACCOUNT" \
  --resource-group "$RG_NAME" \
  --query '[0].value' -o tsv
echo ""
echo "--- Next steps ---"
echo "1. Store the storage account key as GitHub Secret: AZURE_STORAGE_KEY"
echo "2. Store 'stiacstateprod' as GitHub Secret: AZURE_STORAGE_ACCOUNT"
echo "3. Store application secrets in Key Vault:"
echo "   az keyvault secret set --vault-name $KV_NAME --name vercel-token --value '<VERCEL_TOKEN>'"
echo "   az keyvault secret set --vault-name $KV_NAME --name website-database-url --value '<DATABASE_URL>'"
echo "   az keyvault secret set --vault-name $KV_NAME --name unosend-api-key --value '<UNOSEND_API_KEY>'"
