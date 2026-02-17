import * as azure from '@pulumi/azure-native';
import { zone, resourceGroupName } from './index.js';

// =============================================================================
// Root domain (eddacraft.ai) — Google Workspace
// MX records managed manually in Azure DNS (not by Pulumi)
// =============================================================================

// Root TXT records (Google Workspace SPF)
new azure.dns.RecordSet('root-txt-eddacraft-ai', {
  relativeRecordSetName: '@',
  zoneName: zone.eddacraftAi.name,
  resourceGroupName,
  recordType: 'TXT',
  ttl: 3600,
  txtRecords: [{ value: ['v=spf1 include:_spf.google.com ~all'] }],
});

// DMARC policy for root domain
new azure.dns.RecordSet('dmarc-eddacraft-ai', {
  relativeRecordSetName: '_dmarc',
  zoneName: zone.eddacraftAi.name,
  resourceGroupName,
  recordType: 'TXT',
  ttl: 3600,
  txtRecords: [{ value: ['v=DMARC1; p=none; rua=mailto:dmarc@eddacraft.ai'] }],
});

// =============================================================================
// Subdomain (updates.eddacraft.ai) — Resend transactional email
// =============================================================================

// Resend DKIM record
new azure.dns.RecordSet('resend-dkim-eddacraft-ai', {
  relativeRecordSetName: 'resend._domainkey.updates',
  zoneName: zone.eddacraftAi.name,
  resourceGroupName,
  recordType: 'TXT',
  ttl: 3600,
  txtRecords: [
    {
      value: [
        'p=MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQD0w0TxN5WlfxBXRhkFGrKLroVgvItGAulg3hz733ze/SDskSIUwOTPpr+E74AW+OcrCD3rw2Xw/mM4UesiQXYFMUCB4kAgrTSBi5WNMGE/G/46i+6ACgB89kX2Lq6oJnX6GnL6I9x8WfVParZFzPH3bULspN6FsXLzPEQ8iZpfYQIDAQAB',
      ],
    },
  ],
});

// Resend bounce-handling MX record
new azure.dns.RecordSet('mx-send-updates-eddacraft-ai', {
  relativeRecordSetName: 'send.updates',
  zoneName: zone.eddacraftAi.name,
  resourceGroupName,
  recordType: 'MX',
  ttl: 3600,
  mxRecords: [{ exchange: 'feedback-smtp.ap-northeast-1.amazonses.com', preference: 10 }],
});

// SPF for Resend sending
new azure.dns.RecordSet('txt-send-updates-eddacraft-ai', {
  relativeRecordSetName: 'send.updates',
  zoneName: zone.eddacraftAi.name,
  resourceGroupName,
  recordType: 'TXT',
  ttl: 3600,
  txtRecords: [
    {
      value: ['v=spf1 include:amazonses.com ~all'],
    },
  ],
});
