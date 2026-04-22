import { describe, expect, it } from 'vitest';
import { buildCargoArgs } from './build-command';

const ctx = { projectName: 'my-crate' } as const;

describe('buildCargoArgs', () => {
  it('emits +toolchain before the subcommand', () => {
    const args = buildCargoArgs('build', { toolchain: 'nightly' }, ctx);
    expect(args.slice(0, 2)).toEqual(['+nightly', 'build']);
  });

  it('skips the toolchain token when stable', () => {
    const args = buildCargoArgs('build', { toolchain: 'stable' }, ctx);
    expect(args[0]).toBe('build');
  });

  it('appends --package from projectName when not supplied', () => {
    const args = buildCargoArgs('build', {}, ctx);
    expect(args).toContain('-p');
    expect(args[args.indexOf('-p') + 1]).toBe('my-crate');
  });

  it('respects explicit package override', () => {
    const args = buildCargoArgs('build', { package: 'other' }, ctx);
    expect(args[args.indexOf('-p') + 1]).toBe('other');
  });

  it('joins features with a comma into a single flag', () => {
    const args = buildCargoArgs('build', { features: ['serde', 'tokio', 'async'] }, ctx);
    const featuresIdx = args.indexOf('--features');
    expect(featuresIdx).toBeGreaterThan(-1);
    expect(args[featuresIdx + 1]).toBe('serde,tokio,async');
    // Only one --features flag, not three
    expect(args.filter((a) => a === '--features')).toHaveLength(1);
  });

  it('repeats non-features array flags', () => {
    // `bin` isn't in BaseCargoOptions but the runtime iterator walks every
    // own property, so we cast through unknown to exercise the general path.
    const args = buildCargoArgs(
      'build',
      { bin: ['a', 'b'] } as unknown as Parameters<typeof buildCargoArgs>[1],
      ctx
    );
    expect(args.filter((a) => a === '--bin')).toHaveLength(2);
  });

  it('emits boolean flags only when true', () => {
    const args = buildCargoArgs('build', { release: true, 'all-features': false }, ctx);
    expect(args).toContain('--release');
    expect(args).not.toContain('--all-features');
  });

  it('drops --release when a profile is set', () => {
    const args = buildCargoArgs('build', { release: true, profile: 'dev' }, ctx);
    expect(args).not.toContain('--release');
    expect(args).toContain('--profile');
  });

  it('splits passthrough args under --', () => {
    const args = buildCargoArgs('run', { args: ['hello', 'world'] }, ctx);
    const dashIdx = args.indexOf('--');
    expect(dashIdx).toBeGreaterThan(-1);
    expect(args.slice(dashIdx + 1)).toEqual(['hello', 'world']);
  });

  it('skips undefined and null values', () => {
    const args = buildCargoArgs(
      'build',
      { target: undefined, manifest: null } as unknown as Record<string, unknown>,
      ctx
    );
    expect(args).not.toContain('--target');
    expect(args).not.toContain('--manifest');
  });
});
