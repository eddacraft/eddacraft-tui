import * as pulumi from '@pulumi/pulumi';
import { DnsZone } from '../components/dns-zone.js';
import { isTrustedStack, warnUntrustedSkip } from '../stack-trust.js';

// CIB-119 / CIB-136: the DNS zone lookup (`azure.dns.getZoneOutput`) and the
// RecordSet definitions read and mutate the live production Azure DNS zone —
// and the zone lookup runs during `pulumi preview`, not just `pulumi up`. Only
// the trusted prod stack may touch it. Untrusted stacks (the CI PR preview
// runs an untrusted stack) skip DNS entirely, so the preview needs neither the
// `azure-dns:resourceGroupName` configuration nor any Azure credential.
function defineDns(): DnsZone {
  const resourceGroupName = new pulumi.Config('azure-dns').require('resourceGroupName');

  return new DnsZone('eddacraft-ai', {
    zoneName: 'eddacraft.ai',
    resourceGroupName,
    records: {
      // Root domain — Google Workspace SPF
      'root-txt-eddacraft-ai': {
        relativeRecordSetName: '@',
        recordType: 'TXT',
        txtRecords: [{ value: ['v=spf1 include:_spf.google.com ~all'] }],
      },

      // Root domain — DMARC policy
      'dmarc-eddacraft-ai': {
        relativeRecordSetName: '_dmarc',
        recordType: 'TXT',
        txtRecords: [{ value: ['v=DMARC1; p=none; rua=mailto:dmarc@eddacraft.ai'] }],
      },

      // api.eddacraft.ai — Anvil API on Vercel
      'api-cname-eddacraft-ai': {
        relativeRecordSetName: 'api',
        recordType: 'CNAME',
        cnameRecord: { cname: 'cname.vercel-dns.com' },
      },

      // updates.eddacraft.ai — Resend DKIM
      'resend-dkim-eddacraft-ai': {
        relativeRecordSetName: 'resend._domainkey.updates',
        recordType: 'TXT',
        txtRecords: [
          {
            value: [
              'p=MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQD0w0TxN5WlfxBXRhkFGrKLroVgvItGAulg3hz733ze/SDskSIUwOTPpr+E74AW+OcrCD3rw2Xw/mM4UesiQXYFMUCB4kAgrTSBi5WNMGE/G/46i+6ACgB89kX2Lq6oJnX6GnL6I9x8WfVParZFzPH3bULspN6FsXLzPEQ8iZpfYQIDAQAB',
            ],
          },
        ],
      },

      // send.updates.eddacraft.ai — Resend bounce-handling MX
      'mx-send-updates-eddacraft-ai': {
        relativeRecordSetName: 'send.updates',
        recordType: 'MX',
        mxRecords: [{ exchange: 'feedback-smtp.ap-northeast-1.amazonses.com', preference: 10 }],
      },

      // send.updates.eddacraft.ai — Resend SPF
      'txt-send-updates-eddacraft-ai': {
        relativeRecordSetName: 'send.updates',
        recordType: 'TXT',
        txtRecords: [{ value: ['v=spf1 include:amazonses.com ~all'] }],
      },

      // install.eddacraft.ai — GitHub Pages (Anvil CLI install scripts)
      'install-cname-eddacraft-ai': {
        relativeRecordSetName: 'install',
        recordType: 'CNAME',
        cnameRecord: { cname: 'eddacraft.github.io' },
      },
    },
  });
}

const trusted = isTrustedStack();
if (!trusted) {
  warnUntrustedSkip('production DNS zone records');
}

export const eddacraftAi = trusted ? defineDns() : undefined;
