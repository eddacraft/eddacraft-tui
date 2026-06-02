import { describe, expect, it } from 'vitest';
import { featureFlagManifest, flagAudiences, flagEnvironments } from '../src/index.js';

// FLAGCAT-006 consistency check: cross-manifest references + the naming map,
// run on every CI invocation via `pnpm nx test flags-catalogue`. This promotes
// the catalogue's fail-loud load-time integrity assert into explicit, named
// assertions and adds the rules that assert isn't (yet) responsible for:
// targeting-value membership, the JSON-key → Rust/TS naming map, and key
// reservation. (primaryGroup ∈ groups.json and group defaultAudiences ∈
// audiences.json are asserted in manifest.test.ts.)

// Canonical-audience targeting attributes — values under these must resolve to
// a `flags/audiences.json` id. `organisationId` is free-form per-tenant and
// `targetingKey` is the session key, so both are excluded.
const AUDIENCE_ATTRS = new Set(['accountTier', 'licencePlan', 'userRole', 'cohort']);

function targetingValues(value: unknown): string[] {
  if (Array.isArray(value)) return value.map(String);
  if (typeof value === 'string') return [value];
  return []; // numeric (percentage) — not an inventory reference
}

describe('FLAGCAT-006 cross-manifest references', () => {
  const flags = featureFlagManifest().flags;
  const audienceIds = new Set(flagAudiences().audiences.map((a) => a.id));
  const envIds = new Set(flagEnvironments().environments.map((e) => e.id));

  it('every canonical-audience targeting value exists in audiences.json', () => {
    for (const flag of flags) {
      for (const rule of flag.targeting ?? []) {
        for (const cond of rule.conditions) {
          if (!AUDIENCE_ATTRS.has(cond.attribute)) continue;
          for (const v of targetingValues(cond.value)) {
            expect(
              audienceIds.has(v),
              `${flag.key}: audience "${v}" (attr ${cond.attribute})`
            ).toBe(true);
          }
        }
      }
    }
  });

  it('every environment targeting value exists in environments.json', () => {
    for (const flag of flags) {
      for (const rule of flag.targeting ?? []) {
        for (const cond of rule.conditions) {
          if (cond.attribute !== 'environment') continue;
          for (const v of targetingValues(cond.value)) {
            expect(envIds.has(v), `${flag.key}: environment "${v}"`).toBe(true);
          }
        }
      }
    }
  });
});

describe('FLAGCAT-006 naming map + key reservation', () => {
  const flags = featureFlagManifest().flags;
  // JSON key → Rust module path: `.` / `-` → `_` (must match build.rs).
  const rustModule = (key: string) => key.replace(/[.-]/g, '_');

  it('each flag key derives a unique Rust module name (no collision)', () => {
    const byModule = new Map<string, string>();
    for (const flag of flags) {
      const module = rustModule(flag.key);
      expect(
        byModule.has(module),
        `keys "${byModule.get(module)}" and "${flag.key}" both map to Rust module "${module}"`
      ).toBe(false);
      byModule.set(module, flag.key);
    }
  });

  it('flag keys are unique — ADR-041 stable join keys are never reused', () => {
    const keys = flags.map((f) => f.key);
    expect(new Set(keys).size).toBe(keys.length);
  });
});
