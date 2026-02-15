import * as azure from '@pulumi/azure-native';
import { zone, resourceGroupName } from './index.js';

// =============================================================================
// Root domain (eddacraft.ai) — Google Workspace
// MX records managed manually in Azure DNS (not by Pulumi)
// =============================================================================

// Root TXT records (Unosend domain verification + Google Workspace SPF)
new azure.dns.RecordSet('root-txt-eddacraft-ai', {
  relativeRecordSetName: '@',
  zoneName: zone.eddacraftAi.name,
  resourceGroupName,
  recordType: 'TXT',
  ttl: 3600,
  txtRecords: [
    { value: ['_cux88fmbdoc8oeyu9sy0paxt0yd4mzm'] },
    { value: ['v=spf1 include:_spf.google.com ~all'] },
  ],
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
// Subdomain (mail.eddacraft.ai) — Unosend transactional email
// =============================================================================

// Unosend inbound MX record
new azure.dns.RecordSet('mx-mail-eddacraft-ai', {
  relativeRecordSetName: 'mail',
  zoneName: zone.eddacraftAi.name,
  resourceGroupName,
  recordType: 'MX',
  ttl: 3600,
  mxRecords: [{ exchange: 'mail.unosend.co', preference: 10 }],
});

// SPF for Unosend sending via Amazon SES
new azure.dns.RecordSet('txt-mail-eddacraft-ai', {
  relativeRecordSetName: 'mail',
  zoneName: zone.eddacraftAi.name,
  resourceGroupName,
  recordType: 'TXT',
  ttl: 3600,
  txtRecords: [{ value: ['v=spf1 include:amazonses.com ~all'] }],
});

// DMARC for mail subdomain
new azure.dns.RecordSet('dmarc-mail-eddacraft-ai', {
  relativeRecordSetName: '_dmarc.mail',
  zoneName: zone.eddacraftAi.name,
  resourceGroupName,
  recordType: 'TXT',
  ttl: 3600,
  txtRecords: [{ value: ['v=DMARC1; p=none; rua=mailto:dmarc@eddacraft.ai'] }],
});

// DKIM for Unosend on mail subdomain
// TODO: Update with DKIM key from Unosend after verifying mail.eddacraft.ai
new azure.dns.RecordSet('unosend-dkim-mail-eddacraft-ai', {
  relativeRecordSetName: 'unosend._domainkey.mail',
  zoneName: zone.eddacraftAi.name,
  resourceGroupName,
  recordType: 'TXT',
  ttl: 3600,
  txtRecords: [
    {
      value: [
        'v=DKIM1; k=rsa; p=MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAtd1NhcoEly2Ih7nSDZ9Th6FY8s3C1LLop+WyGpkQgxlAOpWaG66L8fOQVRj7MFZ4YEHTkGIYbPwUQ7qkowxRjKc9WgVw8tkV/66tZYs9YffKoKrXt6gzcs7tlxoBf8Yzotd0f94mmUoNPaAZAmzMsL3KFhi8MUSwB0sElRzHLk9',
        'qNr2czNzlHwiCqI6j5H7HUUf2OCw5NWyniMCOpT/vQ1S5wv+oeT5sLE6rXN4Njh0Qj9Z9hj4rZNwl1T0eIu7iL2MIfRg6vnMGtUCcTlzO2RfnxlaBnHuOFHzKed/MR3zv1b5tXTLTMbkP8MuSeKYYu0GpF6hxk8HTYkHeJu0yfwIDAQAB',
      ],
    },
  ],
});
