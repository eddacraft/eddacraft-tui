import { describe, it, expect, beforeAll } from 'vitest';
import * as pulumi from '@pulumi/pulumi';

describe('DNS resources', () => {
  const resources: pulumi.runtime.MockResourceArgs[] = [];

  beforeAll(async () => {
    pulumi.runtime.setAllConfig({
      'azure-dns:resourceGroupName': 'rg-prd-ap-public-web',
    });
    pulumi.runtime.setMocks(
      {
        newResource(args: pulumi.runtime.MockResourceArgs) {
          resources.push(args);
          return { id: `${args.name}-mock-id`, state: args.inputs };
        },
        call(args: pulumi.runtime.MockCallArgs) {
          return {
            ...args.inputs,
            name: args.inputs.zoneName ?? 'mock-zone',
          };
        },
      },
      'test-project',
      'test-stack',
      false
    );

    await import('../../src/dns/index.js');
    await import('../../src/dns/eddacraft-ai.js');

    // Allow Pulumi to process async resource registrations
    await new Promise((resolve) => setTimeout(resolve, 500));
  });

  it('creates DNS RecordSet resources for eddacraft.ai', () => {
    const recordSets = resources.filter((r) => r.type === 'azure-native:dns:RecordSet');

    expect(recordSets.length).toBe(5);

    const names = recordSets.map((r) => r.name);
    expect(names).toContain('root-txt-eddacraft-ai');
    expect(names).toContain('dmarc-eddacraft-ai');
    expect(names).toContain('resend-dkim-eddacraft-ai');
    expect(names).toContain('mx-send-updates-eddacraft-ai');
    expect(names).toContain('txt-send-updates-eddacraft-ai');
  });
});
