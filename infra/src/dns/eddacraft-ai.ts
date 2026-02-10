import * as azure from '@pulumi/azure-native';
import { zone, resourceGroupName } from './index.js';

// DKIM for Unosend (Amazon SES)
new azure.dns.RecordSet('unosend-dkim-eddacraft-ai', {
  relativeRecordSetName: 'unosend._domainkey',
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

// MX for send subdomain (Amazon SES)
new azure.dns.RecordSet('send-mx-eddacraft-ai', {
  relativeRecordSetName: 'send',
  zoneName: zone.eddacraftAi.name,
  resourceGroupName,
  recordType: 'MX',
  ttl: 3600,
  mxRecords: [
    {
      exchange: 'feedback-smtp.us-east-1.amazonses.com',
      preference: 10,
    },
  ],
});

// SPF for send subdomain (Amazon SES)
new azure.dns.RecordSet('send-spf-eddacraft-ai', {
  relativeRecordSetName: 'send',
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
