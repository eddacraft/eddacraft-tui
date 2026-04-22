import type { ExecutorContext } from '@nx/devkit';
import type { BaseCargoOptions } from '../../models/base-options';
import { buildCargoArgs } from '../../utils/build-command';
import { cargoCommand } from '../../utils/cargo';

export interface RunExecutorSchema extends BaseCargoOptions {
  bin?: string;
  example?: string;
}

export default async function runExecutor(
  options: RunExecutorSchema,
  context: ExecutorContext
): Promise<{ success: boolean }> {
  const args = buildCargoArgs('run', options, context);
  return cargoCommand(...args);
}
