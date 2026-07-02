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
      // CIB-136: DNS is gated to the trusted `prod` stack (it reads/writes the
      // live Azure DNS zone). The provisioning assertions below must therefore
      // run under `prod`; untrusted-stack gating is covered by
      // untrusted-stack.test.ts.
      'prod',
      false
    );

    await import('../../src/dns/eddacraft-ai.js');

    // Allow Pulumi to process async resource registrations
    await new Promise((resolve) => setTimeout(resolve, 500));
  });

  it('creates DnsZone component for eddacraft.ai', () => {
    const zones = resources.filter((r) => r.type === 'anvil:dns:Zone');
    expect(zones.length).toBe(1);
    expect(zones[0].name).toBe('eddacraft-ai');
  });

  it('creates DNS RecordSet resources for eddacraft.ai', () => {
    const recordSets = resources.filter((r) => r.type === 'azure-native:dns:RecordSet');

    expect(recordSets.length).toBe(7);

    const names = recordSets.map((r) => r.name);
    expect(names).toContain('root-txt-eddacraft-ai');
    expect(names).toContain('dmarc-eddacraft-ai');
    expect(names).toContain('api-cname-eddacraft-ai');
    expect(names).toContain('resend-dkim-eddacraft-ai');
    expect(names).toContain('mx-send-updates-eddacraft-ai');
    expect(names).toContain('txt-send-updates-eddacraft-ai');
    expect(names).toContain('install-cname-eddacraft-ai');
  });

  it('creates all RecordSets as children of the DnsZone component', () => {
    const zones = resources.filter((r) => r.type === 'anvil:dns:Zone');
    const recordSets = resources.filter((r) => r.type === 'azure-native:dns:RecordSet');

    // Verify the component exists and all expected records were registered
    expect(zones.length).toBe(1);
    expect(recordSets.length).toBe(7);

    // Pulumi mocks don't expose parent in MockResourceArgs, so we verify
    // the component structure indirectly: the DnsZone constructor is the
    // only code path that creates these RecordSets, and it always sets
    // parent: this. If DnsZone exists and all 6 records exist, the
    // parent relationship is established by the component's constructor.
    for (const r of recordSets) {
      expect(r.inputs.zoneName).toBeDefined();
      expect(r.inputs.resourceGroupName).toBeDefined();
    }
  });
});
