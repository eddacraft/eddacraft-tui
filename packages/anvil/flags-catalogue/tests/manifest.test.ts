import { describe, expect, it } from 'vitest';
import {
  FeatureFlagManifestSchema,
  FlagAudienceManifestSchema,
  FlagEnvironmentManifestSchema,
  FlagGroupManifestSchema,
} from '@eddacraft/anvil-contracts';
import {
  API_SCOPE_FLAGS,
  API_SCOPE_NAMES,
  CLI_LICENCE_GATE,
  CLI_LICENCE_GATE_KEY,
  DEFAULT_APPROVAL_SCOPES,
  DOCS_ACCESS_FLAG,
  DOCS_ACCESS_FLAG_KEY,
  GCTX_EGRESS_FLAG,
  GCTX_EGRESS_FLAG_KEY,
  featureFlagManifest,
  flagAudiences,
  flagByKey,
  flagEnvironments,
  flagGroups,
  tryFlagByKey,
} from '../src/index.js';

function assertOperationalInventoryMutationsAreRejected(): void {
  const manifest = featureFlagManifest();
  // @ts-expect-error the authoritative manifest collection is deeply readonly
  manifest.flags.push(manifest.flags[0]!);
  const groups = flagGroups();
  // @ts-expect-error nested group defaults are deeply readonly
  groups.groups[0]!.defaultAudiences.push('tampered');
  const audiences = flagAudiences();
  // @ts-expect-error audience entries are deeply readonly
  audiences.audiences[0]!.name = 'tampered';
  const environments = flagEnvironments();
  // @ts-expect-error environment entries are deeply readonly
  environments.environments[0]!.name = 'tampered';
  const flag = flagByKey(manifest.flags[0]!.key);
  // @ts-expect-error derived flag definitions keep readonly nested arrays
  flag.variants.push(flag.variants[0]!);
}

describe('flags catalogue manifest', () => {
  it('validates against FeatureFlagManifestSchema', () => {
    expect(FeatureFlagManifestSchema.safeParse(featureFlagManifest()).success).toBe(true);
  });

  it('contains exactly the eighteen shipped flags', () => {
    const keys = featureFlagManifest().flags.map((f) => f.key);
    expect(keys).toEqual([
      'api.scope.beta',
      'api.scope.internal',
      'api.scope.preview',
      'cli.licence-gate',
      'daemon.persist-graph',
      'dashboard.web',
      'docs.access',
      'gctx.egress',
      'gv2.reverse-impact-depth',
      'impact.view',
      'kindling.embedded-runtime',
      'track.pack',
      'track.surface',
      'track.surface.dock',
      'track.surface.gha',
      'track.surface.sh',
      'track.surface.sql',
      'tui-dashboard.aps-dashboard',
    ]);
  });

  it('is sorted by key', () => {
    const keys = featureFlagManifest().flags.map((f) => f.key);
    expect(keys).toEqual([...keys].sort());
  });

  it('every flag carries a primaryGroup that exists in groups.json', () => {
    const groupIds = new Set(flagGroups().groups.map((g) => g.id));
    for (const flag of featureFlagManifest().flags) {
      expect(flag.primaryGroup, `${flag.key} missing primaryGroup`).toBeDefined();
      expect(groupIds.has(flag.primaryGroup as string), `${flag.key} -> ${flag.primaryGroup}`).toBe(
        true
      );
    }
  });
});

describe('typed accessors', () => {
  it('CLI_LICENCE_GATE matches the manifest entry', () => {
    expect(CLI_LICENCE_GATE).toEqual(flagByKey(CLI_LICENCE_GATE_KEY));
    expect(CLI_LICENCE_GATE.key).toBe('cli.licence-gate');
    expect(CLI_LICENCE_GATE.primaryGroup).toBe('cli');
  });

  it('GCTX_EGRESS_FLAG matches the manifest entry', () => {
    expect(GCTX_EGRESS_FLAG).toEqual(flagByKey(GCTX_EGRESS_FLAG_KEY));
    expect(GCTX_EGRESS_FLAG.key).toBe('gctx.egress');
    expect(GCTX_EGRESS_FLAG.defaultVariant).toBe('disabled');
    expect(GCTX_EGRESS_FLAG.primaryGroup).toBe('daemon');
  });

  it('DOCS_ACCESS_FLAG matches the manifest and uses canonical audience ids', () => {
    expect(DOCS_ACCESS_FLAG).toEqual(flagByKey(DOCS_ACCESS_FLAG_KEY));
    expect(DOCS_ACCESS_FLAG.defaultVariant).toBe('disabled');
    expect(DOCS_ACCESS_FLAG.targeting?.[0]?.conditions[0]?.value).toEqual([
      'plan-beta',
      'plan-pro',
      'plan-enterprise',
    ]);
  });

  it('API_SCOPE_FLAGS covers every scope name', () => {
    expect(Object.keys(API_SCOPE_FLAGS).sort()).toEqual([...API_SCOPE_NAMES].sort());
    for (const name of API_SCOPE_NAMES) {
      expect(API_SCOPE_FLAGS[name].key).toBe(`api.scope.${name}`);
      expect(API_SCOPE_FLAGS[name].primaryGroup).toBe('api');
    }
  });

  it('DEFAULT_APPROVAL_SCOPES is [beta]', () => {
    expect(DEFAULT_APPROVAL_SCOPES).toEqual(['beta']);
  });

  it('tryFlagByKey returns undefined for an unknown key', () => {
    expect(tryFlagByKey('nope.missing')).toBeUndefined();
  });

  it('flagByKey throws for an unknown key', () => {
    expect(() => flagByKey('nope.missing')).toThrow();
  });
});

describe('gating-model inventories', () => {
  it('groups.json validates and carries the eleven primary groups', () => {
    expect(FlagGroupManifestSchema.safeParse(flagGroups()).success).toBe(true);
    expect(flagGroups().groups.map((g) => g.id)).toEqual([
      'cli',
      'docs',
      'api',
      'dashboard',
      'tui-dashboard',
      'ide',
      'daemon',
      'hook',
      'gv2',
      'track-surface',
      'track-pack',
    ]);
  });

  it('audiences.json validates and carries the ten canonical audiences', () => {
    expect(FlagAudienceManifestSchema.safeParse(flagAudiences()).success).toBe(true);
    expect(flagAudiences().audiences).toHaveLength(10);
  });

  it('environments.json validates with the renamed five-environment set', () => {
    expect(FlagEnvironmentManifestSchema.safeParse(flagEnvironments()).success).toBe(true);
    const ids = flagEnvironments().environments.map((e) => e.id);
    expect(ids).toEqual(['local', 'development', 'preview', 'demo', 'production']);
    expect(ids).not.toContain('prod');
    expect(ids).not.toContain('dev');
    expect(ids).not.toContain('staging');
  });

  it('every group defaultAudience exists in the audience inventory', () => {
    const audienceIds = new Set(flagAudiences().audiences.map((a) => a.id));
    for (const group of flagGroups().groups) {
      for (const aud of group.defaultAudiences) {
        expect(audienceIds.has(aud), `${group.id} -> ${aud}`).toBe(true);
      }
    }
  });
});

describe('read-only operational inventories', () => {
  it('rejects nested mutation before derived flag lookup', () => {
    const manifest = featureFlagManifest();
    const groups = flagGroups();
    const audiences = flagAudiences();
    const environments = flagEnvironments();
    const flag = manifest.flags[0]!;
    const group = groups.groups[0]!;
    const audience = audiences.audiences[0]!;
    const environment = environments.environments[0]!;
    const before = {
      manifest: JSON.stringify(manifest),
      groups: JSON.stringify(groups),
      audiences: JSON.stringify(audiences),
      environments: JSON.stringify(environments),
      flag: JSON.stringify(flagByKey(flag.key)),
    };

    for (const value of [
      manifest,
      manifest.flags,
      flag,
      flag.variants,
      flag.variants[0],
      groups,
      groups.groups,
      group,
      group.defaultAudiences,
      audiences,
      audiences.audiences,
      audience,
      environments,
      environments.environments,
      environment,
    ]) {
      expect(Object.isFrozen(value)).toBe(true);
    }

    expect(Reflect.set(flag.variants[0]!, 'key', 'tampered')).toBe(false);
    expect(() => Array.prototype.push.call(group.defaultAudiences, 'tampered')).toThrow(TypeError);
    expect(Reflect.set(audience, 'name', 'tampered')).toBe(false);
    expect(Reflect.set(environment, 'name', 'tampered')).toBe(false);

    expect(JSON.stringify(featureFlagManifest())).toBe(before.manifest);
    expect(JSON.stringify(flagGroups())).toBe(before.groups);
    expect(JSON.stringify(flagAudiences())).toBe(before.audiences);
    expect(JSON.stringify(flagEnvironments())).toBe(before.environments);
    expect(JSON.stringify(flagByKey(flag.key))).toBe(before.flag);
    expect(flagByKey(flag.key)).toBe(flag);
    expect(assertOperationalInventoryMutationsAreRejected).toBeTypeOf('function');
  });
});
