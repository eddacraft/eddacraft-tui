import type { ExecutorContext } from '@nx/devkit';
import type { BaseCargoOptions } from '../../models/base-options';
import { buildCargoArgs } from '../../utils/build-command';
import { cargoCommand } from '../../utils/cargo';

export interface ClippyExecutorSchema extends BaseCargoOptions {
  'all-targets'?: boolean;
  fix?: boolean;
}

export default async function clippyExecutor(
  options: ClippyExecutorSchema,
  context: ExecutorContext
): Promise<{ success: boolean }> {
  const args = buildCargoArgs('clippy', options, context);
  return cargoCommand(...args);
}
