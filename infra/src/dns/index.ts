import * as pulumi from '@pulumi/pulumi';

const config = new pulumi.Config('azure-dns');
export const resourceGroupName = config.require('resourceGroupName');
