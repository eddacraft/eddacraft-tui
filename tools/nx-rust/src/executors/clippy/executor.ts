import type { ExecutorContext } from '@nx/devkit';
import type { BaseCargoOptions } from '../../models/base-options';
import { buildCargoArgs } from '../../utils/build-command';
import { cargoCommand } from '../../utils/cargo';

export interface ClippyExecutorSchema extends BaseCargoOptions {
  'all-targets'?: boolean;
  fix?: boolean;
}

const CLIPPY_KEYS = ['all-targets', 'fix'] as const;

export default async function clippyExecutor(
  options: ClippyExecutorSchema,
  context: ExecutorContext
): Promise<{ success: boolean }> {
  const args = buildCargoArgs('clippy', options, context, CLIPPY_KEYS);
  return cargoCommand(...args);
}
