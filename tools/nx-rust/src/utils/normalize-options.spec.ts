import { describe, expect, it } from 'vitest';
import { normalizeOptions } from './normalize-options';

const tree = {} as never;

describe('normalizeOptions', () => {
  it('derives projectRoot from directory + name', () => {
    const out = normalizeOptions(tree, { name: 'my-crate' });
    expect(out.projectRoot).toBe('crates/my-crate');
    expect(out.projectName).toBe('my-crate');
  });

  it('honours a custom directory', () => {
    const out = normalizeOptions(tree, {
      name: 'my-crate',
      directory: 'libs/rust',
    });
    expect(out.projectRoot).toBe('libs/rust/my-crate');
  });

  it('snake-cases the cargo name for libName', () => {
    const out = normalizeOptions(tree, { name: 'my-crate' });
    expect(out.libName).toBe('my_crate');
  });

  it('defaults edition to 2021', () => {
    const out = normalizeOptions(tree, { name: 'my-crate' });
    expect(out.edition).toBe('2021');
  });

  it('parses comma-separated tags into an array', () => {
    const out = normalizeOptions(tree, {
      name: 'c',
      tags: 'scope:core, type:lib',
    });
    expect(out.parsedTags).toEqual(['scope:core', 'type:lib']);
  });

  it('rejects an empty name', () => {
    expect(() => normalizeOptions(tree, { name: '   ' })).toThrow(/non-empty `name`/);
  });
});
