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
  featureFlagManifest,
  flagAudiences,
  flagByKey,
  flagEnvironments,
  flagGroups,
  tryFlagByKey,
} from '../src/index.js';

describe('flags catalogue manifest', () => {
  it('validates against FeatureFlagManifestSchema', () => {
    expect(FeatureFlagManifestSchema.safeParse(featureFlagManifest()).success).toBe(true);
  });

  it('contains exactly the five shipped flags', () => {
    const keys = featureFlagManifest().flags.map((f) => f.key);
    expect(keys).toEqual([
      'api.scope.beta',
      'api.scope.internal',
      'api.scope.preview',
      'cli.licence-gate',
      'docs.access',
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
  it('groups.json validates and carries the seven primary groups', () => {
    expect(FlagGroupManifestSchema.safeParse(flagGroups()).success).toBe(true);
    expect(flagGroups().groups.map((g) => g.id)).toEqual([
      'cli',
      'docs',
      'api',
      'dashboard',
      'ide',
      'daemon',
      'hook',
    ]);
  });

  it('audiences.json validates and carries the nine canonical audiences', () => {
    expect(FlagAudienceManifestSchema.safeParse(flagAudiences()).success).toBe(true);
    expect(flagAudiences().audiences).toHaveLength(9);
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
