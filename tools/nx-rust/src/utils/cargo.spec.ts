import { describe, expect, it } from 'vitest';
import { isExternal } from './cargo';
import type { CargoDependency, CargoPackage } from '../models/cargo-metadata';

const makeDep = (over: Partial<CargoDependency> = {}): CargoDependency => ({
  name: 'some',
  req: '^1',
  optional: false,
  uses_default_features: true,
  features: [],
  ...over,
});

const makePkg = (over: Partial<CargoPackage> = {}): CargoPackage => ({
  name: 'some',
  version: '0.1.0',
  id: 'some 0.1.0',
  dependencies: [],
  targets: [],
  features: {},
  manifest_path: '/ws/crates/some/Cargo.toml',
  ...over,
});

describe('isExternal', () => {
  const ws = '/ws';

  it('treats registry source as external', () => {
    expect(isExternal(makeDep({ source: 'registry+https://crates.io' }), ws)).toBe(true);
  });

  it('treats git source as external', () => {
    expect(isExternal(makeDep({ source: 'git+https://github.com/x/y' }), ws)).toBe(true);
  });

  it('treats a path dep inside the workspace as internal', () => {
    expect(isExternal(makeDep({ path: '/ws/crates/other', source: null }), ws)).toBe(false);
  });

  it('treats a path dep outside the workspace as external', () => {
    expect(isExternal(makeDep({ path: '/elsewhere/other', source: null }), ws)).toBe(true);
  });

  it('treats a workspace-inherited dep with no source and no path as external', () => {
    // Common for `{ workspace = true }` inheritance under some cargo versions.
    expect(isExternal(makeDep({ source: null }), ws)).toBe(true);
  });

  it('classifies workspace-member packages as internal', () => {
    expect(isExternal(makePkg(), ws)).toBe(false);
  });

  it('classifies packages with manifest_path outside workspace as external', () => {
    expect(isExternal(makePkg({ manifest_path: '/elsewhere/other/Cargo.toml' }), ws)).toBe(true);
  });
});
