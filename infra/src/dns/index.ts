import * as azure from '@pulumi/azure-native';
import * as pulumi from '@pulumi/pulumi';

const config = new pulumi.Config('azure-dns');
export const resourceGroupName = config.require('resourceGroupName');

// Look up existing zones (not managed by Pulumi — they already exist in Azure)
export const zone = {
  eddacraftAi: azure.dns.getZoneOutput({
    zoneName: 'eddacraft.ai',
    resourceGroupName,
  }),
};
