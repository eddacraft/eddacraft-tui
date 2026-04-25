import {
  addProjectConfiguration,
  formatFiles,
  generateFiles,
  names,
  offsetFromRoot,
  type Tree,
} from '@nx/devkit';
import * as path from 'node:path';
import { addToCargoWorkspace } from '../../utils/add-to-workspace';
import { normalizeOptions } from '../../utils/normalize-options';
import {
  buildTargetConfig,
  checkTargetConfig,
  clippyTargetConfig,
  fmtCheckTargetConfig,
  fmtTargetConfig,
  runTargetConfig,
  testTargetConfig,
} from '../../utils/target-configs';
import initGenerator from '../init/generator';
import type { CrateGeneratorSchema } from './schema';

/**
 * Escape a string for use inside a TOML basic string (`"..."`). Only the
 * four characters TOML forbids in bare basic strings get escaped; we stay
 * ASCII-safe for the common case.
 */
function tomlEscape(value: string): string {
  return value
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"')
    .replace(/\n/g, '\\n')
    .replace(/\r/g, '\\r');
}

export default async function crateGenerator(
  tree: Tree,
  options: CrateGeneratorSchema
): Promise<void> {
  await initGenerator(tree, { skipFormat: true });

  const normalized = normalizeOptions(tree, options);

  const targets = {
    build: buildTargetConfig(),
    check: checkTargetConfig(),
    clippy: clippyTargetConfig(),
    fmt: fmtTargetConfig(),
    'fmt-check': fmtCheckTargetConfig(),
    test: testTargetConfig(),
    ...(options.bin ? { run: runTargetConfig() } : {}),
  };

  addProjectConfiguration(tree, normalized.projectName, {
    root: normalized.projectRoot,
    projectType: options.bin ? 'application' : 'library',
    sourceRoot: `${normalized.projectRoot}/src`,
    tags: normalized.parsedTags,
    targets,
  });

  const templateDir = options.bin
    ? path.join(__dirname, 'files', 'bin')
    : path.join(__dirname, 'files', 'lib');

  generateFiles(tree, templateDir, normalized.projectRoot, {
    ...normalized,
    ...names(normalized.cargoName),
    // TOML-escape the free-form description so a name like `He said "hi"` or
    // a backslash can't produce a corrupt manifest.
    description: normalized.description
      ? tomlEscape(normalized.description)
      : normalized.description,
    offsetFromRoot: offsetFromRoot(normalized.projectRoot),
    template: '',
  });

  addToCargoWorkspace(tree, normalized.projectRoot);

  if (!options.skipFormat) {
    await formatFiles(tree);
  }
}
