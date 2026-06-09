import { describe, expect, it } from 'vitest';
import { FlagSurfaceManifestSchema } from '@eddacraft/anvil-contracts';
import { flagAudiences, flagSurfaces, mustAlwaysBeOpenSurfaces } from '../src/index.js';

// ADR-076: the surface registry back-capture. These are the static checks the
// ADR puts in scope (existence, acyclicity, the MUST_ALWAYS_BE_OPEN floor,
// cross-inventory integrity). Runtime cascade-off / auth-list derivation are
// deferred and intentionally not exercised here.

describe('surface registry (flags/surfaces.json)', () => {
  it('validates against FlagSurfaceManifestSchema', () => {
    expect(FlagSurfaceManifestSchema.safeParse(flagSurfaces()).success).toBe(true);
  });

  it('carries the nine capability categories', () => {
    // Set comparison — category order in the JSON is not a schema invariant.
    expect(new Set(flagSurfaces().categories.map((c) => c.id))).toEqual(
      new Set([
        'governance',
        'mcp',
        'dashboard',
        'save-time',
        'hooks',
        'admin',
        'tools',
        'setup',
        'foundational',
      ])
    );
  });

  it('back-captures the full CLI surface inventory', () => {
    // ~43 features after the per-view dashboard split (ADR-076 appendix).
    expect(flagSurfaces().surfaces.length).toBeGreaterThanOrEqual(40);
  });

  it('every surface references a defined category', () => {
    const categoryIds = new Set(flagSurfaces().categories.map((c) => c.id));
    for (const s of flagSurfaces().surfaces) {
      expect(categoryIds.has(s.category), `${s.key} -> ${s.category}`).toBe(true);
    }
  });

  it('every requires-target exists', () => {
    const keys = new Set(flagSurfaces().surfaces.map((s) => s.key));
    for (const s of flagSurfaces().surfaces) {
      for (const dep of s.requires ?? []) {
        expect(keys.has(dep), `${s.key} requires ${dep}`).toBe(true);
      }
    }
  });

  it('every gating audience exists in the audience inventory', () => {
    const audienceIds = new Set(flagAudiences().audiences.map((a) => a.id));
    for (const s of flagSurfaces().surfaces) {
      for (const a of s.audiences ?? []) {
        expect(audienceIds.has(a), `${s.key} -> ${a}`).toBe(true);
      }
    }
  });

  it('the MUST_ALWAYS_BE_OPEN floor is the recovery-critical surfaces and they are open', () => {
    // Intentionally hardcoded — adding a surface to the floor requires deliberate review.
    expect([...mustAlwaysBeOpenSurfaces()].sort()).toEqual(['admin.credential', 'auth']);
    const byKey = new Map(flagSurfaces().surfaces.map((s) => [s.key, s]));
    const categoryDefault = new Map(flagSurfaces().categories.map((c) => [c.id, c.defaultAccess]));
    for (const key of mustAlwaysBeOpenSurfaces()) {
      const s = byKey.get(key)!;
      const effective = s.access ?? categoryDefault.get(s.category);
      expect(effective, `${key} must resolve open`).toBe('open');
    }
  });

  it('system-invoked surfaces are marked (git hooks must not be kill-switched)', () => {
    const hook = flagSurfaces().surfaces.find((s) => s.key === 'hook');
    expect(hook?.invocation).toBe('system');
  });

  it('the CIB-046 dashboard.aps surface is staff-gated', () => {
    const aps = flagSurfaces().surfaces.find((s) => s.key === 'dashboard.aps');
    expect(aps?.access).toBe('staff');
    expect(aps?.audiences).toContain('staff-internal-developer');
  });
});

describe('surface registry schema rejects malformed registries', () => {
  const base = {
    schemaVersion: 1,
    categories: [
      { id: 'governance', name: 'Governance', defaultAccess: 'licence', defaultStatus: 'active' },
      { id: 'tools', name: 'Tools', defaultAccess: 'open', defaultStatus: 'active' },
    ],
  };

  it('rejects an unknown requires-target', () => {
    const result = FlagSurfaceManifestSchema.safeParse({
      ...base,
      surfaces: [{ key: 'a', name: 'a', category: 'tools', requires: ['nope'] }],
    });
    expect(result.success).toBe(false);
  });

  it('rejects a cycle in the requires graph', () => {
    const result = FlagSurfaceManifestSchema.safeParse({
      ...base,
      surfaces: [
        { key: 'a', name: 'a', category: 'tools', requires: ['b'] },
        { key: 'b', name: 'b', category: 'tools', requires: ['a'] },
      ],
    });
    expect(result.success).toBe(false);
    if (!result.success) {
      expect(JSON.stringify(result.error)).toContain('acyclic');
    }
  });

  it('rejects a self-loop in the requires graph', () => {
    const result = FlagSurfaceManifestSchema.safeParse({
      ...base,
      surfaces: [{ key: 'a', name: 'a', category: 'tools', requires: ['a'] }],
    });
    expect(result.success).toBe(false);
  });

  it('rejects duplicate category ids', () => {
    const result = FlagSurfaceManifestSchema.safeParse({
      schemaVersion: 1,
      categories: [
        { id: 'tools', name: 'Tools', defaultAccess: 'open', defaultStatus: 'active' },
        { id: 'tools', name: 'Tools 2', defaultAccess: 'open', defaultStatus: 'active' },
      ],
      surfaces: [],
    });
    expect(result.success).toBe(false);
  });

  it('rejects a mustAlwaysBeOpen surface that resolves to a gated access', () => {
    const result = FlagSurfaceManifestSchema.safeParse({
      ...base,
      surfaces: [{ key: 'a', name: 'a', category: 'governance', mustAlwaysBeOpen: true }],
    });
    expect(result.success).toBe(false);
  });

  it('rejects an unknown category reference', () => {
    const result = FlagSurfaceManifestSchema.safeParse({
      ...base,
      surfaces: [{ key: 'a', name: 'a', category: 'ghost' }],
    });
    expect(result.success).toBe(false);
  });

  it('rejects duplicate surface keys', () => {
    const result = FlagSurfaceManifestSchema.safeParse({
      ...base,
      surfaces: [
        { key: 'a', name: 'a', category: 'tools' },
        { key: 'a', name: 'a2', category: 'tools' },
      ],
    });
    expect(result.success).toBe(false);
  });
});
