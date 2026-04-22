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

export default async function buildExecutor(
  options: BuildExecutorSchema,
  context: ExecutorContext
): Promise<{ success: boolean }> {
  const args = buildCargoArgs('build', options, context);
  return cargoCommand(...args);
}
