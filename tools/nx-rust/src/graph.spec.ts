import type { CreateDependenciesContext } from '@nx/devkit';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { CargoMetadata } from './models/cargo-metadata';

// Mock `cargoMetadata` so tests don't invoke the real cargo binary.
vi.mock('./utils/cargo', async () => {
  const actual = await vi.importActual<typeof import('./utils/cargo')>('./utils/cargo');
  return {
    ...actual,
    cargoMetadata: vi.fn(),
  };
});

async function load() {
  const graph = await import('./graph');
  graph.__resetGraphCacheForTests();
  return graph;
}

async function setMetadata(metadata: CargoMetadata | null) {
  const cargo = await import('./utils/cargo');
  (cargo.cargoMetadata as ReturnType<typeof vi.fn>).mockReturnValue(metadata);
}

const ws = '/ws';

function pkg(over: Partial<CargoMetadata['packages'][0]>): CargoMetadata['packages'][0] {
  return {
    name: 'x',
    version: '0.1.0',
    id: `${over.name ?? 'x'} 0.1.0`,
    dependencies: [],
    targets: [{ kind: ['lib'], crate_types: ['lib'], name: 'x', src_path: '' }],
    features: {},
    manifest_path: `/ws/crates/${over.name ?? 'x'}/Cargo.toml`,
    ...over,
  };
}

function md(packages: CargoMetadata['packages']): CargoMetadata {
  return {
    packages,
    workspace_members: packages.map((p) => p.id),
    workspace_root: ws,
    resolve: null,
    target_directory: `${ws}/target`,
    version: 1,
    metadata: null,
  };
}

describe('createDependencies', () => {
  beforeEach(async () => {
    const { __resetGraphCacheForTests } = await load();
    __resetGraphCacheForTests();
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('emits a static edge between two workspace members with a path dep', async () => {
    await setMetadata(
      md([
        pkg({
          name: 'app',
          targets: [{ kind: ['bin'], crate_types: ['bin'], name: 'app', src_path: '' }],
          dependencies: [
            {
              name: 'lib',
              req: '*',
              path: '/ws/crates/lib',
              source: null,
              optional: false,
              uses_default_features: true,
              features: [],
              kind: null,
            },
          ],
        }),
        pkg({ name: 'lib' }),
      ])
    );
    const { createDependencies } = await load();

    const ctx: CreateDependenciesContext = {
      workspaceRoot: ws,
      projects: {
        app: { root: 'crates/app', name: 'app' },
        lib: { root: 'crates/lib', name: 'lib' },
      } as never,
      externalNodes: {},
      fileMap: { projectFileMap: {}, nonProjectFiles: [] },
      filesToProcess: { projectFileMap: {}, nonProjectFiles: [] },
      nxJsonConfiguration: {},
    };

    const edges = createDependencies({}, ctx);
    expect(edges).toHaveLength(1);
    expect(edges[0]).toMatchObject({ source: 'app', target: 'lib' });
  });

  it('skips dev-dependencies so they do not trigger downstream rebuilds', async () => {
    await setMetadata(
      md([
        pkg({
          name: 'app',
          dependencies: [
            {
              name: 'lib',
              req: '*',
              path: '/ws/crates/lib',
              source: null,
              optional: false,
              uses_default_features: true,
              features: [],
              kind: 'dev',
            },
          ],
        }),
        pkg({ name: 'lib' }),
      ])
    );
    const { createDependencies } = await load();

    const ctx: CreateDependenciesContext = {
      workspaceRoot: ws,
      projects: {
        app: { root: 'crates/app', name: 'app' },
        lib: { root: 'crates/lib', name: 'lib' },
      } as never,
      externalNodes: {},
      fileMap: { projectFileMap: {}, nonProjectFiles: [] },
      filesToProcess: { projectFileMap: {}, nonProjectFiles: [] },
      nxJsonConfiguration: {},
    };

    expect(createDependencies({}, ctx)).toHaveLength(0);
  });

  it('emits external edges for registry deps', async () => {
    await setMetadata(
      md([
        pkg({
          name: 'app',
          dependencies: [
            {
              name: 'serde',
              req: '^1',
              source: 'registry+https://crates.io',
              optional: false,
              uses_default_features: true,
              features: [],
              kind: null,
            },
          ],
        }),
      ])
    );
    const { createDependencies } = await load();

    const ctx: CreateDependenciesContext = {
      workspaceRoot: ws,
      projects: { app: { root: 'crates/app', name: 'app' } } as never,
      externalNodes: {
        'cargo:serde': {
          type: 'cargo' as never,
          name: 'cargo:serde' as never,
          data: { packageName: 'serde', version: '1.0.0' },
        },
      },
      fileMap: { projectFileMap: {}, nonProjectFiles: [] },
      filesToProcess: { projectFileMap: {}, nonProjectFiles: [] },
      nxJsonConfiguration: {},
    };

    const edges = createDependencies({}, ctx);
    expect(edges).toHaveLength(1);
    expect(edges[0]).toMatchObject({ source: 'app', target: 'cargo:serde' });
  });

  it('returns no edges when cargo metadata is unavailable', async () => {
    await setMetadata(null);
    const { createDependencies } = await load();
    const ctx: CreateDependenciesContext = {
      workspaceRoot: ws,
      projects: {},
      externalNodes: {},
      fileMap: { projectFileMap: {}, nonProjectFiles: [] },
      filesToProcess: { projectFileMap: {}, nonProjectFiles: [] },
      nxJsonConfiguration: {},
    };
    expect(createDependencies({}, ctx)).toEqual([]);
  });
});
