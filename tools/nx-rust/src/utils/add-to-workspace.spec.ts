import { createTree } from '@nx/devkit/testing';
import { describe, expect, it } from 'vitest';
import { addToCargoWorkspace } from './add-to-workspace';

describe('addToCargoWorkspace', () => {
  it('bootstraps a new Cargo.toml when none exists', () => {
    const tree = createTree();
    addToCargoWorkspace(tree, 'crates/my-crate');
    const written = tree.read('Cargo.toml')?.toString() ?? '';
    expect(written).toContain('[workspace]');
    expect(written).toContain('crates/my-crate');
  });

  it('appends to an existing [workspace] members list', () => {
    const tree = createTree();
    tree.write('Cargo.toml', '[workspace]\nresolver = "2"\nmembers = ["crates/existing"]\n');
    addToCargoWorkspace(tree, 'crates/new-crate');
    const written = tree.read('Cargo.toml')?.toString() ?? '';
    expect(written).toContain('crates/existing');
    expect(written).toContain('crates/new-crate');
  });

  it('is idempotent for an exact-match existing member', () => {
    const tree = createTree();
    tree.write('Cargo.toml', '[workspace]\nmembers = ["crates/foo"]\n');
    const before = tree.read('Cargo.toml')?.toString();
    addToCargoWorkspace(tree, 'crates/foo');
    const after = tree.read('Cargo.toml')?.toString();
    expect(after).toBe(before);
  });

  it('does not duplicate when a glob entry already covers the path', () => {
    const tree = createTree();
    tree.write('Cargo.toml', '[workspace]\nmembers = ["crates/*"]\n');
    addToCargoWorkspace(tree, 'crates/new-crate');
    const written = tree.read('Cargo.toml')?.toString() ?? '';
    // Should NOT add a literal `crates/new-crate` alongside the glob.
    expect(written).not.toMatch(/crates\/new-crate/);
  });
});
