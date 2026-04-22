import type { ExecutorContext } from '@nx/devkit';
import type { BaseCargoOptions } from '../../models/base-options';
import { buildCargoArgs } from '../../utils/build-command';
import { cargoCommand } from '../../utils/cargo';

export interface TestExecutorSchema extends BaseCargoOptions {
  doc?: boolean;
  lib?: boolean;
  bin?: string | string[];
  bins?: boolean;
  test?: string | string[];
  tests?: boolean;
  'all-targets'?: boolean;
  'no-run'?: boolean;
  'no-fail-fast'?: boolean;
}

export default async function testExecutor(
  options: TestExecutorSchema,
  context: ExecutorContext
): Promise<{ success: boolean }> {
  const args = buildCargoArgs('test', options, context);
  return cargoCommand(...args);
}
