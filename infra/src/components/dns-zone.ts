import * as azure from '@pulumi/azure-native';
import * as pulumi from '@pulumi/pulumi';

export interface DnsRecordArgs {
  relativeRecordSetName: string;
  recordType: string;
  ttl?: number;
  txtRecords?: azure.types.input.dns.TxtRecordArgs[];
  cnameRecord?: azure.types.input.dns.CnameRecordArgs;
  mxRecords?: azure.types.input.dns.MxRecordArgs[];
}

export interface DnsZoneArgs {
  zoneName: string;
  resourceGroupName: pulumi.Input<string>;
  records: Record<string, DnsRecordArgs>;
}

export class DnsZone extends pulumi.ComponentResource {
  public readonly zoneName: string;

  constructor(name: string, args: DnsZoneArgs, opts?: pulumi.ComponentResourceOptions) {
    super('anvil:dns:Zone', name, {}, opts);

    const zoneOutput = azure.dns.getZoneOutput({
      zoneName: args.zoneName,
      resourceGroupName: args.resourceGroupName,
    });

    for (const [recordName, record] of Object.entries(args.records)) {
      new azure.dns.RecordSet(
        recordName,
        {
          relativeRecordSetName: record.relativeRecordSetName,
          zoneName: zoneOutput.name,
          resourceGroupName: args.resourceGroupName,
          recordType: record.recordType,
          ttl: record.ttl ?? 3600,
          txtRecords: record.txtRecords,
          cnameRecord: record.cnameRecord,
          mxRecords: record.mxRecords,
        },
        {
          parent: this,
          aliases: [{ name: recordName, parent: pulumi.rootStackResource }],
        }
      );
    }

    this.zoneName = args.zoneName;

    this.registerOutputs({
      zoneName: this.zoneName,
    });
  }
}
