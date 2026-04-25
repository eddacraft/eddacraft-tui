import type { ExecutorContext } from '@nx/devkit';
import type { BaseCargoOptions } from '../../models/base-options';
import { buildCargoArgs } from '../../utils/build-command';
import { cargoCommand } from '../../utils/cargo';

export interface BuildExecutorSchema extends BaseCargoOptions {
  lib?: boolean;
  bin?: string | string[];
  bins?: boolean;
  example?: string | string[];
  examples?: boolean;
  'all-targets'?: boolean;
}

const BUILD_KEYS = ['lib', 'bin', 'bins', 'example', 'examples', 'all-targets'] as const;

export default async function buildExecutor(
  options: BuildExecutorSchema,
  context: ExecutorContext
): Promise<{ success: boolean }> {
  const args = buildCargoArgs('build', options, context, BUILD_KEYS);
  return cargoCommand(...args);
}
